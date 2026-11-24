//! The assistant: a panel of the right dock that talks to the agent about the open note, and the
//! round button at the note's foot that opens it. The agent is `hoocode` in its RPC mode, started
//! without a shell or a write tool: it reads the note and answers. A review's answer is a list of
//! findings, and Inkuno writes them into the note as comments (one undo step). This file holds the
//! state, the panel and the process side (the child process is `platform::assistant`).

use super::*;
use crate::logic::{assistant as words, pending};
use crate::platform::assistant::{Agent, Event};
use crate::platform::revisions::{self as history, Home};
use crate::platform::vault::Signal;
use crate::ui::line::Line;
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

/// How long a stopped agent has to report its end before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(3);

/// Starts the agent process. In a test there is no process: a detached agent stands in for it.
#[cfg(not(test))]
fn start_agent(command: &str, folder: &Path, signal: Arc<Signal>) -> Result<Agent, String> {
    Agent::start(command, folder, signal)
}

#[cfg(test)]
fn start_agent(_command: &str, _folder: &Path, _signal: Arc<Signal>) -> Result<Agent, String> {
    Ok(Agent::detached())
}

/// What the agent is doing, as the button and the panel show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// The command is not installed: nothing can be asked.
    Missing,
    Idle,
    Working,
    /// An answer, a finished review or an error arrived while the panel was not shown.
    NeedsYou,
}

/// Who a line of the transcript is from. `Note` is a quiet system line: errors, tool activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Who {
    You,
    Agent,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub who: Who,
    pub text: String,
}

/// A review in progress: the note under review, and the answer collected so far. The answer is
/// not shown; it becomes comments when the agent ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Review {
    pub path: PathBuf,
    pub answer: String,
}

/// An improvement in progress: the note, the text it had when asked (`base`), the passage asked
/// about (`None`: the whole note), and the answer collected so far. The answer is not shown; it
/// becomes pending changes when the agent ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Improve {
    pub path: PathBuf,
    pub base: String,
    pub range: Option<Range<usize>>,
    pub answer: String,
}

/// The job the agent runs for the panel. One at a time: the panel is not free while it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Job {
    Review(Review),
    Improve(Improve),
}

impl Job {
    /// The answer collected so far, to add to.
    fn answer(&mut self) -> &mut String {
        match self {
            Job::Review(review) => &mut review.answer,
            Job::Improve(improve) => &mut improve.answer,
        }
    }
}

pub(crate) struct Assistant {
    pub status: Status,
    pub entries: Vec<Entry>,
    /// The prompt box, at the foot of the panel.
    pub field: Entity<field::Field>,
    pub scroll: gpui::ScrollHandle,
    /// The running agent, started on the first prompt in `folder`.
    pub agent: Option<Agent>,
    /// Raised by the agent's reader thread for each event; `Shell::assistant_listen` waits on it.
    pub signal: Arc<Signal>,
    /// The folder the running agent was started in.
    pub folder: Option<PathBuf>,
    /// The job the agent runs, if any: its answer is collected, not shown.
    pub job: Option<Job>,
    /// Stop was asked: what the agent still says before it ends is dropped.
    pub stopping: bool,
    /// Counts the stops asked: a timer of an older stop does nothing.
    pub stop_gen: u64,
    /// The agent ended a message of its own: the next text of the chat starts a new paragraph.
    pub paragraph: bool,
    /// The prompts sent to the agent, for the tests.
    #[cfg(test)]
    pub sent: Vec<String>,
    /// Whether the command counts as installed: the tests do not read the machine's PATH.
    #[cfg(test)]
    pub found: bool,
    /// The next start of the agent fails with this sentence (the tests).
    #[cfg(test)]
    pub refuse_start: Option<String>,
}

impl Assistant {
    /// The prompt box listens to its keys: Enter sends, Escape gives the focus back to the note.
    pub(super) fn new(theme: Theme, window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let field = cx.new(|cx| field::Field::new("", theme, cx));
        cx.subscribe_in(
            &field,
            window,
            |this, _, event: &field::FieldEvent, window, cx| match event {
                field::FieldEvent::Done => {
                    let text = this.assistant.field.read(cx).line.text.trim().to_string();
                    if !text.is_empty() && this.assistant_free() {
                        this.assistant_prompt(text, window, cx);
                    }
                }
                field::FieldEvent::Cancel => {
                    window.focus(&this.pane().editor.focus_handle(cx), cx);
                }
                field::FieldEvent::Changed => cx.notify(),
            },
        )
        .detach();
        Self {
            status: Status::Idle,
            entries: Vec::new(),
            field,
            scroll: gpui::ScrollHandle::new(),
            agent: None,
            signal: Arc::new(Signal::default()),
            folder: None,
            job: None,
            stopping: false,
            stop_gen: 0,
            paragraph: false,
            #[cfg(test)]
            sent: Vec::new(),
            #[cfg(test)]
            found: true,
            #[cfg(test)]
            refuse_start: None,
        }
    }
}

/// A text button of the panel. Dimmed when `enabled` is false; its click is the caller's.
fn assistant_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    t: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .h(t.px(26.))
        .px_2()
        .rounded(t.small())
        .flex()
        .items_center()
        .text_size(t.px(13.))
        .text_color(if enabled { t.accent } else { t.dim })
        .when(enabled, |d| d.cursor_pointer().hover(|s| s.bg(t.hover)))
        .child(label)
}

/// Whether an agent's refusal is a login problem: the hint of the login is added after it.
fn login_problem(error: &str) -> bool {
    ["No model selected", "No API key", "Authentication failed"]
        .iter()
        .any(|text| error.contains(text))
}

/// The sentence that ends a review: what was written, what could not be placed.
fn review_summary(placed: usize, skipped: usize) -> String {
    let written = match placed {
        0 => String::new(),
        1 => tr("Wrote 1 comment.", "1 commentaire écrit.").to_string(),
        n => format!("Wrote {n} comments."),
    };
    let missed = match skipped {
        0 => String::new(),
        1 => tr(
            "1 finding could not be placed.",
            "1 remarque n'a pas pu être placée.",
        )
        .to_string(),
        n => format!("{n} findings could not be placed."),
    };
    [written, missed]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Saves an automatic revision of `text` for the note at `path`, as `inkuno write` does around a
/// write. Nothing is saved when the text is the latest revision's already. The error names the note.
/// The versions panel's restore saves its two around its proposal the same way.
pub(super) fn auto_revision(
    home: &Home,
    path: &Path,
    author: &str,
    memo: &str,
    text: &str,
) -> Result<(), String> {
    history::save_auto(home, path, author, memo, text)
        .map(|_| ())
        .map_err(|e| format!("{}: {}", path.display(), crate::lang::io_reason(&e)))
}

/// The line that says why a proposal was not written, in the person's words.
pub(super) fn refusal_words(refusal: pending::Refusal) -> &'static str {
    match refusal {
        pending::Refusal::NothingChanged => tr(
            "The assistant proposed no change.",
            "L'assistant n'a proposé aucun changement.",
        ),
        pending::Refusal::AlreadyPending => tr(
            "Accept or reject the pending changes first.",
            "Acceptez ou refusez d'abord les changements en attente.",
        ),
        pending::Refusal::HoldsMarks => tr(
            "The text holds pending-change marks, so nothing was proposed.",
            "Le texte contient des marques de changement, rien n'a été proposé.",
        ),
        pending::Refusal::HumanChanged { .. } => tr(
            "You changed the same lines meanwhile, so nothing was proposed. Ask again.",
            "Vous avez modifié les mêmes lignes entre-temps, rien n'a été proposé. Redemandez.",
        ),
    }
}

/// The line that counts the pending changes of the note after a proposal.
pub(super) fn pending_words(count: usize) -> String {
    match count {
        1 => tr(
            "1 change is pending in the note: accept or reject it there.",
            "1 changement est en attente dans la note : acceptez-le ou refusez-le là.",
        )
        .to_string(),
        n => format!(
            "{n} {}",
            tr(
                "changes are pending in the note: accept or reject them there.",
                "changements sont en attente dans la note : acceptez-les ou refusez-les là.",
            )
        ),
    }
}

impl Shell {
    /// Opens the assistant view of the right panel, or hides the panel when it is shown. Opening it gives
    /// the prompt box the keys. An answer that waited for the panel is seen now: the status goes back to Idle.
    pub(super) fn toggle_assistant(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.docked(Mode::Assistant) {
            return self.hide_tool(Mode::Assistant, window, cx);
        }
        if self.assistant.status == Status::NeedsYou {
            self.assistant.status = Status::Idle;
        }
        self.show_view(Mode::Assistant, window, cx);
    }

    /// The agent is free to take a prompt: not working, and installed.
    fn assistant_free(&self) -> bool {
        !matches!(self.assistant.status, Status::Working | Status::Missing)
    }

    /// A note is open in the active tab (not a file, not a list): it can be reviewed.
    fn review_open(&self) -> bool {
        self.pane().path.is_some() && self.pane().file.is_none() && self.pane().listing.is_none()
    }

    /// The panel in the right dock: the transcript, the review and stop buttons, and the prompt box.
    pub(super) fn render_assistant(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let t = self.theme;
        let status = self.assistant.status;
        let command = self.prefs.assistant_command().to_string();
        let entries = self
            .assistant
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let row = div()
                    .id(("assistant-entry", i))
                    .debug_selector(move || format!("assistant-entry-{i}"))
                    .w_full();
                match entry.who {
                    Who::You => row.text_size(t.px(13.)).child(
                        div()
                            .px_2()
                            .py_1p5()
                            .rounded(t.large())
                            .bg(t.hover)
                            .text_color(t.text)
                            .child(entry.text.clone()),
                    ),
                    Who::Agent => row
                        .text_size(t.px(13.))
                        .text_color(t.text)
                        .child(entry.text.clone()),
                    Who::Note => row
                        .text_size(t.px(12.))
                        .text_color(t.soft)
                        .child(entry.text.clone()),
                }
            })
            .collect::<Vec<_>>();
        let empty = (self.assistant.entries.is_empty()).then(|| {
            let said = match status {
                // A plain text line: the command's name, not Markdown, so no backticks.
                Status::Missing => format!(
                    "{command} {}",
                    tr(
                        "is not installed or not on the PATH. Set another command with “assistant command…” in the palette.",
                        "n'est pas installé ou pas sur le PATH. Choisissez une autre commande avec « commande de l'assistant… » dans la palette."
                    )
                ),
                _ => tr(
                    "Ask about this note, or have it reviewed. Findings are written as comments.",
                    "Posez une question sur cette note, ou faites-la relire. Les remarques sont écrites en commentaires.",
                )
                .to_string(),
            };
            div()
                .id("assistant-empty")
                .debug_selector(|| "assistant-empty".into())
                .text_size(t.px(13.))
                .text_color(t.soft)
                .child(said)
        });
        let transcript = div()
            .id("assistant-transcript")
            .debug_selector(|| "assistant-transcript".into())
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.assistant.scroll)
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .children(entries)
            .children(empty);
        let review_enabled = self.review_open() && self.assistant_free();
        // The improve button names what it will improve: the selection, else the whole note.
        let selected = !self.pane().editor.read(cx).span().is_empty();
        let improve_label = match selected {
            true => tr("Improve selection", "Améliorer la sélection"),
            false => tr("Improve this note", "Améliorer cette note"),
        };
        // The buttons wrap when the panel is narrow, so that Stop stays inside it.
        let actions = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .px_3()
            .pb_2()
            .child(
                assistant_button(
                    "assistant-review",
                    tr("Review this note", "Relire cette note"),
                    review_enabled,
                    t,
                )
                .on_click(cx.listener(|this, _, _, cx| this.assistant_review(cx))),
            )
            .child(
                assistant_button("assistant-improve", improve_label, review_enabled, t)
                    .on_click(cx.listener(|this, _, _, cx| this.assistant_improve(cx))),
            )
            .children((status == Status::Working).then(|| {
                assistant_button("assistant-stop", tr("Stop", "Arrêter"), true, t)
                    .on_click(cx.listener(|this, _, _, cx| this.assistant_stop(cx)))
            }));
        // The prompt box: a frame that grows with its text (the field wraps, it does not scroll).
        let field = self.assistant.field.clone();
        let focused = field.focus_handle(cx).is_focused(window);
        let blank = field.read(cx).line.text.is_empty();
        let prompt = div().p_3().pt_0().child(
            div()
                .debug_selector(|| "assistant-prompt".into())
                .min_h(t.px(26.))
                .px_2()
                .py_1()
                .rounded(t.small())
                .border_1()
                .bg(t.bg)
                .border_color(if focused { t.accent } else { t.border })
                .relative()
                .child(field)
                .when(blank, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left_2()
                            .top_1()
                            .text_size(t.px(13.))
                            .text_color(t.soft)
                            .child(tr("Ask the assistant…", "Demandez à l'assistant…")),
                    )
                }),
        );
        div()
            .id("assistant")
            .debug_selector(|| "assistant".into())
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(transcript)
            .child(actions)
            .child(prompt)
            .into_any_element()
    }

    /// Adds a line of the transcript and shows the end of it.
    pub(crate) fn assistant_say(&mut self, who: Who, text: &str, cx: &mut Context<Self>) {
        self.assistant.entries.push(Entry {
            who,
            text: text.to_string(),
        });
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();
    }

    /// Sets what the agent is doing.
    pub(crate) fn assistant_status(&mut self, status: Status, cx: &mut Context<Self>) {
        self.assistant.status = status;
        cx.notify();
    }

    /// Streams text into the last line of the agent, or starts one. Text that comes after the end
    /// of a message of the agent starts a new paragraph of that line, when there is text before it.
    pub(crate) fn assistant_more(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.is_empty() {
            return;
        }
        let separate = std::mem::take(&mut self.assistant.paragraph);
        let continues = matches!(self.assistant.entries.last(), Some(e) if e.who == Who::Agent);
        match continues {
            true => {
                if let Some(last) = self.assistant.entries.last_mut() {
                    if separate && !last.text.is_empty() {
                        last.text.push_str("\n\n");
                    }
                    last.text.push_str(text);
                }
            }
            false => self.assistant.entries.push(Entry {
                who: Who::Agent,
                text: text.to_string(),
            }),
        }
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();
    }

    /// Whether the command is installed. The tests set it by hand (`Assistant::found`).
    #[cfg(test)]
    fn command_found(&self) -> bool {
        self.assistant.found
    }

    #[cfg(not(test))]
    fn command_found(&self) -> bool {
        platform::assistant::find(self.prefs.assistant_command()).is_some()
    }

    /// Sets Missing or Idle from whether the command is installed. Working is left alone.
    pub(crate) fn assistant_check(&mut self, cx: &mut Context<Self>) {
        let found = self.command_found();
        let status = &mut self.assistant.status;
        if !found {
            *status = Status::Missing;
        } else if *status == Status::Missing {
            *status = Status::Idle;
        }
        cx.notify();
    }

    /// A run ended: Idle while the panel is shown, NeedsYou while it is not (the answer waits for
    /// the person). Missing when the command is no longer found.
    fn assistant_settle(&mut self, cx: &mut Context<Self>) {
        self.assistant.status = if !self.command_found() {
            Status::Missing
        } else if self.docked(Mode::Assistant) {
            Status::Idle
        } else {
            Status::NeedsYou
        };
        cx.notify();
    }

    /// The folder the agent works in: the vault, or the scratch folder while no vault is open.
    fn assistant_folder(&self) -> PathBuf {
        match &self.vault {
            Some(root) => root.clone(),
            None => {
                let dir = platform::scratch::dir();
                let _ = fs::create_dir_all(&dir);
                dir
            }
        }
    }

    /// A note's path as the agent is told it: relative to the working folder when it is inside.
    fn assistant_words(&self, path: &Path) -> String {
        match path.strip_prefix(self.assistant_folder()) {
            Ok(relative) => relative.display().to_string(),
            Err(_) => path.display().to_string(),
        }
    }

    /// The note of the active tab, as the agent is told it, or `None` when the tab has no note file.
    fn assistant_note(&self) -> Option<String> {
        self.pane()
            .path
            .as_deref()
            .map(|path| self.assistant_words(path))
    }

    /// The prompt is sent to the agent. The agent is started in the working folder on the first
    /// prompt, and again when the folder changes. In a test only the process is not started (see
    /// `start_agent`): the prompt is also recorded in `sent`.
    pub(crate) fn assistant_run(&mut self, prompt: String, cx: &mut Context<Self>) {
        let folder = self.assistant_folder();
        let running =
            self.assistant.agent.is_some() && self.assistant.folder.as_ref() == Some(&folder);
        if !running {
            // The old agent stops, but a review in progress is kept: it is the one being asked.
            self.stop_agent();
            let command = self.prefs.assistant_command().to_string();
            #[cfg(test)]
            let started = match self.assistant.refuse_start.take() {
                Some(error) => Err(error),
                None => start_agent(&command, &folder, self.assistant.signal.clone()),
            };
            #[cfg(not(test))]
            let started = start_agent(&command, &folder, self.assistant.signal.clone());
            match started {
                Ok(agent) => {
                    self.assistant.agent = Some(agent);
                    self.assistant.folder = Some(folder);
                }
                Err(error) => return self.assistant_fail(&error, cx),
            }
        }
        self.assistant_sent(&prompt);
        let sent = self
            .assistant
            .agent
            .as_ref()
            .map(|agent| agent.prompt(&prompt));
        match sent {
            Some(Ok(())) => self.assistant_status(Status::Working, cx),
            Some(Err(error)) => {
                self.assistant.agent = None;
                self.assistant.folder = None;
                self.assistant_fail(&error, cx);
            }
            None => {}
        }
    }

    /// The prompt, as the tests read it.
    #[cfg(test)]
    fn assistant_sent(&mut self, prompt: &str) {
        self.assistant.sent.push(prompt.to_string());
    }

    #[cfg(not(test))]
    fn assistant_sent(&mut self, _prompt: &str) {}

    /// The prompt box is emptied once a run has started. A prompt that did not start stays in the box,
    /// so the person can send it again.
    fn clear_box_if_started(&mut self, cx: &mut Context<Self>) {
        if self.assistant.status != Status::Working {
            return;
        }
        self.assistant.field.update(cx, |field, cx| {
            field.line = Line::default();
            cx.notify();
        });
    }

    /// A prompt that could not be sent: the error is shown, and the agent is free again.
    fn assistant_fail(&mut self, error: &str, cx: &mut Context<Self>) {
        self.assistant.job = None;
        self.assistant.status = if self.command_found() {
            Status::Idle
        } else {
            Status::Missing
        };
        self.assistant_say(Who::Note, error, cx);
    }

    /// Drops the agent process, which stops it. A review in progress is kept: `assistant_run` uses
    /// this to replace the agent, and the review still waits for its answer.
    fn stop_agent(&mut self) {
        self.assistant.agent = None;
        self.assistant.folder = None;
        self.assistant.stopping = false;
    }

    /// Stops the agent now: the setting changed, the vault changed, the app quits. A review in
    /// progress is dropped with it, so that a half answer writes nothing.
    pub(crate) fn assistant_halt(&mut self, cx: &mut Context<Self>) {
        self.assistant.job = None;
        self.stop_agent();
        if self.assistant.status == Status::Working {
            self.assistant.status = Status::Idle;
        }
        self.assistant_check(cx);
    }

    /// A vault change: the agent stops and the transcript starts again.
    pub(crate) fn assistant_reset(&mut self, cx: &mut Context<Self>) {
        self.assistant_halt(cx);
        self.assistant.entries.clear();
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();
    }

    /// Waits for the agent's events (its reader thread raises the signal) and shows them. Started
    /// once, with the window; the tests call `assistant_events` directly.
    pub(super) fn assistant_listen(&mut self, window: &Window, cx: &mut Context<Self>) {
        if cfg!(test) {
            return;
        }
        let signal = self.assistant.signal.clone();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                signal
                    .wait(cx.background_executor().timer(Duration::from_secs(3600)))
                    .await;
                let Ok(()) = this.update_in(cx, |this, window, cx| {
                    let events = this
                        .assistant
                        .agent
                        .as_ref()
                        .map(Agent::take)
                        .unwrap_or_default();
                    this.assistant_events(events, window, cx);
                }) else {
                    return;
                };
            }
        })
        .detach();
    }

    /// Shows what the agent said, in order: its text, the end of a run, its errors.
    pub(crate) fn assistant_events(
        &mut self,
        events: Vec<Event>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for event in events {
            match event {
                Event::Started => {
                    if !self.assistant.stopping {
                        self.assistant_status(Status::Working, cx);
                    }
                }
                Event::Text(text) => {
                    if self.assistant.stopping {
                        continue;
                    }
                    match &mut self.assistant.job {
                        Some(job) => job.answer().push_str(&text),
                        None => self.assistant_more(&text, cx),
                    }
                }
                // A tool call or the end of a message: what follows is another message. The
                // tool's name is not shown: the transcript stays about the answer.
                Event::Tool(_) | Event::Break => {
                    if self.assistant.stopping {
                        continue;
                    }
                    match &mut self.assistant.job {
                        Some(job) => job.answer().push('\n'),
                        None => self.assistant.paragraph = true,
                    }
                }
                Event::Refused(error) => {
                    self.assistant.job = None;
                    self.assistant_say(Who::Note, &error, cx);
                    if login_problem(&error) {
                        self.assistant_say(
                            Who::Note,
                            tr(
                                "Log in with hoocode first: run `hoocode` in a terminal and use /login.",
                                "Connectez-vous d'abord avec hoocode : lancez `hoocode` dans un terminal et utilisez /login.",
                            ),
                            cx,
                        );
                    }
                    self.assistant_settle(cx);
                }
                Event::Failed(error) => {
                    // A failed answer is not reviewed: the run still ends, and writes nothing.
                    self.assistant.job = None;
                    self.assistant_say(Who::Note, &error, cx);
                }
                Event::Ended => {
                    self.assistant.stopping = false;
                    match self.assistant.job.take() {
                        Some(Job::Review(review)) => self.apply_review(review, cx),
                        Some(Job::Improve(improve)) => self.apply_improve(improve, cx),
                        None => {}
                    }
                    self.assistant_settle(cx);
                }
                Event::Exited(tail) => {
                    let working = self.assistant.status == Status::Working;
                    self.stop_agent();
                    self.assistant.job = None;
                    if working {
                        self.assistant_say(
                            Who::Note,
                            tr("The assistant stopped.", "L'assistant s'est arrêté."),
                            cx,
                        );
                        if !tail.is_empty() {
                            self.assistant_say(Who::Note, &tail, cx);
                        }
                        self.assistant_settle(cx);
                    } else {
                        self.assistant_check(cx);
                    }
                }
            }
        }
    }

    /// "Review this note": the note is saved, its review is asked, and its answer is read when the
    /// agent ends (`apply_review`). Ignored while the agent works or is not installed.
    pub(crate) fn assistant_review(&mut self, cx: &mut Context<Self>) {
        if !self.assistant_free() || !self.review_open() {
            return;
        }
        if !self.flush_one(cx) {
            let said = tr(
                "The note could not be saved, so it was not reviewed.",
                "La note n'a pas pu être enregistrée, elle n'a donc pas été relue.",
            );
            return self.assistant_say(Who::Note, said, cx);
        }
        // Read after the save: a `# Title` change renames the note, and the agent is told the new name.
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let note = self.assistant_words(&path);
        self.assistant.job = Some(Job::Review(Review {
            path,
            answer: String::new(),
        }));
        self.assistant_say(Who::You, tr("Review this note", "Relire cette note"), cx);
        self.assistant_say(Who::Note, tr("Reviewing…", "Relecture en cours…"), cx);
        self.assistant_run(words::review_prompt(&note), cx);
    }

    /// Writes the findings of a finished review into the note, as one undo step when the note is
    /// open, or straight to its file when no tab shows it. The note is saved by the usual debounce.
    fn apply_review(&mut self, review: Review, cx: &mut Context<Self>) {
        // The revisions it wrote show in the Versions panel when it is next drawn.
        self.versions_stale = true;
        let found = words::findings(&review.answer);
        if found.is_empty() {
            self.assistant_say(Who::Note, tr("No findings.", "Aucune remarque."), cx);
            return;
        }
        let open = self
            .panes
            .iter()
            .position(|pane| pane.path.as_deref() == Some(review.path.as_path()));
        let home = history::home_of(&review.path, self.vault.as_deref());
        let current = match open {
            Some(i) => self.panes[i].editor.read(cx).text().to_string(),
            // A closed note is read as the editor would show it, so its pending changes stay placed.
            None => match fs::read_to_string(&review.path) {
                Ok(text) => crate::platform::sidecar::display(home.as_ref(), &review.path, &text),
                Err(e) => {
                    let what = tr(
                        "The note could not be read:",
                        "La note n'a pas pu être lue :",
                    );
                    let why = crate::lang::io_reason(&e);
                    return self.assistant_say(Who::Note, &format!("{what} {why}"), cx);
                }
            },
        };
        let author = self.revision_author();
        let time = crate::logic::markdown::utc_minute(crate::platform::revisions::now());
        let by = crate::logic::markdown::Stamp {
            author: &author,
            time: &time,
        };
        let placed = words::place(&current, &found, by);
        if placed.placed > 0 {
            match open {
                Some(i) => {
                    if let Some(edit) = words::edit_between(&current, &placed.text) {
                        self.panes[i]
                            .editor
                            .update(cx, |editor, cx| editor.absorb(&[edit], cx));
                    }
                }
                None => {
                    let saved = crate::platform::sidecar::save(
                        home.as_ref(),
                        &review.path,
                        &placed.text,
                        "assistant",
                    );
                    if let Err(e) = saved {
                        let why = crate::lang::io_reason(&e);
                        return self.assistant_say(Who::Note, &why, cx);
                    }
                }
            }
        }
        let summary = review_summary(placed.placed, placed.skipped);
        self.assistant_say(Who::Note, &summary, cx);
    }

    /// "Improve selection" or "Improve this note": the note is saved, its selection (else the whole
    /// note) is asked to be rewritten, following the prompt box's text (else the default instruction).
    /// The answer is read when the agent ends (`apply_improve`). A note with pending changes is
    /// refused first, with one line, and nothing is sent. Ignored while the agent works or is not
    /// installed, or when no note is open.
    pub(crate) fn assistant_improve(&mut self, cx: &mut Context<Self>) {
        if !self.assistant_free() || !self.review_open() {
            return;
        }
        if pending::has_pending(self.pane().editor.read(cx).text()) {
            let said = tr(
                "Accept or reject the pending changes first.",
                "Acceptez ou refusez d'abord les changements en attente.",
            );
            return self.assistant_say(Who::Note, said, cx);
        }
        if !self.flush_one(cx) {
            let said = tr(
                "The note could not be saved, so it was not improved.",
                "La note n'a pas pu être enregistrée, elle n'a donc pas été améliorée.",
            );
            return self.assistant_say(Who::Note, said, cx);
        }
        // Read after the save: a `# Title` change renames the note, and the prompt names the new file.
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let (base, range) = {
            let editor = self.pane().editor.read(cx);
            let span = editor.span();
            (
                editor.text().to_string(),
                (!span.is_empty()).then_some(span),
            )
        };
        let typed = self.assistant.field.read(cx).line.text.trim().to_string();
        let instruction = match typed.is_empty() {
            true => words::IMPROVE_INSTRUCTION,
            false => typed.as_str(),
        };
        let label = match range {
            Some(_) => tr("Improve selection", "Améliorer la sélection"),
            None => tr("Improve this note", "Améliorer cette note"),
        };
        let said = match typed.is_empty() {
            true => label.to_string(),
            false => format!("{label}: {typed}"),
        };
        let passage = range.clone().map(|range| &base[range]);
        let prompt = words::improve_prompt(&self.assistant_words(&path), passage, instruction);
        self.assistant.job = Some(Job::Improve(Improve {
            path,
            base,
            range,
            answer: String::new(),
        }));
        self.assistant_say(Who::You, &said, cx);
        self.assistant_say(
            Who::Note,
            tr("Working on a proposal…", "Proposition en cours…"),
            cx,
        );
        self.assistant_run(prompt, cx);
        self.clear_box_if_started(cx);
    }

    /// Puts `marked` (the text with its pending changes) in tab `pane`, whose text is `current`: one
    /// undo step. Nothing changes when the two texts are the same. Shared by Improve and the restore
    /// of a version (`shell/versions.rs`).
    pub(super) fn absorb_marked(
        &mut self,
        pane: usize,
        current: &str,
        marked: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(edit) = words::edit_between(current, marked) {
            self.panes[pane]
                .editor
                .update(cx, |editor, cx| editor.absorb(&[edit], cx));
        }
    }

    /// Writes the agent's proposal into its note as pending changes: one undo step when a tab shows
    /// the note, else the file itself. Two automatic revisions surround the write, as `inkuno write`
    /// makes them: the text before (skipped when it is the latest revision), then the proposed text.
    /// A proposal that cannot be placed is said in one line, and nothing is written.
    fn apply_improve(&mut self, improve: Improve, cx: &mut Context<Self>) {
        // The two revisions of the proposal show in the Versions panel when it is next drawn.
        self.versions_stale = true;
        // The text the block replaces: a fenced passage keeps its fences in the proposal.
        let original = match &improve.range {
            Some(range) => &improve.base[range.clone()],
            None => improve.base.as_str(),
        };
        let fenced = words::starts_with_fence(original);
        let Some(text) = words::proposal(&improve.answer, fenced) else {
            let said = tr(
                "The assistant proposed no change.",
                "L'assistant n'a proposé aucun changement.",
            );
            return self.assistant_say(Who::Note, said, cx);
        };
        let proposed = words::rewritten(&improve.base, improve.range.clone(), &text);
        let open = self
            .panes
            .iter()
            .position(|pane| pane.path.as_deref() == Some(improve.path.as_path()));
        let current = match open {
            Some(i) => self.panes[i].editor.read(cx).text().to_string(),
            None => match fs::read_to_string(&improve.path) {
                Ok(text) => text,
                Err(e) => {
                    let what = tr(
                        "The note could not be read:",
                        "La note n'a pas pu être lue :",
                    );
                    let why = crate::lang::io_reason(&e);
                    return self.assistant_say(Who::Note, &format!("{what} {why}"), cx);
                }
            },
        };
        let marked = match pending::propose(&improve.base, &current, &proposed) {
            Ok(marked) => marked,
            Err(refusal) => return self.assistant_say(Who::Note, refusal_words(refusal), cx),
        };
        let home = history::home_of(&improve.path, self.vault.as_deref());
        if let Some(home) = &home {
            let author = self.revision_author();
            // History data: the same fixed English memo as `inkuno write` gives (cli/ops.rs).
            let memo = "before a change by assistant";
            if let Err(why) = auto_revision(home, &improve.path, &author, memo, &current) {
                let what = tr(
                    "Not written: the version before the change was not saved:",
                    "Non écrit : la version avant le changement n'a pas été enregistrée :",
                );
                return self.assistant_say(Who::Note, &format!("{what} {why}"), cx);
            }
        }
        match open {
            Some(i) => self.absorb_marked(i, &current, &marked, cx),
            None => {
                // The file keeps the note's own text; the proposal's changes wait beside it.
                if let Err(e) =
                    platform::sidecar::save(home.as_ref(), &improve.path, &marked, "assistant")
                {
                    let what = tr(
                        "Not written: the proposal was not saved:",
                        "Non écrit : la proposition n'a pas été enregistrée :",
                    );
                    let why = crate::lang::io_reason(&e);
                    return self.assistant_say(Who::Note, &format!("{what} {why}"), cx);
                }
            }
        }
        // A failed save of the proposed version is a warning: the changes are written and pending.
        if let Some(home) = &home {
            let accepted = pending::accept_all(&marked);
            if let Err(why) = auto_revision(
                home,
                &improve.path,
                "assistant",
                "proposed, pending",
                &accepted,
            ) {
                let what = tr(
                    "Warning: the proposed version was not saved:",
                    "Attention : la version proposée n'a pas été enregistrée :",
                );
                self.assistant_say(Who::Note, &format!("{what} {why}"), cx);
            }
        }
        let count = pending::areas(&marked).len();
        self.assistant_say(Who::Note, &pending_words(count), cx);
    }

    /// Stops the agent's turn. A review in progress is dropped first, so nothing is written. The
    /// agent has `STOP_GRACE` to report its end; if it has not by then, it is killed and the panel
    /// is free again. When the abort cannot be sent, the agent is killed at once.
    pub(crate) fn assistant_stop(&mut self, cx: &mut Context<Self>) {
        if self.assistant.status != Status::Working {
            return;
        }
        self.assistant.job = None;
        self.assistant.stopping = true;
        self.assistant_say(Who::Note, tr("Stopped.", "Arrêté."), cx);
        let aborted = self
            .assistant
            .agent
            .as_ref()
            .is_some_and(|agent| agent.abort().is_ok());
        if !aborted {
            return self.assistant_kill(cx);
        }
        self.assistant.stop_gen += 1;
        let generation = self.assistant.stop_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(STOP_GRACE).await;
            this.update(cx, |this, cx| {
                if this.assistant.stop_gen == generation && this.assistant.stopping {
                    this.assistant_kill(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Kills the agent now: the run is over, whatever it still says, and the panel is free.
    fn assistant_kill(&mut self, cx: &mut Context<Self>) {
        self.stop_agent();
        self.assistant.job = None;
        self.assistant_settle(cx);
    }

    /// A prompt from the user (Enter in the box): the note's unsaved edits are written first, so
    /// that the agent reads what the person sees. The transcript shows the question only; the
    /// words sent name the note and its file.
    pub(crate) fn assistant_prompt(
        &mut self,
        question: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.flush_one(cx) {
            // The question stays in the box: it can be sent again once the note is saved.
            let said = tr(
                "The note could not be saved, so the question was not sent.",
                "La note n'a pas pu être enregistrée, la question n'a donc pas été envoyée.",
            );
            return self.assistant_say(Who::Note, said, cx);
        }
        let prompt = words::chat_prompt(self.assistant_note().as_deref(), &question);
        self.assistant_say(Who::You, &question, cx);
        window.focus(&self.assistant.field.focus_handle(cx), cx);
        self.assistant_run(prompt, cx);
        self.clear_box_if_started(cx);
    }
}
