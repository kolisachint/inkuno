//! Pictures from the web: fetched once, then read from the cache folder.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::vault;

/// Largest picture fetched.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;
/// Time allowed for one fetch.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Extensions a cached picture can carry, the first that matches is the one looked for.
const KINDS: [&str; 9] = [
    "png", "jpg", "gif", "webp", "bmp", "tiff", "ico", "svg", "bin",
];

/// The folder of the fetched pictures.
pub fn cache_dir() -> PathBuf {
    vault::config_dir().join("inkuno/cache/images")
}

/// The cache key of an URL: FNV-1a 64 bits, in hex.
pub fn cache_key(url: &str) -> String {
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// The extension a picture's bytes say it is.
pub fn sniff(bytes: &[u8]) -> &'static str {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
    if bytes.starts_with(b"\x89PNG") {
        "png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "jpg"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if bytes.starts_with(b"BM") {
        "bmp"
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        "tiff"
    } else if bytes.starts_with(&[0, 0, 1, 0]) {
        "ico"
    } else if text.contains("<svg") {
        "svg"
    } else {
        "bin"
    }
}

/// The local file of a picture of the web, fetched if it is not cached yet. Only http and https.
// ponytail: system curl, no HTTP crate; next: an agreed HTTP client.
pub fn fetch_image(url: &str) -> Result<PathBuf, String> {
    if !is_http(url) {
        return Err("only http and https pictures are fetched".into());
    }
    let dir = cache_dir();
    let key = cache_key(url);
    if let Some(cached) = KINDS
        .iter()
        .map(|ext| dir.join(format!("{key}.{ext}")))
        .find(|p| p.is_file())
    {
        return Ok(cached);
    }
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-redirs",
            "5",
        ])
        .args(["--proto", "=http,https", "--proto-redir", "=http,https"])
        .args([
            "--max-time",
            "10",
            "--max-filesize",
            "20971520",
            "--output",
            "-",
        ])
        .arg("--")
        .arg(url)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    if output.stdout.len() > MAX_BYTES {
        return Err("picture too large".into());
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{key}.{}", sniff(&output.stdout)));
    vault::write(&path, &output.stdout).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Whether the text is an http or https URL.
pub fn is_http(url: &str) -> bool {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_stable_and_distinct() {
        let a = cache_key("https://x.test/a.png");
        assert_eq!(a.len(), 16);
        assert_eq!(a, cache_key("https://x.test/a.png"));
        assert_ne!(a, cache_key("https://x.test/b.png"));
        assert_eq!(cache_key(""), "cbf29ce484222325");
    }

    #[test]
    fn kinds_are_sniffed_from_bytes() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), "png");
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), "jpg");
        assert_eq!(sniff(b"GIF89a...."), "gif");
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), "webp");
        assert_eq!(sniff(b"BM...."), "bmp");
        assert_eq!(sniff(b"  <?xml version=\"1.0\"?><svg>"), "svg");
        assert_eq!(sniff(b"<html>404</html>"), "bin");
    }

    #[test]
    fn only_web_addresses_are_fetched() {
        assert!(is_http("HTTPS://x.test/a.png"));
        assert!(!is_http("file:///etc/passwd"));
        assert!(fetch_image("file:///etc/passwd").is_err());
        assert!(fetch_image("a.png").is_err());
    }
}
