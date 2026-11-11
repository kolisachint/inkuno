//! The command palette as the shell opens it: notes, commands, outline, comments, places, questions.

use super::*;

impl Shell {
    /// Asks for a line of free text (a URL) in the palette, then passes it to `then`.
    pub(super) fn ask_text(
        &mut self,
        label: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: fn(&mut Self, String, &mut Window, &mut Context<Self>),
    ) {
        let theme = self.theme;
        let palette = cx.new(|cx| Palette::prompt(label, "", theme, cx));
        cx.subscribe_in(&palette, window, move |this, _, event, window, cx| {
            this.palette = None;
            match this.vault {
                Some(_) => window.focus(&this.pane().editor.focus_handle(cx)),
                None => window.focus(&this.focus),
            }
            if let PaletteEvent::Submit(text) = event
                && !text.trim().is_empty()
            {
                then(this, text.trim().to_string(), window, cx);
            }
            cx.notify();
        })
        .detach();
        window.focus(&palette.focus_handle(cx));
        self.palette = Some(palette);
        cx.notify();
    }

    /// The note's comments, in the same list as its outline: the commented text, then
    /// what is said about it.
    pub(super) fn open_comments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.comments_shut = false;
        let said = markdown::all_comments(self.pane().editor.read(cx).text())
            .into_iter()
            .map(|n| {
                let who = if n.noted.is_empty() {
                    n.scope.clone()
                } else {
                    n.noted.to_string()
                };
                (
                    if who.is_empty() {
                        n.said.to_string()
                    } else {
                        format!("{who} — {}", n.said)
                    },
                    n.at,
                )
            })
            .collect();
        let none = tr(
            "This note has no comments",
            "Cette note n'a pas de commentaires",
        );
        self.open_places(tr("Comments", "Commentaires"), none, said, window, cx)
    }

    /// List of places in the note (label, byte), the one we are at preselected; with
    /// none, `none` says so.
    pub(super) fn open_places(
        &mut self,
        title: &str,
        none: &str,
        heads: Vec<(String, usize)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.vault.is_none() || self.pane().file.is_some() {
            return;
        }
        let before = self.pane().editor.read(cx).position();
        if heads.is_empty() {
            self.palette = None;
            self.say(Tone::Info, none, cx);
            window.focus(&self.pane().editor.focus_handle(cx));
            return cx.notify();
        }
        let current = heads.iter().rposition(|(_, at)| *at <= before).unwrap_or(0);
        let (theme, options) = (
            self.theme,
            heads.iter().map(|(label, _)| label.clone()).collect(),
        );
        let palette = cx.new(|cx| Palette::choose(title, options, "", theme, cx).select(current));
        cx.subscribe_in(&palette, window, move |this, palette, event, window, cx| {
            let to = match event {
                PaletteEvent::Preview(_) | PaletteEvent::Submit(_) => {
                    palette.read(cx).chosen().and_then(|i| heads.get(i))
                }
                _ => None,
            };
            let (at, top) = to.map_or((before, false), |(_, at)| (*at, true));
            this.pane().editor.update(cx, |e, cx| e.jump(at, top, cx));
            if !matches!(event, PaletteEvent::Preview(_)) {
                this.palette = None;
                window.focus(&this.pane().editor.focus_handle(cx));
            }
            cx.notify();
        })
        .detach();
        window.focus(&palette.focus_handle(cx));
        self.palette = Some(palette);
        cx.notify();
    }

    pub(super) fn open_palette(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_palette(query, window, cx)
    }

    pub(super) fn show_palette(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.vault.is_none() {
            return;
        }
        self.flush(cx);
        let now = std::time::SystemTime::now();
        let entries = self
            .by_recency()
            .into_iter()
            .map(|n| Entry {
                name: n.name.clone(),
                path: n.path.clone(),
                tags: n.tags.clone(),
                days: now
                    .duration_since(n.mtime)
                    .ok()
                    .map(|age| age.as_secs() / 86_400),
                body: n.body.clone(),
                lower: n.lower.clone(),
            })
            // The vault's other files (tables, images, diagrams) are also found by their name.
            .chain(self.images.iter().map(|path| {
                Entry {
                    name: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: path.clone(),
                    ..Entry::default()
                }
            }))
            .collect();
        let root = self.vault.clone().unwrap_or_default();
        let theme = self.theme;
        let updates = self.prefs.updates;
        let installable = self
            .update
            .as_ref()
            .filter(|r| r.asset.is_some())
            .map(|r| r.version.clone());
        let table = self.shown_sheet().is_some();
        let lists =
            self.pane().file.is_none() && self.pane().listing.is_none() && !self.board_shown(cx);
        let cells = lists && self.pane().editor.read(cx).in_table();
        let swap = !self.swapped.is_empty();
        let (commands, here) = self.palette_commands(window, cx);
        let context = self.here_group(cx).1;
        let palette = cx.new(|cx| {
            Palette::new(entries, query, theme, cx)
                .with_root(root)
                .with_updates(updates, installable)
                .with_table(table)
                .with_lists(lists)
                .with_cells(cells)
                .with_undo_swap(swap)
                .with_commands(commands, here, context)
        });
        // A command chosen in the palette goes to what had input before it.
        let back = window.focused(cx);
        cx.subscribe_in(&palette, window, move |this, palette, event, window, cx| {
            this.palette = None;
            window.focus(&this.pane().editor.focus_handle(cx));
            let aside = palette.read(cx).aside;
            match event {
                PaletteEvent::Open(path) if aside => this.open_aside(path, window, cx),
                PaletteEvent::Open(path) => this.open(path, window, cx),
                PaletteEvent::Create(name) => this.open_wiki(name, window, cx),
                PaletteEvent::ChangeVault => this.choose_vault(window, cx),
                PaletteEvent::Help => this.set_help(true, window, cx),
                PaletteEvent::NewDiagram => this.new_diagram(window, cx),
                PaletteEvent::NewFolder => this.new_folder(window, cx),
                PaletteEvent::Table(pick) => this.choose_table(*pick, window, cx),
                PaletteEvent::ImportDiagram => this.import_diagram(window, cx),
                PaletteEvent::Setting(setting) => this.choose_setting(*setting, window, cx),
                PaletteEvent::ToggleUpdates => this.toggle_updates(cx),
                PaletteEvent::CheckUpdate => this.check_now(cx),
                PaletteEvent::Outline => this.toggle_outline(window, cx),
                PaletteEvent::Comments => this.open_comments(window, cx),
                PaletteEvent::Grid(op) => {
                    // The gesture goes to what is shown: a CSV's sheet, or the note.
                    match this.shown_sheet() {
                        Some(sheet) => window.focus(&sheet.focus_handle(cx)),
                        None => window.focus(&this.pane().editor.focus_handle(cx)),
                    }
                    window.dispatch_action(op.action(), cx);
                }
                PaletteEvent::Run(name) => {
                    match &back {
                        Some(back) => window.focus(back),
                        None => this.focus_pane(this.active, window, cx),
                    }
                    if let Ok(action) = cx.build_action(name, None) {
                        window.dispatch_action(action, cx);
                    }
                }
                PaletteEvent::Restyle(style) => {
                    window.focus(&this.pane().editor.focus_handle(cx));
                    this.pane().editor.update(cx, |e, cx| e.restyle(*style, cx));
                }
                PaletteEvent::UndoSwap => this.undo_swap(cx),
                PaletteEvent::NewBoard => {
                    this.fresh_tab(window, cx);
                    this.new_note(kanban::new_board(), cx);
                }
                PaletteEvent::Today => this.open_today(window, cx),
                PaletteEvent::Travel(forward) => this.travel(*forward, window, cx),
                PaletteEvent::Trash => this.open_trash(window, cx),
                PaletteEvent::Backup => this.choose_backup(window, cx),
                PaletteEvent::Sync => this.git_sync(window, cx),
                PaletteEvent::CloneVault => this.ask_clone(window, cx),
                PaletteEvent::Terminal => this.open_terminal(cx),
                PaletteEvent::CaptureKey => this.capture_shortcut(cx),
                PaletteEvent::InstallUpdate => this.install_update(cx),
                // Closed without a choice: we get back what the pane showed, table or diagram included.
                PaletteEvent::Dismiss => this.focus_pane(this.active, window, cx),
                PaletteEvent::Submit(_) | PaletteEvent::Preview(_) => {}
            }
            cx.notify();
        })
        .detach();
        window.focus(&palette.focus_handle(cx));
        self.palette = Some(palette);
        cx.notify();
    }

    /// The catalogue commands the view can do, with their keys, the view's own first
    /// (how many: the second number). Those it cannot do are not offered.
    pub(super) fn palette_commands(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Vec<commands::Command>, usize) {
        use commands::Group;
        let available: std::collections::HashSet<&str> = window
            .available_actions(cx)
            .iter()
            .map(|action| action.name())
            .collect();
        let here = self.here_group(cx).0;
        let mut list: Vec<commands::Command> = commands::catalog()
            .into_iter()
            .filter(|(action, ..)| available.contains(action))
            // Gestures of a block, a table or a diagram only count there: elsewhere, they would do nothing.
            .filter(|&(_, _, _, group)| {
                !matches!(group, Group::Block | Group::Table | Group::Diagram)
                    || Some(group) == here
            })
            .filter_map(|(action, label, icon, group)| {
                let keys = keys::of(cx, cx.build_action(action, None).ok()?.as_ref());
                Some(commands::Command {
                    action,
                    label,
                    icon,
                    group,
                    keys,
                })
            })
            .collect();
        // Aliases: actions found by typing what they do. Most name chorded actions the
        // catalogue does not (multicursor, column alignment); the block transforms below
        // repeat catalogue lines on purpose (overrule 2026-11-08: `›` stays a blessed path
        // next to `/` and the grip). Same availability and Group rules as the catalogue
        // lines above; the keys shown come from the bindings in effect. An alias the
        // catalogue already names is set aside, so no action lists twice.
        let aliases: Vec<(&str, &'static str, &'static str, Group)> = vec![
            (
                "editor::SelectNext",
                tr(
                    "Multicursor: select the next occurrence",
                    "Multicurseur : prendre l'occurrence suivante",
                ),
                "in-selection.svg",
                Group::Note,
            ),
            (
                "editor::AlignLeft",
                tr(
                    "Align the column of the cursor to the left",
                    "Aligner la colonne du curseur à gauche",
                ),
                "table.svg",
                Group::Table,
            ),
            (
                "editor::AlignCenter",
                tr(
                    "Center the column of the cursor",
                    "Centrer la colonne du curseur",
                ),
                "table.svg",
                Group::Table,
            ),
            (
                "editor::AlignRight",
                tr(
                    "Align the column of the cursor to the right",
                    "Aligner la colonne du curseur à droite",
                ),
                "table.svg",
                Group::Table,
            ),
            (
                "editor::BlockHeading1",
                tr("Turn into heading 1", "Transformer en titre 1"),
                "c-heading.svg",
                Group::Block,
            ),
            (
                "editor::BlockHeading2",
                tr("Turn into heading 2", "Transformer en titre 2"),
                "c-heading.svg",
                Group::Block,
            ),
            (
                "editor::BlockHeading3",
                tr("Turn into heading 3", "Transformer en titre 3"),
                "c-heading.svg",
                Group::Block,
            ),
            (
                "editor::BlockBullet",
                tr("Turn into a bulleted list", "Transformer en liste à puces"),
                "list-bullet.svg",
                Group::Block,
            ),
            (
                "editor::BlockOrdered",
                tr(
                    "Turn into a numbered list",
                    "Transformer en liste numérotée",
                ),
                "list-number.svg",
                Group::Block,
            ),
            (
                "editor::BlockTask",
                tr("Turn into a task list", "Transformer en liste de tâches"),
                "list-task.svg",
                Group::Block,
            ),
            (
                "editor::BlockQuote",
                tr("Turn into a quote", "Transformer en citation"),
                "c-quote.svg",
                Group::Block,
            ),
            (
                "editor::BlockCode",
                tr("Turn into a code block", "Transformer en bloc de code"),
                "code.svg",
                Group::Block,
            ),
            (
                "editor::BlockDuplicate",
                tr("Duplicate the block", "Dupliquer le bloc"),
                "copy.svg",
                Group::Block,
            ),
            (
                "app::ToggleFocus",
                tr(
                    "Focus mode: hide the sidebars, the note alone",
                    "Mode focus : masquer les panneaux, la note seule",
                ),
                "expand.svg",
                Group::Look,
            ),
        ];
        let seen: std::collections::HashSet<&str> = list.iter().map(|c| c.action).collect();
        list.extend(
            aliases
                .into_iter()
                .filter(|(action, ..)| !seen.contains(action))
                .filter(|(action, ..)| available.contains(action))
                .filter(|&(_, _, _, group)| {
                    !matches!(group, Group::Block | Group::Table | Group::Diagram)
                        || Some(group) == here
                })
                .filter_map(|(action, label, icon, group)| {
                    let keys = keys::of(cx, cx.build_action(action, None).ok()?.as_ref());
                    Some(commands::Command {
                        action,
                        label,
                        icon,
                        group,
                        keys,
                    })
                }),
        );
        commands::arrange(list, here)
    }

    /// The command palette (F10, or the rail's first button): the palette, only the commands.
    pub(super) fn open_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() {
            return;
        }
        self.show_palette(">", window, cx)
    }
}
