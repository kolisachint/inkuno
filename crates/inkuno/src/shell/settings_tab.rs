//! The Settings tab: a tab of a pane that shows pages in place of its note, with a mini-nav
//! (Appearance, Editor, Files, Shortcuts, About). Each control writes the preferences the palette's
//! choice lists write (`settings.rs`); the Shortcuts page is the help's list (`help.rs`). Nothing
//! typed here reaches a note: the tab's own editor is never shown.

use super::*;
use crate::ui::components::{
    button::button as plain_button,
    card::{card, card_row},
    segmented::{segment, segmented as seg_track},
    switch::switch as sw,
};
use crate::ui::theme::Page;
use crate::views::field::{Field, FieldEvent};
use gpui::{FontWeight, MouseButton, Stateful};

/// A page of the Settings tab, in the order of the mini-nav.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Section {
    Appearance,
    Editor,
    Files,
    Shortcuts,
    About,
}

impl Section {
    pub(super) const ALL: [Section; 5] = [
        Section::Appearance,
        Section::Editor,
        Section::Files,
        Section::Shortcuts,
        Section::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::Appearance => tr("Appearance", "Apparence"),
            Section::Editor => tr("Editor", "Éditeur"),
            Section::Files => tr("Files", "Fichiers"),
            Section::Shortcuts => tr("Shortcuts", "Raccourcis"),
            Section::About => tr("About", "À propos"),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Section::Appearance => "appearance",
            Section::Editor => "editor",
            Section::Files => "files",
            Section::Shortcuts => "shortcuts",
            Section::About => "about",
        }
    }
}

/// The Settings tab of a pane: the page shown, its keyboard focus, and the fields that type text.
pub(super) struct SettingsTab {
    pub(super) section: Section,
    pub(super) focus: FocusHandle,
    pub(super) scroll: gpui::ScrollHandle,
    /// Revision author and assistant command: saved as they are typed, or on Enter.
    pub(super) author: Entity<Field>,
    pub(super) assistant: Entity<Field>,
}

/// What a click on a control does.
type Pick = Box<dyn Fn(&mut Shell, &mut Window, &mut Context<Shell>)>;

/// A click that edits the preferences: a choice, a switch.
fn pick(edit: impl Fn(&mut Prefs) + 'static) -> Pick {
    Box::new(
        move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
            this.edit_prefs(window, cx, |prefs| edit(prefs))
        },
    )
}

/// A segmented control: one option is chosen, a click on another one picks it.
fn segmented(
    t: Theme,
    id: &str,
    options: Vec<(String, bool, Pick)>,
    cx: &mut Context<Shell>,
) -> Div {
    seg_track(
        t,
        options
            .into_iter()
            .enumerate()
            .map(|(i, (label, on, pick))| {
                segment(format!("{id}-{i}"), label, on, t)
                    .on_click(cx.listener(move |this, _, window, cx| pick(this, window, cx)))
            })
            .collect(),
    )
}

/// A switch: its click edits the preferences.
fn switch(t: Theme, id: &str, on: bool, pick: Pick, cx: &mut Context<Shell>) -> Stateful<Div> {
    sw(id, on, t).on_click(cx.listener(move |this, _, window, cx| pick(this, window, cx)))
}

/// A button of the rows: its label, and what a click does.
fn button(t: Theme, id: &str, label: String, pick: Pick, cx: &mut Context<Shell>) -> Stateful<Div> {
    plain_button(id, label, t)
        .on_click(cx.listener(move |this, _, window, cx| pick(this, window, cx)))
}

/// A link of the About page: an icon and a label, opened in the browser.
fn link(
    t: Theme,
    id: &'static str,
    icon: &'static str,
    label: &str,
    url: &'static str,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_none()
        .h(t.px(26.))
        .px_2p5()
        .flex()
        .items_center()
        .gap_1p5()
        .rounded(t.small())
        .text_size(t.px(12.))
        .text_color(t.accent)
        .cursor_pointer()
        .hover(|s| s.bg(t.hover))
        .child(
            svg()
                .path(icon)
                .size(t.px(14.))
                .flex_none()
                .text_color(t.accent),
        )
        .child(label.to_string())
        .on_mouse_down(MouseButton::Left, move |_, _, cx| cx.open_url(url))
}

fn title(t: Theme, text: &str) -> Div {
    div()
        .text_size(t.px(22.))
        .font_weight(FontWeight::BOLD)
        .text_color(t.text)
        .child(text.to_string())
}

impl Shell {
    /// Shows the Settings tab on `section`: the one tab of the window comes to the front, or opens
    /// in a tab of its own right of the shown one. Input goes to its page.
    pub(super) fn open_settings(
        &mut self,
        section: Section,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab = match self.panes.iter().position(|pane| pane.settings.is_some()) {
            Some(tab) => tab,
            None => {
                self.add_tab(window, cx);
                let theme = self.theme;
                let (author, assistant) = (self.prefs.author.clone(), self.prefs.assistant.clone());
                let author = cx.new(|cx| Field::new(&author, theme, cx));
                let assistant = cx.new(|cx| Field::new(&assistant, theme, cx));
                cx.subscribe_in(
                    &author,
                    window,
                    |this, author, event: &FieldEvent, _, cx| {
                        if let FieldEvent::Changed = event {
                            let text = author.read(cx).line.text.clone();
                            this.prefs.author = logic::revision::clean_author(&text);
                            this.save_prefs();
                        }
                    },
                )
                .detach();
                cx.subscribe_in(
                    &assistant,
                    window,
                    |this, assistant, event: &FieldEvent, _, cx| {
                        match event {
                            FieldEvent::Done => {
                                let text = assistant.read(cx).line.text.trim().to_string();
                                let changed = text != this.prefs.assistant;
                                this.prefs.assistant = text;
                                this.save_prefs();
                                // Another command: the agent that runs is stopped, the next prompt starts the new one.
                                if changed {
                                    this.assistant_halt(cx);
                                }
                                this.assistant_check(cx);
                            }
                            FieldEvent::Cancel => {
                                let saved = this.prefs.assistant.clone();
                                assistant.update(cx, |field, cx| field.set(&saved, cx));
                            }
                            FieldEvent::Changed => {}
                        }
                    },
                )
                .detach();
                let page = SettingsTab {
                    section,
                    focus: cx.focus_handle(),
                    scroll: Default::default(),
                    author,
                    assistant,
                };
                let tab = self.active;
                self.panes[tab].settings = Some(page);
                // Its own editor is never shown, and read-only: no text can reach it, even by a stray focus.
                self.panes[tab]
                    .editor
                    .update(cx, |editor, _| editor.set_readonly(true));
                tab
            }
        };
        self.ensure_find(window, cx);
        // The shortcuts page opens with its search empty, as the help panel did.
        if let (Section::Shortcuts, Some(find)) = (section, &self.help.find) {
            find.update(cx, |find, cx| {
                find.line = line::Line::default();
                cx.notify();
            });
        }
        let (author, assistant) = (self.prefs.author.clone(), self.prefs.assistant.clone());
        if let Some(page) = self.panes[tab].settings.as_mut() {
            page.section = section;
            let (a, b) = (page.author.clone(), page.assistant.clone());
            a.update(cx, |field, cx| field.set(&author, cx));
            b.update(cx, |field, cx| field.set(&assistant, cx));
        }
        self.help.sel = None;
        self.help.edit = None;
        self.active = tab;
        self.focus_pane(tab, window, cx);
        self.focus_section(window, cx);
        cx.notify();
    }

    /// The page the window's Settings tab shows, `None` when there is no such tab.
    #[cfg(test)]
    pub(super) fn settings_section(&self) -> Option<Section> {
        self.panes
            .iter()
            .find_map(|pane| pane.settings.as_ref().map(|page| page.section))
    }

    /// The mini-nav: another page of the Settings tab.
    pub(super) fn set_section(
        &mut self,
        section: Section,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(page) = self.pane_mut().settings.as_mut() {
            page.section = section;
        }
        self.help.edit = None;
        self.focus_section(window, cx);
        cx.notify();
    }

    /// Gives input to the Settings tab's page: the search field on Shortcuts, the page otherwise.
    pub(super) fn focus_section(&self, window: &mut Window, cx: &mut App) {
        if let Some(focus) = self.settings_focus(cx) {
            window.focus(&focus, cx);
        }
    }

    /// Where input goes in the active pane when it is the Settings tab: `None` for a note.
    pub(super) fn settings_focus(&self, cx: &App) -> Option<FocusHandle> {
        let page = self.pane().settings.as_ref()?;
        match (page.section, self.help.find.as_ref()) {
            (Section::Shortcuts, Some(find)) => Some(find.focus_handle(cx)),
            _ => Some(page.focus.clone()),
        }
    }

    /// Gives input back to what the active pane shows: its note, or its Settings page. Used when a
    /// palette or a prompt closes.
    pub(super) fn focus_note(&self, window: &mut Window, cx: &mut App) {
        match self.settings_focus(cx) {
            Some(focus) => window.focus(&focus, cx),
            None => window.focus(&self.pane().editor.focus_handle(cx), cx),
        }
    }

    /// The search field of the shortcuts page, made once: its subscriptions stay for the window.
    pub(super) fn ensure_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.help.find.is_some() {
            return;
        }
        let theme = self.theme;
        let find = cx.new(|cx| Field::new("", theme, cx));
        cx.observe(&find, |_, _, cx| cx.notify()).detach();
        // Enter on the picked row: its keys are awaited. Escape clears the field.
        cx.subscribe_in(
            &find,
            window,
            |this, find, event: &FieldEvent, _, cx| match event {
                FieldEvent::Done => {
                    if let Some(action) = this
                        .help
                        .sel
                        .filter(|sel| this.help_actions(cx).contains(sel))
                    {
                        this.key_edit(action, cx);
                    }
                }
                FieldEvent::Cancel => find.update(cx, |find, cx| {
                    find.line = line::Line::default();
                    cx.notify();
                }),
                FieldEvent::Changed => {}
            },
        )
        .detach();
        self.help.find = Some(find);
    }

    /// Changes the preferences the palette's choices change: saved, and applied to the window.
    pub(super) fn edit_prefs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut Prefs),
    ) {
        edit(&mut self.prefs);
        self.save_prefs();
        self.restyle(window, cx);
    }

    /// The Settings tab of the active pane, drawn in place of its note.
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let Some(page) = self.pane().settings.as_ref() else {
            return div();
        };
        let section = page.section;
        let nav = div()
            .flex_none()
            .w(t.px(180.))
            .h_full()
            .flex()
            .flex_col()
            .gap(t.px(2.))
            .py(t.px(22.))
            .px(t.px(10.))
            .children(Section::ALL.map(|item| {
                let on = item == section;
                div()
                    .id(gpui::SharedString::from(format!("settings-{}", item.key())))
                    .debug_selector(move || format!("settings-{}", item.key()))
                    .px(t.px(10.))
                    .py(t.px(6.))
                    .rounded(t.small())
                    .text_size(t.px(13.))
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.hover).text_color(t.text))
                    .when(!on, |d| {
                        d.text_color(t.soft)
                            .hover(|s| s.bg(t.hover).text_color(t.text))
                    })
                    .child(item.label())
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.set_section(item, window, cx)),
                    )
            }));
        let body = match section {
            // Capped like the other pages (640 px), so the description and the keys keep a reading width.
            Section::Shortcuts => div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .px(t.px(40.))
                .pt(t.px(28.))
                .child(
                    div()
                        .w_full()
                        .max_w(t.px(640.))
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .child(title(t, section.label()))
                        .child(self.render_shortcuts(cx)),
                )
                .into_any_element(),
            _ => {
                let rows = match section {
                    Section::Appearance => self.appearance_page(cx),
                    Section::Editor => self.editor_page(page, cx),
                    Section::Files => self.files_page(cx),
                    _ => self.about_page(),
                };
                let mut column = div()
                    .w_full()
                    .max_w(t.px(640.))
                    .mx_auto()
                    .px(t.px(40.))
                    .pt(t.px(28.))
                    .pb(t.px(80.))
                    .flex()
                    .flex_col()
                    .gap(t.px(18.))
                    .child(title(t, section.label()));
                for group in rows {
                    column = column.child(group);
                }
                div()
                    .id("settings-page")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&page.scroll)
                    .child(column)
                    .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .track_focus(&page.focus)
            .child(nav)
            .child(body)
    }

    fn appearance_page(&self, cx: &mut Context<Self>) -> Vec<Div> {
        let (t, p) = (self.theme, &self.prefs);
        let theme = segmented(
            t,
            "theme",
            vec![
                (
                    tr("System", "Système").into(),
                    p.theme.is_empty(),
                    pick(|p| p.theme.clear()),
                ),
                (
                    tr("Light", "Clair").into(),
                    p.theme == "Light",
                    pick(|p| p.theme = "Light".into()),
                ),
                (
                    tr("Dark", "Sombre").into(),
                    p.theme == "Dark",
                    pick(|p| p.theme = "Dark".into()),
                ),
            ],
            cx,
        );
        let more = button(
            t,
            "theme-more",
            tr("More themes…", "Plus de thèmes…").into(),
            Box::new(
                |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.choose_setting(Setting::Theme, window, cx)
                },
            ),
            cx,
        );
        let size = [14., 15., 16., 18.].map(|size: f32| {
            (
                format!("{size}"),
                (p.size - size).abs() < 0.5,
                Box::new(
                    move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                        this.edit_prefs(window, cx, |p| p.size = size)
                    },
                ) as Pick,
            )
        });
        let size = segmented(t, "size", size.into(), cx);
        let width = Page::ALL.map(|page| {
            (
                page.label().to_string(),
                Page::of(&p.page) == page,
                pick(move |p| p.page = page.key().into()),
            )
        });
        let width = segmented(t, "width", width.into(), cx);
        let spacing = [
            (tr("Compact", "Serré"), "compact"),
            (tr("Normal", "Normal"), "normal"),
            (tr("Relaxed", "Aéré"), "relaxed"),
            (tr("Loose", "Large"), "loose"),
        ]
        .map(|(label, key)| {
            (
                label.to_string(),
                p.spacing.trim() == key || (key == "normal" && p.spacing.trim().is_empty()),
                pick(move |p| p.spacing = key.into()),
            )
        });
        let spacing = segmented(t, "spacing", spacing.into(), cx);
        let motion = p.motion != "off";
        let animations = switch(
            t,
            "motion",
            motion,
            Box::new(
                move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.edit_prefs(window, cx, |p| {
                        p.motion = (if motion { "off" } else { "on" }).into()
                    })
                },
            ),
            cx,
        );
        vec![card(
            t,
            vec![
                card_row(
                    t,
                    tr("Theme", "Thème"),
                    tr(
                        "Quiet follows the system unless a light or dark theme is chosen",
                        "Quiet suit le système sauf si un thème clair ou sombre est choisi",
                    ),
                    div().flex().gap_2().child(theme).child(more),
                ),
                card_row(
                    t,
                    tr("Text size", "Taille du texte"),
                    if cfg!(target_os = "macos") {
                        tr("Also ⌘+ and ⌘−", "Aussi ⌘+ et ⌘−")
                    } else {
                        tr("Also Ctrl+ and Ctrl−", "Aussi Ctrl+ et Ctrl−")
                    },
                    size,
                ),
                card_row(
                    t,
                    tr("Line spacing", "Interligne"),
                    tr(
                        "The space between the lines of the text and of code",
                        "L'espace entre les lignes du texte et du code",
                    ),
                    spacing,
                ),
                card_row(
                    t,
                    tr("Page width", "Largeur de page"),
                    tr(
                        "The column the text is set in",
                        "La colonne où le texte est composé",
                    ),
                    width,
                ),
                card_row(
                    t,
                    tr("Animations", "Animations"),
                    tr(
                        "Short fades when a panel or a folder opens",
                        "De courts fondus à l'ouverture d'un panneau ou d'un dossier",
                    ),
                    animations,
                ),
            ],
        )]
    }

    fn editor_page(&self, page: &SettingsTab, cx: &mut Context<Self>) -> Vec<Div> {
        let (t, p) = (self.theme, &self.prefs);
        let named = |name: &str, fallback: &'static str| {
            if name.is_empty() {
                fallback.to_string()
            } else {
                name.to_string()
            }
        };
        let font = button(
            t,
            "font",
            named(&p.font, tr("Default", "Par défaut")),
            Box::new(
                |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.choose_setting(Setting::Font, window, cx)
                },
            ),
            cx,
        );
        let mono_font = button(
            t,
            "mono",
            named(&p.mono, tr("Default", "Par défaut")),
            Box::new(
                |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.choose_setting(Setting::Mono, window, cx)
                },
            ),
            cx,
        );
        let bar = p.bar != "off";
        let bar = switch(
            t,
            "bar",
            bar,
            Box::new(
                move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.edit_prefs(window, cx, |p| {
                        p.bar = (if bar { "off" } else { "on" }).into()
                    })
                },
            ),
            cx,
        );
        let gutter = p.gutter != "off";
        let gutter = switch(
            t,
            "gutter",
            gutter,
            Box::new(
                move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.edit_prefs(window, cx, |p| {
                        p.gutter = (if gutter { "off" } else { "on" }).into()
                    })
                },
            ),
            cx,
        );
        // A field sits on the window's ground with a line border; focused, a 2 px accent ring.
        let field = |field: &Entity<Field>, cx: &App| {
            let focus = field.read(cx).focus_handle(cx);
            div()
                .w(t.px(240.))
                .h(t.px(28.))
                .px_2()
                .flex()
                .items_center()
                .min_w_0()
                .overflow_hidden()
                .rounded(t.small())
                .bg(t.ground)
                .border_1()
                .border_color(t.line)
                .text_size(t.px(13.))
                .track_focus(&focus)
                .focus(|s| s.border_2().border_color(t.accent))
                .child(field.clone())
        };
        vec![
            card(
                t,
                vec![
                    card_row(
                        t,
                        tr("Font", "Police"),
                        tr("The text of the notes", "Le texte des notes"),
                        font,
                    ),
                    card_row(
                        t,
                        tr("Code font", "Police du code"),
                        tr(
                            "Code blocks and inline code",
                            "Blocs de code et code en ligne",
                        ),
                        mono_font,
                    ),
                    card_row(
                        t,
                        tr("Formatting bar", "Barre de mise en forme"),
                        tr(
                            "Shown above a selection",
                            "Affichée au-dessus d'une sélection",
                        ),
                        bar,
                    ),
                    card_row(
                        t,
                        tr("Code line numbers", "Numéros des lignes de code"),
                        tr(
                            "Numbers beside the lines of a code block",
                            "Numéros à côté des lignes d'un bloc de code",
                        ),
                        gutter,
                    ),
                ],
            ),
            card(
                t,
                vec![
                    card_row(
                        t,
                        tr("Revision author", "Auteur des versions"),
                        tr(
                            "Empty: INKUNO_AUTHOR, else your user name",
                            "Vide : INKUNO_AUTHOR, sinon votre nom d'utilisateur",
                        ),
                        field(&page.author, cx),
                    ),
                    card_row(
                        t,
                        tr("Assistant command", "Commande de l'assistant"),
                        tr(
                            "Enter saves. Empty: hoocode",
                            "Entrée enregistre. Vide : hoocode",
                        ),
                        field(&page.assistant, cx),
                    ),
                ],
            ),
        ]
    }

    fn files_page(&self, cx: &mut Context<Self>) -> Vec<Div> {
        let (t, p) = (self.theme, &self.prefs);
        let restore = p.restore != "off";
        let restore = switch(
            t,
            "restore",
            restore,
            Box::new(
                move |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    this.edit_prefs(window, cx, |p| {
                        p.restore = (if restore { "off" } else { "on" }).into()
                    })
                },
            ),
            cx,
        );
        let vault_folder = self
            .vault
            .as_ref()
            .map(|root| vault::shown(root))
            .unwrap_or_else(|| tr("No vault open", "Aucun coffre ouvert").to_string());
        let trash = self
            .vault
            .as_ref()
            .map(|root| vault::shown(&root.join(".trash")))
            .unwrap_or_default();
        let scratch = vault::shown(&platform::scratch::dir());
        vec![card(
            t,
            vec![
                card_row(
                    t,
                    tr("Vault folder", "Dossier du coffre"),
                    &vault_folder,
                    button(
                        t,
                        "vault",
                        tr("Change…", "Changer…").into(),
                        Box::new(
                            |this: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                                this.choose_vault(window, cx)
                            },
                        ),
                        cx,
                    ),
                ),
                card_row(
                    t,
                    tr("Scratch folder", "Dossier des brouillons"),
                    &scratch,
                    div(),
                ),
                card_row(t, tr("Trash", "Corbeille"), &trash, div()),
                card_row(
                    t,
                    tr(
                        "Open last notes on start",
                        "Rouvrir les dernières notes au lancement",
                    ),
                    tr(
                        "The tabs of last time come back when Inkuno starts",
                        "Les onglets de la dernière fois reviennent au lancement",
                    ),
                    restore,
                ),
                card_row(
                    t,
                    tr("Config files", "Fichiers de configuration"),
                    tr(
                        "themes.json and keybindings.json are reloaded live when saved",
                        "themes.json et keybindings.json sont rechargés dès l'enregistrement",
                    ),
                    button(
                        t,
                        "config",
                        tr("Show folder", "Afficher le dossier").into(),
                        Box::new(|_: &mut Shell, _: &mut Window, cx: &mut Context<Shell>| {
                            // The folder may not exist yet: it is made so that the file manager opens it.
                            let folder = vault::themes_path();
                            if let Some(dir) = folder.parent() {
                                let _ = std::fs::create_dir_all(dir);
                                cx.reveal_path(dir);
                            }
                        }),
                        cx,
                    ),
                ),
            ],
        )]
    }

    fn about_page(&self) -> Vec<Div> {
        let t = self.theme;
        vec![
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(logo(t))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(t.px(13.))
                        .text_color(t.soft)
                        .child(tr(
                            "Fast, minimal Markdown notes, in plain files you own.",
                            "Des notes Markdown rapides et minimales, dans de simples fichiers qui vous appartiennent.",
                        )),
                ),
            card(
                t,
                vec![
                    card_row(t, tr("Version", "Version"), "", div().text_size(t.px(13.)).text_color(t.soft).child(format!("inkuno {}", env!("CARGO_PKG_VERSION")))),
                    card_row(t, tr("Licence", "Licence"), tr("Free software, in the repository's LICENSE file", "Logiciel libre, voir le fichier LICENSE du dépôt"), div().text_size(t.px(13.)).text_color(t.soft).child("MIT")),
                ],
            ),
            card(
                t,
                vec![
                    card_row(
                        t,
                        tr("Source", "Code source"),
                        tr("The code, the issues and the releases", "Le code, les demandes et les versions"),
                        link(t, "about-github", "code.svg", "GitHub", env!("CARGO_PKG_REPOSITORY")),
                    ),
                    card_row(
                        t,
                        tr("Support", "Soutien"),
                        tr("Keeps the project going", "Fait vivre le projet"),
                        link(t, "about-kofi", "heart.svg", tr("Support on Ko-fi", "Soutenir sur Ko-fi"), KOFI),
                    ),
                ],
            ),
        ]
    }
}
