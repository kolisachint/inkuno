//! The assistant: a panel of the right dock that talks to the agent about the open note, and the
//! round button at the note's foot that opens it. The agent is `hoocode` in its RPC mode, started
//! without a shell or a write tool: it reads the note and answers. A review's answer is a list of
//! findings, and Inkuno writes them into the note as comments (one undo step). This file holds the
//! state, the panel and the process side (the child process is `platform::assistant`).

use super::*;
use crate::logic::assistant as words;
use crate::platform::assistant::{Agent, Event};
use crate::platform::vault::Signal;
use crate::ui::line::Line;
use std::{
    fs,
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
    pub review: Option<Review>,
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
                    window.focus(&this.pane().editor.focus_handle(cx));
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
            review: None,
            stopping: false,
            stop_gen: 0,
            paragraph: false,
            #[cfg(test)]
            sent: Vec::new(),
            #[cfg(test)]
            found: true,
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

impl Shell {
    /// Opens the assistant's panel in the right dock, or hides it when it is shown. Opening it
    /// stacks it under what the dock shows (the outline stays), and gives the prompt box the keys.
    /// An answer that waited for the panel is seen now: the status goes back to Idle.
    pub(super) fn toggle_assistant(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.docked(Mode::Assistant) {
            return self.hide_tool(Mode::Assistant, window, cx);
        }
        if self.nav.panel == Panel::Full {
            self.nav.panel = Panel::Split;
        }
        self.nav.right_shown.retain(|m| *m != Mode::Assistant);
        self.nav.right_shown.push(Mode::Assistant);
        self.nav.folded.remove(&Mode::Assistant);
        self.nav.activate(Mode::Assistant);
        self.nav.reveal = true;
        if self.assistant.status == Status::NeedsYou {
            self.assistant.status = Status::Idle;
        }
        window.focus(&self.assistant.field.focus_handle(cx));
        self.settle_nav(window, cx);
    }

    /// The agent is free to take a prompt: not working, and installed.
    fn assistant_free(&self) -> bool {
        !matches!(self.assistant.status, Status::Working | Status::Missing)
    }

    /// A note is open in the active tab (not a file, not a list): it can be reviewed.
    fn review_open(&self) -> bool {
        self.pane().path.is_some() && self.pane().file.is_none() && self.pane().listing.is_none()
    }

    /// The round button at the note's foot, on the right: its icon and its tooltip follow the status.
    pub(super) fn assistant_toggle(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let status = self.assistant.status;
        let command = self.prefs.assistant_command().to_string();
        let (label, color): (String, gpui::Hsla) = match status {
            Status::Idle => (tr("Assistant", "Assistant").into(), t.soft),
            Status::Working => (
                tr("Assistant: working…", "Assistant : en cours…").into(),
                t.accent,
            ),
            Status::NeedsYou => (
                tr("Assistant: needs you", "Assistant : attend votre réponse").into(),
                t.soft,
            ),
            Status::Missing => (
                format!(
                    "{} {command} {}",
                    tr("Assistant:", "Assistant :"),
                    tr("is not installed", "n'est pas installé")
                ),
                t.dim,
            ),
        };
        let icon = svg()
            .path("assistant.svg")
            .size(t.px(16.))
            .flex_none()
            .text_color(color);
        // A slow pulse while the agent works; none with the motion off.
        let icon = match status == Status::Working && t.motion {
            true => icon
                .with_animation(
                    "assistant-pulse",
                    Animation::new(Duration::from_millis(1800))
                        .repeat()
                        .with_easing(gpui::pulsating_between(0.35, 1.)),
                    |icon, delta| icon.opacity(delta),
                )
                .into_any_element(),
            false => icon.into_any_element(),
        };
        div()
            .id("assistant-toggle")
            .debug_selector(|| "assistant-toggle".into())
            .tooltip(ui::widgets::tip(label, keys::of(cx, &ToggleAssistant), t))
            .absolute()
            .bottom_4()
            .right(t.px(12.))
            .size(t.px(30.))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .occlude()
            .hover(|s| s.bg(t.hover))
            .child(icon)
            // An answer waiting for the panel: the dot the comments bubble also draws.
            .children((status == Status::NeedsYou).then(|| {
                div()
                    .debug_selector(|| "assistant-badge".into())
                    .absolute()
                    .top(t.px(-3.))
                    .right(t.px(-3.))
                    .min_w(t.px(15.))
                    .h(t.px(15.))
                    .rounded_full()
                    .bg(t.accent)
            }))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_assistant(window, cx)))
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
                Status::Missing => format!(
                    "`{command}` {}",
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
        let actions = div()
            .flex()
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
        self.assistant_sent(&prompt);
        let folder = self.assistant_folder();
        let running =
            self.assistant.agent.is_some() && self.assistant.folder.as_ref() == Some(&folder);
        if !running {
            // The old agent stops, but a review in progress is kept: it is the one being asked.
            self.stop_agent();
            let command = self.prefs.assistant_command().to_string();
            match start_agent(&command, &folder, self.assistant.signal.clone()) {
                Ok(agent) => {
                    self.assistant.agent = Some(agent);
                    self.assistant.folder = Some(folder);
                }
                Err(error) => return self.assistant_fail(&error, cx),
            }
        }
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

    /// A prompt that could not be sent: the error is shown, and the agent is free again.
    fn assistant_fail(&mut self, error: &str, cx: &mut Context<Self>) {
        self.assistant.review = None;
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
        self.assistant.review = None;
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
                    match &mut self.assistant.review {
                        Some(review) => review.answer.push_str(&text),
                        None => self.assistant_more(&text, cx),
                    }
                }
                // A tool call or the end of a message: what follows is another message. The
                // tool's name is not shown: the transcript stays about the answer.
                Event::Tool(_) | Event::Break => {
                    if self.assistant.stopping {
                        continue;
                    }
                    match &mut self.assistant.review {
                        Some(review) => review.answer.push('\n'),
                        None => self.assistant.paragraph = true,
                    }
                }
                Event::Refused(error) => {
                    self.assistant.review = None;
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
                    self.assistant.review = None;
                    self.assistant_say(Who::Note, &error, cx);
                }
                Event::Ended => {
                    self.assistant.stopping = false;
                    if let Some(review) = self.assistant.review.take() {
                        self.apply_review(review, cx);
                    }
                    self.assistant_settle(cx);
                }
                Event::Exited(tail) => {
                    let working = self.assistant.status == Status::Working;
                    self.stop_agent();
                    self.assistant.review = None;
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
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        if !self.flush_one(cx) {
            let said = tr(
                "The note could not be saved, so it was not reviewed.",
                "La note n'a pas pu être enregistrée, elle n'a donc pas été relue.",
            );
            return self.assistant_say(Who::Note, said, cx);
        }
        let note = self.assistant_words(&path);
        self.assistant.review = Some(Review {
            path,
            answer: String::new(),
        });
        self.assistant_say(Who::You, tr("Review this note", "Relire cette note"), cx);
        self.assistant_say(Who::Note, tr("Reviewing…", "Relecture en cours…"), cx);
        self.assistant_run(words::review_prompt(&note), cx);
    }

    /// Writes the findings of a finished review into the note, as one undo step when the note is
    /// open, or straight to its file when no tab shows it. The note is saved by the usual debounce.
    fn apply_review(&mut self, review: Review, cx: &mut Context<Self>) {
        let found = words::findings(&review.answer);
        if found.is_empty() {
            self.assistant_say(Who::Note, tr("No findings.", "Aucune remarque."), cx);
            return;
        }
        let open = self
            .panes
            .iter()
            .position(|pane| pane.path.as_deref() == Some(review.path.as_path()));
        let current = match open {
            Some(i) => self.panes[i].editor.read(cx).text().to_string(),
            None => match fs::read_to_string(&review.path) {
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
        let placed = words::place(&current, &found);
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
                    if let Err(e) = vault::write(&review.path, &placed.text) {
                        let why = crate::lang::io_reason(&e);
                        return self.assistant_say(Who::Note, &why, cx);
                    }
                }
            }
            // The comments are in the note: the column that lists them is shown.
            self.comments_shut = false;
        }
        let summary = review_summary(placed.placed, placed.skipped);
        self.assistant_say(Who::Note, &summary, cx);
    }

    /// Stops the agent's turn. A review in progress is dropped first, so nothing is written. The
    /// agent has `STOP_GRACE` to report its end; if it has not by then, it is killed and the panel
    /// is free again. When the abort cannot be sent, the agent is killed at once.
    pub(crate) fn assistant_stop(&mut self, cx: &mut Context<Self>) {
        if self.assistant.status != Status::Working {
            return;
        }
        self.assistant.review = None;
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
        self.assistant.review = None;
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
        self.assistant.field.update(cx, |field, cx| {
            field.line = Line::default();
            cx.notify();
        });
        window.focus(&self.assistant.field.focus_handle(cx));
        self.assistant_run(prompt, cx);
    }
}
