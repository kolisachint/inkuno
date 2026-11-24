//! User preferences (`settings` file): parsed from and written to `key=value` lines.

/// Appearance chosen by the user; an empty field keeps the default value.
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    pub theme: String,
    pub font: String,
    pub mono: String,
    /// Size of the note's body text, in pixels.
    pub size: f32,
    /// Width of the text column (a key of `editor::Page`); empty: the default choice.
    pub page: String,
    /// The former "Wide tables" choice. Wide blocks now always stay in the text column: the key is
    /// still read and written back so that an older settings file loads and keeps its line.
    pub wide: String,
    /// Formatting bar at the bottom of the note: `off` hides it, anything else shows it.
    pub bar: String,
    /// Short fades when something opens: `off` makes everything instant, anything else keeps them.
    pub motion: String,
    /// Author of the revisions a person saves; empty: `INKUNO_AUTHOR`, else the OS user name.
    pub author: String,
    /// Command that starts the assistant; empty: `hoocode` (see `assistant_command`).
    pub assistant: String,
    /// Line spacing of the text: `compact`, `normal`, `relaxed` or `loose` (see `leading`); empty: normal.
    pub spacing: String,
    /// Line numbers of a code block: `off` hides them, anything else shows them.
    pub gutter: String,
    /// Line numbers of the note's lines, in a gutter left of the text: `off` hides them, empty or anything else
    /// shows them (on by default). While they show, a code block has no numbers of its own.
    pub line_numbers: String,
    /// The tabs of last time come back on start: `off` starts with an empty window.
    pub restore: String,
}

impl Prefs {
    /// The default body size; the reference every size proportion (UI scale, measure, headings) is taken from.
    pub const SIZE: f32 = 15.;
    /// Version of the file's layout, written on every save. Files without it (or with an older one) were
    /// saved with the former default of 16 px, which then reads as `SIZE`.
    pub const VERSION: u32 = 2;
    /// The assistant's command when the setting is empty.
    pub const ASSISTANT: &'static str = "hoocode";

    /// The keys of the line spacing, from the most compact to the loosest.
    pub const SPACINGS: [&'static str; 4] = ["compact", "normal", "relaxed", "loose"];

    /// The multiplier of every line height for the spacing key: empty or unknown is normal (1).
    pub fn leading(&self) -> f32 {
        match self.spacing.trim() {
            "compact" => 0.9,
            "relaxed" => 1.15,
            "loose" => 1.3,
            _ => 1.,
        }
    }

    /// The command the assistant starts: the setting, trimmed; `hoocode` when it is empty.
    pub fn assistant_command(&self) -> &str {
        match self.assistant.trim() {
            "" => Self::ASSISTANT,
            command => command,
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut version = 0;
        let mut prefs = Self {
            theme: String::new(),
            font: String::new(),
            mono: String::new(),
            size: Self::SIZE,
            page: String::new(),
            wide: String::new(),
            bar: String::new(),
            motion: String::new(),
            author: String::new(),
            assistant: String::new(),
            spacing: String::new(),
            gutter: String::new(),
            line_numbers: String::new(),
            restore: String::new(),
        };
        for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
            match key {
                "prefs" => version = value.parse().unwrap_or(0),
                // Settings written before the rename still name the Quiet themes.
                "theme" => {
                    prefs.theme = match value {
                        "Quiet Dark" => "Dark",
                        "Quiet Light" => "Light",
                        other => other,
                    }
                    .into()
                }
                "font" => prefs.font = value.into(),
                "mono" => prefs.mono = value.into(),
                "size" => {
                    prefs.size = value
                        .parse()
                        .ok()
                        .filter(|s: &f32| s.is_finite())
                        .unwrap_or(Self::SIZE)
                }
                "page" => prefs.page = value.into(),
                "wide" => prefs.wide = value.into(),
                "bar" => prefs.bar = value.into(),
                "motion" => prefs.motion = value.into(),
                "author" => prefs.author = value.into(),
                "assistant" => prefs.assistant = value.into(),
                "spacing" => prefs.spacing = value.into(),
                "gutter" => prefs.gutter = value.into(),
                "line_numbers" => prefs.line_numbers = value.into(),
                "restore" => prefs.restore = value.into(),
                _ => {}
            }
        }
        // The former default of 16 px, kept by a file of an older version, is the default now.
        if version < Self::VERSION && prefs.size == 16. {
            prefs.size = Self::SIZE;
        }
        prefs.size = prefs.size.clamp(11., 32.);
        prefs
    }

    pub fn to_text(&self) -> String {
        let mut text = format!(
            "prefs={}\ntheme={}\nfont={}\nmono={}\nsize={}\n",
            Self::VERSION,
            self.theme,
            self.font,
            self.mono,
            self.size
        );
        for (key, value) in [
            ("page", &self.page),
            ("wide", &self.wide),
            ("bar", &self.bar),
            ("motion", &self.motion),
            ("author", &self.author),
            ("assistant", &self.assistant),
            ("spacing", &self.spacing),
            ("gutter", &self.gutter),
            ("line_numbers", &self.line_numbers),
            ("restore", &self.restore),
        ] {
            if !value.is_empty() {
                text.push_str(&format!("{key}={value}\n"));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_former_default_size_moves_to_the_new_one() {
        // Saved before the Quiet typography (no `prefs` key, or an older one): a size of 16 was the default.
        assert_eq!(Prefs::parse("theme=Quiet\nsize=16\n").size, 15.);
        assert_eq!(Prefs::parse("prefs=1\nsize=16\n").size, 15.);
        // Any other size is kept as saved.
        assert_eq!(Prefs::parse("size=18\n").size, 18.);
        // Saved with the version: a size of 16 is a choice.
        let kept = Prefs::parse("prefs=2\nsize=16\n");
        assert_eq!(kept.size, 16.);
        // Every save writes the version, and the migrated size is what the next read finds.
        assert_eq!(Prefs::parse(&kept.to_text()), kept);
        let migrated = Prefs::parse("size=16\n");
        assert_eq!(Prefs::parse(&migrated.to_text()).size, 15.);
        assert!(migrated.to_text().starts_with("prefs=2\n"));
    }

    #[test]
    fn reads_settings() {
        let prefs = Prefs::parse("theme=Dracula\nfont=Inter\nsize=99\nbogus\nmono=\n");
        assert_eq!(
            (
                prefs.theme.as_str(),
                prefs.font.as_str(),
                prefs.mono.as_str()
            ),
            ("Dracula", "Inter", "")
        );
        // Size clamped, and unreadable value ignored.
        assert_eq!((prefs.size, Prefs::parse("size=NaN").size), (32., 15.));
        assert_eq!(Prefs::parse(&prefs.to_text()), prefs);
        assert!(prefs.to_text().contains("prefs=2\n"));
        // Unknown keys are ignored (the removed `updates` setting reads as defaults).
        assert_eq!(Prefs::parse("updates=off\n"), Prefs::parse(""));
        // Page width and wide tables: absent until chosen.
        let wide = Prefs::parse("page=wide\nwide=column\n");
        assert_eq!((wide.page.as_str(), wide.wide.as_str()), ("wide", "column"));
        assert_eq!(Prefs::parse(&wide.to_text()), wide);
        // Motion: absent until chosen, anything but `off` keeps the fades.
        let still = Prefs::parse("motion=off\n");
        assert_eq!(still.motion, "off");
        assert_eq!(Prefs::parse(&still.to_text()), still);
        assert!(!prefs.to_text().contains("motion="));
        // Revision author: absent until chosen, kept as typed.
        let named = Prefs::parse("author=Sachin Kolisetty\n");
        assert_eq!(named.author, "Sachin Kolisetty");
        assert_eq!(Prefs::parse(&named.to_text()), named);
        assert!(!prefs.to_text().contains("author="));
        assert!(!prefs.to_text().contains("page=") && !prefs.to_text().contains("wide="));
        // Assistant command: absent until chosen, `hoocode` when empty or blank.
        assert_eq!(prefs.assistant_command(), "hoocode");
        assert!(!prefs.to_text().contains("assistant="));
        let agent = Prefs::parse("assistant=mybot --fast\n");
        assert_eq!(agent.assistant_command(), "mybot --fast");
        assert_eq!(Prefs::parse(&agent.to_text()), agent);
        assert_eq!(
            Prefs::parse("assistant=  \n").assistant_command(),
            "hoocode"
        );
    }

    #[test]
    fn line_spacing_gutter_and_restore_round_trip() {
        // Absent until chosen: normal spacing, numbered code, tabs restored.
        let plain = Prefs::parse("theme=Quiet\n");
        assert_eq!(
            (
                plain.spacing.as_str(),
                plain.gutter.as_str(),
                plain.restore.as_str()
            ),
            ("", "", "")
        );
        assert_eq!(plain.leading(), 1.);
        assert!(
            !plain.to_text().contains("spacing=")
                && !plain.to_text().contains("gutter=")
                && !plain.to_text().contains("restore=")
        );
        // Each key is kept and written back; the multipliers of the four choices.
        let set = Prefs::parse("spacing=relaxed\ngutter=off\nrestore=off\n");
        assert_eq!(Prefs::parse(&set.to_text()), set);
        assert!(
            set.to_text().contains("spacing=relaxed\n")
                && set.to_text().contains("gutter=off\n")
                && set.to_text().contains("restore=off\n")
        );
        assert_eq!(set.leading(), 1.15);
        let leads: Vec<f32> = Prefs::SPACINGS
            .iter()
            .map(|key| Prefs::parse(&format!("spacing={key}")).leading())
            .collect();
        assert_eq!(leads, [0.9, 1., 1.15, 1.3]);
        // An unknown key reads as normal.
        assert_eq!(Prefs::parse("spacing=huge").leading(), 1.);
    }

    #[test]
    fn line_numbers_are_on_until_switched_off() {
        // Absent: on, and not written back.
        let plain = Prefs::parse("theme=Quiet\n");
        assert_eq!(plain.line_numbers, "");
        assert!(!plain.to_text().contains("line_numbers="));
        // Off is kept and written back.
        let off = Prefs::parse("line_numbers=off\n");
        assert_eq!(off.line_numbers, "off");
        assert_eq!(Prefs::parse(&off.to_text()), off);
        assert!(off.to_text().contains("line_numbers=off\n"));
    }

    #[test]
    fn quiet_theme_names_read_as_the_renamed_ones() {
        assert_eq!(Prefs::parse("theme=Quiet Dark\n").theme, "Dark");
        assert_eq!(Prefs::parse("theme=Quiet Light\n").theme, "Light");
        assert_eq!(Prefs::parse("theme=Quiet\n").theme, "Quiet");
    }
}
