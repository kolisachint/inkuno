//! The custom protocol: `inkuno://workspace/<path>` answered from the files under the root.
//! Pure over a root and a URL, so it is tested without a window.

use std::{borrow::Cow, fs, path::Path};

use inkuno_platform::web::{self, Refusal};
use wry::http::{Response, header};

/// The HTTP status that answers a refusal.
pub fn status_for(refusal: &Refusal) -> u16 {
    match refusal {
        Refusal::NotFound => 404,
        Refusal::OutsideRoot | Refusal::BadPath => 403,
        Refusal::NotThisScheme => 400,
    }
}

/// The response to a request for `url`: the file read now (never cached, so a reload sees the
/// change), or an empty body with the status of the refusal.
pub fn respond(root: &Path, url: &str) -> Response<Cow<'static, [u8]>> {
    let served = web::resolve(root, url).and_then(|path| {
        // A file that vanished or cannot be read between the check and the read is not found.
        let bytes = fs::read(&path).map_err(|_| Refusal::NotFound)?;
        Ok((web::content_type(&path), bytes))
    });
    let (status, content_type, body) = match served {
        Ok((content_type, bytes)) => (200, content_type, Cow::Owned(bytes)),
        Err(refusal) => (
            status_for(&refusal),
            "text/plain; charset=utf-8",
            Cow::Borrowed(&[][..]),
        ),
    };
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_SECURITY_POLICY, web::CSP)
        .body(body)
        .expect("static header names and a valid status")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_refusal_has_its_status() {
        assert_eq!(status_for(&Refusal::NotFound), 404);
        assert_eq!(status_for(&Refusal::OutsideRoot), 403);
        assert_eq!(status_for(&Refusal::BadPath), 403);
        assert_eq!(status_for(&Refusal::NotThisScheme), 400);
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("inkuno-view-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_file_is_served_fresh_with_type_and_policy() {
        let dir = scratch("serve");
        fs::write(dir.join("a.css"), "a{}").unwrap();
        let first = respond(&dir, "inkuno://workspace/a.css");
        assert_eq!(first.status(), 200);
        assert_eq!(
            first.headers()[header::CONTENT_TYPE],
            "text/css; charset=utf-8"
        );
        assert_eq!(first.headers()[header::CONTENT_SECURITY_POLICY], web::CSP);
        assert_eq!(first.body().as_ref(), b"a{}");
        fs::write(dir.join("a.css"), "b{}").unwrap();
        assert_eq!(
            respond(&dir, "inkuno://workspace/a.css").body().as_ref(),
            b"b{}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refusals_carry_their_status() {
        let dir = scratch("refuse");
        let status = |url: &str| respond(&dir, url).status();
        assert_eq!(status("inkuno://workspace/missing.html"), 404);
        assert_eq!(status("inkuno://workspace/%2e%2e/x"), 403);
        assert_eq!(status("https://example.com/"), 400);
        let _ = fs::remove_dir_all(&dir);
    }
}
