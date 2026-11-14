//! Scratch notes (Ctrl+N): a raw thought in a tab, written as it is typed to the scratch folder
//! (`platform/scratch.rs`), never to the vault. Closing one with text asks what becomes of it:
//! kept as scratch (the default), moved into the vault, or discarded.

use super::*;

impl Shell {
    /// Ctrl+N: a scratch note in a tab of its own, the cursor in it. Nothing is filed: its file is
    /// made at its first text, in the scratch folder, and no vault is needed.
    pub(super) fn new_scratch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.fresh_tab(window, cx);
        self.new_note(String::new(), cx);
        let pane = self.pane_mut();
        pane.scratch = true;
        pane.fresh = false;
        pane.synced = false;
        if self.nav.panel == Panel::Full {
            self.nav.panel = Panel::Split;
        }
        window.focus(&self.pane().editor.focus_handle(cx));
        self.settle_nav(window, cx);
    }

    /// A scratch note is written to its own file as it is typed, vault or not. Its file is made
    /// at its first text: a note left blank has none, and nothing is filed in the vault.
    pub(super) fn flush_scratch(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.pane().dirty {
            return true;
        }
        if let Some(path) = self.pane().path.clone()
            && !self.take_outside(&path, cx)
        {
            return false;
        }
        let content = self.pane().editor.read(cx).text().to_string();
        let path = match self.pane().path.clone() {
            Some(path) => path,
            None if logic::scratch::is_blank(&content) => {
                self.pane_mut().dirty = false;
                return true;
            }
            None => platform::scratch::new_path(),
        };
        match platform::scratch::write(&path, &content) {
            Ok(()) => {
                let first = self.pane().path.is_none();
                let pane = self.pane_mut();
                pane.path = Some(path);
                pane.dirty = false;
                pane.base = content;
                if first {
                    self.push_dirs(cx);
                }
                self.calm();
            }
            Err(e) => {
                let name = vault::stem(&path);
                self.say(
                    Tone::Failed,
                    format!(
                        "{} « {name} » : {}",
                        tr("Scratch note not saved", "Brouillon non enregistré"),
                        crate::lang::io_reason(&e)
                    ),
                    cx,
                )
            }
        }
        cx.notify();
        !self.pane().dirty
    }

    /// A scratch note left blank goes with its tab: its file is deleted, so no empty note stays.
    pub(super) fn drop_blank_scratch(&mut self, cx: &App) {
        let pane = self.pane();
        if !pane.scratch || !logic::scratch::is_blank(pane.editor.read(cx).text()) {
            return;
        }
        if let Some(path) = pane.path.clone() {
            let _ = fs::remove_file(path);
        }
    }

    /// Closing a scratch note with text asks what becomes of it, in a short list: Keep as scratch
    /// (the default, Enter) closes the tab and leaves the file; Keep in vault moves the note into
    /// the vault; Discard deletes it. Escape cancels the close.
    pub(super) fn ask_scratch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choices = logic::scratch::choices(self.vault.is_some());
        let options: Vec<String> = choices.iter().map(|c| c.label().to_string()).collect();
        let label = tr(
            "Scratch note: what becomes of it?",
            "Brouillon : que devient-il ?",
        );
        let theme = self.theme;
        let current = choices[0].label();
        let prompt = cx.new(|cx| Palette::choose(label, options, current, theme, cx));
        cx.subscribe_in(&prompt, window, move |this, asked, event, window, cx| {
            // Only the prompt that asked may answer: another palette may have taken its place.
            if this.palette.as_ref() != Some(asked) {
                return;
            }
            match event {
                PaletteEvent::Submit(text) => {
                    this.palette = None;
                    if let Some(choice) = choices.iter().copied().find(|c| c.label() == text) {
                        this.choose_scratch(choice, window, cx);
                    }
                }
                PaletteEvent::Dismiss => {
                    this.palette = None;
                    this.scratch_ask = None;
                    this.focus_pane(this.active, window, cx);
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        window.focus(&prompt.focus_handle(cx));
        self.palette = Some(prompt);
        self.scratch_ask = Some(self.pane().editor.clone());
        cx.notify();
    }

    /// The choice made for the scratch note whose close was asked.
    fn choose_scratch(
        &mut self,
        choice: logic::scratch::Choice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use logic::scratch::Choice;
        let Some(editor) = self.scratch_ask.take() else {
            return;
        };
        let Some(tab) = self.panes.iter().position(|pane| pane.editor == editor) else {
            return;
        };
        self.focus_pane(tab, window, cx);
        match choice {
            Choice::Keep => _ = self.close_now(window, cx),
            Choice::Vault => _ = self.keep_in_vault(cx),
            Choice::Discard => self.discard_scratch(window, cx),
        }
    }

    /// Discard: the file goes, and the tab with it. A file that cannot be deleted keeps its tab
    /// and its text.
    fn discard_scratch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.pane().path.clone()
            && let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            let name = vault::stem(&path);
            let what = tr("Not discarded", "Non jeté");
            self.say(
                Tone::Failed,
                format!("{what} « {name} » : {}", crate::lang::io_reason(&e)),
                cx,
            );
            return;
        }
        // Nothing is left to write: the tab closes without saving the text it had.
        let pane = self.pane_mut();
        pane.dirty = false;
        pane.path = None;
        self.close_now(window, cx);
    }

    /// Moves the scratch note of the active tab into the vault: under its title, in the folder the
    /// tree shows (else the vault root), saved as any new note is. The tab stays, now a note of the
    /// vault; the scratch file goes once the note is written. False, with a message, when it is not.
    pub(super) fn keep_in_vault(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(root) = self.vault.clone() else {
            return false;
        };
        if !self.pane().scratch {
            return false;
        }
        let dir = self.nav.target_dir().unwrap_or(root);
        let scratch_file = self.pane().path.clone();
        let pane = self.pane_mut();
        (pane.scratch, pane.path, pane.new_dir) = (false, None, Some(dir));
        (pane.synced, pane.fresh, pane.dirty, pane.origin) = (true, false, true, None);
        if self.flush_one(cx) {
            // A copy left behind by a failed delete is no loss: the note is in the vault.
            if let Some(file) = scratch_file {
                let _ = fs::remove_file(file);
            }
            self.pane_mut().new_dir = None;
            return true;
        }
        let pane = self.pane_mut();
        (pane.scratch, pane.path, pane.new_dir, pane.synced) = (true, scratch_file, None, false);
        false
    }
}
