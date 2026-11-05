//! Colour tokens for the editor and the app's UI (task 10.1).
//!
//! Every colour the editor and the app draw is a field of [`Theme`]. There are two palettes,
//! [`Theme::light`] (the colours the editor had before theming, so Light looks as it did) and
//! [`Theme::dark`] (a GitHub-dark-like palette). The app picks one of them (its `theme` module
//! holds the Auto, Light and Dark mode logic) and stores it with [`set`], as the global
//! [`ActiveTheme`]. Drawing code reads it with [`current`].
//!
//! The split: the token struct and the palettes live here, so the editor can read them without
//! depending on the app. The mode (Auto, Light, Dark), the window-appearance observer and the
//! cycle action live in `app/src/theme.rs`.
//!
//! Pure palette functions take the tokens they need (`&CodeColors`), so they stay testable.

use gpui::{hsla, rgb, rgba, App, Global, Hsla};
use md_render::highlight::TokenKind;

/// The colours of highlighted code, by token kind (task 5.3, and Source Mode in 8.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeColors {
    pub keyword: Hsla,
    pub string: Hsla,
    pub comment: Hsla,
    /// Numbers, constants, properties, attributes and headings.
    pub number: Hsla,
    pub function: Hsla,
    pub type_: Hsla,
    pub link: Hsla,
    /// Variables and plain text inside code.
    pub plain: Hsla,
    /// Source Mode only: markers such as `#`, `*` and link brackets.
    pub punctuation: Hsla,
    /// Source Mode only: emphasis markers.
    pub emphasis: Hsla,
}

/// Every colour token. Field names say where a colour is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    // Editor: the document.
    /// Editor background.
    pub bg: Hsla,
    /// Body text.
    pub text: Hsla,
    /// Muted text: block quotes, markers of hidden syntax, the placeholder note.
    pub text_muted: Hsla,
    /// Thematic breaks and other rules.
    pub marker: Hsla,
    /// Links.
    pub link: Hsla,
    /// Accent for the checked checkbox. Same family as links.
    pub accent: Hsla,
    /// Text on the accent colour (the tick of a checked box).
    pub on_accent: Hsla,
    /// Selection highlight, drawn behind the text.
    pub selection: Hsla,
    /// The caret.
    pub caret: Hsla,
    /// Background of inline code.
    pub code_bg: Hsla,
    /// Background of a code block.
    pub code_block_bg: Hsla,
    /// Grid lines of a table, and the border of an image placeholder.
    pub table_border: Hsla,
    /// Header row of a table, and the fill of an image placeholder.
    pub table_header_bg: Hsla,
    /// Outline of an unchecked checkbox.
    pub box_border: Hsla,
    /// Colours of highlighted code.
    pub code: CodeColors,

    // App: the sidebar, search panel, text fields, banner and status text.
    /// Panel background (sidebar, search panel, the button hover of the banner).
    pub ui_surface: Hsla,
    /// Raised surface: a text field, a banner button, the popup background.
    pub ui_surface_raised: Hsla,
    /// Hover background of a row or button.
    pub ui_hover: Hsla,
    /// Background of the selected sidebar row.
    pub ui_selected: Hsla,
    /// Primary UI text (sidebar rows, field text, banner text).
    pub ui_text: Hsla,
    /// Secondary UI text (search snippets).
    pub ui_text_secondary: Hsla,
    /// Muted UI text (section headers, hints, status messages).
    pub ui_muted: Hsla,
    /// Faint UI text and outlines (mode status, banner button outline).
    pub ui_faint: Hsla,
    /// Borders of panels and text fields.
    pub ui_border: Hsla,
    /// Highlight of a search match.
    pub ui_match: Hsla,
    /// Background of the "file changed" banner.
    pub banner_bg: Hsla,
    /// Bottom border of the banner.
    pub banner_border: Hsla,
    /// The dot that marks unsaved edits, and the banner's accent.
    pub banner_accent: Hsla,
}

/// Hex colour `0xRRGGBB` as an opaque colour.
fn hex(value: u32) -> Hsla {
    Hsla::from(rgb(value))
}

/// Hex colour `0xRRGGBBAA` with alpha.
fn hex_alpha(value: u32) -> Hsla {
    Hsla::from(rgba(value))
}

impl CodeColors {
    /// The GitHub light palette used by the code blocks before theming.
    pub fn light() -> Self {
        Self {
            keyword: hex(0xcf222e),
            string: hex(0x0a3069),
            comment: hex(0x6e7781),
            number: hex(0x0550ae),
            function: hex(0x8250df),
            type_: hex(0x953800),
            link: hex(0x0969da),
            plain: hex(0x24292f),
            punctuation: hex(0x8c959f),
            emphasis: hex(0x6639ba),
        }
    }

    /// A GitHub-dark-like palette.
    pub fn dark() -> Self {
        Self {
            keyword: hex(0xff7b72),
            string: hex(0xa5d6ff),
            comment: hex(0x8b949e),
            number: hex(0x79c0ff),
            function: hex(0xd2a8ff),
            type_: hex(0xffa657),
            link: hex(0x58a6ff),
            plain: hex(0xe6edf3),
            punctuation: hex(0x6e7681),
            emphasis: hex(0xbc8cff),
        }
    }

    /// Every code token, with its name.
    fn all(&self) -> Vec<(&'static str, Hsla)> {
        vec![
            ("code.keyword", self.keyword),
            ("code.string", self.string),
            ("code.comment", self.comment),
            ("code.number", self.number),
            ("code.function", self.function),
            ("code.type", self.type_),
            ("code.link", self.link),
            ("code.plain", self.plain),
            ("code.punctuation", self.punctuation),
            ("code.emphasis", self.emphasis),
        ]
    }
}

/// The colour of a token in a code block (GitHub palette, see [`CodeColors`]).
pub fn code_token_color(kind: TokenKind, colors: &CodeColors) -> Hsla {
    use TokenKind::*;
    match kind {
        Keyword | Operator => colors.keyword,
        String => colors.string,
        Comment => colors.comment,
        Number | Constant | Property | Attribute | Heading => colors.number,
        Function => colors.function,
        Type => colors.type_,
        Link => colors.link,
        Variable | Punctuation | Emphasis | Other => colors.plain,
    }
}

impl Theme {
    /// The Light palette. The editor colours are those used before theming.
    pub fn light() -> Self {
        let link = hsla(0.61, 0.75, 0.50, 1.0);
        Self {
            bg: hex(0xffffff),
            text: hsla(0.0, 0.0, 0.0, 1.0),
            text_muted: hsla(0.0, 0.0, 0.45, 1.0),
            marker: hsla(0.0, 0.0, 0.70, 1.0),
            link,
            accent: link,
            on_accent: hex(0xffffff),
            selection: hex_alpha(0x3366_ff40),
            caret: hsla(0.0, 0.0, 0.0, 1.0),
            code_bg: hsla(0.0, 0.0, 0.50, 0.15),
            code_block_bg: hex(0xf6f8fa),
            table_border: hsla(0.0, 0.0, 0.80, 1.0),
            table_header_bg: hsla(0.0, 0.0, 0.96, 1.0),
            box_border: hsla(0.0, 0.0, 0.55, 1.0),
            code: CodeColors::light(),
            ui_surface: hex(0xf3f4f6),
            ui_surface_raised: hex(0xffffff),
            ui_hover: hex(0xe5e7eb),
            ui_selected: hex(0xdbeafe),
            ui_text: hex(0x1f2937),
            ui_text_secondary: hex(0x374151),
            ui_muted: hex(0x6b7280),
            ui_faint: hex(0x9ca3af),
            ui_border: hex(0xd1d5db),
            ui_match: hex(0xfde68a),
            banner_bg: hex(0xfef3c7),
            banner_border: hex(0xf59e0b),
            banner_accent: hex(0xd97706),
        }
    }

    /// The Dark palette: a GitHub-dark-like scheme.
    pub fn dark() -> Self {
        let link = hex(0x58a6ff);
        Self {
            bg: hex(0x0d1117),
            text: hex(0xe6edf3),
            text_muted: hex(0x8b949e),
            marker: hex(0x484f58),
            link,
            accent: hex(0x388bfd),
            on_accent: hex(0x0d1117),
            selection: hex_alpha(0x388b_fd66),
            caret: hex(0xe6edf3),
            code_bg: hex_alpha(0x6e76_8166),
            code_block_bg: hex(0x161b22),
            table_border: hex(0x30363d),
            table_header_bg: hex(0x161b22),
            box_border: hex(0x8b949e),
            code: CodeColors::dark(),
            ui_surface: hex(0x161b22),
            ui_surface_raised: hex(0x0d1117),
            ui_hover: hex(0x21262d),
            ui_selected: hex(0x1c2d44),
            ui_text: hex(0xe6edf3),
            ui_text_secondary: hex(0xc9d1d9),
            ui_muted: hex(0x8b949e),
            ui_faint: hex(0x6e7681),
            ui_border: hex(0x30363d),
            ui_match: hex(0x6b5a14),
            banner_bg: hex(0x3b2f10),
            banner_border: hex(0x9e6a03),
            banner_accent: hex(0xd29922),
        }
    }

    /// Every colour token with its name, in a fixed order. Used by the tests, and handy when
    /// a new token is added: it must be listed here too.
    pub fn all(&self) -> Vec<(&'static str, Hsla)> {
        let mut out = vec![
            ("bg", self.bg),
            ("text", self.text),
            ("text_muted", self.text_muted),
            ("marker", self.marker),
            ("link", self.link),
            ("accent", self.accent),
            ("on_accent", self.on_accent),
            ("selection", self.selection),
            ("caret", self.caret),
            ("code_bg", self.code_bg),
            ("code_block_bg", self.code_block_bg),
            ("table_border", self.table_border),
            ("table_header_bg", self.table_header_bg),
            ("box_border", self.box_border),
            ("ui_surface", self.ui_surface),
            ("ui_surface_raised", self.ui_surface_raised),
            ("ui_hover", self.ui_hover),
            ("ui_selected", self.ui_selected),
            ("ui_text", self.ui_text),
            ("ui_text_secondary", self.ui_text_secondary),
            ("ui_muted", self.ui_muted),
            ("ui_faint", self.ui_faint),
            ("ui_border", self.ui_border),
            ("ui_match", self.ui_match),
            ("banner_bg", self.banner_bg),
            ("banner_border", self.banner_border),
            ("banner_accent", self.banner_accent),
        ];
        out.extend(self.code.all());
        out
    }
}

/// The palette in use, as a GPUI global. The app sets it (see the module docs).
#[derive(Clone, Copy, Debug)]
pub struct ActiveTheme(pub Theme);

impl Global for ActiveTheme {}

/// The palette in use. Falls back to Light when no palette was set, so drawing never fails.
pub fn current(cx: &App) -> Theme {
    cx.try_global::<ActiveTheme>()
        .map_or_else(Theme::light, |active| active.0)
}

/// Makes `theme` the palette in use. Windows redraw on their next frame.
pub fn set(theme: Theme, cx: &mut App) {
    cx.set_global(ActiveTheme(theme));
}

/// Sets the Light palette if none is set yet. Called from [`crate::editor::init`].
pub fn init(cx: &mut App) {
    if cx.try_global::<ActiveTheme>().is_none() {
        set(Theme::light(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_differ_for_every_token() {
        let light = Theme::light().all();
        let dark = Theme::dark().all();
        assert_eq!(light.len(), dark.len());
        for ((name, light), (dark_name, dark)) in light.iter().zip(dark.iter()) {
            assert_eq!(name, dark_name);
            assert_ne!(light, dark, "token {name} is the same in Light and Dark");
        }
    }

    #[test]
    fn every_token_is_listed_once() {
        let names: Vec<&str> = Theme::light().all().iter().map(|(n, _)| *n).collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn code_token_colours_follow_the_palette() {
        let colors = CodeColors::light();
        assert_eq!(
            code_token_color(TokenKind::Keyword, &colors),
            colors.keyword
        );
        assert_eq!(
            code_token_color(TokenKind::Operator, &colors),
            colors.keyword
        );
        assert_eq!(code_token_color(TokenKind::Heading, &colors), colors.number);
        assert_eq!(code_token_color(TokenKind::Other, &colors), colors.plain);
    }
}
