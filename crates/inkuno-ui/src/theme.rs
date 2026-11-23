//! Colours and page geometry: the themes, how one is picked from the preferences and the system
//! appearance, and the contrast rules (WCAG) that keep text readable.

use gpui::{BoxShadow, Hsla, Pixels, WindowAppearance, hsla, point, px, rgb};

use inkuno_logic::{lang::tr, prefs::Prefs};

/// Height of the top band of the window (`app-ui.md` §20.1): the top row of the left sidebar (its
/// buttons), of the right one (its tool switcher) and of the editor (its tab bar, or its strip). The
/// window buttons float in the band at the top right.
pub const BAND: f32 = 38.;
/// The band once the title bar is hidden (`app::ToggleTitleBar`): what is left of it is 12 px of
/// top padding, in both sidebars and the editor.
pub const BAND_HIDDEN: f32 = 12.;
/// Room the native traffic lights float over at the window's top left, at scale 1 (macOS). Applied only
/// inside the band that sits at the window's top left in the current state.
pub const LIGHTS_ROOM: f32 = 78.;

/// Width of the text column ("Page width" setting).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Page {
    Narrow,
    #[default]
    Normal,
    Wide,
    /// The whole room of the window.
    Full,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Narrow, Page::Normal, Page::Wide, Page::Full];

    pub fn key(self) -> &'static str {
        match self {
            Page::Narrow => "narrow",
            Page::Normal => "normal",
            Page::Wide => "wide",
            Page::Full => "full",
        }
    }

    /// An unknown or missing setting means the normal width.
    pub fn of(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.key() == key)
            .unwrap_or_default()
    }

    pub fn label(self) -> &'static str {
        match self {
            Page::Narrow => tr("Narrow", "Étroite"),
            Page::Normal => tr("Normal", "Normale"),
            Page::Wide => tr("Wide", "Large"),
            Page::Full => tr("Full width", "Pleine largeur"),
        }
    }

    /// The widest column, for text of `size` px: it follows the body size, so that
    /// the number of characters per line stays the same when zooming in.
    pub fn measure(self, size: f32) -> Pixels {
        let base = match self {
            Page::Narrow => 560.,
            Page::Normal => 800.,
            Page::Wide => 960.,
            Page::Full => return px(f32::MAX),
        };
        px(base * size / Prefs::SIZE)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Theme {
    pub bg: Hsla,
    pub text: Hsla,
    pub dim: Hsla,
    /// Secondary text that stays readable (4.5:1 on the background and the panel): labels, hints,
    /// counters. `dim` stays for Markdown markers, which do not need to be read.
    pub soft: Hsla,
    pub accent: Hsla,
    pub selection: Hsla,
    pub code_bg: Hsla,
    pub panel: Hsla,
    pub border: Hsla,
    /// The window's own background, a step darker than `bg` (see `ground_of`; Quiet sets its own): the sheets (`bg`) lie on it.
    pub ground: Hsla,
    /// Text of a link: the accent, unless the theme sets its own (Quiet).
    pub link: Hsla,
    /// The colours of a code block's tokens, by the indices of `syntax`.
    pub syntax: [Hsla; 6],
    /// The border of an unchecked task box: the text at 22 % in Quiet, `soft` elsewhere.
    pub todo_box: Hsla,
    /// The bar beside a quote: `border` unless the theme sets its own (Quiet).
    pub quote_bar: Hsla,
    /// The dots of a rule (`---`) in Quiet; `None` draws the solid 1 px line in `border`.
    pub dots: Option<Hsla>,
    /// The fill of inline code in Quiet, a chip with a 1 px rim of `chip_line`; `None` keeps the
    /// code's background, `code_bg`.
    pub chip: Option<Hsla>,
    pub chip_line: Hsla,
    /// Quiet's code block: the 1 px border of its box, its line-number gutter (fill, numbers, and the
    /// rule beside it). The other themes draw the box in `code_bg` and no gutter.
    pub code_line: Hsla,
    pub gutter: Hsla,
    pub gutter_num: Hsla,
    /// Quiet's table: the outer border and the grid lines, its header fill. The others draw them in `code_bg`.
    pub table_line: Hsla,
    pub th: Hsla,
    /// Quiet's image ring: a 1 px line over the picture's edge.
    pub ring: Hsla,
    /// Quiet's alert tints (NOTE, TIP, IMPORTANT, WARNING, CAUTION), legible on their fill; `None` elsewhere.
    pub alerts: Option<[Hsla; 5]>,
    /// The fill of a menu or the palette: the raised panel of Quiet, `bg` elsewhere.
    pub popover: Hsla,
    /// The red of destructive items (move to the trash, delete) in Quiet; the accent elsewhere.
    pub caution: Hsla,
    /// Lines that are not text but must be seen (3:1): graph edges, diagram strokes.
    pub line: Hsla,
    /// The surface of a hovered button or row: a little of the text over what is under it, so that it
    /// reads on `bg`, `panel`, `code_bg` and `selection` alike, and is never the colour of a line.
    pub hover: Hsla,
    /// The same, a little stronger, while the pointer is down (`.active`).
    pub pressed: Hsla,
    /// The neutral pill of a selected sidebar row: the text at 7.2 % (Quiet; the others keep `selection`).
    pub pill: Hsla,
    /// Background of a line the agent added to a note, pending (`pending.rs`): a green wash, quiet.
    pub added: Hsla,
    /// Background of a line the agent removed, pending: a red wash, quiet. Its text is `soft`.
    pub removed: Hsla,
    /// The wash behind a commented passage (v2 `--cmw`): yellow, a little stronger on dark.
    pub comment_wash: Hsla,
    /// The gold of a comment: the underline and dot in the note, the quote bar on its card (v2 `--cmd`).
    pub comment_gold: Hsla,
    /// Size of the note's body text, in pixels.
    pub size: f32,
    /// Scale of the interface around the note (panel, menus, help, palette): it follows the note's size,
    /// 1 at 16, from 0.85 to 1.5 so that the panel stays usable.
    pub ui: f32,
    pub page: Page,
    /// Formatting bar at the bottom of the note.
    pub bar: bool,
    /// Short fades when a panel, the format bar or a folder opens; off: everything is instant.
    pub motion: bool,
    /// Multiplier of every line height (see `Prefs::leading`): 1 is normal spacing.
    pub leading: f32,
    /// Line numbers of a code block are drawn (`gutter` is the colour of the gutter).
    pub numbered: bool,
}

/// Offered themes: (name, [background, text, faint, accent, code background, panel, border]).
/// The first two are Dark and Light, which follow the system; the first is also the default
/// base of a custom theme. Inkuno's two come next, then the others.
pub const THEMES: &[(&str, [u32; 7])] = &[
    (
        "Dark",
        [
            0x1c1c1e, 0xe6e6e6, 0x737477, 0x3a85f7, 0x232325, 0x2c2c2e, 0x38393b,
        ],
    ),
    (
        "Light",
        [
            0xffffff, 0x1f2225, 0x9c9fa0, 0x3a85f7, 0xfbfbfb, 0xffffff, 0xeaebeb,
        ],
    ),
    (
        "Inkuno Dark",
        [
            0x171a22, 0xf5f5f2, 0x77716a, 0x4263eb, 0x262423, 0x242221, 0x3a3735,
        ],
    ),
    (
        "Inkuno Light",
        [
            0xf5f5f2, 0x20232b, 0xa8a29a, 0x4263eb, 0xf1eeea, 0xffffff, 0xe2ddd6,
        ],
    ),
    (
        "Dracula",
        [
            0x282a36, 0xf8f8f2, 0x6272a4, 0xbd93f9, 0x21222c, 0x343746, 0x44475a,
        ],
    ),
    (
        "One Dark",
        [
            0x282c34, 0xabb2bf, 0x5c6370, 0x61afef, 0x21252b, 0x2c313a, 0x3e4451,
        ],
    ),
    (
        "Gruvbox Dark",
        [
            0x282828, 0xebdbb2, 0x928374, 0xfabd2f, 0x32302f, 0x3c3836, 0x504945,
        ],
    ),
    (
        "Nord",
        [
            0x2e3440, 0xd8dee9, 0x616e88, 0x88c0d0, 0x3b4252, 0x3b4252, 0x4c566a,
        ],
    ),
    (
        "Catppuccin Mocha",
        [
            0x1e1e2e, 0xcdd6f4, 0x6c7086, 0xcba6f7, 0x181825, 0x313244, 0x45475a,
        ],
    ),
    (
        "Tokyo Night",
        [
            0x1a1b26, 0xc0caf5, 0x565f89, 0x7aa2f7, 0x16161e, 0x24283b, 0x3b4261,
        ],
    ),
    (
        "Rosé Pine",
        [
            0x191724, 0xe0def4, 0x6e6a86, 0xebbcba, 0x1f1d2e, 0x26233a, 0x403d52,
        ],
    ),
    (
        "Solarized Dark",
        [
            0x002b36, 0x93a1a1, 0x586e75, 0x268bd2, 0x073642, 0x073642, 0x0a4856,
        ],
    ),
    (
        "Solarized Light",
        [
            0xfdf6e3, 0x586e75, 0x93a1a1, 0x268bd2, 0xeee8d5, 0xfffbf0, 0xd9d2c2,
        ],
    ),
    (
        "Catppuccin Latte",
        [
            0xeff1f5, 0x4c4f69, 0x9ca0b0, 0x8839ef, 0xe6e9ef, 0xffffff, 0xccd0da,
        ],
    ),
    (
        "Gruvbox Light",
        [
            0xfbf1c7, 0x3c3836, 0x928374, 0xb57614, 0xf2e5bc, 0xf9f5d7, 0xd5c4a1,
        ],
    ),
    (
        "Sepia",
        [
            0xf4ecd8, 0x433422, 0xa89a83, 0x7c4fb0, 0xe9dcbf, 0xfffdf2, 0xd8cba6,
        ],
    ),
];

/// Version of the `themes.json` schema (`{"version": 1, "themes": […]}`).
pub const THEMES_VERSION: u32 = 1;

/// Seed tokens a custom theme sets, in order: `bg`, `text`, `dim`, `accent`,
/// `code_bg`, `panel`, `border`. `of` adjusts and derives the rest.
pub const THEME_TOKENS: [&str; 7] = ["bg", "text", "dim", "accent", "code_bg", "panel", "border"];

/// Optional extra tokens of a custom theme, by the names of `Extras`: each overrides the colour
/// derived from the seeds (`derived_extras`). Missing ones stay derived.
pub const EXTRA_TOKENS: [&str; 13] = [
    "link",
    "ground",
    "quote_bar",
    "chip",
    "chip_line",
    "code_line",
    "gutter",
    "gutter_num",
    "th",
    "popover",
    "caution",
    "syntax",
    "alerts",
];

/// The overrides a custom theme sets, see `EXTRA_TOKENS`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Overrides {
    pub link: Option<u32>,
    pub ground: Option<u32>,
    pub quote_bar: Option<u32>,
    pub chip: Option<u32>,
    pub chip_line: Option<u32>,
    pub code_line: Option<u32>,
    pub gutter: Option<u32>,
    pub gutter_num: Option<u32>,
    pub th: Option<u32>,
    pub popover: Option<u32>,
    pub caution: Option<u32>,
    pub syntax: Option<[u32; 6]>,
    pub alerts: Option<[u32; 5]>,
}

/// One custom theme from `themes.json`: its name, the built-in it inherits
/// missing tokens from, its seven seed colors, and the extra tokens it sets.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomTheme {
    pub name: String,
    pub base: String,
    pub seeds: [u32; 7],
    pub overrides: Overrides,
}

impl Overrides {
    /// The override slot of the single-colour token `token` (`syntax` and `alerts` are lists: `None`).
    fn slot(&mut self, token: &str) -> Option<&mut Option<u32>> {
        Some(match token {
            "link" => &mut self.link,
            "ground" => &mut self.ground,
            "quote_bar" => &mut self.quote_bar,
            "chip" => &mut self.chip,
            "chip_line" => &mut self.chip_line,
            "code_line" => &mut self.code_line,
            "gutter" => &mut self.gutter,
            "gutter_num" => &mut self.gutter_num,
            "th" => &mut self.th,
            "popover" => &mut self.popover,
            "caution" => &mut self.caution,
            _ => return None,
        })
    }

    /// `e` with the colours this theme sets in place of the derived ones.
    fn apply(&self, e: &mut Extras) {
        e.link = self.link.unwrap_or(e.link);
        e.ground = self.ground.unwrap_or(e.ground);
        e.quote_bar = self.quote_bar.unwrap_or(e.quote_bar);
        e.chip = self.chip.unwrap_or(e.chip);
        e.chip_line = self.chip_line.unwrap_or(e.chip_line);
        e.code_line = self.code_line.unwrap_or(e.code_line);
        e.gutter = self.gutter.unwrap_or(e.gutter);
        e.gutter_num = self.gutter_num.unwrap_or(e.gutter_num);
        e.th = self.th.unwrap_or(e.th);
        e.popover = self.popover.unwrap_or(e.popover);
        e.caution = self.caution.unwrap_or(e.caution);
        e.syntax = self.syntax.unwrap_or(e.syntax);
        e.alerts = self.alerts.unwrap_or(e.alerts);
    }
}

/// A list of `N` `#rrggbb` colors; `None` when it is not one.
fn hex_list<const N: usize>(value: &serde_json::Value) -> Option<[u32; N]> {
    let items = value.as_array()?;
    if items.len() != N {
        return None;
    }
    let mut out = [0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_str().and_then(parse_hex)?;
    }
    Some(out)
}

/// A `#rrggbb` color (the `#` may be omitted); `None` when it is not one.
pub fn parse_hex(text: &str) -> Option<u32> {
    let text = text.strip_prefix('#').unwrap_or(text);
    (text.len() == 6 && text.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(text, 16).ok())
        .flatten()
}

fn base_seeds(base: &str) -> ([u32; 7], bool) {
    // Settings and themes written before the rename still name the Quiet themes.
    let base = match base {
        "Quiet Dark" => "Dark",
        "Quiet Light" => "Light",
        other => other,
    };
    match THEMES.iter().find(|t| t.0 == base) {
        Some(theme) => (theme.1, true),
        // Unknown base: the default theme (THEMES[0]), and the caller warns.
        None => (THEMES[0].1, false),
    }
}

/// Reads `themes.json` (`{"version": 1, "themes": [{name, base, colors}, …]}`):
/// the valid custom themes, the faulty ones (ignored one by one) and the warnings
/// (unknown base, tokens inherited from it). A missing token inherits the base's;
/// an unreadable color drops the whole theme. Unknown fields are ignored.
pub fn parse_themes(text: &str) -> (Vec<CustomTheme>, Vec<String>, Vec<String>) {
    if text.trim().is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(fault) => {
            return (
                Vec::new(),
                vec![format!("invalid JSON: {fault}")],
                Vec::new(),
            );
        }
    };
    let mut themes = Vec::new();
    let mut errors = Vec::new();
    let mut warns = Vec::new();
    if value.get("version").and_then(|v| v.as_u64()) != Some(u64::from(THEMES_VERSION)) {
        errors.push(format!(
            "version: expected {}, got {}",
            THEMES_VERSION,
            value.get("version").unwrap_or(&serde_json::Value::Null)
        ));
    }
    let Some(list) = value.get("themes").and_then(|v| v.as_array()) else {
        errors.push("themes: expected an array of custom themes".into());
        return (themes, errors, warns);
    };
    for (i, entry) in list.iter().enumerate() {
        let at = format!("themes[{i}]");
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            errors.push(format!("{at}: missing name, the theme is ignored"));
            continue;
        }
        let at = format!("{at} ({name})");
        let base = entry
            .get("base")
            .and_then(|v| v.as_str())
            .unwrap_or(THEMES[0].0);
        let (fallback, known) = base_seeds(base);
        if !known {
            warns.push(format!(
                "{at}: unknown base {base}, {} used instead",
                THEMES[0].0
            ));
        }
        let Some(colors) = entry.get("colors").and_then(|v| v.as_object()) else {
            errors.push(format!("{at}: missing colors, the theme is ignored"));
            continue;
        };
        let mut seeds = fallback;
        let mut inherited = Vec::new();
        let mut faulty = false;
        for (n, token) in THEME_TOKENS.into_iter().enumerate() {
            match colors.get(token) {
                None => inherited.push(token),
                Some(color) => match color.as_str().and_then(parse_hex) {
                    Some(hex) => seeds[n] = hex,
                    None => {
                        errors.push(format!(
                            "{at}: colors.{token} is not a #rrggbb color, the theme is ignored"
                        ));
                        faulty = true;
                        break;
                    }
                },
            }
        }
        // The optional extra tokens: a single colour, or a list for `syntax` and `alerts`.
        let mut overrides = Overrides::default();
        for token in EXTRA_TOKENS {
            let Some(value) = colors.get(token) else {
                continue;
            };
            let read = match token {
                "syntax" => hex_list(value).map(|list| overrides.syntax = Some(list)),
                "alerts" => hex_list(value).map(|list| overrides.alerts = Some(list)),
                _ => match (value.as_str().and_then(parse_hex), overrides.slot(token)) {
                    (Some(hex), Some(slot)) => {
                        *slot = Some(hex);
                        Some(())
                    }
                    _ => None,
                },
            };
            if read.is_none() {
                errors.push(format!(
                    "{at}: colors.{token} is not a #rrggbb color (six for syntax, five for alerts), the theme is ignored"
                ));
                faulty = true;
                break;
            }
        }
        if faulty {
            continue;
        }
        if !inherited.is_empty() {
            warns.push(format!(
                "{at}: inherits {} from {base}",
                inherited.join(", ")
            ));
        }
        if themes.iter().any(|t: &CustomTheme| t.name == name) {
            errors.push(format!("{at}: duplicate name, the theme is ignored"));
            continue;
        }
        themes.push(CustomTheme {
            name: name.into(),
            base: base.into(),
            seeds,
            overrides,
        });
    }
    (themes, errors, warns)
}

/// Pairs of a custom theme below the legibility bar, before `of` auto-fixes them:
/// text kinds under 4.5:1 on their ground, panel kinds under 1.03:1.
fn seed_report(seeds: [u32; 7]) -> Vec<String> {
    let [bg, text, _, accent, code_bg, panel, _] = seeds.map(|c| Hsla::from(rgb(c)));
    let mut short = Vec::new();
    for (kind, pair, bar) in [
        ("text", contrast(text, bg), 4.5),
        ("text on code", contrast(text, code_bg), 4.5),
        ("accent", contrast(accent, bg), 4.5),
        ("panel", contrast(bg, panel), 1.03),
    ] {
        if pair < bar {
            short.push(format!("{kind} {pair:.2}:1"));
        }
    }
    short
}

/// Validates `themes.json` without applying it (for `--check-config`): the custom theme
/// names, the errors (a faulty theme is dropped) and the warnings (inherited tokens,
/// colors `of` will auto-fix for legibility).
pub fn check_themes(text: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (themes, errors, warns) = parse_themes(text);
    let mut warns = warns;
    for theme in &themes {
        for short in seed_report(theme.seeds) {
            warns.push(format!(
                "theme {}: low contrast, auto-fixed ({short})",
                theme.name
            ));
        }
    }
    (themes.into_iter().map(|t| t.name).collect(), errors, warns)
}

/// Slots of `Theme::syntax`: a code block's comments, keywords, strings, numbers, types and calls.
pub mod syntax {
    pub const COMMENT: usize = 0;
    pub const KEYWORD: usize = 1;
    pub const STRING: usize = 2;
    pub const NUMBER: usize = 3;
    pub const TYPE: usize = 4;
    pub const FUNCTION: usize = 5;
}

/// What the Quiet look sets beyond the seeds: the link, the ground, the six code colours and the
/// block marks. Dark and Light have their own (`fixed_extras`, the Quiet.md design spec); the other
/// themes derive theirs from the seeds (`derived_extras`), then take the ones their file sets.
struct Extras {
    link: u32,
    ground: u32,
    syntax: [u32; 6],
    /// Alphas of the text for the task box's border and the rule's dots.
    todo_box: f32,
    dots: f32,
    quote_bar: u32,
    chip: u32,
    chip_line: u32,
    code_line: u32,
    gutter: u32,
    gutter_num: u32,
    th: u32,
    /// Ink of the table lines and the ring (black in Light, white in Dark), at these alphas.
    ink: u32,
    table_alpha: f32,
    ring_alpha: f32,
    /// The five alert tints, NOTE to CAUTION.
    alerts: [u32; 5],
    /// The fill of menus and the palette, and the red of destructive items.
    popover: u32,
    caution: u32,
}

/// Alerts of Dark and Light: NOTE, TIP, IMPORTANT, WARNING, CAUTION.
const DARK_ALERTS: [u32; 5] = [0x6ea6ff, 0x58c48a, 0xa98bff, 0xe0a23c, 0xff7b72];
const LIGHT_ALERTS: [u32; 5] = [0x2f6fd6, 0x2f8a57, 0x7c4dd6, 0xa8650a, 0xc93a3a];

/// The Quiet extras of the built-in theme `name`, when it is Dark or Light.
fn fixed_extras(name: &str) -> Option<Extras> {
    match name {
        "Light" => Some(Extras {
            link: 0x105592,
            ground: 0xeff0f1,
            syntax: [0x8d9096, 0xb2565c, 0x89713a, 0x9759b3, 0x3878b1, 0x5f7e43],
            todo_box: 0.40,
            dots: 0.45,
            quote_bar: 0xc9cacb,
            chip: 0xf6f6f7,
            chip_line: 0xe2e3e3,
            code_line: 0xeaebeb,
            gutter: 0xf8f8f8,
            gutter_num: 0xb9babc,
            th: 0xf8f8f8,
            ink: 0x000000,
            table_alpha: 0.09,
            ring_alpha: 0.08,
            alerts: LIGHT_ALERTS,
            popover: 0xfafafa,
            caution: 0xc93a3a,
        }),
        "Dark" => Some(Extras {
            link: 0x8ab4f8,
            ground: 0x141517,
            syntax: [0x8d9096, 0xe06c75, 0xd8b25c, 0xc58af9, 0x6cb6ff, 0x98c379],
            todo_box: 0.40,
            dots: 0.40,
            quote_bar: 0x676767,
            chip: 0x262628,
            chip_line: 0x3a3b3e,
            code_line: 0x36373a,
            gutter: 0x1f1f21,
            gutter_num: 0x868686,
            th: 0x232325,
            ink: 0xffffff,
            table_alpha: 0.12,
            ring_alpha: 0.12,
            alerts: DARK_ALERTS,
            popover: 0x2a2a2c,
            caution: 0xff7b72,
        }),
        _ => None,
    }
}

/// The custom theme `name`, if `themes.json` holds it.
fn custom_theme(name: &str) -> Option<CustomTheme> {
    custom_in(name, &inkuno_platform::vault::load_themes())
}

/// The custom theme `name` in the `themes.json` text (pure half of [`custom_theme`]).
fn custom_in(name: &str, text: &str) -> Option<CustomTheme> {
    let (custom, _, _) = parse_themes(text);
    custom.into_iter().find(|theme| theme.name == name)
}

/// The Quiet look of a theme that is neither Dark nor Light, derived from its seeds the way Dark and
/// Light relate to their own: the ground is `ground_of`, the chip, code lines and gutter are blends of
/// the text over the sheet or the code background at the alphas of those two, the alerts are their set
/// (the dark one for a dark theme) and the caution is its red. The text, the soft grey and the accent
/// are read here as `Theme::of` reads them, so the colours are legible on their own grounds.
fn derived_extras(seeds: [u32; 7], dark: bool) -> Extras {
    let [bg, text, _, accent, code_bg, panel, _] = seeds.map(|c| Hsla::from(rgb(c)));
    let ground = ground_of(bg);
    let reads = [bg, panel, code_bg, ground];
    let text = reads.iter().fold(text, |c, b| legible(c, *b));
    let soft = faintest(text, bg, reads);
    let accent = read_accent(accent, bg, ground, &[bg, panel]);
    let code = |by: f32| {
        [bg, code_bg].iter().fold(
            Hsla {
                h: (accent.h + by).fract(),
                ..accent
            },
            |c, b| legible(c, *b),
        )
    };
    let syntax = [soft, accent, code(0.33), code(0.6), text, text];
    let alerts = if dark { DARK_ALERTS } else { LIGHT_ALERTS };
    let gutter = hex(blend(text, code_bg, 0.02));
    Extras {
        link: hex(accent),
        ground: hex(ground),
        syntax: syntax.map(hex),
        todo_box: 0.40,
        dots: if dark { 0.40 } else { 0.45 },
        quote_bar: hex(blend(text, bg, if dark { 0.40 } else { 0.24 })),
        chip: hex(blend(text, code_bg, 0.03)),
        chip_line: hex(blend(text, code_bg, 0.12)),
        code_line: hex(blend(text, bg, if dark { 0.13 } else { 0.08 })),
        gutter,
        gutter_num: hex(soft),
        th: gutter,
        ink: if dark { 0xffffff } else { 0x000000 },
        table_alpha: if dark { 0.12 } else { 0.09 },
        ring_alpha: if dark { 0.12 } else { 0.08 },
        alerts,
        popover: hex(panel),
        caution: alerts[4],
    }
}

/// `color` as `0xrrggbb`, each channel to the nearest step (the alpha is dropped).
fn hex(color: Hsla) -> u32 {
    let c = color.to_rgb();
    let channel = |x: f32| (x * 255.).round() as u32;
    (channel(c.r) << 16) | (channel(c.g) << 8) | channel(c.b)
}

/// The faintest grey between `text` and `bg` that reads at 4.5:1 on each of `reads`, else `text`.
fn faintest(text: Hsla, bg: Hsla, reads: [Hsla; 4]) -> Hsla {
    (0..=20)
        .map(|i| blend(text, bg, 0.5 + i as f32 * 0.025))
        .find(|c| reads.iter().all(|b| contrast(*c, *b) >= 4.5))
        .unwrap_or(text)
}

/// The accent, as strong as the text needs: legible on `reads`, on the ground and in an open pill (10 %
/// of itself over `bg` or the ground). The pill takes its tint from the accent it holds, so it is read
/// again from the accent made legible, until it settles.
fn read_accent(accent: Hsla, bg: Hsla, ground: Hsla, reads: &[Hsla]) -> Hsla {
    (0..4).fold(accent, |a, _| {
        let pills = [blend(a, bg, 0.1), blend(a, ground, 0.1)];
        reads
            .iter()
            .chain([&ground])
            .chain(&pills)
            .fold(a, |c, b| legible(c, *b))
    })
}

impl Theme {
    /// The chosen theme; failing that, Quiet's one matching the system appearance.
    pub fn of(prefs: &Prefs, appearance: WindowAppearance) -> Self {
        let dark = matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let system = &THEMES[if dark { 0 } else { 1 }];
        // Settings written before the rename still name the Quiet themes.
        let requested = match prefs.theme.as_str() {
            "Quiet Dark" => "Dark",
            "Quiet Light" => "Light",
            other => other,
        };
        // Built-ins win by name (their tests stay immune to the config folder); a custom
        // theme from `themes.json` applies otherwise, its low contrast auto-fixed below.
        let (seeds, fixed, overrides) = match THEMES.iter().find(|t| t.0 == requested) {
            Some(theme) => (theme.1, fixed_extras(theme.0), Overrides::default()),
            None => match custom_theme(requested) {
                Some(custom) => (custom.seeds, None, custom.overrides),
                None => (system.1, fixed_extras(system.0), Overrides::default()),
            },
        };
        let [bg, text, dim, accent, code_bg, panel, border] = seeds.map(|c| Hsla::from(rgb(c)));
        // Every theme has the Quiet look: Dark and Light their own extras, the others derived
        // from their seeds, and the ones their file sets on top.
        let derived = fixed.is_none();
        let mut extras = fixed.unwrap_or_else(|| derived_extras(seeds, bg.l < 0.5));
        overrides.apply(&mut extras);
        // A derived ground is `ground_of` itself: the hex of `extras` would lose a step of its 1.08:1.
        let ground = if derived && overrides.ground.is_none() {
            ground_of(bg)
        } else {
            Hsla::from(rgb(extras.ground))
        };
        // The selection, as strong as the text over it allows.
        let share = (0..=15)
            .map(|i| if bg.l < 0.5 { 0.25 } else { 0.2 } - i as f32 * 0.01)
            .find(|&a| {
                [bg, panel]
                    .iter()
                    .all(|b| contrast(text, blend(accent, *b, a)) >= 4.5)
            })
            .unwrap_or(0.12);
        let picked = [
            bg,
            panel,
            blend(accent, bg, share),
            blend(accent, panel, share),
        ];
        // Some themes' own text is under 4.5:1 on their code background or selected (Solarized).
        let text = picked
            .iter()
            .chain([&code_bg, &ground])
            .fold(text, |c, b| legible(c, *b));
        // The faintest grey between text and background that still reads on the page, the panel and code.
        let soft = faintest(text, bg, [bg, panel, code_bg, ground]);
        // Links and tags are written in the accent: it must read like text, selected too, on the ground
        // and in an open pill.
        let accent_read = read_accent(accent, bg, ground, &picked);
        // The extras' colours read on the sheet and on a code block, as the derived ones do.
        let read = |c: Hsla| [bg, code_bg].iter().fold(c, |c, b| legible(c, *b));
        // The link also reads selected, and on the ground.
        let link = [bg, code_bg]
            .iter()
            .chain(picked.iter())
            .chain([&ground])
            .fold(Hsla::from(rgb(extras.link)), |c, b| legible(c, *b));
        let syntax = extras.syntax.map(|c| read(Hsla::from(rgb(c))));
        // The block marks and the blocks' own lines of the Quiet look.
        let ink = Hsla::from(rgb(extras.ink));
        let fill = |c: u32| blend(Hsla::from(rgb(c)), bg, 0.07);
        let todo_box = text.opacity(extras.todo_box);
        let quote_bar = Hsla::from(rgb(extras.quote_bar));
        let dots = Some(text.opacity(extras.dots));
        let chip = Some(Hsla::from(rgb(extras.chip)));
        let chip_line = Hsla::from(rgb(extras.chip_line));
        let code_line = Hsla::from(rgb(extras.code_line));
        let gutter = Hsla::from(rgb(extras.gutter));
        let gutter_num = Hsla::from(rgb(extras.gutter_num));
        let table_line = ink.opacity(extras.table_alpha);
        let th = Hsla::from(rgb(extras.th));
        let ring = ink.opacity(extras.ring_alpha);
        // The alerts' tints read on their fill (7 % of the tint over the page).
        let alerts = extras.alerts.map(|c| legible(Hsla::from(rgb(c)), fill(c)));
        let popover = Hsla::from(rgb(extras.popover));
        let caution = Hsla::from(rgb(extras.caution));
        // The comment tokens of v2 (`--cmw`, `--cmd`) by theme lightness. The wash is v2's share, lowered
        // until the text and the accent read on it at 4.5:1 (a light text on a dark theme is lifted by it).
        let dark = bg.l < 0.5;
        let yellow = Hsla::from(rgb(0xe8b01e));
        let wash = (0..=12)
            .map(|i| if dark { 0.22 } else { 0.26 } - i as f32 * 0.01)
            .find(|&a| {
                [bg, panel].iter().all(|b| {
                    [text, accent_read]
                        .iter()
                        .all(|c| contrast(*c, blend(yellow, *b, a)) >= 4.5)
                })
            })
            .unwrap_or(0.12);
        Self {
            popover,
            caution,
            todo_box,
            quote_bar,
            dots,
            chip,
            chip_line,
            code_line,
            gutter,
            gutter_num,
            table_line,
            th,
            ring,
            alerts: Some(alerts),
            bg,
            text,
            dim,
            soft,
            accent: accent_read,
            link,
            syntax,
            line: reach(reach(dim, bg, 3.), code_bg, 3.),
            selection: accent.opacity(share),
            hover: text.opacity(0.07),
            pressed: text.opacity(0.13),
            pill: text.opacity(0.072),
            added: pending_wash(GREEN, bg, [text, soft]),
            removed: pending_wash(RED, bg, [text, soft]),
            comment_wash: yellow.opacity(wash),
            comment_gold: Hsla::from(rgb(if dark { 0xf0c040 } else { 0xc98a00 })),
            code_bg,
            panel,
            border,
            ground,
            size: prefs.size,
            ui: (prefs.size / Prefs::SIZE).clamp(0.85, 1.5),
            page: Page::of(&prefs.page),
            bar: prefs.bar != "off",
            motion: prefs.motion != "off",
            leading: prefs.leading(),
            numbered: prefs.gutter != "off",
        }
    }
}

impl Theme {
    /// `v` pixels of the interface at its scale.
    pub fn px(&self, v: f32) -> Pixels {
        px(v * self.ui)
    }

    /// The small radius of the interface: fields, buttons, pills, rows.
    pub fn small(&self) -> Pixels {
        self.px(6.)
    }

    /// The radius of a tab's pill.
    pub fn tab(&self) -> Pixels {
        self.px(9.)
    }

    /// The large radius: cards, bars, panels, blocks of the note.
    pub fn large(&self) -> Pixels {
        self.px(8.)
    }

    /// The radius of a card of rows: the Settings page's groups.
    pub fn card(&self) -> Pixels {
        self.px(10.)
    }
    /// The radius of a sheet laid on the ground: the note, the panel, the tools.
    #[allow(dead_code)] // used by the views from step 3 of #91
    pub fn sheet(&self) -> Pixels {
        self.px(12.)
    }

    /// The radius of a tinted band inside a sheet (`code_bg`): table headers.
    #[allow(dead_code)] // used by the views from step 3 of #91
    pub fn band(&self) -> Pixels {
        self.px(10.)
    }

    /// What sits on the page: a sheet, tooltips, a dragged tab.
    pub fn lifted(&self) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color: hsla(0., 0., 0., 0.18),
            offset: point(px(0.), self.px(1.)),
            blur_radius: self.px(6.),
            spread_radius: px(0.),
            inset: false,
        }]
    }

    /// Quiet's sheet edge (v2 `--shadow-note`): a 1 px hairline and a 10 px fall on the left of the note. Fainter
    /// on a light ground than on a dark one (the sheet's luminance tells which).
    pub fn note_shadow(&self) -> Vec<BoxShadow> {
        let dark = self.bg.l < 0.5;
        let (near, far) = if dark { (0.30, 0.35) } else { (0.035, 0.045) };
        vec![
            BoxShadow {
                color: hsla(0., 0., 0., near),
                offset: point(px(-1.), px(0.)),
                blur_radius: px(1.),
                spread_radius: px(0.),
                inset: false,
            },
            BoxShadow {
                color: hsla(0., 0., 0., far),
                offset: point(px(-2.), px(0.)),
                blur_radius: px(if dark { 12. } else { 10. }),
                spread_radius: px(0.),
                inset: false,
            },
        ]
    }

    /// Quiet's menu shadows: 0 1 3 at 5 % and 0 8 24 at 10 %, under a popover.
    pub fn popover_shadow(&self) -> Vec<BoxShadow> {
        let shadow = |y: f32, blur: f32, alpha: f32| BoxShadow {
            color: hsla(0., 0., 0., alpha),
            offset: point(px(0.), px(y)),
            blur_radius: px(blur),
            spread_radius: px(0.),
            inset: false,
        };
        vec![shadow(1., 3., 0.05), shadow(8., 24., 0.10)]
    }

    /// What covers the page: palette, menus, help, toasts.
    pub fn floating(&self) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color: hsla(0., 0., 0., 0.28),
            offset: point(px(0.), self.px(6.)),
            blur_radius: self.px(20.),
            spread_radius: px(0.),
            inset: false,
        }]
    }

    /// Background of a highlight (`==text==`) of color `i` (see `md::MARKS`), opaque.
    pub fn highlight(&self, i: usize) -> Hsla {
        let tint = highlight_tint(i);
        // As strong as the text over it allows.
        (0..=30)
            .map(|k| blend(tint, self.bg, 0.4 - k as f32 * 0.01))
            .find(|c| contrast(self.text, *c) >= 4.5)
            .unwrap_or(self.bg)
    }

    /// One of the five hues of the recipe (`HUES`), readable on `under`: icons of the messages, pills.
    pub fn hue(&self, h: f32, under: Hsla) -> Hsla {
        legible(
            Hsla {
                h,
                s: 0.7,
                l: 0.5,
                a: 1.,
            },
            under,
        )
    }

    /// Hover of the close button: the red of the callout recipe, translucent like `hover`, so it lies on any surface.
    pub fn danger_hover(&self) -> Hsla {
        Hsla {
            a: 0.2,
            ..Hsla {
                h: RED,
                s: 0.7,
                l: 0.5,
                a: 1.,
            }
        }
    }

    /// The label of a callout of hue `tint`: legible on the callout's tint (10 % over the page), then drawn
    /// a tenth of the way toward the text so that it does not outshine the body text. With no side band, the
    /// label and the tint alone say the type.
    pub fn callout_label(&self, tint: Hsla) -> Hsla {
        blend(legible(tint, blend(tint, self.bg, 0.1)), self.text, 0.1)
    }

    /// The radius of a block's box (a code block, a table, a callout): 8 px at size 15.
    pub fn block_radius(&self) -> Pixels {
        px(8. * self.size / Prefs::SIZE)
    }

    /// Quiet's tint of a `[!TYPE]` alert (`label` with its brackets), if it is one of the five it has.
    pub fn alert(&self, label: &str) -> Option<Hsla> {
        let alerts = self.alerts?;
        let kind = [
            "[!note]",
            "[!tip]",
            "[!important]",
            "[!warning]",
            "[!caution]",
        ]
        .iter()
        .position(|k| label.eq_ignore_ascii_case(k))?;
        Some(alerts[kind])
    }
}

/// The ground under the sheets of `bg`: its lightness lowered by 7.5 %, then by 2 % more until the two tell apart
/// (1.08:1). The hue stays and so does the saturation (up to 0.3, or a yellow sheet would get a mustard ground): a
/// warm sheet gets a warm ground, where a blend toward black would grey it.
/// Light themes get their 7.5 % at once; the dark ones, whose `bg` is already near black, need a third.
fn ground_of(bg: Hsla) -> Hsla {
    let lower = |by: f32| Hsla {
        l: bg.l * (1. - by),
        s: bg.s.min(0.3),
        ..bg
    };
    (0..25)
        .map(|i| lower(0.075 + i as f32 * 0.02))
        .find(|g| contrast(*g, bg) >= 1.08)
        .unwrap_or(hsla(0., 0., 0., 1.))
}

/// Radius of the window itself when the app draws its frame (Linux): it matches the system's corners, so it is
/// no part of the two radii of the interface.
pub fn frame_radius() -> Pixels {
    px(12.)
}

/// The shadow around the framed window, inside the transparent margin.
pub fn frame_shadow(blur: Pixels) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0., 0., 0., 0.35),
        offset: point(px(0.), px(2.)),
        blur_radius: blur,
        spread_radius: px(0.),
        inset: false,
    }]
}

/// The pure color of highlight `i`, for its badge in the formatting bar.
pub fn highlight_tint(i: usize) -> Hsla {
    Hsla {
        h: HIGHLIGHTS[i.min(HIGHLIGHTS.len() - 1)],
        s: 0.9,
        l: 0.55,
        a: 1.,
    }
}

/// A quiet wash of hue `h` over `bg` for the pending changes (see `Theme::added`): the strongest
/// of the steps from 22 % down that lets every text in `reads` stay at 4.5:1 on it, else 4 %.
fn pending_wash(h: f32, bg: Hsla, reads: [Hsla; 2]) -> Hsla {
    let tint = Hsla {
        h,
        s: 0.6,
        l: 0.5,
        a: 1.,
    };
    (0..=18)
        .map(|k| blend(tint, bg, 0.22 - k as f32 * 0.01))
        .find(|c| reads.iter().all(|t| contrast(*t, *c) >= 4.5))
        .unwrap_or_else(|| blend(tint, bg, 0.04))
}

/// Hues of the highlights, in the order of `md::MARKS`: yellow, green, blue, pink, purple.
const HIGHLIGHTS: [f32; 5] = [0.14, 0.36, 0.58, 0.93, 0.76];

/// The five hues of the callout recipe: green, purple, orange, red, blue.
#[cfg(test)]
const HUES: [f32; 5] = [GREEN, 0.75, 0.09, RED, 0.58];
pub const GREEN: f32 = 0.38;
pub const RED: f32 = 0.;

/// Relative luminance of a color, in the WCAG sense.
fn luminance(color: Hsla) -> f32 {
    let c = color.to_rgb();
    let linear = |x: f32| {
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// Contrast ratio of two colors (WCAG): at least 4.5 for body text.
pub fn contrast(a: Hsla, b: Hsla) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// `color`, darkened on a light background or lightened on a dark one, just enough to
/// read on `bg`; it keeps its hue.
pub fn legible(color: Hsla, bg: Hsla) -> Hsla {
    reach(color, bg, 4.5)
}

/// `color`, moved like `legible` until its contrast with `bg` reaches `ratio`.
pub fn reach(color: Hsla, bg: Hsla, ratio: f32) -> Hsla {
    let step = if luminance(bg) > 0.18 { -0.02 } else { 0.02 };
    let mut color = color;
    while contrast(color, bg) < ratio && (0.0..=1.0).contains(&(color.l + step)) {
        color.l += step;
    }
    color
}

/// `top` laid over `under` with opacity `share`.
pub fn blend(top: Hsla, under: Hsla, share: f32) -> Hsla {
    let (a, b) = (top.to_rgb(), under.to_rgb());
    let mix = |x: f32, y: f32| x * share + y * (1. - share);
    Hsla::from(gpui::Rgba {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: 1.,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::WindowAppearance;

    /// Dark and Light keep the subtler bands and grounds of their own spec (Quiet's); the derived themes
    /// keep the stricter bars they always had.
    fn is_spec_look(name: &str) -> bool {
        matches!(name, "Dark" | "Light")
    }

    /// In each theme, secondary text, the accent (links, tags) and the title of a `[!TIP]`
    /// callout read: at least 4.5:1 (WCAG AA).
    #[test]
    fn every_theme_reads() {
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            // The window is cut into zones by `bg` and `panel` alone, with no line between them. Dark and Light
            // cut them by their ground instead (their spec gives the light panel the sheet's own white); the
            // derived themes keep the zones of `bg` and `panel`.
            if !is_spec_look(name) {
                assert!(
                    contrast(t.bg, t.panel) >= 1.03,
                    "{name}: bg and panel too close to tell the zones apart"
                );
            }
            assert!(
                contrast(t.soft, t.bg) >= 4.5 && contrast(t.soft, t.panel) >= 4.5,
                "{name}: soft"
            );
            // The sheets (`bg`) lie on the ground: told apart, and the text, the secondary text and the accent
            // (sidebar, tabs) read on both. Dark and Light's ground is their spec's own (1.07:1 on the dark sheet).
            let ground_bar = if is_spec_look(name) { 1.07 } else { 1.08 };
            assert!(
                contrast(t.ground, t.bg) >= ground_bar,
                "{name}: ground and sheet too close ({:.3})",
                contrast(t.ground, t.bg)
            );
            for (what, fg) in [("text", t.text), ("soft", t.soft), ("accent", t.accent)] {
                assert!(
                    contrast(fg, t.ground) >= 4.5,
                    "{name}: {what} on the ground {:.2}",
                    contrast(fg, t.ground)
                );
            }
            assert!(contrast(t.accent, t.bg) >= 4.5, "{name}: accent");
            for h in HUES {
                let tint = Hsla {
                    h,
                    s: 0.7,
                    l: 0.5,
                    a: 1.,
                };
                let under = blend(tint, t.bg, 0.1);
                assert!(
                    contrast(legible(tint, under), under) >= 4.5,
                    "{name}: panel {h}"
                );
                // The label of a callout reads, and stays within a step of the body text.
                let label = contrast(t.callout_label(tint), under);
                assert!(
                    label >= 4.5 && label <= contrast(t.text, under).max(4.5) + 1.,
                    "{name}: callout label {h} {label:.2}"
                );
            }
        }
    }

    /// A band (`code_bg`) stands out from the page, and what sits on it (`bg`) from the band, in every theme.
    #[test]
    fn bands_stand_out() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            for look in [WindowAppearance::Light, WindowAppearance::Dark] {
                let t = Theme::of(&Prefs::parse(&format!("theme={name}\n")), look);
                // What sits on a band is a `bg` sheet on it: its edge is the band's own contrast. The check
                // box's mark (drawn in `bg`) reads on its `accent` fill.
                let (page, mark) = (contrast(t.code_bg, t.bg), contrast(t.bg, t.accent));
                // Quiet's band is subtler, as its spec sets it (1.03:1 on the white sheet).
                let band_bar = if is_spec_look(name) { 1.03 } else { 1.05 };
                if page < band_bar || mark < 2.5 {
                    fails.push(format!(
                        "{name}: column on page {page:.2}, check mark on accent {mark:.2}"
                    ));
                }
            }
        }
        assert!(fails.is_empty(), "{}", fails.join("\n"));
    }

    /// `color` with its opacity, laid over `under`.
    fn over(color: Hsla, under: Hsla) -> Hsla {
        blend(Hsla { a: 1., ..color }, under, color.a)
    }

    /// Every pair of colors the views draw one on the other reads, in each theme: text at 4.5:1,
    /// lines that must be seen at 3:1. A view that puts a color on another background adds its pair here.
    #[test]
    fn every_pair_reads() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            let (picked, picked_panel) = (over(t.selection, t.bg), over(t.selection, t.panel));
            let mut pairs = vec![
                // Secondary text, and content that steps back (front matter, addresses, comments) in a code block too.
                ("soft/bg", t.soft, t.bg, 4.5),
                ("soft/panel", t.soft, t.panel, 4.5),
                ("soft/code", t.soft, t.code_bg, 4.5),
                // A selected row, a found passage, a commented passage: their text, and a key cap on the row.
                ("text/selection", t.text, picked, 4.5),
                ("text/selection panel", t.text, picked_panel, 4.5),
                (
                    "text/key cap",
                    t.text,
                    over(t.panel.opacity(0.7), picked_panel),
                    4.5,
                ),
                // Links, tags, active icons: on the page, the panel and the selection.
                ("accent/bg", t.accent, t.bg, 4.5),
                ("accent/panel", t.accent, t.panel, 4.5),
                ("accent/selection", t.accent, picked, 4.5),
                ("accent/selection panel", t.accent, picked_panel, 4.5),
                // The selected row of the panel when it has not the focus.
                (
                    "text/selection half",
                    t.text,
                    over(t.selection.opacity(0.5), t.panel),
                    4.5,
                ),
                (
                    "accent/selection half",
                    t.accent,
                    over(t.selection.opacity(0.5), t.panel),
                    4.5,
                ),
                // Atelier: text on the ground, and the accent of an open pill (10 % over the ground or a sheet).
                ("text/ground", t.text, t.ground, 4.5),
                ("accent/ground", t.accent, t.ground, 4.5),
                (
                    "accent/pill",
                    t.accent,
                    over(t.accent.opacity(0.1), t.bg),
                    4.5,
                ),
                // The calendar's today: the digit (drawn in `bg`) on its `accent` circle.
                ("bg/accent", t.bg, t.accent, 4.5),
                (
                    "accent/pill ground",
                    t.accent,
                    over(t.accent.opacity(0.1), t.ground),
                    4.5,
                ),
                ("link/bg", t.link, t.bg, 4.5),
                ("link/selection", t.link, picked, 4.5),
                ("line/bg", t.line, t.bg, 3.),
                ("line/code", t.line, t.code_bg, 3.),
            ];
            // The six code colours (comments, keywords, strings, numbers, types, calls), on the sheet and a code block.
            for color in t.syntax {
                pairs.push(("syntax/bg", color, t.bg, 4.5));
                pairs.push(("syntax/code", color, t.code_bg, 4.5));
            }
            for i in 0..HIGHLIGHTS.len() {
                pairs.push(("text/highlight", t.text, t.highlight(i), 4.5));
            }
            for h in HUES {
                pairs.push(("hue/panel", t.hue(h, t.panel), t.panel, 4.5));
            }
            pairs.push(("text/added", t.text, t.added, 4.5));
            pairs.push(("text/removed", t.text, t.removed, 4.5));
            pairs.push(("soft/removed", t.soft, t.removed, 4.5));
            for (what, fg, bg, min) in pairs {
                let got = contrast(fg, bg);
                if got < min {
                    fails.push(format!("{name}: {what} {got:.2}"));
                }
            }
        }
        assert!(
            fails.is_empty(),
            "{} pairs under the bar:\n{}",
            fails.len(),
            fails.join("\n")
        );
    }

    /// In each theme, a hovered surface shows on every background it can be over, is never the colour of a
    /// 1 px line, and a selected row hovered then pressed shows three different states.
    #[test]
    fn hover_and_pressed_show() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            let picked = over(t.selection, t.panel);
            for (what, under) in [
                ("bg", t.bg),
                ("panel", t.panel),
                ("code", t.code_bg),
                ("selection", picked),
            ] {
                let (hover, pressed) = (over(t.hover, under), over(t.pressed, under));
                if contrast(hover, under) < 1.05 || contrast(pressed, hover) < 1.03 {
                    fails.push(format!("{name}: hover or pressed invisible on {what}"));
                }
                if hover == t.border || pressed == t.border {
                    fails.push(format!("{name}: hover is the colour of a line on {what}"));
                }
            }
        }
        assert!(fails.is_empty(), "{}", fails.join("\n"));
    }

    /// In each theme, the open row of the panel (accent 10 %) shows on the sheet and is told from the
    /// selection.
    #[test]
    fn the_open_row_differs_from_the_selection() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            let (open, picked) = (
                over(crate::widgets::pill_tint(t), t.bg),
                over(t.selection, t.bg),
            );
            if contrast(open, t.bg) < 1.03 {
                fails.push(format!(
                    "{name}: the open row does not show on the sheet ({:.3})",
                    contrast(open, t.bg)
                ));
            }
            if contrast(picked, open) < 1.02 {
                fails.push(format!(
                    "{name}: the selection is not told from the open row ({:.3})",
                    contrast(picked, open)
                ));
            }
        }
        assert!(fails.is_empty(), "{}", fails.join("\n"));
    }

    /// The pending changes (the agent's added and removed lines) show on the sheet, and their text reads on them.
    #[test]
    fn pending_changes_show_and_read() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            for look in [WindowAppearance::Light, WindowAppearance::Dark] {
                let t = Theme::of(&Prefs::parse(&format!("theme={name}\n")), look);
                for (what, wash) in [("added", t.added), ("removed", t.removed)] {
                    if contrast(wash, t.bg) < 1.03 {
                        fails.push(format!(
                            "{name}: {what} wash does not show ({:.3})",
                            contrast(wash, t.bg)
                        ));
                    }
                    for (text, label) in [(t.text, "text"), (t.soft, "soft")] {
                        if contrast(text, wash) < 4.5 {
                            fails.push(format!(
                                "{name}: {label} on {what} {:.2}",
                                contrast(text, wash)
                            ));
                        }
                    }
                }
            }
        }
        assert!(fails.is_empty(), "{}", fails.join("\n"));
    }

    /// A commented passage shows on the sheet, reads under the text, and is not taken for the selection.
    #[test]
    fn commented_passages_differ_from_the_selection() {
        let mut fails = Vec::new();
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            let passage = over(crate::widgets::passage_tint(t), t.bg);
            let picked = over(t.selection, t.bg);
            // Told from the selection by its hue or its lightness, as the two read on the same sheet. A yellow
            // accent (Gruvbox) is near the wash in hue, so the bar is low there.
            let apart = (passage.h - picked.h)
                .abs()
                .min(1. - (passage.h - picked.h).abs())
                + (passage.s - picked.s).abs()
                + (passage.l - picked.l).abs();
            if contrast(passage, t.bg) < 1.03 {
                fails.push(format!(
                    "{name}: passage/sheet {:.3} < 1.03",
                    contrast(passage, t.bg)
                ));
            }
            if apart < 0.03 {
                fails.push(format!(
                    "{name}: passage is the selection's colour ({apart:.3})"
                ));
            }
            for (what, color) in [("text", t.text), ("accent", t.accent)] {
                if contrast(color, passage) < 4.5 {
                    fails.push(format!(
                        "{name}: {what} does not read on the commented passage ({:.2})",
                        contrast(color, passage)
                    ));
                }
            }
        }
        assert!(fails.is_empty(), "{}", fails.join("\n"));
    }

    /// Print the ground/sheet contrast of every theme (`cargo test ground_table -- --nocapture --ignored`).
    #[test]
    #[ignore]
    fn ground_table() {
        for (name, _) in THEMES {
            let t = Theme::of(
                &Prefs::parse(&format!("theme={name}\n")),
                WindowAppearance::Light,
            );
            let hex = |c: Hsla| {
                let c = c.to_rgb();
                format!(
                    "#{:02x}{:02x}{:02x}",
                    (c.r * 255.).round() as u8,
                    (c.g * 255.).round() as u8,
                    (c.b * 255.).round() as u8
                )
            };
            println!(
                "{name}: ground {} sheet {} ratio {:.3} text/ground {:.2} soft/ground {:.2} accent/ground {:.2}",
                hex(t.ground),
                hex(t.bg),
                contrast(t.ground, t.bg),
                contrast(t.text, t.ground),
                contrast(t.soft, t.ground),
                contrast(t.accent, t.ground)
            );
        }
    }

    #[test]
    fn page_width_follows_the_text_size() {
        assert_eq!(Page::Normal.measure(15.), px(800.));
        assert_eq!(Page::Normal.measure(30.), px(1600.));
        assert!(
            Page::Narrow.measure(15.) < Page::Normal.measure(15.)
                && Page::Normal.measure(15.) < Page::Wide.measure(15.)
        );
        assert!(Page::Wide.measure(15.) < Page::Full.measure(15.));
        // Keys are read back; an unknown or missing key means the default choice.
        assert!(Page::ALL.into_iter().all(|p| Page::of(p.key()) == p));
        assert_eq!((Page::of(""), Page::of("?")), (Page::Normal, Page::Normal));
    }

    #[test]
    fn json_themes_parse_and_warn() {
        assert_eq!(parse_hex("#1b1a19"), Some(0x1b1a19));
        assert_eq!(parse_hex("e4e0da"), Some(0xe4e0da));
        assert_eq!(parse_hex("#zzz"), None);
        assert_eq!(parse_hex("#12345"), None);
        let (themes, errors, warns) = parse_themes(
            r##"{"version": 1, "themes": [
                {"name": "My Dark", "base": "Inkuno Dark", "colors": {"bg": "#1b1a19", "text": "#e4e0da", "dim": "#77716a", "accent": "#e0926b", "code_bg": "#262423", "panel": "#242221", "border": "#3a3735"}},
                {"name": "Thin", "colors": {"bg": "#ffffff"}},
                {"name": "", "colors": {}},
                {"name": "Bad", "base": "Inkuno Dark", "colors": {"bg": "red", "text": "#000000", "dim": "#000000", "accent": "#000000", "code_bg": "#000000", "panel": "#000000", "border": "#000000"}},
                {"name": "My Dark", "colors": {"bg": "#000000"}}
            ]}"##,
        );
        assert_eq!(themes.len(), 2);
        assert_eq!(themes[0].seeds[0], 0x1b1a19);
        // Thin inherits the six missing tokens from its base.
        assert_eq!(themes[1].base, "Dark");
        assert_eq!(themes[1].seeds[0], 0xffffff);
        assert_eq!(themes[1].seeds[1], THEMES[0].1[1]);
        // Faulty themes, one by one: nameless, unreadable color, duplicate name. The dropped
        // duplicate still reports the tokens it would inherit.
        assert_eq!(errors.len(), 3);
        assert_eq!(warns.len(), 2);
        // Low contrast is reported, then auto-fixed by the `legible` in `of`.
        let (names, errors, warns) = check_themes(
            r##"{"version": 1, "themes": [
                {"name": "Flat", "base": "Inkuno Light", "colors": {"bg": "#ffffff", "text": "#777777", "dim": "#77716a", "accent": "#e0926b", "code_bg": "#262423", "panel": "#ffffff", "border": "#3a3735"}}
            ]}"##,
        );
        assert_eq!(names, vec!["Flat".to_string()]);
        assert!(errors.is_empty());
        assert!(warns.iter().any(|w| w.contains("auto-fixed")));
        assert_eq!(parse_themes(""), (Vec::new(), Vec::new(), Vec::new()));
        assert_eq!(parse_themes("nope").1.len(), 1);
        // Lookup by name, as `of` does; the file faults never reach the theme.
        let text =
            r##"{"version": 1, "themes": [{"name": "My Dark", "colors": {"bg": "#000000"}}]}"##;
        assert_eq!(
            custom_in("My Dark", text).map(|t| t.seeds[0]),
            Some(0x000000)
        );
        assert_eq!(custom_in("Missing", text), None);
        assert_eq!(custom_in("My Dark", "nope"), None);
    }

    /// Every theme has the Quiet look: the block marks, chip, gutter, table, ring and alerts are set,
    /// whatever its colours. The derived themes keep the todo box at the spec's 40 % of the text.
    #[test]
    fn every_theme_has_the_quiet_look() {
        for (name, _) in THEMES {
            for look in [WindowAppearance::Light, WindowAppearance::Dark] {
                let t = Theme::of(&Prefs::parse(&format!("theme={name}\n")), look);
                assert!(t.dots.is_some() && t.chip.is_some(), "{name}");
                assert!(t.alerts.is_some(), "{name}");
                assert_eq!(t.todo_box, t.text.opacity(0.40), "{name}");
            }
        }
    }

    /// The tokens a `themes.json` theme may set beyond the seeds: they override the derived colours, a
    /// bad one drops the theme, and a missing one stays derived.
    #[test]
    fn json_themes_take_extra_tokens() {
        let (themes, errors, _) = parse_themes(
            r##"{"version": 1, "themes": [
                {"name": "Tinted", "base": "Light", "colors": {"chip": "#101010", "syntax": ["#000001", "#000002", "#000003", "#000004", "#000005", "#000006"], "alerts": ["#000011", "#000012", "#000013", "#000014", "#000015"]}},
                {"name": "Short", "colors": {"syntax": ["#000001"]}},
                {"name": "Bad", "colors": {"link": "blue"}}
            ]}"##,
        );
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(themes.len(), 1);
        let tinted = &themes[0];
        assert_eq!(tinted.overrides.chip, Some(0x101010));
        assert_eq!(tinted.overrides.link, None);
        assert_eq!(tinted.overrides.syntax.map(|s| s[5]), Some(0x000006));
        let mut extras = derived_extras(tinted.seeds, false);
        tinted.overrides.apply(&mut extras);
        assert_eq!(extras.chip, 0x101010);
        assert_eq!(extras.syntax[0], 0x000001);
        assert_eq!(
            extras.alerts,
            [0x000011, 0x000012, 0x000013, 0x000014, 0x000015]
        );
        assert_eq!(extras.popover, hex(Hsla::from(rgb(tinted.seeds[5]))));
    }

    #[test]
    fn quiet_base_names_seed_the_renamed_themes() {
        assert_eq!(base_seeds("Quiet Dark"), base_seeds("Dark"));
        assert_eq!(base_seeds("Quiet Light"), base_seeds("Light"));
        assert!(base_seeds("Quiet Dark").1);
        assert!(!base_seeds("Nope").1);
    }
}
