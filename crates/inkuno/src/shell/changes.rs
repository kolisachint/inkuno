//! Comments on a pending change (`Comment on the change`): asks for the comment's line in a prompt,
//! then writes it into the note with the change still pending (`Editor::comment_on_change`).

use super::*;

impl Shell {
    /// The hint at the note's foot, left of the corner buttons: "2 changes pending". A click moves the
    /// caret to the first change. It is drawn over the text like the corner buttons, so no line moves.
    pub(super) fn pending_hint(
        &self,
        count: usize,
        corner: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let label = if count == 1 {
            tr("1 change pending", "1 changement en attente").to_string()
        } else {
            format!(
                "{count} {}",
                tr("changes pending", "changements en attente")
            )
        };
        div()
            .id("pending-hint")
            .debug_selector(|| "pending-hint".into())
            .absolute()
            .bottom_4()
            .right(t.px(12. + 38. * corner as f32))
            .h(t.px(30.))
            .px(t.px(10.))
            .flex()
            .items_center()
            .rounded_full()
            .bg(t.panel)
            .border_1()
            .border_color(t.border)
            .text_color(t.soft)
            .text_size(t.px(13.))
            .cursor_pointer()
            .occlude()
            .hover(|s| s.bg(t.hover))
            .child(label)
            .on_click(cx.listener(|this, _, window, cx| {
                let editor = this.pane().editor.clone();
                editor.update(cx, |note, cx| note.go_to_first_change(cx));
                window.focus(&editor.focus_handle(cx));
                cx.notify();
            }))
    }

    /// Asks for the line of a comment on pending change `index` of `editor`'s note. Enter writes it,
    /// Escape cancels; the note keeps the focus after either.
    pub(super) fn ask_change_comment(
        &mut self,
        editor: Entity<Editor>,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (theme, label) = (
            self.theme,
            tr("What does the comment say?", "Que dit le commentaire ?"),
        );
        let prompt = cx.new(|cx| Palette::prompt(label, "", theme, cx));
        cx.subscribe_in(&prompt, window, move |this, asked, event, window, cx| {
            // Only the prompt that asked may answer: another palette may have taken its place.
            if this.palette.as_ref() != Some(asked) {
                return;
            }
            match event {
                PaletteEvent::Submit(memo) => {
                    this.palette = None;
                    window.focus(&editor.focus_handle(cx));
                    let memo = memo.clone();
                    editor.update(cx, |note, cx| note.comment_on_change(index, &memo, cx));
                }
                PaletteEvent::Dismiss => {
                    this.palette = None;
                    window.focus(&editor.focus_handle(cx));
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        window.focus(&prompt.focus_handle(cx));
        self.palette = Some(prompt);
        cx.notify();
    }
}
