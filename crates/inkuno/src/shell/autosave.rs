//! The debounced autosave: the 400 ms wait, the background write of an existing note
//! (`platform::vault::save_quiet`) and the UI-side application of its outcome.

use super::*;
use std::{
    io,
    sync::{Arc, Condvar, Mutex},
};

impl Shell {
    /// Starts the 400 ms wait before the next debounced save; a newer change starts it again.
    pub(super) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_gen += 1;
        let generation = self.save_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(400))
                .await;
            this.update(cx, |this, cx| {
                if this.save_gen == generation {
                    this.flush_debounced(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The debounced save: an existing note is written in the background (`start_quiet_save`), the other
    /// panes are flushed as before. A note whose background save is still running is left to it: when
    /// that ends, it saves what was typed meanwhile.
    fn flush_debounced(&mut self, cx: &mut Context<Self>) {
        self.each_pane(|this| {
            if this.pane().saving.is_some() {
                return;
            }
            match this.quiet_root(cx) {
                Some(root) => this.start_quiet_save(root, cx),
                None => {
                    this.flush_one(cx);
                }
            }
        });
    }

    /// The vault's root, when the pane's note can be written in the background: a dirty note of the vault
    /// that has a file, whose title keeps its file name and that needs no comment migrated. Anything else
    /// takes the synchronous path.
    // ponytail: `pane.h1` is set when the note loads, so a note that gains a title saves on the UI
    // thread until it is reopened.
    fn quiet_root(&self, cx: &Context<Self>) -> Option<PathBuf> {
        let pane = self.pane();
        let root = self.vault.clone()?;
        let path = pane.path.as_ref()?;
        if pane.scratch || !pane.dirty || pane.new_dir.is_some() {
            return None;
        }
        let text = pane.editor.read(cx).text();
        if !markdown::migration_edits(text).is_empty()
            || vault::h1_of(text) != pane.h1
            || (pane.synced && vault::title_of(text) != vault::stem(path))
        {
            return None;
        }
        Some(root)
    }

    /// Writes the pane's note in the background (`platform::vault::save_quiet`). Its outcome comes back to
    /// `quiet_done` on the UI thread.
    fn start_quiet_save(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let editor = self.pane().editor.clone();
        let shown = editor.read(cx).text().to_string();
        let version = editor.read(cx).version();
        let base = self.pane().base.clone();
        let home = platform::revisions::home_of(&path, Some(root.as_path()));
        let author = self.revision_author();
        let slot: Arc<Slot> = Arc::new((Mutex::new(Outcome::Queued), Condvar::new()));
        self.pane_mut().saving = Some(Saving {
            slot: slot.clone(),
            path: path.clone(),
            shown: shown.clone(),
            version,
        });
        let work = {
            let slot = slot.clone();
            cx.background_executor().spawn(async move {
                run_quiet(&slot, || {
                    platform::vault::save_quiet(home.as_ref(), &path, &shown, &base, &author)
                });
            })
        };
        cx.spawn(async move |this, cx| {
            work.await;
            this.update(cx, |this, cx| this.quiet_done(&editor, &slot, cx))
                .ok();
        })
        .detach();
    }

    /// The UI side of a background save: its outcome is applied to the pane that holds the note. A file
    /// another program changed meanwhile is merged here, as the synchronous save does.
    fn quiet_done(&mut self, editor: &Entity<Editor>, slot: &Arc<Slot>, cx: &mut Context<Self>) {
        let Some(index) = self.panes.iter().position(|pane| pane.editor == *editor) else {
            return;
        };
        // Taken already by a save on the UI thread: nothing is left to apply.
        let Some(saving) = self.panes[index]
            .saving
            .take_if(|saving| Arc::ptr_eq(&saving.slot, slot))
        else {
            return;
        };
        let Some(result) = saving.settle() else {
            return;
        };
        self.in_pane(index, |this| {
            match this.applied_quiet(&saving, result, cx) {
                Applied::Saved => {
                    this.count_unplaced_changes();
                    if this.pane().dirty {
                        this.schedule_save(cx);
                    }
                }
                Applied::Outside => {
                    this.flush_one(cx);
                }
                Applied::Failed | Applied::Stale => {}
            }
        });
        cx.notify();
    }

    /// Applies what a background save did to the pane's note. Nothing is applied when the note has been
    /// renamed or closed meanwhile: its own save takes it from there.
    // ponytail: the count of unplaced changes and the link follow of a renamed heading still read or
    // write the disk on the UI thread; each is one small file.
    fn applied_quiet(
        &mut self,
        saving: &Saving,
        result: io::Result<platform::vault::Quiet>,
        cx: &mut Context<Self>,
    ) -> Applied {
        if self.pane().path.as_ref() != Some(&saving.path) {
            return Applied::Stale;
        }
        match result {
            Ok(platform::vault::Quiet::Saved { content }) => {
                // Typing went on meanwhile: the note stays dirty, and the next save takes the rest.
                if self.pane().editor.read(cx).version() == saving.version {
                    self.pane_mut().dirty = false;
                }
                self.saved(
                    Some(saving.path.clone()),
                    saving.path.clone(),
                    saving.shown.clone(),
                    content,
                    cx,
                );
                Applied::Saved
            }
            Ok(platform::vault::Quiet::Outside) => Applied::Outside,
            Err(e) => {
                let name = vault::stem(&saving.path);
                self.say(
                    Tone::Failed,
                    format!(
                        "{} « {name} » : {}",
                        tr("Note not saved", "Note non enregistrée"),
                        crate::lang::io_reason(&e)
                    ),
                    cx,
                );
                Applied::Failed
            }
        }
    }

    /// A save on the UI thread needs the file: the background save of the pane's note, if one is running,
    /// is settled first (`Saving::settle`), and its outcome applied.
    pub(super) fn settle_saving(&mut self, cx: &mut Context<Self>) {
        if let Some(saving) = self.pane_mut().saving.take()
            && let Some(result) = saving.settle()
        {
            self.applied_quiet(&saving, result, cx);
        }
    }
}

/// A debounced save running off the UI thread (`Shell::start_quiet_save`): what it was given, and its
/// outcome, which whoever comes first takes.
pub(super) struct Saving {
    slot: Arc<Slot>,
    pub(super) path: PathBuf,
    pub(super) shown: String,
    pub(super) version: u64,
}

type Slot = (Mutex<Outcome>, Condvar);

/// Where a background save stands. `Done` is a save that was cancelled, or whose outcome was taken.
enum Outcome {
    Queued,
    Running,
    Finished(io::Result<platform::vault::Quiet>),
    Done,
}

/// What applying a background save came to.
enum Applied {
    Saved,
    /// The file changed since it was read: the caller merges it.
    Outside,
    /// The error is shown already.
    Failed,
    /// The note was renamed or closed meanwhile.
    Stale,
}

impl Saving {
    /// The outcome of the save, for its caller to apply. A save that has not started is cancelled and
    /// writes nothing (`None`); one that is running is waited for, which ends: the worker never waits on
    /// the UI thread.
    pub(super) fn settle(&self) -> Option<io::Result<platform::vault::Quiet>> {
        let (lock, done) = &*self.slot;
        let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            match &*outcome {
                Outcome::Running => {
                    outcome = done.wait(outcome).unwrap_or_else(|e| e.into_inner());
                }
                Outcome::Queued => {
                    *outcome = Outcome::Done;
                    return None;
                }
                Outcome::Done => return None,
                Outcome::Finished(_) => break,
            }
        }
        match std::mem::replace(&mut *outcome, Outcome::Done) {
            Outcome::Finished(result) => Some(result),
            _ => None,
        }
    }
}

/// Runs `save` for `slot`, unless the UI thread cancelled it first. The lock is not held while it runs.
fn run_quiet(slot: &Slot, save: impl FnOnce() -> io::Result<platform::vault::Quiet>) {
    let (lock, done) = slot;
    {
        let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(*outcome, Outcome::Queued) {
            return;
        }
        *outcome = Outcome::Running;
    }
    let result = save();
    let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
    *outcome = Outcome::Finished(result);
    done.notify_all();
}
