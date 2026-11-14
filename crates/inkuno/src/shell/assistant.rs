//! The assistant: a panel of the right dock that talks to the agent about the open note, and the
//! round button at the note's foot that opens it. This file holds the state and the panel. The
//! process side (starting the agent's command, its protocol) is the seam at the bottom: the
//! `assistant_*` methods are what it calls, and nothing here starts a process.

use super::*;

/// What the agent is doing, as the button and the panel show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Missing is set by the process side, when the command is not installed.
pub(crate) enum Status {
    Missing,
    Idle,
    Working,
    /// The agent asks something: a request waits for Allow or Deny.
    NeedsYou,
}

/// Who a line of the transcript is from. `Note` is a quiet system line: errors, tool activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Agent and Note are written by the process side.
pub(crate) enum Who {
    You,
    Agent,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // The process side reads and writes the transcript.
pub(crate) struct Entry {
    pub who: Who,
    pub text: String,
}

/// A request from the agent that waits for Allow or Deny.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // The id is what the process side answers.
pub(crate) struct Ask {
    pub id: String,
    pub text: String,
}

pub(crate) struct Assistant {
    pub status: Status,
    pub entries: Vec<Entry>,
    pub ask: Option<Ask>,
    /// The prompt box, at the foot of the panel.
    pub field: Entity<field::Field>,
    pub scroll: gpui::ScrollHandle,
    /// The prompts sent to the agent, for the tests.
    #[cfg(test)]
    pub sent: Vec<String>,
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
            ask: None,
            field,
            scroll: gpui::ScrollHandle::new(),
            #[cfg(test)]
            sent: Vec::new(),
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

impl Shell {
    /// Opens the assistant's panel in the right dock, or hides it when it is shown. Opening it
    /// stacks it under what the dock shows (the outline stays), and gives the prompt box the keys.
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
        window.focus(&self.assistant.field.focus_handle(cx));
        self.settle_nav(window, cx);
    }

    /// The agent is free to take a prompt: not working, and installed.
    fn assistant_free(&self) -> bool {
        !matches!(self.assistant.status, Status::Working | Status::Missing)
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
            // A waiting request: the dot the comments bubble also draws.
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

    /// The panel in the right dock: the transcript, a request waiting for an answer, the review and
    /// stop buttons, and the prompt box.
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
        // A request from the agent, with the two answers.
        let ask = self.assistant.ask.clone().map(|ask| {
            div()
                .id("assistant-ask")
                .debug_selector(|| "assistant-ask".into())
                .mx_3()
                .mb_2()
                .p_2()
                .rounded(t.large())
                .bg(t.hover)
                .flex()
                .flex_col()
                .gap_2()
                .text_size(t.px(13.))
                .text_color(t.text)
                .child(ask.text)
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            assistant_button("assistant-allow", tr("Allow", "Autoriser"), true, t)
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.assistant_answer(true, cx)),
                                ),
                        )
                        .child(
                            assistant_button("assistant-deny", tr("Deny", "Refuser"), true, t)
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.assistant_answer(false, cx)),
                                ),
                        ),
                )
        });
        let review = self.review_prompt().filter(|_| self.assistant_free());
        let actions = div()
            .flex()
            .gap_2()
            .px_3()
            .pb_2()
            .child(
                assistant_button(
                    "assistant-review",
                    tr("Review this note", "Relire cette note"),
                    review.is_some(),
                    t,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    if let Some(prompt) = this.review_prompt().filter(|_| this.assistant_free()) {
                        this.assistant_prompt(prompt, window, cx);
                    }
                })),
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
            .children(ask)
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
    #[allow(dead_code)] // Called by the process side.
    pub(crate) fn assistant_status(&mut self, status: Status, cx: &mut Context<Self>) {
        self.assistant.status = status;
        cx.notify();
    }

    /// Streams text into the last line of the agent, or starts one.
    #[allow(dead_code)] // Called by the process side.
    pub(crate) fn assistant_more(&mut self, text: &str, cx: &mut Context<Self>) {
        let continues = matches!(self.assistant.entries.last(), Some(e) if e.who == Who::Agent);
        match continues {
            true => {
                if let Some(last) = self.assistant.entries.last_mut() {
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

    /// Sets or clears the request waiting for an answer. A request makes the status `NeedsYou`.
    #[allow(dead_code)] // Called by the process side.
    pub(crate) fn assistant_asks(&mut self, ask: Option<Ask>, cx: &mut Context<Self>) {
        if ask.is_some() {
            self.assistant.status = Status::NeedsYou;
        }
        self.assistant.ask = ask;
        cx.notify();
    }

    /// A prompt from the user (Enter, or "Review this note"): it is shown as a line of the
    /// transcript, the box is cleared, and it goes to the agent.
    pub(crate) fn assistant_prompt(
        &mut self,
        prompt: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.assistant_say(Who::You, &prompt, cx);
        self.assistant.field.update(cx, |field, cx| {
            field.line = crate::ui::line::Line::default();
            cx.notify();
        });
        window.focus(&self.assistant.field.focus_handle(cx));
        self.assistant_run(prompt, cx);
    }
}

// The process side: filled in by the wiring step.
impl Shell {
    /// The prompt is sent to the agent.
    pub(crate) fn assistant_run(&mut self, prompt: String, _cx: &mut Context<Self>) {
        // The process side: filled in by the wiring step.
        #[cfg(test)]
        self.assistant.sent.push(prompt);
        #[cfg(not(test))]
        drop(prompt);
    }

    /// Allow or Deny: the request is answered, and the agent goes on.
    pub(crate) fn assistant_answer(&mut self, allow: bool, cx: &mut Context<Self>) {
        // The process side: the answer goes to the agent with the request's id.
        let _ = allow;
        self.assistant.ask = None;
        if self.assistant.status == Status::NeedsYou {
            self.assistant.status = Status::Working;
        }
        cx.notify();
    }

    /// Stops the agent's turn.
    pub(crate) fn assistant_stop(&mut self, cx: &mut Context<Self>) {
        // The process side: stops the running turn.
        self.assistant.ask = None;
        self.assistant.status = Status::Idle;
        cx.notify();
    }

    /// The prompt for "Review this note", or `None` when no note is open.
    pub(crate) fn review_prompt(&self) -> Option<String> {
        let open = self.pane().path.is_some()
            && self.pane().file.is_none()
            && self.pane().listing.is_none();
        // The wiring step writes the real prompt.
        open.then(|| "Review this note".to_string())
    }
}
