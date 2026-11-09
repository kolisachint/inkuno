//! Inkuno: a fast, minimal Markdown note-taking app. ARCHITECTURE.md maps the code.

mod actions;
mod config_watch;
#[cfg(test)]
mod dbg_test;
mod lang;
mod logic;
mod mcp;
mod platform;
mod shell;
mod ui;
mod views;

use platform::vault;
use ui::{keys, theme};

/// Usage, config paths and the agent-editable JSON schemas.
fn help_text() -> String {
    let dir = vault::config_dir().join("inkuno");
    format!(
        "inkuno {} — fast, minimal Markdown notes\n\
         \n\
         usage:\n\
         \u{20} inkuno                  open the app\n\
         \u{20} inkuno mcp               run the Model Context Protocol server on stdio\n\
         \u{20} inkuno --check-config    validate the JSON config below, exit 0/1\n\
         \u{20} inkuno --help            show this text\n\
         \n\
         config dir:\n\
         \u{20} {} (Linux ~/.config/inkuno/, macOS ~/Library/Application Support/inkuno/)\n\
         \n\
         agent-editable JSON: when present, it wins over the legacy `keys` and `settings`\n\
         files; the first run without it exports the current state there (nothing is\n\
         deleted). A faulty entry is ignored on its own: startup toasts it,\n\
         `inkuno --check-config` lists it. Key clashes with the defaults are checked at\n\
         startup; edit, save, and either restart or wait for the hot-reload below.\n\
         \n\
         keybindings.json:\n\
         \u{20} {{\"version\": 1, \"bindings\": {{\"app::Palette\": [\"secondary-p\"],\n\
         \u{20} \u{20} \"editor::Find\": [\"secondary-f\"], \"nav::Graph\": []}},\n\
         \u{20} \u{20} \"reset\": [\"app::Help\"]}}\n\
         \u{20} one chord per action (the legacy file allows several, space-separated);\n\
         \u{20} `\"\"` or `[]` unbinds; `\"reset\"` restores the default keys of its actions.\n\
         \u{20} action names are the stable `group::Name` rows of the help (e.g. app::Palette).\n\
         \n\
         themes.json:\n\
         \u{20} {{\"version\": 1, \"themes\": [{{\"name\": \"My Dark\", \"base\": \"Inkuno Dark\",\n\
         \u{20} \u{20} \"colors\": {{\"bg\": \"#1b1a19\", \"text\": \"#e4e0da\", \"dim\": \"#77716a\",\n\
         \u{20} \u{20} \"accent\": \"#e0926b\", \"code_bg\": \"#262423\", \"panel\": \"#242221\",\n\
         \u{20} \u{20} \"border\": \"#3a3735\"}}]}}\n\
         \u{20} 7 seed tokens (`bg text dim accent code_bg panel border`); the rest is derived.\n\
         \u{20} a missing token inherits `base` (a built-in theme); low contrast is auto-fixed\n\
         \u{20} for legibility and reported. pick it with `theme=My Dark` in `settings`.\n\
         \n\
         chords are canon: `secondary-` is Cmd on macOS, Ctrl elsewhere; e.g.\n\
         `secondary-p`, `secondary-shift-p`, `alt-left`, `ctrl-tab`, `f9`.\n\
         \n\
         validation (`inkuno --check-config`, machine-readable `file: LEVEL: detail` lines):\n\
         JSON shape and version, readable and allowed chords, chords taken twice,\n\
         chords the system may keep, theme colors (#rrggbb) and contrast.\n\
         \n\
         hot-reload: the app watches both files and live-applies a save (debounced):\n\
         shortcuts rebind at once, themes restyle at once; a faulty file keeps the\n\
         previous state and toasts its faults, it never crashes the app.\n",
        env!("CARGO_PKG_VERSION"),
        dir.display()
    )
}

/// Dry-run validation of both JSON files: machine-readable `file: LEVEL: detail` lines on
/// standard output, `0` when nothing faulty (warnings are fine), `1` otherwise. Reads
/// nothing else, writes nothing.
fn check_config() -> i32 {
    let mut failed = false;
    println!("config dir: {}", vault::config_dir().join("inkuno").display());
    // Shortcuts: JSON when present, else the legacy `keys` file, else the defaults.
    let text = vault::load_keybindings();
    if text.trim().is_empty() {
        let legacy = vault::load_file("keys");
        let lines: Vec<&str> = legacy
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        if lines.is_empty() {
            println!("keybindings.json: OK: absent, defaults in force");
        } else {
            // Light check without the default table: only the chord readability.
            let mut faults = 0;
            for line in &lines {
                let unreadable = line.split_once('=').is_some_and(|(_, chord)| {
                    let chord = chord.trim();
                    !chord.is_empty() && keys::canon(chord).is_none()
                });
                if line.split_once('=').is_none() || unreadable {
                    println!("keybindings.json: ERROR: legacy keys line ignored: {line}");
                    faults += 1;
                }
            }
            if faults > 0 {
                failed = true;
            } else {
                println!(
                    "keybindings.json: OK: absent, legacy keys file in force ({} shortcuts)",
                    lines.len()
                );
            }
        }
    } else {
        let (changes, errors, warns) = keys::check_json(&text);
        for error in &errors {
            println!("keybindings.json: ERROR: {error}");
        }
        for warn in &warns {
            println!("keybindings.json: WARN: {warn}");
        }
        if errors.is_empty() {
            let unbound = changes.values().filter(|keys| keys.is_empty()).count();
            println!(
                "keybindings.json: OK: {} bindings ({unbound} unbound)",
                changes.len()
            );
        } else {
            failed = true;
        }
    }
    // Themes: JSON when present, else the built-in list.
    let text = vault::load_themes();
    if text.trim().is_empty() {
        println!("themes.json: OK: absent, built-in themes in force");
    } else {
        let (names, errors, warns) = theme::check_themes(&text);
        for error in &errors {
            println!("themes.json: ERROR: {error}");
        }
        for warn in &warns {
            println!("themes.json: WARN: {warn}");
        }
        if errors.is_empty() {
            println!(
                "themes.json: OK: {} custom themes ({})",
                names.len(),
                names.join(", ")
            );
        } else {
            failed = true;
        }
    }
    i32::from(failed)
}

/// First run without the JSON files: export the current state there, so agents find the
/// two files. The legacy `keys` file is converted verbatim (canon chords, faulty lines
/// dropped); themes start empty, the built-ins stay in the code. Best effort: a failure
/// only means the legacy files stay in force.
fn export_json_once() {
    if vault::load_keybindings().trim().is_empty() {
        let mut changes = keys::Changes::new();
        for line in vault::load_file("keys").lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((action, chord)) = line.split_once('=') {
                let (action, chord) = (action.trim(), chord.trim());
                if action.is_empty() {
                    continue;
                }
                if chord.is_empty() {
                    changes.insert(action.into(), String::new());
                } else if let Some(canon) = keys::canon(chord) {
                    changes.insert(action.into(), canon);
                }
            }
        }
        vault::save_keybindings(&keys::write_json(&changes));
    }
    if vault::load_themes().trim().is_empty() {
        vault::save_themes("{\n  \"version\": 1,\n  \"themes\": []\n}\n");
    }
}

fn main() {
    // `inkuno mcp` is a Model Context Protocol server on stdio: no window, no gpui.
    if std::env::args().nth(1).as_deref() == Some("mcp") {
        std::process::exit(mcp::run());
    }
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print!("{}", help_text());
        return;
    }
    if std::env::args().any(|arg| arg == "--check-config") {
        std::process::exit(check_config());
    }
    // The last panic is kept in the config folder.
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        platform::vault::save_file(
            "crash",
            &format!("inkuno {}\n{info}\n", env!("CARGO_PKG_VERSION")),
        );
        report(info);
    }));
    export_json_once();
    shell::run();
}
