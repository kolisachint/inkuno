//! The default shortcuts, the help sections that list them, and their installation.

use super::*;

/// Content of the help panel, and with it the list of shortcuts that can be changed: one `k`
/// line per action, whose keys are the ones in force (`bound`); one `t` line for what
/// is not a binding (mouse, typed text, palette command, fixed keys).
pub(super) fn help_sections(bound: &[keys::Bind], changes: &keys::Changes) -> keys::Sections {
    use gpui::Action;
    use keys::Row;
    let k = |action: &dyn Action, effect: &'static str| Row {
        keys: keys::shown(bound, action.name()),
        effect,
        action: Some(action.name()),
        changed: changes.contains_key(action.name()),
    };
    let t = |keys: &str, effect: &'static str| Row::text(keys, effect);
    // A palette command: its keys, then the word to type in it.
    let p = |word: &str, effect: &'static str| {
        let palette = keys::shown(bound, OpenPalette.name());
        Row::text(
            format!(
                "{} › {word}",
                if palette.is_empty() {
                    "Palette"
                } else {
                    &palette
                }
            ),
            effect,
        )
    };
    let click = tr("click", "clic");
    let shift = tr("Shift", "Maj");
    let word = if cfg!(target_os = "macos") {
        "Alt"
    } else {
        "Ctrl"
    };
    vec![
        (
            "Notes",
            vec![
                k(&OpenPalette, tr("Find by name or text, create, filter by #tag", "Chercher par nom ou texte, créer, filtrer par #tag")),
                k(&ToggleCommands, tr("Command palette: every command, found by what it does (also the first button of the rail, or > in the palette)", "Palette des commandes : toutes les commandes, trouvées par ce qu'elles font (aussi le premier bouton du rail, ou > dans la palette)")),
                k(&editor::Find, tr("Find in the note", "Chercher dans la note")),
                k(&editor::FindReplace, tr("Find and replace in the note", "Chercher et remplacer dans la note")),
                k(&editor::FindNext, tr("Search bar: next match (Enter too)", "Barre de recherche : passage suivant (Entrée aussi)")),
                k(&editor::FindPrev, tr("Search bar: previous match", "Barre de recherche : passage précédent")),
                k(&editor::FindCase, tr("Search bar: match case", "Barre de recherche : respecter la casse")),
                k(&editor::FindWord, tr("Search bar: whole words", "Barre de recherche : mots entiers")),
                k(&editor::FindRegex, tr("Search bar: regular expression ($1 in the replacement)", "Barre de recherche : expression régulière ($1 dans le remplacement)")),
                t("Tab", tr("Search bar: replace field", "Barre de recherche : champ de remplacement")),
                k(&editor::ReplaceAll, tr("Search bar: replace all (Enter: this match)", "Barre de recherche : tout remplacer (Entrée : ce passage)")),
                k(&NewNote, tr("New note", "Nouvelle note")),
                k(&OpenVault, tr("Change vault", "Changer de coffre")),
                p("terminal", tr("Open a terminal in the vault", "Ouvrir un terminal dans le coffre")),
                p("clone", tr("Clone a vault from a git address (also on the welcome screen)", "Cloner un coffre depuis une adresse git (aussi sur l'écran d'accueil)")),
                t(tr("A cloud folder", "Un dossier synchronisé"), tr("iCloud, Dropbox, Google Drive, Syncthing: put the vault there, nothing else; never a git repository inside one", "iCloud, Dropbox, Google Drive, Syncthing : y poser le coffre, rien d'autre ; jamais un dépôt git dans un tel dossier")),
                k(&CopyAll, tr("Copy the code block, else the note", "Copier le bloc de code, sinon la note")),
                t(&format!("{MOD}+{click}"), tr("Open a [[link]], #tag or URL", "Ouvrir un [[lien]], #tag ou URL")),
                k(&ToggleHelp, tr("This help", "Cette aide")),
                k(&Quit, tr("Save and quit; stay open if saving fails", "Enregistrer et quitter ; rester ouvert si l’enregistrement échoue")),
            ],
        ),
        (
            "Navigation",
            vec![
                k(&SplitPane, tr("Split the pane: a new one on its right, opened on the palette to choose what goes there", "Découper le volet : un nouveau à sa droite, ouvert sur la palette pour choisir ce qui y va")),
                k(&SplitDown, tr("Split the pane: a new one below it", "Découper le volet : un nouveau en dessous")),
                k(&PaneLeft, tr("Go to the pane on the left", "Aller au volet de gauche")),
                k(&PaneRight, tr("Go to the pane on the right", "Aller au volet de droite")),
                k(&PaneUp, tr("Go to the pane above", "Aller au volet du dessus")),
                k(&PaneDown, tr("Go to the pane below", "Aller au volet du dessous")),
                k(&FocusLeft, tr("Go to the first pane", "Aller au premier volet")),
                k(&FocusRight, tr("Go to the second pane", "Aller au deuxième volet")),
                k(&TabToLeft, tr("Move the tab to the pane on the left (split off if there is none)", "Déplacer l'onglet vers le volet de gauche (découpé s'il n'y en a pas)")),
                k(&TabToRight, tr("Move the tab to the pane on the right (split off if there is none)", "Déplacer l'onglet vers le volet de droite (découpé s'il n'y en a pas)")),
                k(&TabToUp, tr("Move the tab to the pane above (split off if there is none)", "Déplacer l'onglet vers le volet du dessus (découpé s'il n'y en a pas)")),
                k(&TabToDown, tr("Move the tab to the pane below (split off if there is none)", "Déplacer l'onglet vers le volet du dessous (découpé s'il n'y en a pas)")),
                k(&EvenPanes, tr("Give every pane the same room", "Donner la même place à chaque volet")),
                k(&MaximizePane, tr("The pane on the whole window, and back (the others keep what they hold)", "Le volet sur toute la fenêtre, et retour (les autres gardent ce qu'ils ont)")),
                t(tr("Shift+click on the rail, title of a view", "Maj+clic sur le rail, titre d'une vue"), tr("Shift+click stacks the view under those shown in its dock; drag its title to another place in a dock, or to the right edge to open the right dock. Escape lets go", "Maj+clic empile la vue sous celles montrées dans son dock ; glisser son titre vers une autre place d'un dock, ou vers le bord droit pour ouvrir le dock de droite. Échap lâche")),
                t(tr("Dragging a tab", "Glisser un onglet"), tr("Onto a tab bar: it moves there; onto the middle of a pane: it joins it; near an edge of a pane: a new pane opens there with it. A file from the tree does the same. Escape lets go without moving anything", "Sur une barre d'onglets : il s'y déplace ; au milieu d'un volet : il le rejoint ; près d'un bord d'un volet : un nouveau volet s'y ouvre avec lui. Un fichier de l'arbre fait de même. Échap lâche sans rien déplacer")),
                t(tr("Border between panes", "Bordure entre volets"), tr("Drag it to share the room; double click: even shares", "La glisser pour partager la place ; double-clic : parts égales")),
                t(tr("Opening a file", "Ouvrir un fichier"), tr("Click in the tree, choose in the palette, follow a link: the file opens in a tab of its own, or its tab comes to the front if it is already open; the arrows in the tree preview notes in a single italic tab", "Clic dans l'arbre, choix dans la palette, lien suivi : le fichier s'ouvre dans son propre onglet, ou son onglet passe devant s'il est déjà ouvert ; les flèches dans l'arbre montrent les notes en aperçu dans un seul onglet en italique")),
                k(&ClosePane, tr("Save and close the focused tab; keep it open if saving fails", "Enregistrer et fermer l'onglet actif ; le garder ouvert si l'enregistrement échoue")),
                k(&CloseOtherTabs, tr("Close the other tabs of the pane, each saved first; one that fails to save stays open", "Fermer les autres onglets du volet, chacun enregistré d'abord ; celui qui échoue reste ouvert")),
                k(&CloseTabsInPane, tr("Close all the tabs of the pane; the other panes keep theirs", "Fermer tous les onglets du volet ; les autres volets gardent les leurs")),
                k(&CloseAllFiles, tr("Close all files, in every pane, without deleting any; the window is left on a new empty note", "Fermer tous les fichiers, dans tous les volets, sans en supprimer aucun ; la fenêtre reste sur une note vide")),
                t(tr("Right click on a tab", "Clic droit sur un onglet"), tr("Close it, close the other tabs, or all the tabs of its pane", "Le fermer, fermer les autres onglets, ou tous ceux de son volet")),
                k(&ReopenTab, tr("Reopen the last closed tab", "Rouvrir le dernier onglet fermé")),
                k(&NextTab, tr("Next tab", "Onglet suivant")),
                k(&PrevTab, tr("Previous tab", "Onglet précédent")),
                k(&MoveTabLeft, tr("Move the tab one place to the left", "Déplacer l'onglet d'un cran à gauche")),
                k(&MoveTabRight, tr("Move the tab one place to the right", "Déplacer l'onglet d'un cran à droite")),
                t(tr("Middle click", "Clic du milieu"), tr("On a tab: close it", "Sur un onglet : le fermer")),
                k(&palette::ConfirmAside, tr("In the palette: open the chosen note in the other pane", "Dans la palette : ouvrir la note choisie dans l'autre volet")),
                t(&format!("{MOD}+{shift}+{click}"), tr("Open a [[link]] in the other pane", "Ouvrir un [[lien]] dans l'autre volet")),
                t(tr("Drag from the tree", "Glisser depuis l'arbre"), tr("Onto a pane: open the file there; onto the right half of a single pane: open a second one", "Sur un volet : y ouvrir le fichier ; sur la moitié droite du seul volet : en ouvrir un second")),
                k(&nav::ShowTree, tr("Vault tree", "Arbre du coffre")),
                t(tr("Current file", "Fichier affiché"), tr("The tree marks the file shown in the active tab and pane, including diagrams, pictures and tables; selection stays separate", "L'arbre marque le fichier du volet et de l'onglet actifs, y compris les schémas, images et tableaux ; la sélection reste distincte")),
                k(&nav::ShowRecent, tr("Recent notes", "Notes récentes")),
                t(tr("A picture", "Une image"), tr("Shown in place of the note", "Affichée à la place de la note")),
                k(&nav::ToggleFull, tr("Panel on the whole window", "Panneau en pleine fenêtre")),
                k(&ToggleSidebar, tr("Sidebar: show or hide the side panel (the rail stays)", "Barre latérale : afficher ou masquer le panneau latéral (le rail reste)")),
                t(tr("Arrows", "Flèches"), tr("Select and preview", "Sélectionner en aperçu")),
                t(tr("Enter / Esc", "Entrée / Échap"), tr("Open the note / back to the note", "Ouvrir la note / revenir à la note")),
                k(&nav::NewFolder, tr("New folder (also in the palette)", "Nouveau dossier (aussi dans la palette)")),
                t(tr("Point at an icon", "Pointer une icône"), tr("Its name and its shortcut", "Son nom et son raccourci")),
                t("Alt + drag", tr("Move the window from anywhere (Linux)", "Déplacer la fenêtre depuis n'importe où (Linux)")),
                k(&nav::Rename, tr("Panel: rename", "Panneau : renommer")),
                k(&nav::Duplicate, tr("Panel: duplicate", "Panneau : dupliquer")),
                k(&nav::Trash, tr("Panel: move to the trash", "Panneau : mettre à la corbeille")),
                t(&format!("{MOD}+{click} / {shift}+{click}"), tr("Select several rows: move, duplicate, trash them together", "Sélectionner plusieurs lignes : les déplacer, dupliquer, jeter ensemble")),
                t(tr("Right click", "Clic droit"), tr("Copy the link or the path, set an icon, show a folder or a tag as a list…", "Copier le lien ou le chemin, donner une icône, afficher un dossier ou un tag en liste…")),
                k(&nav::OpenAside, tr("Open the note in the other pane", "Ouvrir la note dans l'autre volet")),
                k(&nav::CopyLink, tr("Copy the [[link]] of the note", "Copier le [[lien]] de la note")),
                k(&nav::CopyPath, tr("Copy the path of the note", "Copier le chemin de la note")),
                k(&nav::CopyRelative, tr("Copy the relative path of the note", "Copier le chemin relatif de la note")),
                k(&nav::Reveal, commands::reveal_label()),
                k(&nav::MoveTo, tr("Move the note to a folder…", "Déplacer la note vers un dossier…")),
                k(&nav::SetIcon, tr("Icon of the note…", "Icône de la note…")),
                k(&nav::ShowFolder, tr("Show the note's folder as a list", "Afficher le dossier de la note en liste")),
                k(&FocusNext, tr("From the panel: go from button to button (Shift+Tab: back); Enter or Space presses it", "Depuis le panneau : aller de bouton en bouton (Maj+Tab : revenir) ; Entrée ou Espace l'actionne")),
                k(&FocusPrev, tr("Back to the previous button", "Revenir au bouton précédent")),
                k(&ContextMenu, tr("The same menu from the keys, for the selected row or the cursor: arrows, Enter, Esc", "Le même menu au clavier, pour la ligne choisie ou le curseur : flèches, Entrée, Échap")),
                t(tr("In a list", "Dans une liste"), tr("Type to filter (key:text for one column), click a cell to rewrite it", "Taper pour filtrer (clé:texte pour une colonne), cliquer une cellule pour la réécrire")),
            ],
        ),
        (
            tr("Diagrams", "Schémas"),
            vec![
                t("![](Schéma.svg)", tr("Show the diagram in a note", "Afficher le schéma dans une note")),
                t(".svg", tr("An SVG from elsewhere (draw.io, Mermaid.js…) opens in a viewer: drag or wheel to move, + - 0 to zoom", "Un SVG d'ailleurs (draw.io, Mermaid.js…) s'ouvre dans une visionneuse : glisser ou molette pour se déplacer, + - 0 pour zoomer")),
                t(tr("Picture, formula, diagram", "Image, formule, schéma"), tr("Point at one in a note: a button at its corner shows it full size, to zoom and move around (Esc goes back)", "Pointer l'un d'eux dans une note : un bouton à son coin l'affiche en grand, pour zoomer et se déplacer (Échap revient)")),
            ],
        ),
        (
            tr("Appearance", "Apparence"),
            vec![
                p(tr("theme", "thème"), tr("Theme, previewed as you browse", "Thème, en aperçu pendant le choix")),
                k(&ChooseTheme, tr("Theme…", "Thème…")),
                p(tr("font", "police"), tr("Font of the app / of the code", "Police de l'app / du code")),
                p(tr("page width", "largeur de page"), tr("Width of the text column: narrow, normal, wide, full", "Largeur de la colonne de texte : étroite, normale, large, pleine")),
                p(tr("wide tables", "grands tableaux"), tr("A table wider than the text: in the column, or centered and wider", "Un tableau plus large que le texte : dans la colonne, ou centré et plus large")),
                p(tr("animations", "animations"), tr("Short fades (about 0.13 s) when the panel, the formatting bar or a folder opens; none on typing, scrolling or hover. Off: everything is instant (motion=off in settings)", "Courts fondus (environ 0,13 s) à l'ouverture du panneau, de la barre de mise en forme ou d'un dossier ; rien à la frappe, au défilement ni au survol. Désactivées : tout est instantané (motion=off dans settings)")),
                k(&ZoomIn, tr("Bigger text; the panel, menus and help follow, up to 150 %", "Texte plus grand ; le panneau, les menus et l'aide suivent, jusqu'à 150 %")),
                k(&ZoomOut, tr("Smaller text; the interface follows, down to 85 %", "Texte plus petit ; l'interface suit, jusqu'à 85 %")),
                k(&ZoomReset, tr("Default text size", "Taille du texte d'origine")),
            ],
        ),
        (
            // The table gestures of the menu: the frame says where they act, each can have its key.
            tr("Tables in a note", "Tableaux d'une note"),
            std::iter::once(t(
                tr("Grips", "Poignées"),
                tr(
                    "Over a table: a small grip above the column and left of the row under the pointer, a round one at the corner for the whole table. Click one to take the column, the row or the table (copy, cut, erase, or any gesture below), drag it to move it, Esc lets go; the + between two grips inserts there",
                    "Sur un tableau : une petite poignée au-dessus de la colonne et à gauche de la ligne sous le pointeur, une ronde au coin pour tout le tableau. Un clic prend la colonne, la ligne ou le tableau (copier, couper, effacer, ou un geste ci-dessous), tirer la déplace, Échap la lâche ; le + entre deux poignées insère à cet endroit",
                ),
            ))
            .chain(editor::TableOp::ALL.into_iter().map(|op| k(op.action().as_ref(), op.label())))
            .chain([
                k(
                    &editor::TableDeleteSpan,
                    tr("Remove the row, or the columns taken by their grip", "Supprimer la ligne, ou les colonnes prises par leur poignée"),
                ),
            ])
            .collect(),
        ),
        (
            tr("Typing", "À la frappe"),
            vec![
                t("/", tr("Components: headings, lists, panel, table, code, formula…", "Composants : titres, listes, panneau, tableau, code, formule…")),
                t("/table", tr("Table: pick its size on the grid (mouse, arrows), or type it: 12x5", "Tableau : choisir sa taille sur la grille (souris, flèches), ou la taper : 12x5")),
                t("> [!NOTE]", tr("Colored panel: NOTE, TIP, IMPORTANT, WARNING, CAUTION", "Panneau coloré : NOTE, TIP, IMPORTANT, WARNING, CAUTION")),
                t("- ", tr("Bullet list", "Liste à puces")),
                t("1. ", tr("Numbered list", "Liste numérotée")),
                t("[] ", tr("Task", "Tâche à cocher")),
                t("# ## ###", tr("Headings; the first # names the file", "Titres ; le premier # nomme le fichier")),
                t("> ", tr("Quote", "Citation")),
                t("```rust", tr("Code block: colors, copy icon", "Bloc de code : couleurs, icône de copie")),
                t("---", tr("Divider", "Séparateur")),
                t("[[", tr("Link to a note", "Lien vers une note")),
                t("@", tr("Link to the note of a day: @today, @monday, @2026-10-09", "Lien vers la note d'un jour : @demain, @lundi, @2026-10-09")),
                t("![](image.png)", tr("Picture, under its line", "Image, sous sa ligne")),
                t("![[", tr("Suggests the pictures and diagrams of the vault", "Propose les images et les schémas du coffre")),
                t(tr("Drop files", "Déposer des fichiers"), tr("Drag files or folders from your file manager onto a note (a line shows where) or onto the tree (the folder under the pointer): they are copied, never overwritten, and a note gets a link per item", "Glisser des fichiers ou des dossiers du gestionnaire de fichiers sur une note (un trait montre où) ou sur l'arbre (le dossier sous le pointeur) : ils sont copiés, jamais écrasés, et la note reçoit un lien par élément")),
                t(tr("Double / triple click", "Double / triple clic"), tr("Select a word / a line; drag to extend by words / lines", "Sélectionner un mot / une ligne ; glisser étend par mots / par lignes")),
                t("```mermaid", tr("Diagram, under its block; its magnifier shows it full size", "Diagramme, sous son bloc ; sa loupe le montre en grand")),
                t("$x^2$  $$…$$", tr("LaTeX formula, under its line", "Formule LaTeX, sous sa ligne")),
                t("-> != <= =>", tr("Shown as → ≠ ≤ ⇒", "Affichés → ≠ ≤ ⇒")),
            ],
        ),
        (
            tr("Editing", "Édition"),
            vec![
                t(tr("Enter", "Entrée"), tr("Continue the list, or leave it on an empty item", "Continuer la liste, ou en sortir sur un item vide")),
                t(&format!("Tab / {shift}+Tab"), tr("Indent / outdent", "Indenter / désindenter")),
                t(tr("Tab / Enter in a table", "Tab / Entrée dans un tableau"), tr("Next cell / new row; columns stay aligned; + buttons add a row or a column", "Cellule suivante / nouvelle ligne ; les colonnes restent alignées ; les boutons + ajoutent une ligne ou une colonne")),
                k(&editor::AlignLeft, tr("Align the column of the cursor to the left", "Aligner la colonne du curseur à gauche")),
                k(&editor::AlignCenter, tr("Center the column of the cursor", "Centrer la colonne du curseur")),
                k(&editor::AlignRight, tr("Align the column of the cursor to the right", "Aligner la colonne du curseur à droite")),
                t(tr("Pasting a table", "Coller un tableau"), tr("Spreadsheet cells or Markdown fill the grid, or become a table", "Des cellules de tableur ou du Markdown remplissent la grille, ou deviennent un tableau")),
                k(&editor::ListCycle, tr("Turn the selected lines into a list, then the next kind: bullets, numbered, tasks, plain text", "Mettre les lignes choisies en liste, puis au genre suivant : puces, numérotée, tâches, texte")),
                k(&editor::ListBullet, tr("Selected lines: bulleted list (again: plain text)", "Lignes choisies : liste à puces (encore : texte)")),
                k(&editor::ListOrdered, tr("Selected lines: numbered list (again: plain text)", "Lignes choisies : liste numérotée (encore : texte)")),
                k(&editor::ListTask, tr("Selected lines: task list (again: plain text)", "Lignes choisies : liste de tâches (encore : texte)")),
                k(&editor::ListText, tr("Selected lines: plain text", "Lignes choisies : texte simple")),
                k(&editor::ToggleTask, tr("Check / uncheck a task or a [ ] box; in a table cell, add one", "Cocher / décocher une tâche ou une case [ ] ; dans une cellule, en ajouter une")),
                k(&editor::Bold, tr("Bold", "Gras")),
                k(&editor::Italic, tr("Italic", "Italique")),
                k(&editor::Underline, tr("Underline", "Souligné")),
                k(&editor::Strike, tr("Strikethrough", "Barré")),
                k(&editor::Highlight, tr("Highlight", "Surligné")),
                k(&editor::InlineCode, tr("Inline code", "Code en ligne")),
                p(tr("formatting bar", "barre de mise en forme"), tr("Under the note, centered: bold, italic, underline, strikethrough, highlight (five colors), code, link, lists, tasks, table (in a table: add or delete a row or a column); a lit button means the selection has the style, click again to remove it; only the active pane shows it; in a narrow note it folds into a button at the bottom, in the middle, that opens it; hide it with the palette", "Sous la note, au centre : gras, italique, souligné, barré, surligné (cinq couleurs), code, lien, listes, tâches, tableau (dans un tableau : ajouter ou supprimer une ligne ou une colonne) ; un bouton allumé : la sélection a le style, un second clic le retire ; seul le pane actif la montre ; dans une note étroite, elle se replie en un bouton en bas, au centre, qui l'ouvre ; masquable par la palette")),
                k(&editor::SelectNext, tr("Several cursors: select the word, then its next occurrence (Esc: back to one)", "Plusieurs curseurs : prendre le mot, puis son occurrence suivante (Échap : un seul)")),
                k(&editor::Cancel, tr("Close the find bar and the menus, back to one cursor", "Fermer la barre de recherche et les menus, revenir à un seul curseur")),
                t(&format!("Alt+{click}"), tr("Add a cursor", "Ajouter un curseur")),
                k(&editor::InsertLink, tr("Link: [text](address) from the selection; on a link, change its address", "Lien : [texte](adresse) depuis la sélection ; sur un lien, changer son adresse")),
                k(&editor::Comment, tr("Comment the selection or the line (in a table: the cell, or the taken row, column or table); on a comment, resolve it", "Commenter la sélection ou la ligne (dans un tableau : la cellule, ou la ligne, la colonne, le tableau pris) ; sur un commentaire, le résoudre")),
                p(tr("comments", "commentaires"), tr("Comments: show their panel, jump to one (a badge on the speech bubble counts them)", "Commentaires : montrer leur panneau, aller à l'un d'eux (une pastille sur la bulle les compte)")),
                t(tr("Right click", "Clic droit"), tr("Menu of the note: link, comment, cut, copy, paste, bold, italic", "Menu de la note : lien, commentaire, couper, copier, coller, gras, italique")),
                k(&editor::Copy, tr("Copy", "Copier")),
                k(&editor::Cut, tr("Cut", "Couper")),
                k(&editor::Paste, tr("Paste text or a picture; an address over a selection makes it a link", "Coller du texte ou une image ; une adresse sur une sélection en fait un lien")),
                k(&editor::SelectAll, tr("Select all", "Tout sélectionner")),
                k(&editor::Undo, tr("Undo", "Annuler")),
                k(&editor::Redo, tr("Redo", "Rétablir")),
                t(&format!("{word}+{}", tr("Left / Right", "Gauche / Droite")), tr("Move by word", "Se déplacer par mot")),
            ],
        ),
        (
            tr("Blocks", "Blocs"),
            std::iter::once(t(
                tr("Grip", "Poignée"),
                tr(
                    "Left of the line under the pointer: drag it to move the block (a line shows where it lands), click it to select the block and open its menu; the + left of it adds a block under it and opens the / list",
                    "À gauche de la ligne sous le pointeur : la tirer déplace le bloc (un trait montre où il arrive), un clic sélectionne le bloc et ouvre son menu ; le + à sa gauche ajoute un bloc dessous et ouvre la liste /",
                ),
            ))
            .chain(editor::BlockOp::ALL.into_iter().map(|op| k(op.action().as_ref(), op.effect())))
            .collect(),
        ),
    ]
}

/// The actions whose shortcut can be changed: those that have their line in the help.
pub(super) fn editable() -> std::collections::HashSet<&'static str> {
    help_sections(&[], &keys::Changes::new())
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .filter_map(|row| row.action)
        .collect()
}

/// The default shortcut table. Its order matters: for the same context, the last binding
/// wins.
pub(super) fn defaults() -> Vec<keys::Bind> {
    use editor::*;
    use keys::bind as b;
    use palette::{Confirm, Dismiss, Next, Prev};
    let e = Some("Editor");
    let p = Some("Palette");
    let f = Some("Find");
    let n = Some("Nav");
    // `secondary` = Cmd on macOS, Ctrl elsewhere; words are traversed with Alt on macOS.
    let word = if cfg!(target_os = "macos") {
        "alt"
    } else {
        "ctrl"
    };
    let mut all = vec![
        b("secondary-p", &OpenPalette, None),
        b("secondary-n", &NewNote, None),
        b("secondary-o", &OpenVault, None),
        b("secondary-shift-c", &CopyAll, None),
        b("f1", &ToggleHelp, None),
        b("secondary-/", &ToggleHelp, None),
        b("escape", &CloseHelp, Some("Shell")),
        b("secondary-\\", &SplitPane, None),
        b("secondary-alt-\\", &SplitDown, None),
        b("secondary-1", &FocusLeft, None),
        b("secondary-2", &FocusRight, None),
        // Ctrl+Alt+arrows belong to the desktop (workspaces) on Linux: Alt+Shift there.
        b(
            if cfg!(target_os = "macos") {
                "cmd-alt-left"
            } else {
                "alt-shift-left"
            },
            &PaneLeft,
            None,
        ),
        b(
            if cfg!(target_os = "macos") {
                "cmd-alt-right"
            } else {
                "alt-shift-right"
            },
            &PaneRight,
            None,
        ),
        b(
            if cfg!(target_os = "macos") {
                "cmd-alt-up"
            } else {
                "alt-shift-up"
            },
            &PaneUp,
            None,
        ),
        b(
            if cfg!(target_os = "macos") {
                "cmd-alt-down"
            } else {
                "alt-shift-down"
            },
            &PaneDown,
            None,
        ),
        b("alt-shift-enter", &MaximizePane, None),
        b("secondary-w", &ClosePane, None),
        // Reopen: the browser chord. Recent notes keep no chord: Ctrl+P already opens on
        // them, and the palette lists them as `Recent notes`. Frees Ctrl+Alt+T.
        b("secondary-shift-t", &ReopenTab, None),
        b("ctrl-tab", &NextTab, None),
        b("ctrl-shift-tab", &PrevTab, None),
        b("ctrl-shift-pageup", &MoveTabLeft, None),
        b("ctrl-shift-pagedown", &MoveTabRight, None),
        // On macOS, Alt+arrow traverses words: brackets, as in a browser.
        b("secondary-=", &ZoomIn, None),
        b("secondary-+", &ZoomIn, None),
        b("secondary--", &ZoomOut, None),
        b("secondary-0", &ZoomReset, None),
        b("secondary-q", &Quit, None),
        // The command palette: F10, like the menu bar of a desktop app.
        b("f10", &ToggleCommands, None),
        b("secondary-shift-p", &ToggleCommands, None),
        // From button to button, where Tab has no other use (the note indents with it). In the window's own
        // context: a binding with none counts as the deepest and would take Tab from the views that use it.
        b("tab", &FocusNext, Some("Shell")),
        b("enter", &ui::widgets::Press, Some("Button")),
        b("shift-tab", &FocusPrev, Some("Shell")),
        // The context menu of the selected row or of the cursor, as on every desktop.
        b("shift-f10", &ContextMenu, None),
        b("menu", &ContextMenu, None),
        b("secondary-e", &nav::ShowTree, Some("Shell")),
        b("secondary-m", &nav::ToggleFull, Some("Shell")),
        b("secondary-b", &ToggleSidebar, None),
        b("secondary-shift-n", &nav::NewFolder, Some("Shell")),
        b("f2", &nav::Rename, n),
        b("secondary-d", &nav::Duplicate, n),
        b("delete", &nav::Trash, n),
        b("up", &nav::Prev, n),
        b("down", &nav::Next, n),
        b("left", &nav::Fold, n),
        b("right", &nav::Unfold, n),
        b("enter", &nav::Open, n),
        b("escape", &nav::Close, n),
        b("up", &Prev, p),
        b("down", &Next, p),
        b("enter", &Confirm, p),
        b("secondary-enter", &palette::ConfirmAside, p),
        b("escape", &Dismiss, p),
        b("backspace", &Backspace, e),
        b("delete", &Delete, e),
        b(&format!("{word}-backspace"), &DeleteWordLeft, e),
        b(&format!("{word}-delete"), &DeleteWordRight, e),
        b("left", &Left, e),
        b("right", &Right, e),
        b("up", &Up, e),
        b("down", &Down, e),
        b(&format!("{word}-left"), &WordLeft, e),
        b(&format!("{word}-right"), &WordRight, e),
        b("home", &Home, e),
        b("end", &End, e),
        b("secondary-home", &DocStart, e),
        b("secondary-end", &DocEnd, e),
        b("pageup", &PageUp, e),
        b("pagedown", &PageDown, e),
        b("shift-left", &SelectLeft, e),
        b("shift-right", &SelectRight, e),
        b("shift-up", &SelectUp, e),
        b("shift-down", &SelectDown, e),
        b(&format!("{word}-shift-left"), &SelectWordLeft, e),
        b(&format!("{word}-shift-right"), &SelectWordRight, e),
        b("shift-home", &SelectHome, e),
        b("shift-end", &SelectEnd, e),
        b("secondary-shift-home", &SelectDocStart, e),
        b("secondary-shift-end", &SelectDocEnd, e),
        b("secondary-a", &SelectAll, e),
        // Search in the note: the bar has its own context as long as it receives input.
        b("secondary-f", &Find, e),
        b("secondary-f", &Find, f),
        b("secondary-r", &FindReplace, e),
        b("secondary-r", &FindReplace, f),
        b("f3", &FindNext, e),
        b("shift-f3", &FindPrev, e),
        b("f3", &FindNext, f),
        b("shift-f3", &FindPrev, f),
        b("enter", &FindEnter, f),
        b("shift-enter", &FindPrev, f),
        b("secondary-enter", &ReplaceAll, f),
        b("tab", &FindSwitch, f),
        b("shift-tab", &FindSwitch, f),
        // Also when the bar is open but input has gone back to the note.
        b("alt-c", &FindCase, f),
        b("alt-w", &FindWord, f),
        b("alt-r", &FindRegex, f),
        b("alt-r", &FindRegex, e),
        b("alt-c", &FindCase, e),
        b("alt-w", &FindWord, e),
        b("enter", &Newline, e),
        b("tab", &Indent, e),
        b("shift-tab", &Outdent, e),
        b("secondary-enter", &ToggleTask, e),
        // The four direct choices have no key: the menu and the palette offer them, and the help grants them.
        b("secondary-shift-b", &ListCycle, e),
        // A table: move the row up or down, delete it; the rest via the menu, the palette or the help.
        // Outside a table, Alt+arrow moves the cursor's block up or down; in a table, its row.
        b("alt-up", &BlockUp, e),
        b("alt-down", &BlockDown, e),
        b("secondary-shift-backspace", &TableDeleteSpan, e),
        b("secondary-shift-l", &AlignLeft, e),
        b("secondary-shift-e", &AlignCenter, e),
        b("secondary-shift-r", &AlignRight, e),
        // Ctrl+B is the sidebar's (any view, the note included); bold moved to Ctrl+Alt+B.
        b("secondary-alt-b", &Bold, e),
        b("secondary-i", &Italic, e),
        b("secondary-u", &Underline, e),
        b("secondary-shift-x", &Strike, e),
        b("secondary-shift-h", &Highlight, e),
        b("secondary-shift-k", &InlineCode, e),
        b("secondary-k", &InsertLink, e),
        b("secondary-d", &SelectNext, e),
        b("secondary-shift-m", &Comment, e),
        b("secondary-c", &Copy, e),
        b("secondary-x", &Cut, e),
        b("secondary-v", &Paste, e),
        b("secondary-z", &Undo, e),
        b("secondary-shift-z", &Redo, e),
        b("secondary-y", &Redo, e),
        b("escape", &editor::Cancel, e),
    ];
    // After the fields that use them (palette, grid): its keys take precedence over theirs.
    all.extend(line::bindings(word));
    // macOS conventions: Cmd+arrows for line and document ends.
    #[cfg(target_os = "macos")]
    all.extend([
        b("cmd-left", &Home, e),
        b("cmd-right", &End, e),
        b("cmd-up", &DocStart, e),
        b("cmd-down", &DocEnd, e),
        b("cmd-shift-left", &SelectHome, e),
        b("cmd-shift-right", &SelectEnd, e),
        b("cmd-shift-up", &SelectDocStart, e),
        b("cmd-shift-down", &SelectDocEnd, e),
    ]);
    all
}

/// Replaces all of the app's bindings with these.
pub(super) fn install(cx: &mut App, binds: &[keys::Bind]) {
    let binds: Vec<KeyBinding> = binds
        .iter()
        .filter_map(|bind| {
            let action = cx.build_action(bind.action, None).ok()?;
            let context = bind.context.map(|context| {
                gpui::KeyBindingContextPredicate::parse(context)
                    .unwrap()
                    .into()
            });
            KeyBinding::load(
                &bind.keys,
                action,
                context,
                false,
                None,
                &gpui::DummyKeyboardMapper,
            )
            .ok()
        })
        .collect();
    cx.clear_key_bindings();
    cx.bind_keys(binds);
}

pub(super) fn bind_keys(cx: &mut App) {
    install(cx, &defaults());
}
