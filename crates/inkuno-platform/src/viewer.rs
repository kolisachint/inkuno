//! The HTML viewer, `inkuno view`: the command line the app starts for an HTML page of the
//! vault. Pure: this builds the arguments; the app starts the process.

use std::path::Path;

/// Whether `file` is an HTML page: the extension `.html` or `.htm`, in any case.
pub fn is_html(file: &Path) -> bool {
    file.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm"))
}

/// The arguments of `inkuno view <file> --root <root>` for `file`, when it is an HTML page;
/// `None` for any other file.
pub fn view_args(file: &Path, root: &Path) -> Option<Vec<String>> {
    is_html(file).then(|| {
        vec![
            "view".to_string(),
            file.display().to_string(),
            "--root".to_string(),
            root.display().to_string(),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::view_args;
    use std::path::Path;

    #[test]
    fn an_html_page_gives_the_view_command() {
        let args = view_args(Path::new("/vault/site/index.html"), Path::new("/vault"));
        assert_eq!(
            args,
            Some(vec![
                "view".to_string(),
                "/vault/site/index.html".to_string(),
                "--root".to_string(),
                "/vault".to_string(),
            ])
        );
    }

    #[test]
    fn the_extension_is_read_in_any_case() {
        assert!(view_args(Path::new("/vault/PAGE.HTM"), Path::new("/vault")).is_some());
        assert!(view_args(Path::new("/vault/Page.Html"), Path::new("/vault")).is_some());
    }

    #[test]
    fn a_markdown_note_is_not_a_page() {
        assert_eq!(
            view_args(Path::new("/vault/note.md"), Path::new("/vault")),
            None
        );
        assert_eq!(
            view_args(Path::new("/vault/html"), Path::new("/vault")),
            None
        );
    }
}
