//! Dates, the daily note, scratch notes, and the version of the core.

use inkuno_logic::{
    lang::{self, date_name},
    markdown::{self as md, Date},
    recall, scratch,
};

/// A calendar day: year, month (1 to 12), day (1 to 31).
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ymd {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Ymd {
    pub(crate) fn date(self) -> Date {
        (self.year, self.month, self.day)
    }
}

/// A time of day: hour (0 to 23), minute, second.
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hms {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// A day offered after `@` in the editor.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct DateSuggestion {
    /// As the completion lists it: `Saturday, 21 November`, or the written date `2026-11-21`.
    pub label: String,
    /// The date as a note writes it, `2026-11-21`: what the link and the daily note are named.
    pub iso: String,
    pub date: Ymd,
}

/// The days `query` (the text after `@`) names from `today`: a day name (`sat`, `tomorrow`…) or a
/// written date. Same rules as the desktop editor.
#[uniffi::export]
pub fn date_suggestions(query: String, today: Ymd) -> Vec<DateSuggestion> {
    md::dates(&query, today.date())
        .into_iter()
        .map(|(en, _, date)| DateSuggestion {
            label: date_label(en, date),
            iso: date_name(date),
            date: Ymd {
                year: date.0,
                month: date.1,
                day: date.2,
            },
        })
        .collect()
}

/// The name of the daily note of `today`: `2026-11-24`.
#[uniffi::export]
pub fn daily_note_name(today: Ymd) -> String {
    date_name(today.date())
}

/// The name of a scratch note created at `today` and `now`: `2026-11-24-103200`. Names sort by
/// creation.
#[uniffi::export]
pub fn scratch_name(today: Ymd, now: Hms) -> String {
    scratch::stem(today.date(), (now.hour, now.minute, now.second))
}

/// The title of a scratch note: its first non-blank line without the `#`, `New note` when blank.
#[uniffi::export]
pub fn scratch_title(text: String) -> String {
    scratch::title(&text)
}

/// The moment a scratch note's name stands for, `2026-11-24 10:32`; `None` for another name.
#[uniffi::export]
pub fn scratch_label(stem: String) -> Option<String> {
    scratch::label(&stem)
}

/// Scratch note names, newest first: by moment, then by collision number.
#[uniffi::export]
pub fn scratch_newest_first(mut stems: Vec<String>) -> Vec<String> {
    scratch::newest_first(&mut stems);
    stems
}

/// Whether a scratch note's text is only whitespace: such a note is dropped when its tab closes.
#[uniffi::export]
pub fn scratch_is_blank(text: String) -> bool {
    scratch::is_blank(&text)
}

/// How long ago, in the palette's short form: `today`, `3 d`, `2 wk`, `5 mo`, `1 y`. `days` is the
/// age in whole days.
#[uniffi::export]
pub fn short_age(days: u64) -> String {
    lang::age(days)
}

/// « 3 months ago »: an age in days, told in the largest unit that fits (`today` for none).
#[uniffi::export]
pub fn ago(days: i64) -> String {
    recall::ago(days)
}

/// The date in words, without the year: `Friday, 10 October`.
#[uniffi::export]
pub fn long_date(date: Ymd) -> String {
    recall::long_date(date.date())
}

/// The version of this core (the crate's version).
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The label of a date in the completion list: the day name capitalised with its date, or the
/// written date when there is no day name.
fn date_label(en: &str, date: Date) -> String {
    if en.is_empty() {
        return date_name(date);
    }
    let (_, month, day) = date;
    let weekday = en[..1].to_uppercase() + &en[1..];
    format!("{weekday}, {day} {}", md::MONTHS[month as usize - 1].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: Ymd = Ymd {
        year: 2026,
        month: 11,
        day: 24,
    };

    #[test]
    fn the_daily_note_is_named_by_its_day() {
        assert_eq!(daily_note_name(TODAY), "2026-11-24");
        assert_eq!(
            daily_note_name(Ymd {
                year: 2027,
                month: 1,
                day: 2
            }),
            "2027-01-02"
        );
    }

    #[test]
    fn today_and_tomorrow_are_offered_by_name() {
        let found = date_suggestions("tom".into(), TODAY);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "Tomorrow, 25 November");
        assert_eq!(found[0].iso, "2026-11-25");
        assert_eq!(
            found[0].date,
            Ymd {
                year: 2026,
                month: 11,
                day: 25
            }
        );
    }

    #[test]
    fn a_written_date_is_its_own_label() {
        let found = date_suggestions("2026-10-09".into(), TODAY);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "2026-10-09");
        assert_eq!(found[0].iso, "2026-10-09");
    }

    #[test]
    fn a_weekday_is_the_next_one_and_a_nonsense_date_is_refused() {
        // 2026-11-24 is a Tuesday: `mon` is the next Monday, never today.
        let found = date_suggestions("mon".into(), TODAY);
        assert_eq!(found[0].iso, "2026-11-30");
        assert!(date_suggestions("2026-02-30".into(), TODAY).is_empty());
    }

    #[test]
    fn scratch_names_and_titles() {
        assert_eq!(
            scratch_name(
                TODAY,
                Hms {
                    hour: 10,
                    minute: 32,
                    second: 0
                }
            ),
            "2026-11-24-103200"
        );
        assert_eq!(scratch_title("\n  \n# Buy milk\nmore".into()), "Buy milk");
        assert_eq!(scratch_title("   ".into()), "New note");
    }

    #[test]
    fn scratch_names_read_back_as_a_moment() {
        assert_eq!(
            scratch_label("2026-11-24-103200".into()).as_deref(),
            Some("2026-11-24 10:32")
        );
        assert_eq!(scratch_label("Plan".into()), None);
        let sorted = scratch_newest_first(vec![
            "2026-11-23-090000".into(),
            "2026-11-24-103200".into(),
            "2026-11-24-103200 2".into(),
        ]);
        assert_eq!(sorted[0], "2026-11-24-103200 2");
        assert!(scratch_is_blank(" \n\t".into()));
    }

    #[test]
    fn ages_read_like_the_palette_and_the_recall() {
        assert_eq!(short_age(0), "today");
        assert_eq!(short_age(3), "3 d");
        assert_eq!(short_age(45), "1 mo");
        assert_eq!(ago(1), "yesterday");
        assert_eq!(ago(90), "3 months ago");
        assert_eq!(long_date(TODAY), "Tuesday, November 24");
    }

    #[test]
    fn the_core_version_is_the_crate_version() {
        assert_eq!(core_version(), env!("CARGO_PKG_VERSION"));
    }
}
