//! Local date and time. The standard library only knows UTC.

/// Today's date in local time: year, month, day. The standard library only gives UTC.
#[cfg(unix)]
pub fn today() -> (i32, u32, u32) {
    // SAFETY: `time` accepts a null pointer; `localtime_r` only writes into `tm`, which is ours.
    let tm = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm
    };
    (tm.tm_year + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32)
}

/// The local date and time of a Unix time: year, month, day, hour, minute. The platform's own zone
/// rules are used (no time zone is read by hand, and `TZ` is not needed). Where the platform gives no
/// local time here, the UTC fields of the same time are returned.
#[cfg(unix)]
pub fn local(unix: i64) -> (i32, u32, u32, u32, u32) {
    // SAFETY: `localtime_r` reads the time it is given and writes only into `tm`, which is ours. A
    // null result (a time the system cannot convert) is checked and falls back to UTC.
    let tm = unsafe {
        let at = unix as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&at, &mut tm).is_null() {
            return utc(unix);
        }
        tm
    };
    (
        tm.tm_year + 1900,
        tm.tm_mon as u32 + 1,
        tm.tm_mday as u32,
        tm.tm_hour as u32,
        tm.tm_min as u32,
    )
}

/// The local date and time of a Unix time. No platform call of local time is made on this system
/// yet, so the UTC fields of the same time are returned.
#[cfg(not(unix))]
pub fn local(unix: i64) -> (i32, u32, u32, u32, u32) {
    utc(unix)
}

/// The UTC fields of a Unix time, in the order of `local`: the pure calendar, no platform call.
pub fn utc(unix: i64) -> (i32, u32, u32, u32, u32) {
    crate::logic::revision::utc_parts(unix)
}

/// Local time, `09:05`: hours and minutes.
pub fn clock() -> (u32, u32) {
    let (hour, minute, _) = hms();
    (hour, minute)
}

/// Local time to the second: hours, minutes, seconds.
#[cfg(unix)]
pub fn hms() -> (u32, u32, u32) {
    // SAFETY: like `today`, `localtime_r` only writes into `tm`, which is ours.
    let tm = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm
    };
    (tm.tm_hour as u32, tm.tm_min as u32, tm.tm_sec as u32)
}

#[cfg(windows)]
pub fn hms() -> (u32, u32, u32) {
    #[repr(C)]
    #[derive(Default)]
    #[allow(dead_code)]
    struct Local {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut Local);
    }
    let mut now = Local::default();
    // SAFETY: `GetLocalTime` fills the structure it is given, and does not fail.
    unsafe { GetLocalTime(&mut now) };
    (now.hour as u32, now.minute as u32, now.second as u32)
}

#[cfg(windows)]
pub fn today() -> (i32, u32, u32) {
    /// `SYSTEMTIME` of the Windows API: eight 16-bit words.
    #[repr(C)]
    #[derive(Default)]
    #[allow(dead_code)]
    struct Local {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut Local);
    }
    let mut now = Local::default();
    // SAFETY: `GetLocalTime` fills the structure it is given, and does not fail.
    unsafe { GetLocalTime(&mut now) };
    (now.year as i32, now.month as u32, now.day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::date_name;

    #[test]
    fn names_the_day() {
        assert_eq!(date_name((2026, 1, 2)), "2026-01-02");
        let (year, month, day) = today();
        assert!(year >= 2026 && (1..=12).contains(&month) && (1..=31).contains(&day));
        // The local date is the one the system gives (give or take midnight, between the two reads).
        #[cfg(unix)]
        if let Ok(out) = std::process::Command::new("date").arg("+%F").output() {
            let system = String::from_utf8_lossy(&out.stdout).trim().to_string();
            assert!(system == date_name((year, month, day)) || system == date_name(today()));
        }
    }

    #[test]
    fn the_local_time_of_now_is_today_and_the_clock() {
        // Read around the other reads, so that a minute or a midnight between them is no failure.
        let now = crate::platform::revisions::now();
        let before = local(now);
        let (year, month, day) = today();
        let (hour, minute) = clock();
        let after = local(crate::platform::revisions::now());
        let same_day = |(y, m, d, _, _): (i32, u32, u32, u32, u32)| (y, m, d) == (year, month, day);
        let same_minute = |(_, _, _, h, mi): (i32, u32, u32, u32, u32)| (h, mi) == (hour, minute);
        assert!(same_day(before) || same_day(after), "{before:?} {after:?}");
        assert!(
            same_minute(before) || same_minute(after),
            "{before:?} {after:?}"
        );
    }

    #[test]
    fn a_minute_later_is_the_next_local_minute_and_the_date_follows() {
        // No process call (`date` differs between systems): the local fields of two instants a
        // minute apart are one minute apart, and the date line is built from the same fields.
        let unix = 1_709_208_000_i64;
        let minute_of_day = |(_, _, _, h, m): (i32, u32, u32, u32, u32)| i64::from(h * 60 + m);
        let step = (minute_of_day(local(unix + 60)) - minute_of_day(local(unix))).rem_euclid(1440);
        assert_eq!(step, 1);
        let (year, month, day, hour, minute) = local(unix + 60);
        let mine = format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}");
        assert_eq!(
            crate::logic::revision::local_date(&crate::logic::revision::rfc3339(unix + 60), local),
            mine
        );
    }

    #[test]
    fn the_utc_fallback_follows_the_calendar() {
        // 2024 is a leap year: its 29th of February is a day of its own.
        assert_eq!(utc(1_709_208_000), (2024, 2, 29, 12, 0));
        assert_eq!(utc(1_709_251_200), (2024, 3, 1, 0, 0));
    }
}
