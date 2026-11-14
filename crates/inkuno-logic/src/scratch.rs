//! Scratch notes: a raw thought kept outside the vault until the person decides where it belongs.
//! Which name a new one gets, when it counts as empty, how the list reads and what closing one
//! offers are decided here; the files are the shell's (`platform/scratch.rs`).

use crate::lang::tr;

/// The file name (without extension) of a scratch note created on `date` (year, month, day) at
/// `time` (hour, minute, second), local time: `2026-11-14-103200`. Names sort by creation.
pub fn stem(date: (i32, u32, u32), time: (u32, u32, u32)) -> String {
    format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        date.0, date.1, date.2, time.0, time.1, time.2
    )
}

/// Whether the text is only whitespace: such a scratch note is dropped when its tab closes.
pub fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// The title of a scratch note: its first non-blank line, without the `#` of a heading, cut to 80
/// characters; `New note` for a blank text.
pub fn title(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    let title: String = line
        .trim()
        .trim_start_matches('#')
        .trim()
        .chars()
        .take(80)
        .collect();
    match title.is_empty() {
        true => tr("New note", "Nouvelle note").to_string(),
        false => title,
    }
}

/// Splits a scratch name into its moment and its collision number: `2026-11-14-103200 2` gives
/// (`2026-11-14-103200`, 2). A name that is not a scratch name gives `None`.
fn parts(stem: &str) -> Option<(&str, u32)> {
    let (base, number) = match stem.split_once(' ') {
        Some((base, number)) => (base, number.parse::<u32>().ok()?),
        None => (stem, 1),
    };
    let mut fields = base.split('-');
    let (year, month, day, time) = (
        fields.next()?,
        fields.next()?,
        fields.next()?,
        fields.next()?,
    );
    let digits = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_digit());
    let valid = fields.next().is_none()
        && digits(year, 4)
        && digits(month, 2)
        && digits(day, 2)
        && digits(time, 6);
    valid.then_some((base, number))
}

/// The moment a scratch note's name stands for, as `2026-11-14 10:32`; `None` for another name.
pub fn label(stem: &str) -> Option<String> {
    let (base, _) = parts(stem)?;
    let (day, time) = base.rsplit_once('-')?;
    Some(format!("{day} {}:{}", &time[..2], &time[2..4]))
}

/// Sorts scratch names newest first: by moment, then by collision number.
pub fn newest_first(stems: &mut [String]) {
    stems.sort_by(|a, b| {
        let key = |stem: &String| parts(stem).map(|(base, n)| (base.to_string(), n));
        key(b).cmp(&key(a)).then_with(|| b.cmp(a))
    });
}

/// What closing a scratch note with text offers, the default first. Keeping it in the vault
/// needs a vault to keep it in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    /// Stays in the scratch folder: the tab closes, the file stays. The default.
    Keep,
    /// Moves into the vault, under its title.
    Vault,
    /// Deleted.
    Discard,
}

impl Choice {
    /// The line the choice is shown as.
    pub fn label(self) -> &'static str {
        match self {
            Choice::Keep => tr("Keep as scratch", "Garder en brouillon"),
            Choice::Vault => tr("Keep in vault…", "Garder dans le coffre…"),
            Choice::Discard => tr("Discard", "Jeter"),
        }
    }
}

/// The choices, in the order shown: the default first, discarding last.
pub fn choices(vault_open: bool) -> Vec<Choice> {
    let mut all = vec![Choice::Keep];
    if vault_open {
        all.push(Choice::Vault);
    }
    all.push(Choice::Discard);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_a_note_by_its_moment() {
        assert_eq!(stem((2026, 11, 14), (10, 32, 0)), "2026-11-14-103200");
        assert_eq!(stem((2026, 1, 2), (7, 5, 9)), "2026-01-02-070509");
    }

    #[test]
    fn a_blank_text_is_empty_whatever_its_spaces() {
        assert!(is_blank(""));
        assert!(is_blank(" \n\t  \n"));
        assert!(!is_blank("a"));
        assert!(!is_blank("\n# Titre\n"));
    }

    #[test]
    fn labels_show_the_moment_and_refuse_other_names() {
        assert_eq!(
            label("2026-11-14-103200").as_deref(),
            Some("2026-11-14 10:32")
        );
        assert_eq!(
            label("2026-11-14-103200 2").as_deref(),
            Some("2026-11-14 10:32")
        );
        assert_eq!(label("Projet"), None);
        assert_eq!(label("2026-11-14"), None);
        assert_eq!(label("2026-11-14-10320"), None);
        assert_eq!(label("2026-11-14-103200 x"), None);
    }

    #[test]
    fn lists_the_newest_first_with_the_collisions_in_order() {
        let mut names: Vec<String> = [
            "2026-11-14-093000",
            "2026-11-14-103200",
            "2026-11-14-103200 2",
            "2026-11-13-235959",
            "2026-11-14-103159",
        ]
        .map(String::from)
        .to_vec();
        newest_first(&mut names);
        assert_eq!(
            names,
            [
                "2026-11-14-103200 2",
                "2026-11-14-103200",
                "2026-11-14-103159",
                "2026-11-14-093000",
                "2026-11-13-235959",
            ]
        );
    }

    #[test]
    fn a_title_is_the_first_line_without_its_heading_mark() {
        assert_eq!(title("# Idée du matin\n\ncorps"), "Idée du matin");
        assert_eq!(title("\n\n   \nDeuxième\n"), "Deuxième");
        assert_eq!(title("  ## Titre  "), "Titre");
        assert_eq!(title(" \n "), "New note");
        assert_eq!(title(&"a".repeat(200)).chars().count(), 80);
    }

    #[test]
    fn closing_offers_the_default_first_and_keeps_in_vault_only_with_a_vault() {
        assert_eq!(
            choices(true),
            [Choice::Keep, Choice::Vault, Choice::Discard]
        );
        assert_eq!(choices(false), [Choice::Keep, Choice::Discard]);
        assert_eq!(choices(true)[0].label(), "Keep as scratch");
    }
}
