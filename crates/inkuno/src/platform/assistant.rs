//! The Assistant's agent: `hoocode --mode rpc`, started in the note's folder and spoken to in JSON
//! lines. Its output is parsed here into [`Event`]s, which the panel takes with [`Agent::take`]; a
//! reader thread raises the [`Signal`] for each one. Only the standard library and serde_json: no
//! window, no gpui. The protocol is the one of hoocode's `crates/hoocode-code-rpc`.

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
use crate::lang::tr;

/// The flags Inkuno passes after the user's words: no saved session, and no tool that writes a
/// file or runs a command, so the agent can only read and answer.
const FLAGS: [&str; 5] = [
    "--mode",
    "rpc",
    "--no-session",
    "--disallowed-tools",
    "Shell,ShellKill,Write,Edit",
];

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
            if message["role"] != "assistant" || message["stopReason"] != "error" {
                return None;
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

/// Where the program of `command` is: a path with a folder part is taken as it is when it is a
/// file; a bare name is looked up in each folder of `PATH`. `None` when it is not found.
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
        let file = PathBuf::from(program);
        return file.is_file().then_some(file);
    }
    // Windows also runs `name.exe` for `name`.
    let mut names = vec![program.to_string()];
    if cfg!(windows) && !program.to_ascii_lowercase().ends_with(".exe") {
        names.push(format!("{program}.exe"));
    }
    env::split_paths(path?)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
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
}

impl Agent {
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
        let mut guard = lock(&self.stdin);
        let stdin = guard
            .as_mut()
            .ok_or_else(|| tr("The agent has stopped", "L'agent s'est arrêté").to_string())?;
        let mut bytes = line.to_string();
        bytes.push('\n');
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
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {
                if let Some(event) = parse(&String::from_utf8_lossy(&line)) {
                    lock(events).push(event);
                    signal.raise();
                }
            }
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
        // A stop that is not an error is not a failure.
        assert_eq!(
            parse(r#"{"type":"message_end","message":{"role":"assistant","stopReason":"stop"}}"#),
            None
        );
        assert_eq!(parse("Starting hoocode..."), None);
        assert_eq!(parse(""), None);
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
                "Shell,ShellKill,Write,Edit",
            ]
        );
        assert_eq!(
            arguments("hoocode"),
            [
                "--mode",
                "rpc",
                "--no-session",
                "--disallowed-tools",
                "Shell,ShellKill,Write,Edit"
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
