//! Figures over MCP: Mermaid sources and SVG files, rendered to PNG.
//! `check_mermaid` says whether a source draws and its kind; `render`
//! draws a Mermaid source, an SVG file, or a ```mermaid block of a note.

use std::fs;

use serde_json::{Value, json};

use crate::{
    logic::mermaid,
    ui::{figure, svg},
};

use super::{
    outcome::{Outcome, arg_opt, arg_str, arg_usize, disk},
    store::{self as vault, Vault},
};

pub fn check_mermaid(args: &Value) -> Outcome {
    let source = match arg_str(args, "source") {
        Ok(source) => source,
        Err(message) => return Outcome::err(message),
    };
    let kind = mermaid_kind(source);
    match figure::mermaid(source, &look(false)) {
        Ok(_) => Outcome::ok(json!({"readable": true, "error": null, "kind": kind})),
        Err(error) => Outcome::ok(json!({"readable": false, "error": error, "kind": kind})),
    }
}

/// A PNG of a Mermaid source, an SVG file of the vault, or a note's Mermaid block. `vault` is what
/// the picture of a file or a note needs; `blind` is the sentence said when there is none.
pub fn render(vault: Option<&Vault>, blind: &str, args: &Value) -> Outcome {
    let theme = match arg_opt(args, "theme").unwrap_or("light") {
        "light" => false,
        "dark" => true,
        other => return Outcome::err(format!("unknown theme `{other}`")),
    };
    let width = args.get("width").and_then(|v| v.as_f64()).map(|n| n as f32);
    if width.is_some_and(|n| n <= 0.) {
        return Outcome::err("`width` must be greater than zero");
    }
    let (mermaid_src, path, note) = (
        arg_opt(args, "mermaid"),
        arg_opt(args, "path"),
        arg_opt(args, "note"),
    );
    let given = usize::from(mermaid_src.is_some())
        + usize::from(path.is_some())
        + usize::from(note.is_some());
    if given != 1 {
        return Outcome::err("give one of `path`, `mermaid` or `note`");
    }
    let (svg, kind, shown) = if let Some(source) = mermaid_src {
        match figure::mermaid(source, &look(theme)) {
            Ok(svg) => (svg, "mermaid", None),
            Err(error) => return Outcome::err(error),
        }
    } else {
        let Some(vault) = vault else {
            return Outcome::err(blind);
        };
        match picture(vault, path, note, args, theme) {
            Ok(found) => found,
            Err(outcome) => return outcome,
        }
    };
    match raster(&svg, width, theme) {
        Ok((png, w, h)) => Outcome::image(
            json!({"width": w, "height": h, "kind": kind, "path": shown}),
            png,
        ),
        Err(message) => Outcome::err(message),
    }
}

fn mermaid_kind(source: &str) -> &'static str {
    match mermaid::kind(source) {
        Some(mermaid::Kind::Class) => "class",
        Some(mermaid::Kind::Sequence) => "sequence",
        Some(mermaid::Kind::State) => "state",
        None => {
            let word = source
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with("%%"))
                .unwrap_or("");
            let word = word
                .split(|c: char| c.is_whitespace() || c == ';')
                .next()
                .unwrap_or("");
            if word.eq_ignore_ascii_case("flowchart") || word.eq_ignore_ascii_case("graph") {
                "flowchart"
            } else {
                "other"
            }
        }
    }
}

fn look(dark: bool) -> figure::Look {
    if dark {
        figure::Look {
            dark: true,
            bg: "#1e1e1e".into(),
            fill: "#2a2a2a".into(),
            text: "#e6e6e6".into(),
            line: "#9a9a9a".into(),
            font: "sans-serif".into(),
        }
    } else {
        figure::Look {
            dark: false,
            bg: "#ffffff".into(),
            fill: "#f3f3f3".into(),
            text: "#1c1c1c".into(),
            line: "#5c5c5c".into(),
            font: "sans-serif".into(),
        }
    }
}

/// The SVG to draw, what kind it is, and the vault path when it came from a file.
fn picture(
    vault: &Vault,
    path: Option<&str>,
    note: Option<&str>,
    args: &Value,
    dark: bool,
) -> Result<(String, &'static str, Option<String>), Outcome> {
    if let Some(note) = note {
        let block = arg_usize(args, "block").ok_or_else(|| Outcome::err("`block` is missing"))?;
        if block == 0 {
            return Err(Outcome::err("`block` starts at 1"));
        }
        let found = vault.find_note(note).map_err(Outcome::err)?;
        let text = fs::read_to_string(&found.path)
            .map_err(disk)
            .map_err(Outcome::err)?;
        let blocks = mermaid_blocks(&text);
        let Some(source) = blocks.get(block - 1) else {
            return Err(Outcome::err(format!("no Mermaid block {block}")));
        };
        let svg = figure::mermaid(source, &look(dark)).map_err(Outcome::err)?;
        return Ok((svg, "mermaid", Some(vault::rel(&vault.root, &found.path))));
    }
    let path = path.unwrap_or("");
    let file = vault.find_file(path).map_err(Outcome::err)?;
    let text = fs::read_to_string(&file)
        .map_err(disk)
        .map_err(Outcome::err)?;
    let shown = Some(vault::rel(&vault.root, &file));
    if text.contains("<svg") {
        return Ok((svg::readable(&text, dark), "svg", shown));
    }
    Err(Outcome::err("this file is not a Mermaid block or an SVG"))
}

fn mermaid_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut open = false;
    for line in text.lines() {
        if crate::logic::markdown::is_fence(line) {
            if open {
                blocks.push(std::mem::take(&mut current));
                open = false;
            } else {
                let info = line.trim_start().trim_start_matches('`').trim();
                let word = info.split_whitespace().next().unwrap_or("");
                open = word.eq_ignore_ascii_case("mermaid");
            }
            continue;
        }
        if open {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
}

/// A dark render is drawn on a dark ground: light ink on white is unreadable.
fn raster(svg: &str, width: Option<f32>, dark: bool) -> Result<(Vec<u8>, u32, u32), String> {
    let tree = svg::parse(svg).ok_or("the picture could not be drawn")?;
    let (w, h) = (tree.size().width().max(1.), tree.size().height().max(1.));
    let want = width.map(|px| px / w).unwrap_or(2.).max(0.05);
    let scale = svg::scale_for(w, h, want);
    let png = figure::png(svg, scale, if dark { 0x1e1e1e } else { 0xffffff })
        .ok_or("the picture could not be drawn")?;
    let (pw, ph) = png_size(&png);
    Ok((png, pw, ph))
}

fn png_size(png: &[u8]) -> (u32, u32) {
    if png.len() < 24 {
        return (0, 0);
    }
    let word = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap_or([0; 4]));
    (word(16), word(20))
}
