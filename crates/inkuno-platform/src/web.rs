//! The files of the read-only HTML viewer, served through the custom scheme `inkuno://workspace/`.
//! A request URL names a file under the vault's root: `resolve` maps it to the file's canonical
//! path and refuses anything that would leave the root, and `url_for` makes the URL of a file.
//! Only the standard library, and no window: the webview calls these from its protocol handler.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

/// The scheme of the viewer's URLs.
pub const SCHEME: &str = "inkuno";
/// The host of every workspace URL: `inkuno://workspace/<relative path>`.
pub const HOST: &str = "workspace";

/// The Content-Security-Policy of every response: the page's scripts, styles, images, fonts, media
/// and frames come from the workspace scheme or inline (data and blob URLs for the page's own
/// code). The page itself fetches nothing from the network (`connect-src`). Links to the outside
/// are the only way out: a click the user makes on one opens the system's browser or mail client,
/// which the viewer's script sends to the app (`inkuno-view`, `keys::LINKS`).
pub const CSP: &str = "default-src inkuno: data: blob:; script-src inkuno: 'unsafe-inline' 'unsafe-eval' data: blob:; style-src inkuno: 'unsafe-inline' data:; img-src inkuno: data: blob:; font-src inkuno: data:; media-src inkuno: data: blob:; connect-src inkuno: data: blob:; frame-src inkuno:; form-action 'none'; base-uri inkuno:";

/// Why a request URL is not served.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Not an `inkuno:` URL at all: another scheme, or none.
    NotThisScheme,
    /// A path that cannot name a file: a malformed escape, invalid UTF-8, a NUL, `..`, a `/`, on
    /// Windows also a backslash or a colon (a drive or a stream name), or another host than `workspace`.
    BadPath,
    /// The file lies outside the root once links are followed.
    OutsideRoot,
    /// No file there, or a directory without an `index.html`.
    NotFound,
}

/// The URL of a file under `root`, e.g. root `/a` and file `/a/b c/x.html` give
/// `inkuno://workspace/b%20c/x.html`. None when the file does not exist, is not under the root
/// (both are compared canonical, so links are followed), or has a name `resolve` would refuse.
pub fn url_for(root: &Path, file: &Path) -> Option<String> {
    let root = fs::canonicalize(root).ok()?;
    let file = fs::canonicalize(file).ok()?;
    let relative = file.strip_prefix(&root).ok()?;
    let mut segments = Vec::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        let name = name.to_str().filter(|name| plain_name(name))?;
        segments.push(encode_segment(name));
    }
    (!segments.is_empty()).then(|| format!("{SCHEME}://{HOST}/{}", segments.join("/")))
}

/// The file a request URL names, under `root`, canonical. Query and fragment are dropped. An
/// empty path, or one ending in `/`, names `index.html` of that folder. Escapes are decoded, and
/// a name may not be `..` or carry a separator (`%2F` counts), so the path cannot leave the root
/// by its text; the links are then followed, and a result outside the canonical root is refused.
pub fn resolve(root: &Path, url: &str) -> Result<PathBuf, Refusal> {
    let url = url.split(['?', '#']).next().unwrap_or_default();
    let (scheme, after) = url.split_once("://").ok_or(Refusal::NotThisScheme)?;
    if !scheme.eq_ignore_ascii_case(SCHEME) {
        return Err(Refusal::NotThisScheme);
    }
    let (host, path) = after.split_once('/').unwrap_or((after, ""));
    if !host.eq_ignore_ascii_case(HOST) {
        return Err(Refusal::BadPath);
    }
    let mut relative = PathBuf::new();
    for raw in path.split('/').filter(|raw| !raw.is_empty()) {
        let name = decode_segment(raw)?;
        if name == "." {
            continue;
        }
        if !plain_name(&name) {
            return Err(Refusal::BadPath);
        }
        relative.push(name);
    }
    if relative.as_os_str().is_empty() || path.ends_with('/') {
        relative.push("index.html");
    }
    let root = fs::canonicalize(root).map_err(|_| Refusal::NotFound)?;
    let file = fs::canonicalize(root.join(&relative)).map_err(|_| Refusal::NotFound)?;
    if !file.starts_with(&root) {
        return Err(Refusal::OutsideRoot);
    }
    if !file.is_file() {
        return Err(Refusal::NotFound);
    }
    Ok(file)
}

/// The `Content-Type` of a file, by its extension (case-insensitive). Unknown: a byte stream.
pub fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "txt" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Whether a link goes to the system browser rather than the viewer: `http:`, `https:` and
/// `mailto:`, whatever the case of the scheme.
pub fn is_external(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, _)| {
        ["http", "https", "mailto"]
            .iter()
            .any(|known| scheme.eq_ignore_ascii_case(known))
    })
}

/// Whether a decoded name can be one file under a folder: not `..`, no `/`, and not `\` or `:`
/// on Windows (a separator there, a drive or a stream name). Elsewhere those are plain characters.
fn plain_name(name: &str) -> bool {
    name != ".." && !name.contains('/') && !(cfg!(windows) && name.contains(['\\', ':']))
}

/// One path segment, percent-encoded: letters, digits and `-._~` stay, every other byte is `%XX`.
fn encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Decodes the `%XX` escapes of one segment. A malformed escape, invalid UTF-8 or a NUL is
/// refused. `+` stays a plus sign: a path is not a form.
fn decode_segment(raw: &str) -> Result<String, Refusal> {
    let mut bytes = Vec::with_capacity(raw.len());
    let mut rest = raw.bytes();
    while let Some(byte) = rest.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let high = rest.next().and_then(hex_digit);
        let low = rest.next().and_then(hex_digit);
        match (high, low) {
            (Some(high), Some(low)) => bytes.push((high << 4) | low),
            _ => return Err(Refusal::BadPath),
        }
    }
    if bytes.contains(&0) {
        return Err(Refusal::BadPath);
    }
    String::from_utf8(bytes).map_err(|_| Refusal::BadPath)
}

/// The value of one hexadecimal digit, either case.
fn hex_digit(byte: u8) -> Option<u8> {
    char::from(byte).to_digit(16).map(|digit| digit as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A folder under the system temp dir, unique to this process and call; removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("inkuno-web-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, relative: &str, text: &str) -> PathBuf {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }

        fn canonical(&self, relative: &str) -> PathBuf {
            fs::canonicalize(self.0.join(relative)).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A page, a file in a folder with a space, a folder with an index, a folder without one.
    fn site() -> Scratch {
        let root = Scratch::new();
        root.write("index.html", "home");
        root.write("b c/x.html", "nested");
        root.write("docs/index.html", "docs");
        root.write("empty/readme.txt", "no index here");
        root
    }

    /// The refusal of a URL that must not be served.
    fn refused(root: &Scratch, url: &str) -> Refusal {
        resolve(&root.0, url).unwrap_err()
    }

    #[test]
    fn plain_nested_and_encoded_files_resolve() {
        let root = site();
        let cases = [
            ("inkuno://workspace/index.html", "index.html"),
            ("inkuno://workspace/docs/index.html", "docs/index.html"),
            ("inkuno://workspace/b%20c/x.html", "b c/x.html"),
            ("inkuno://workspace/b%20c/%78.html", "b c/x.html"),
            ("inkuno://workspace/b%20c/x.html?v=2#top", "b c/x.html"),
            ("inkuno://workspace/index.html#a?b", "index.html"),
            ("INKUNO://workspace/index.html", "index.html"),
            ("inkuno://workspace/docs/", "docs/index.html"),
            ("inkuno://workspace/", "index.html"),
            ("inkuno://workspace", "index.html"),
        ];
        for (url, relative) in cases {
            assert_eq!(resolve(&root.0, url), Ok(root.canonical(relative)), "{url}");
        }
    }

    #[test]
    fn dotdot_in_any_spelling_is_refused() {
        let root = site();
        for url in [
            "inkuno://workspace/../secret.txt",
            "inkuno://workspace/docs/../../secret.txt",
            "inkuno://workspace/a/..",
            "inkuno://workspace/%2e%2e/secret.txt",
            "inkuno://workspace/%2E%2E/secret.txt",
            "inkuno://workspace/..%2fsecret.txt",
            "inkuno://workspace/%2Fetc%2Fpasswd",
        ] {
            assert_eq!(refused(&root, url), Refusal::BadPath, "{url}");
        }
    }

    #[test]
    fn bad_escapes_nul_utf8_colons_and_hosts_are_refused() {
        let root = site();
        for url in [
            "inkuno://workspace/%00index.html",
            "inkuno://workspace/%FF.html",
            "inkuno://workspace/index%2",
            "inkuno://workspace/index%zz.html",
            "inkuno://other/index.html",
        ] {
            assert_eq!(refused(&root, url), Refusal::BadPath, "{url}");
        }
    }

    /// On Windows a colon or a backslash in a name is a drive or a stream: refused there.
    #[cfg(windows)]
    #[test]
    fn colons_and_backslashes_are_refused_on_windows() {
        let root = site();
        for url in [
            "inkuno://workspace/C:/x.html",
            "inkuno://workspace/a%5Cb.html",
            "inkuno://workspace/..%5csecret.txt",
            "inkuno://workspace/..\\secret.txt",
        ] {
            assert_eq!(refused(&root, url), Refusal::BadPath, "{url}");
        }
        root.write("a:b.txt", "colon");
        assert_eq!(url_for(&root.0, &root.0.join("a:b.txt")), None);
    }

    /// Elsewhere a colon and a backslash are plain characters: `Meeting 10:30.html` round-trips.
    #[cfg(unix)]
    #[test]
    fn colons_and_backslashes_are_plain_names_on_unix() {
        let root = site();
        for name in ["Meeting 10:30.html", "back\\slash.html"] {
            root.write(name, "page");
            let file = root.canonical(name);
            let url = url_for(&root.0, &file).unwrap_or_else(|| panic!("{name} has no URL"));
            assert_eq!(resolve(&root.0, &url), Ok(file), "{url}");
        }
    }

    #[test]
    fn missing_and_directory_targets_are_not_found() {
        let root = site();
        for url in [
            "inkuno://workspace/missing.html",
            "inkuno://workspace/b%20c",
            "inkuno://workspace/empty/",
            "inkuno://workspace/empty",
        ] {
            assert_eq!(refused(&root, url), Refusal::NotFound, "{url}");
        }
    }

    #[test]
    fn other_schemes_are_not_this_scheme() {
        let root = site();
        for url in [
            "https://workspace/index.html",
            "http://inkuno.workspace/index.html",
            "file:///etc/passwd",
            "index.html",
        ] {
            assert_eq!(refused(&root, url), Refusal::NotThisScheme, "{url}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_followed_and_escapes_refused() {
        let root = site();
        let outside = Scratch::new();
        outside.write("secret.txt", "no");
        outside.write("site/index.html", "no");
        let link = |target: PathBuf, name: &str| {
            std::os::unix::fs::symlink(target, root.0.join(name)).unwrap();
        };
        link(root.0.join("index.html"), "alias.html");
        link(outside.0.join("secret.txt"), "link.txt");
        link(outside.0.join("site"), "dir");
        let alias = "inkuno://workspace/alias.html";
        assert_eq!(resolve(&root.0, alias), Ok(root.canonical("index.html")));
        assert_eq!(
            refused(&root, "inkuno://workspace/link.txt"),
            Refusal::OutsideRoot
        );
        assert_eq!(
            refused(&root, "inkuno://workspace/dir/"),
            Refusal::OutsideRoot
        );
    }

    #[test]
    fn url_for_round_trips_through_resolve() {
        let root = site();
        let file = root.canonical("b c/x.html");
        let url = url_for(&root.0, &file).unwrap();
        assert_eq!(url, "inkuno://workspace/b%20c/x.html");
        assert_eq!(resolve(&root.0, &url), Ok(file));

        root.write("café & more #1.md", "text");
        let file = root.canonical("café & more #1.md");
        let url = url_for(&root.0, &file).unwrap();
        assert_eq!(url, "inkuno://workspace/caf%C3%A9%20%26%20more%20%231.md");
        assert_eq!(resolve(&root.0, &url), Ok(file));
    }

    #[test]
    fn url_for_refuses_files_outside_root_or_unnameable() {
        let root = site();
        let outside = Scratch::new();
        let file = outside.write("x.html", "no");
        assert_eq!(url_for(&root.0, &file), None);
        assert_eq!(url_for(&root.0, &root.0.join("missing.html")), None);
    }

    #[test]
    fn content_types_by_extension() {
        let cases = [
            ("a.HTML", "text/html; charset=utf-8"),
            ("a.htm", "text/html; charset=utf-8"),
            ("a.mjs", "text/javascript; charset=utf-8"),
            ("a.jpeg", "image/jpeg"),
            ("a.woff2", "font/woff2"),
            ("a.webm", "video/webm"),
            ("a.txt", "text/plain; charset=utf-8"),
            ("a.wasm", "application/wasm"),
            ("a.pdf", "application/pdf"),
            ("a.exe", "application/octet-stream"),
            ("noext", "application/octet-stream"),
        ];
        for (name, expected) in cases {
            assert_eq!(content_type(Path::new(name)), expected, "{name}");
        }
    }

    #[test]
    fn external_links_are_http_https_and_mailto() {
        for url in ["https://a.org/x", "HTTP://a.org", "mailto:a@b.c"] {
            assert!(is_external(url), "{url}");
        }
        for url in [
            "inkuno://workspace/x.html",
            "file:///etc/passwd",
            "#top",
            "",
        ] {
            assert!(!is_external(url), "{url}");
        }
    }

    #[test]
    fn escapes_decode_and_malformed_ones_are_refused() {
        assert_eq!(decode_segment("a%20b+c").as_deref(), Ok("a b+c"));
        assert_eq!(decode_segment("%C3%A9").as_deref(), Ok("é"));
        for raw in ["%2", "%g0", "%00"] {
            assert_eq!(decode_segment(raw), Err(Refusal::BadPath), "{raw}");
        }
    }

    #[test]
    fn csp_keeps_the_network_closed() {
        assert!(CSP.starts_with("default-src inkuno: data: blob:;"));
        assert!(CSP.ends_with("base-uri inkuno:"));
        assert!(!CSP.contains("http"));
        // A frame of data or blob URL would be a top-level document with no policy of its own.
        assert!(CSP.contains("frame-src inkuno:;"));
        assert!(!CSP.contains("frame-src inkuno: data"));
    }
}
