//! `inkuno keys`: the keyboard shortcuts, read and written as `keybindings.json`, with the rules the
//! app applies (`ui/keys.rs`): the actions that can change, the chords the system refuses, the clashes.

use std::{collections::BTreeSet, fs};

use serde_json::{Value, json};

use super::{failed, options, suggest, usage};
use crate::lang::tr;
use crate::platform::vault;
use crate::shell::keymap;
use crate::ui::keys::{self, Bind, Changes, Why};

/// The help of `inkuno keys` and `inkuno help keys`.
pub const HELP: &str = "\
inkuno keys list [--json]
    every action that has a shortcut: its chords now, and whether it is changed
inkuno keys set <action> <chord>
    give an action a chord, written to keybindings.json; \"\" removes its shortcut
inkuno keys reset <action>...
    back to the default chords of these actions

An action is named as the code names it, group::Name (app::OpenPalette,
editor::Bold); `keys list` shows them all. A chord is written as in the file:
secondary-p (Cmd on macOS, Ctrl elsewhere), ctrl-tab, alt-left, f9, shift-enter.
A chord that types text (a letter alone) is refused, and so is one that another
action already has in the same place. The app reads the file at its next start:
a change made while it runs applies after a restart.

examples:
  inkuno keys set app::OpenPalette secondary-shift-o
  inkuno keys list --json
  inkuno keys reset app::OpenPalette
";

/// Runs `keys` with its arguments (after the word `keys`).
pub fn run(args: &[String]) -> i32 {
    let Some(sub) = args.first().map(String::as_str) else {
        return usage(tr(
            "keys needs list, set or reset",
            "keys attend list, set ou reset",
        ));
    };
    if matches!(sub, "--help" | "-h") {
        print!("{HELP}");
        return 0;
    }
    let opts = match options(&args[1..], &[], &[]) {
        Ok(opts) => opts,
        Err(reason) => return usage(&format!("keys {sub}: {reason}")),
    };
    if opts.help {
        print!("{HELP}");
        return 0;
    }
    let result = match sub {
        "list" => list(&opts),
        "set" => set(&opts),
        "reset" => reset(&opts),
        other => return usage(&format!("unknown keys command `{other}`")),
    };
    match result {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(reason) => failed(reason),
    }
}

/// The actions whose chord can be changed: the ones with a line in the help.
fn editable() -> BTreeSet<&'static str> {
    keymap::editable().into_iter().collect()
}

/// The user's changes now, as the app reads them: the JSON file, else the legacy `keys` file.
fn changes_now(defaults: &[Bind], open: &BTreeSet<&'static str>) -> Changes {
    keys::read(&keys::user_lines(), defaults, &|a| open.contains(a)).0
}

/// Writes the changes to `keybindings.json`, after checking that the app's own reader takes them
/// back without a fault: what is written is what the app will use.
fn write(
    changes: &Changes,
    defaults: &[Bind],
    open: &BTreeSet<&'static str>,
) -> Result<(), String> {
    let (_, faults) = keys::read(&keys::write(changes), defaults, &|a| open.contains(a));
    if !faults.is_empty() {
        return Err(format!(
            "the app would not take that: {}",
            faults.join("; ")
        ));
    }
    let path = vault::keybindings_path();
    let saved = match path.parent() {
        Some(dir) => fs::create_dir_all(dir),
        None => Ok(()),
    }
    .and_then(|()| vault::write(&path, keys::write_json(changes)));
    saved.map_err(|e| format!("{}: {}", path.display(), crate::lang::io_reason(&e)))
}

fn list(opts: &super::Opts) -> Result<String, String> {
    let defaults = keymap::defaults();
    let open = editable();
    let changes = changes_now(&defaults, &open);
    let bound = keys::effective(&defaults, &changes, &|a| open.contains(a));
    let chords = |binds: &[Bind], action: &str| -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for bind in binds.iter().filter(|b| b.action == action) {
            if !found.contains(&bind.keys) {
                found.push(bind.keys.clone());
            }
        }
        found
    };
    if opts.json {
        let items: Vec<Value> = open
            .iter()
            .map(|action| {
                json!({
                    "action": action,
                    "keys": chords(&bound, action),
                    "default": chords(&defaults, action),
                    "changed": changes.contains_key(*action),
                })
            })
            .collect();
        return Ok(serde_json::to_string_pretty(&Value::Array(items)).unwrap_or_default() + "\n");
    }
    let width = open.iter().map(|a| a.len()).max().unwrap_or(0);
    let mut out = String::new();
    for action in &open {
        let now = chords(&bound, action);
        let shown = if now.is_empty() {
            "-".to_string()
        } else {
            now.join(" ")
        };
        let status = if changes.contains_key(*action) {
            "changed"
        } else {
            "default"
        };
        out += &format!("{action:<width$}  {shown:<22}  {status}\n");
    }
    Ok(out)
}

/// `keys set <action> <chord>`: checks the action, the chord and the clashes, then writes the file.
fn set(opts: &super::Opts) -> Result<String, String> {
    let open = editable();
    let Some(action) = opts.words.first() else {
        return Err("an action is needed: keys set <action> <chord>".into());
    };
    if !open.contains(action.as_str()) {
        let names: Vec<String> = open.iter().map(|a| a.to_string()).collect();
        let near = suggest(action, &names);
        return Err(if near.is_empty() {
            format!("unknown action `{action}`: keys list shows them all")
        } else {
            format!(
                "unknown action `{action}`: did you mean {}?",
                near.join(", ")
            )
        });
    }
    let chords = &opts.words[1..];
    let chord = match chords {
        [] => return Err("a chord is needed, or \"\" to remove the shortcut".into()),
        [one] => one.as_str(),
        _ => {
            return Err(
                "one chord per action: the file holds one (secondary-p, ctrl-tab, f9…)".into(),
            );
        }
    };
    let defaults = keymap::defaults();
    let mut changes = changes_now(&defaults, &open);
    let canon = if chord.is_empty() {
        String::new()
    } else {
        let canon = keys::canon(chord).ok_or_else(|| {
            format!(
                "unreadable chord `{chord}`: write it like secondary-p, ctrl-tab, alt-left or f9"
            )
        })?;
        match keys::refusal(&canon) {
            None => {}
            Some(Why::Types) => {
                return Err(format!(
                    "`{chord}` types text: a shortcut needs Ctrl, Alt or Cmd"
                ));
            }
            Some(Why::Panel) => return Err(format!("`{chord}` is kept for the panels (Escape)")),
            Some(Why::Unreadable) => {
                return Err(format!("unreadable chord `{chord}`"));
            }
        }
        let bound = keys::effective(&defaults, &changes, &|a| open.contains(a));
        if let Some(other) = keys::clashes(&bound, &defaults, action, &canon).first() {
            return Err(format!(
                "{canon} is already the shortcut of {other}: reset that one first, or choose another chord"
            ));
        }
        canon
    };
    changes.insert(action.clone(), canon.clone());
    write(&changes, &defaults, &open)?;
    Ok(if canon.is_empty() {
        format!("{action}: no shortcut\n")
    } else {
        format!("{action}: {canon}\n")
    })
}

/// `keys reset <action>...`: the default chords of these actions come back.
fn reset(opts: &super::Opts) -> Result<String, String> {
    let open = editable();
    if opts.words.is_empty() {
        return Err("an action is needed: keys reset <action>...".into());
    }
    let defaults = keymap::defaults();
    let mut changes = changes_now(&defaults, &open);
    let mut out = String::new();
    for action in &opts.words {
        if !open.contains(action.as_str()) {
            return Err(format!(
                "unknown action `{action}`: keys list shows them all"
            ));
        }
        if changes.remove(action).is_none() {
            out += &format!("{action}: already the default\n");
        } else {
            out += &format!("{action}: back to its default\n");
        }
    }
    write(&changes, &defaults, &open)?;
    Ok(out)
}
