//! The Assistant's agent: `hoocode --mode rpc`, started in the note's folder and spoken to in JSON
//! lines. Its output is parsed here into [`Event`]s, which the panel takes with [`Agent::take`]; a
//! reader thread raises the [`Signal`] for each one. Only the standard library and serde_json: no
//! window, no UI toolkit. The protocol is the one of hoocode's `crates/hoocode-code-rpc`.

use std::{
    env,
    ffi::OsStr,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::vault::Signal;
use inkuno_logic::lang::tr;

/// The flags Inkuno passes after the user's words: no saved session, and no tool that writes a
/// file, runs a command or fetches from the web, so the agent can only read and answer.
const FLAGS: [&str; 5] = [
    "--mode",
    "rpc",
    "--no-session",
    "--disallowed-tools",
    "Shell,ShellKill,Write,Edit,WebFetch,WebSearch,Agent",
];

/// The longest record (a line of the agent's output) that is read. A longer one is dropped: a
/// child that prints without a line feed must not grow the memory without limit.
const MAX_LINE: usize = 8 * 1024 * 1024;

/// The end of the agent's stderr, kept for [`Event::Exited`].
const STDERR_TAIL: usize = 2_000;

/// How long the agent has to exit once its stdin is closed, before it is killed.
const GRACE: Duration = Duration::from_secs(1);

/// What the agent's output means to the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A prompt response with `success: false`: the error text.
    Refused(String),
    /// `agent_start`: the agent is working.
    Started,
    /// A `text_delta` of the assistant's answer.
    Text(String),
    /// `tool_execution_start`: the name of the tool it runs.
    Tool(String),
    /// The end of one assistant message that did not fail: what follows is another message.
    Break,
    /// An assistant `message_end` with stop reason `error`: its error message.
    Failed(String),
    /// `agent_end`: the run is over and the agent is idle.
    Ended,
    /// The process ended or its output could not be read: the end of its stderr, trimmed.
    Exited(String),
}

/// One stdout line of the agent as an [`Event`]. Accepted responses, thinking, tool call
/// deltas, the other event types and text that is not JSON give `None`.
pub fn parse(line: &str) -> Option<Event> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value["type"].as_str()? {
        "response" => {
            // Only a refused prompt is news; an accepted one is silent, and so is a line the
            // agent could not parse.
            if value["command"] != "prompt" || value["success"] != false {
                return None;
            }
            Some(Event::Refused(text(
                &value["error"],
                tr(
                    "The agent refused the question",
                    "L'agent a refusé la question",
                ),
            )))
        }
        "agent_start" => Some(Event::Started),
        "agent_end" => Some(Event::Ended),
        "tool_execution_start" => Some(Event::Tool(value["toolName"].as_str()?.to_string())),
        "message_update" => {
            let event = &value["assistantMessageEvent"];
            if event["type"] != "text_delta" {
                return None;
            }
            Some(Event::Text(event["delta"].as_str()?.to_string()))
        }
        "message_end" => {
            let message = &value["message"];
            if message["role"] != "assistant" {
                return None;
            }
            if message["stopReason"] != "error" {
                return Some(Event::Break);
            }
            Some(Event::Failed(text(
                &message["errorMessage"],
                tr(
                    "The agent stopped with an error",
                    "L'agent s'est arrêté sur une erreur",
                ),
            )))
        }
        _ => None,
    }
}

/// The text of a JSON string, trimmed, or `default` when it is missing or empty.
fn text(value: &Value, default: &str) -> String {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(default)
        .to_string()
}

/// The program of `command`: its first whitespace-separated word.
fn program(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or("")
}

/// Where the program of `command` is, as an absolute path: the path that the child is started
/// with. A path with a folder part is taken as it is when it is a file (a relative one from the
/// current folder); a bare name is looked up in each folder of `PATH`, where an empty entry is
/// skipped. `None` when it is not found.
pub fn find(command: &str) -> Option<PathBuf> {
    find_in(command, env::var_os("PATH").as_deref())
}

/// [`find`] with the `PATH` given, so that a test does not change the process environment.
fn find_in(command: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    let program = program(command);
    if program.is_empty() {
        return None;
    }
    if Path::new(program)
        .parent()
        .is_some_and(|dir| !dir.as_os_str().is_empty())
    {
        let file = absolute(PathBuf::from(program))?;
        return file.is_file().then_some(file);
    }
    // Windows also runs `name.exe` for `name`.
    let mut names = vec![program.to_string()];
    if cfg!(windows) && !program.to_ascii_lowercase().ends_with(".exe") {
        names.push(format!("{program}.exe"));
    }
    env::split_paths(path?)
        .filter(|dir| !dir.as_os_str().is_empty())
        .filter_map(absolute)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// `path` as an absolute path: a relative one is joined with the current folder. `None` when that
/// folder cannot be read.
fn absolute(path: PathBuf) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path)
    } else {
        env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

/// The arguments after the program of `command`: the user's own words, then [`FLAGS`].
pub fn arguments(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .chain(FLAGS.iter().map(|flag| flag.to_string()))
        .collect()
}

/// A running agent. Dropping it stops the agent.
pub struct Agent {
    child: Option<Child>,
    /// Its stdin; `None` once closed by [`Agent::stop`].
    stdin: Mutex<Option<ChildStdin>>,
    events: Arc<Mutex<Vec<Event>>>,
    /// The number of commands sent, for the ids `p<n>` and `a<n>`.
    sent: AtomicU64,
    /// Under test, an agent with no process keeps the lines sent to it here.
    #[cfg(any(test, feature = "test-support"))]
    written: Option<Mutex<Vec<String>>>,
}

impl Agent {
    /// An agent with no process, for the panel's tests: what is sent to it is kept (see
    /// [`Agent::written`]), and no event ever comes.
    #[cfg(any(test, feature = "test-support"))]
    pub fn detached() -> Agent {
        Agent {
            child: None,
            stdin: Mutex::new(None),
            events: Arc::new(Mutex::new(Vec::new())),
            sent: AtomicU64::new(0),
            written: Some(Mutex::new(Vec::new())),
        }
    }

    /// The lines sent to a detached agent, in order.
    #[cfg(any(test, feature = "test-support"))]
    pub fn written(&self) -> Vec<String> {
        self.written
            .as_ref()
            .map(|lines| lock(lines).clone())
            .unwrap_or_default()
    }

    /// Starts the agent `command` in `folder`. The error is a sentence for the panel.
    pub fn start(command: &str, folder: &Path, signal: Arc<Signal>) -> Result<Agent, String> {
        let word = program(command);
        if word.is_empty() {
            return Err(tr(
                "No agent command is set",
                "Aucune commande d'agent n'est réglée",
            )
            .into());
        }
        let path = find(command)
            .ok_or_else(|| format!("`{word}` {}", tr("was not found", "est introuvable")))?;
        let mut child = super::command(&path)
            .args(arguments(command))
            .current_dir(folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "`{word}` {}: {e}",
                    tr("could not start", "n'a pas pu démarrer")
                )
            })?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            // All three are piped above: this cannot happen, but the child is not left behind.
            let _ = child.kill();
            return Err(tr("the agent has no pipes", "l'agent n'a pas de tubes").into());
        };
        let tail = Arc::new(Mutex::new(Vec::new()));
        let errors = {
            let tail = tail.clone();
            thread::Builder::new()
                .name("agent errors".into())
                .spawn(move || keep_tail(stderr, &tail))
                .map_err(|e| e.to_string())?
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        {
            let events = events.clone();
            thread::Builder::new()
                .name("agent output".into())
                .spawn(move || read_output(stdout, errors, &tail, &events, &signal))
                .map_err(|e| e.to_string())?;
        }
        Ok(Agent {
            child: Some(child),
            stdin: Mutex::new(Some(stdin)),
            events,
            sent: AtomicU64::new(0),
            #[cfg(any(test, feature = "test-support"))]
            written: None,
        })
    }

    /// Asks `text` as a prompt.
    pub fn prompt(&self, text: &str) -> Result<(), String> {
        let id = self.next_id('p');
        self.send(json!({"id": id, "type": "prompt", "message": text}))
    }

    /// Stops the turn running now; the agent then reports its end.
    pub fn abort(&self) -> Result<(), String> {
        let id = self.next_id('a');
        self.send(json!({"id": id, "type": "abort"}))
    }

    /// The events collected since the last call, in order.
    pub fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *lock(&self.events))
    }

    /// Closes the agent's stdin, which makes it exit, and kills it if it is still running after
    /// a moment. Returns at once: the wait is on a helper thread. Calling it again does nothing.
    pub fn stop(&mut self) {
        lock(&self.stdin).take();
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = thread::Builder::new()
            .name("agent stop".into())
            .spawn(move || {
                let deadline = Instant::now() + GRACE;
                loop {
                    match child.try_wait() {
                        Ok(None) if Instant::now() < deadline => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Ok(Some(_)) => return,
                        _ => {
                            let _ = child.kill();
                            let _ = child.wait();
                            return;
                        }
                    }
                }
            });
    }

    fn next_id(&self, kind: char) -> String {
        format!("{kind}{}", self.sent.fetch_add(1, Ordering::Relaxed) + 1)
    }

    /// Writes one JSON line to the agent.
    fn send(&self, line: Value) -> Result<(), String> {
        let mut bytes = line.to_string();
        bytes.push('\n');
        #[cfg(any(test, feature = "test-support"))]
        if let Some(lines) = &self.written {
            lock(lines).push(bytes);
            return Ok(());
        }
        let mut guard = lock(&self.stdin);
        let stdin = guard
            .as_mut()
            .ok_or_else(|| tr("The agent has stopped", "L'agent s'est arrêté").to_string())?;
        stdin
            .write_all(bytes.as_bytes())
            .and_then(|()| stdin.flush())
            .map_err(|e| {
                format!(
                    "{}: {e}",
                    tr(
                        "The agent did not take the message",
                        "L'agent n'a pas reçu le message"
                    )
                )
            })
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Reads stdout line by line, pushes each event and raises the signal. At the end, it waits for
/// the stderr reader and pushes [`Event::Exited`] with the end of stderr.
fn read_output(
    stdout: ChildStdout,
    errors: JoinHandle<()>,
    tail: &Mutex<Vec<u8>>,
    events: &Mutex<Vec<Event>>,
    signal: &Signal,
) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        match read_record(&mut reader, &mut line, MAX_LINE) {
            Ok(Record::End) => break,
            Ok(Record::Line) => {
                if let Some(event) = parse(&String::from_utf8_lossy(&line)) {
                    lock(events).push(event);
                    signal.raise();
                }
            }
            // Too long to be an event of the protocol: dropped, and the next line is read.
            Ok(Record::TooLong) => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    // The stderr of an exited process ends too: its last words are complete once it is joined.
    let _ = errors.join();
    let exit = String::from_utf8_lossy(&lock(tail)).trim().to_string();
    lock(events).push(Event::Exited(exit));
    signal.raise();
}

/// What one [`read_record`] gave.
#[derive(Debug, PartialEq, Eq)]
enum Record {
    /// The end of the input: no record is left.
    End,
    /// A record, with its line feed when it has one; it is in the buffer.
    Line,
    /// A record longer than the cap: its bytes were dropped up to its line feed.
    TooLong,
}

/// Reads one record (a line, through its line feed) into `line`. At most `cap` bytes of the
/// record are kept; a longer one is read to its line feed and dropped, so that the memory used
/// does not depend on what the child prints.
fn read_record<R: BufRead>(reader: &mut R, line: &mut Vec<u8>, cap: usize) -> io::Result<Record> {
    line.clear();
    let (mut any, mut too_long) = (false, false);
    loop {
        let (used, found) = {
            let buf = match reader.fill_buf() {
                Ok(buf) => buf,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            if buf.is_empty() {
                break;
            }
            any = true;
            let (used, found) = match buf.iter().position(|&b| b == b'\n') {
                Some(at) => (at + 1, true),
                None => (buf.len(), false),
            };
            let content = if found { used - 1 } else { used };
            if !too_long {
                if line.len() + content > cap {
                    too_long = true;
                    line.clear();
                } else {
                    line.extend_from_slice(&buf[..used]);
                }
            }
            (used, found)
        };
        reader.consume(used);
        if found {
            break;
        }
    }
    Ok(match (any, too_long) {
        (false, _) => Record::End,
        (true, true) => Record::TooLong,
        (true, false) => Record::Line,
    })
}

/// Reads stderr to its end, keeping only its last [`STDERR_TAIL`] bytes.
fn keep_tail(mut stderr: ChildStderr, tail: &Mutex<Vec<u8>>) {
    let mut chunk = [0u8; 1024];
    loop {
        match stderr.read(&mut chunk) {
            Ok(0) => return,
            Ok(n) => {
                let mut kept = lock(tail);
                kept.extend_from_slice(&chunk[..n]);
                let excess = kept.len().saturating_sub(STDERR_TAIL);
                kept.drain(..excess);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

/// A lock that a panic on another thread cannot make a panic here.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_reads_the_events_of_the_panel() {
        // Shapes as hoocode writes them (crates/hoocode-code-rpc, its snapshot replay__rpc-basic).
        assert_eq!(parse(r#"{"type":"agent_start"}"#), Some(Event::Started));
        assert_eq!(
            parse(r#"{"type":"agent_end","messages":[]}"#),
            Some(Event::Ended)
        );
        assert_eq!(
            parse(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"Let me r","partial":{"role":"assistant","content":[]}},"message":{"role":"assistant"}}"#
            ),
            Some(Event::Text("Let me r".into()))
        );
        assert_eq!(
            parse(
                r#"{"type":"tool_execution_start","toolCallId":"call_read_1","toolName":"Read","args":{"path":"notes.txt","limit":2}}"#
            ),
            Some(Event::Tool("Read".into()))
        );
        assert_eq!(
            parse(
                r#"{"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"error","errorMessage":"Rate limit exceeded","timestamp":0}}"#
            ),
            Some(Event::Failed("Rate limit exceeded".into()))
        );
        assert_eq!(
            parse(
                r#"{"id":"p1","type":"response","command":"prompt","success":false,"error":"No API key found for anthropic. Set it first."}"#
            ),
            Some(Event::Refused(
                "No API key found for anthropic. Set it first.".into()
            ))
        );
    }

    #[test]
    fn parse_gives_a_sentence_when_the_error_has_no_text() {
        assert!(matches!(
            parse(r#"{"type":"message_end","message":{"role":"assistant","stopReason":"error"}}"#),
            Some(Event::Failed(text)) if !text.is_empty()
        ));
        assert!(matches!(
            parse(r#"{"id":"p2","type":"response","command":"prompt","success":false}"#),
            Some(Event::Refused(text)) if !text.is_empty()
        ));
    }

    #[test]
    fn parse_ignores_what_the_panel_does_not_show() {
        // An accepted prompt, the parse error of a line the agent could not read.
        assert_eq!(
            parse(r#"{"id":"p1","type":"response","command":"prompt","success":true}"#),
            None
        );
        assert_eq!(
            parse(
                r#"{"type":"response","command":"parse","success":false,"error":"Failed to parse command: x"}"#
            ),
            None
        );
        assert_eq!(parse(r#"{"type":"turn_start"}"#), None);
        assert_eq!(parse(r#"{"type":"queue_update","steering":[]}"#), None);
        assert_eq!(
            parse(r#"{"type":"auto_retry_start","attempt":1,"maxAttempts":3}"#),
            None
        );
        // Thinking and tool call deltas, the tool's result, the user's message.
        assert_eq!(
            parse(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","contentIndex":0,"delta":"hmm"},"message":{}}"#
            ),
            None
        );
        assert_eq!(
            parse(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"toolcall_delta","contentIndex":1,"delta":"{\"path\":"},"message":{}}"#
            ),
            None
        );
        assert_eq!(
            parse(
                r#"{"type":"tool_execution_end","toolCallId":"c","toolName":"Read","isError":false}"#
            ),
            None
        );
        assert_eq!(
            parse(r#"{"type":"message_end","message":{"role":"user","content":[]}}"#),
            None
        );
        assert_eq!(parse("Starting hoocode..."), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn the_end_of_an_assistant_message_is_a_break_unless_it_failed() {
        // A message that ends with a tool call, or with its stop: the next one is another message.
        assert_eq!(
            parse(
                r#"{"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"toolUse"}}"#
            ),
            Some(Event::Break)
        );
        assert_eq!(
            parse(r#"{"type":"message_end","message":{"role":"assistant","stopReason":"stop"}}"#),
            Some(Event::Break)
        );
        // A user's message is not the assistant's: nothing for the panel.
        assert_eq!(
            parse(r#"{"type":"message_end","message":{"role":"user","stopReason":"stop"}}"#),
            None
        );
    }

    #[test]
    fn read_record_keeps_a_line_up_to_the_cap_and_drops_a_longer_one() {
        let input: &[u8] = b"ab\ncdefgh\nabcd\nabcde\nij\nxyz";
        let mut reader = io::Cursor::new(input);
        let mut line = Vec::new();
        let mut seen = Vec::new();
        loop {
            let record = read_record(&mut reader, &mut line, 4).unwrap();
            let content = line.iter().filter(|b| **b != b'\n').count();
            assert!(content <= 4, "the cap holds: {line:?}");
            let done = record == Record::End;
            seen.push((record, line.clone()));
            if done {
                break;
            }
        }
        assert_eq!(
            seen,
            [
                (Record::Line, b"ab\n".to_vec()),
                // Six bytes: over the cap, dropped up to its line feed.
                (Record::TooLong, Vec::new()),
                // Exactly the cap (its line feed is not counted): kept.
                (Record::Line, b"abcd\n".to_vec()),
                (Record::TooLong, Vec::new()),
                (Record::Line, b"ij\n".to_vec()),
                // The last record has no line feed: it is kept as it is.
                (Record::Line, b"xyz".to_vec()),
                (Record::End, Vec::new()),
            ]
        );
    }

    #[test]
    fn read_record_drops_an_endless_line_without_growing() {
        // Without a line feed, the rest of the input is dropped while the cap is kept.
        let mut input = io::repeat(b'x').take(100_000);
        let mut reader = BufReader::new(&mut input);
        let mut line = Vec::new();
        assert_eq!(
            read_record(&mut reader, &mut line, 1_000).unwrap(),
            Record::TooLong
        );
        assert!(line.is_empty());
        assert_eq!(
            read_record(&mut reader, &mut line, 1_000).unwrap(),
            Record::End
        );
    }

    #[test]
    fn find_takes_the_first_folder_of_path_that_holds_the_program() {
        let base = env::temp_dir().join(format!("inkuno-assistant-find-{}", std::process::id()));
        let (first, second) = (base.join("first"), base.join("second"));
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(second.join("fakeagent"), "").unwrap();
        let path = env::join_paths([&first, &second]).unwrap();

        assert_eq!(
            find_in("fakeagent --x", Some(path.as_os_str())),
            Some(second.join("fakeagent"))
        );
        assert_eq!(find_in("missingagent", Some(path.as_os_str())), None);
        assert_eq!(find_in("fakeagent", None), None);
        assert_eq!(find_in("", Some(path.as_os_str())), None);

        // A path with a folder part is taken as it is, when it is a file.
        let file = second.join("fakeagent");
        assert_eq!(find_in(&file.to_string_lossy(), None), Some(file.clone()));
        assert_eq!(
            find_in(&second.join("missing").to_string_lossy(), None),
            None
        );
        assert_eq!(
            find_in(&second.to_string_lossy(), None),
            None,
            "a folder is not a program"
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn find_gives_the_path_the_child_is_started_with() {
        // A relative path with a folder part is found from the current folder, and given as an
        // absolute path: the child runs in another folder, so a relative one would not match.
        let relative = find_in("src/lib.rs --x", None).expect("a file of this crate");
        assert!(relative.is_absolute(), "{relative:?}");
        assert_eq!(relative, env::current_dir().unwrap().join("src/lib.rs"));

        // An empty entry of PATH is skipped: it is not the current folder. `Cargo.toml` is in the
        // current folder, and is not in the folder of the entry.
        let base = env::temp_dir().join(format!(
            "inkuno-assistant-find-empty-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).unwrap();
        let path = env::join_paths([PathBuf::new(), base.clone()]).unwrap();
        assert_eq!(find_in("Cargo.toml", Some(path.as_os_str())), None);
        let agent = base.join("fakeagent");
        fs::write(&agent, "").unwrap();
        assert_eq!(
            find_in("fakeagent", Some(path.as_os_str())),
            Some(agent.clone())
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn arguments_are_the_user_words_then_the_fixed_flags() {
        assert_eq!(
            arguments("hoocode --model fast"),
            [
                "--model",
                "fast",
                "--mode",
                "rpc",
                "--no-session",
                "--disallowed-tools",
                "Shell,ShellKill,Write,Edit,WebFetch,WebSearch,Agent",
            ]
        );
        assert_eq!(
            arguments("hoocode"),
            [
                "--mode",
                "rpc",
                "--no-session",
                "--disallowed-tools",
                "Shell,ShellKill,Write,Edit,WebFetch,WebSearch,Agent"
            ]
        );
    }

    #[test]
    fn a_missing_program_does_not_start() {
        let folder = env::temp_dir();
        let missing = Agent::start(
            "inkuno-no-such-agent --x",
            &folder,
            Arc::new(Signal::default()),
        );
        match missing {
            Err(message) => assert!(message.contains("inkuno-no-such-agent"), "{message}"),
            Ok(_) => panic!("a missing program started"),
        }
        assert!(Agent::start("   ", &folder, Arc::new(Signal::default())).is_err());
    }

    /// An executable `sh` script with `body` after its shebang, in a folder of its own. The
    /// command runs it through `sh`: a script written a moment before cannot be run at once
    /// on Linux (text file busy), and the test does not depend on the mode of the file.
    #[cfg(unix)]
    fn fake_agent(name: &str, body: &str) -> (PathBuf, String) {
        let dir = env::temp_dir().join(format!("inkuno-assistant-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("agent.sh");
        fs::write(&script, format!("#!/bin/sh\n{body}")).unwrap();
        let command = format!("sh {}", script.display());
        (dir, command)
    }

    /// Takes the agent's events until `done` holds for them, or fails after ten seconds.
    #[cfg(unix)]
    fn wait_until(agent: &Agent, events: &mut Vec<Event>, done: impl Fn(&[Event]) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(events.as_slice()) {
            assert!(Instant::now() < deadline, "no answer in time: {events:?}");
            events.extend(agent.take());
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[cfg(unix)]
    fn a_prompt_gets_its_answer_and_stop_ends_the_agent() {
        let (dir, command) = fake_agent(
            "chat",
            r#"while IFS= read -r line; do
  case "$line" in
    *'"type":"prompt"'*)
      echo '{"id":"p1","type":"response","command":"prompt","success":true}'
      echo '{"type":"agent_start"}'
      echo '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"Hello ","partial":{}},"message":{}}'
      echo '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"world","partial":{}},"message":{}}'
      echo '{"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"stop"}}'
      echo '{"type":"agent_end","messages":[]}'
      ;;
    *'"type":"abort"'*)
      echo '{"type":"agent_end","messages":[]}'
      ;;
  esac
done
"#,
        );
        let mut agent = Agent::start(&command, &dir, Arc::new(Signal::default())).unwrap();
        let mut events = Vec::new();

        // A quote and a line break in the prompt stay inside one JSON line.
        agent.prompt("say \"hi\"\nnow").unwrap();
        wait_until(&agent, &mut events, |e| e.contains(&Event::Ended));

        agent.abort().unwrap();
        wait_until(&agent, &mut events, |e| {
            e.iter().filter(|x| **x == Event::Ended).count() == 2
        });

        agent.stop();
        agent.stop(); // a second stop does nothing
        wait_until(&agent, &mut events, |e| {
            e.iter().any(|x| matches!(x, Event::Exited(_)))
        });
        assert_eq!(
            events,
            [
                Event::Started,
                Event::Text("Hello ".into()),
                Event::Text("world".into()),
                Event::Break,
                Event::Ended,
                Event::Ended,
                Event::Exited(String::new()),
            ]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn an_agent_that_exits_at_once_reports_its_stderr() {
        let (dir, command) = fake_agent("exit", "echo 'hoocode: no API key found' >&2\nexit 3\n");
        let mut agent = Agent::start(&command, &dir, Arc::new(Signal::default())).unwrap();
        let mut events = Vec::new();
        wait_until(&agent, &mut events, |e| {
            e.iter().any(|x| matches!(x, Event::Exited(_)))
        });
        assert_eq!(events, [Event::Exited("hoocode: no API key found".into())]);
        agent.stop();
        fs::remove_dir_all(&dir).ok();
    }
}
