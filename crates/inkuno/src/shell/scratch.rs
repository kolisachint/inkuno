//! Scratch notes (Ctrl+N): a raw thought in a tab, written as it is typed to the scratch folder
//! (`platform/scratch.rs`), never to the vault.

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
}
