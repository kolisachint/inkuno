//! The bindings generator: `uniffi-bindgen generate --language swift --out-dir DIR [--config FILE]
//! [--no-format] LIBRARY`, where LIBRARY is the built `libinkuno_ffi.a` (its metadata is read from it).
//!
//! It is uniffi's `generate` command without its CLI: uniffi's `cli` feature brings clap with its
//! derive, which the app's Cargo.lock cannot hold (clap_derive needs a newer syn than the lock has),
//! and it would change the locked clap 4.6.9 used by criterion.

use uniffi::{GenerateOptions, TargetLanguage};

const USAGE: &str = "usage: uniffi-bindgen generate --language swift --out-dir DIR \
[--config FILE] [--no-format] [--library] LIBRARY";

fn main() {
    if let Err(message) = run(std::env::args().skip(1).collect()) {
        eprintln!("uniffi-bindgen: {message}");
        std::process::exit(2);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut args = args.into_iter();
    if args.next().as_deref() != Some("generate") {
        return Err(USAGE.to_string());
    }
    let mut languages = Vec::new();
    let mut out_dir = None;
    let mut config_override = None;
    let mut format = true;
    let mut source = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--language" | "-l" => languages.push(language(&value()?)?),
            "--out-dir" | "-o" => out_dir = Some(value()?),
            "--config" | "-c" => config_override = Some(value()?.into()),
            "--no-format" => format = false,
            // Accepted for uniffi's command line; a library is detected from the file itself.
            "--library" => {}
            option if option.starts_with('-') => {
                return Err(format!("unknown option {option}\n{USAGE}"));
            }
            _ => source = Some(arg),
        }
    }
    let (Some(source), Some(out_dir)) = (source, out_dir) else {
        return Err(USAGE.to_string());
    };
    if languages.is_empty() {
        return Err("give at least one --language (swift)".to_string());
    }
    uniffi::generate(GenerateOptions {
        languages,
        source: source.into(),
        out_dir: out_dir.into(),
        config_override,
        format,
        crate_filter: None,
        metadata_no_deps: false,
    })
    .map_err(|e| e.to_string())
}

fn language(name: &str) -> Result<TargetLanguage, String> {
    match name {
        "swift" => Ok(TargetLanguage::Swift),
        other => Err(format!("only swift is generated here, not {other}")),
    }
}
