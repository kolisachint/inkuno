//! What a command's work returns, the readers of its arguments, and the disk errors in words.
//! A failure is a sentence, not a panic: `is_error` is set and `structured` holds `{"error": …}`.

use std::io;

use serde_json::{Value, json};

/// The result of one piece of work: its JSON, or the sentence it failed with, and optionally a PNG.
pub struct Outcome {
    pub structured: Value,
    pub is_error: bool,
    pub png: Option<Vec<u8>>,
}

impl Outcome {
    pub fn ok(value: Value) -> Self {
        Self {
            structured: value,
            is_error: false,
            png: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            structured: json!({"error": message.into()}),
            is_error: true,
            png: None,
        }
    }

    pub fn image(value: Value, png: Vec<u8>) -> Self {
        Self {
            structured: value,
            is_error: false,
            png: Some(png),
        }
    }

    /// The sentence a failed outcome holds.
    pub fn sentence(&self) -> String {
        self.structured["error"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

pub fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    match args.get(key) {
        Some(Value::String(text)) if !text.is_empty() => Ok(text),
        Some(_) => Err(format!("`{key}` must be a text")),
        None => Err(format!("`{key}` is missing")),
    }
}

pub fn arg_opt<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|text| !text.is_empty())
}

pub fn arg_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

/// Why a disk operation failed, in the words the app already uses, in English.
pub fn disk(error: io::Error) -> String {
    use io::ErrorKind::*;
    match error.kind() {
        PermissionDenied => "permission denied",
        NotFound => "not found",
        AlreadyExists => "this name is already taken",
        ReadOnlyFilesystem => "the disk is read-only",
        StorageFull => "the disk is full",
        InvalidFilename => "invalid name",
        NotADirectory => "not a folder",
        IsADirectory => "this is a folder",
        DirectoryNotEmpty => "the folder is not empty",
        _ => return error.to_string(),
    }
    .into()
}
