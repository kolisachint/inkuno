//! Dates, the daily note, scratch notes, and the version of the core.

use inkuno_logic::{
    lang::date_name,
    markdown::{self as md, Date},
    scratch,
};

/// A calendar day: year, month (1 to 12), day (1 to 31).
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ymd {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Ymd {
    fn date(self) -> Date {
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
    fn the_core_version_is_the_crate_version() {
        assert_eq!(core_version(), env!("CARGO_PKG_VERSION"));
    }
}
