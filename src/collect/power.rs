//! Sleep, wakes, session and share of time on battery.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

use crate::fmt::from_unix;
use crate::model::{Power, Ts};
use crate::sys;

pub fn power() -> Power {
    let mut power = Power {
        boot: sys::sysctl_time("kern.boottime").and_then(from_unix),
        last_wake: sys::sysctl_time("kern.waketime").and_then(from_unix),
        last_sleep: sys::sysctl_time("kern.sleeptime").and_then(from_unix),
        awake_secs: sys::awake_secs(),
        login: sys::sessions()
            .iter()
            .find(|s| s.line == "console")
            .and_then(|s| from_unix(s.since)),
        ..Power::default()
    };
    if let Some((sleeps, darks, users)) = sys::run("pmset", &["-g", "stats"])
        .as_deref()
        .map(parse_stats)
    {
        power.sleep_count = sleeps;
        power.dark_wakes = darks;
        power.user_wakes = users;
    }
    if let Some((share, days)) = sys::run("pmset", &["-g", "log"])
        .as_deref()
        .and_then(battery_share)
    {
        power.on_battery_share = Some(share);
        power.power_log_days = Some(days);
    }
    if let Some(log) = sys::run("last", &["reboot"]) {
        power.reboots_logged = Some(log.lines().filter(|l| l.starts_with("reboot")).count() as u64);
        power.log_since = parse_wtmp_start(&log);
    }
    power
}

fn parse_stats(text: &str) -> (Option<u64>, Option<u64>, Option<u64>) {
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.trim_start_matches(':').trim().parse().ok())
    };
    (
        value("Sleep Count"),
        value("Dark Wake Count"),
        value("User Wake Count"),
    )
}

/// Each log entry says what was powering the machine at that moment. The interval
/// up to the next entry is attributed to that source.
pub fn battery_share(log: &str) -> Option<(f64, f64)> {
    let mut events: Vec<(DateTime<Local>, bool)> = Vec::new();
    for line in log.lines() {
        let on_battery = if line.contains("Using Batt") {
            true
        } else if line.contains("Using AC") {
            false
        } else {
            continue;
        };
        if let Some(stamp) = parse_stamp(line) {
            events.push((stamp, on_battery));
        }
    }
    let (first, last) = (events.first()?.0, events.last()?.0);
    let total = (last - first).num_seconds() as f64;
    if total <= 0.0 {
        return None;
    }
    let battery: i64 = events
        .windows(2)
        .filter(|pair| pair[0].1)
        .map(|pair| (pair[1].0 - pair[0].0).num_seconds())
        .sum();
    Some((battery as f64 / total, total / 86_400.0))
}

fn parse_stamp(line: &str) -> Option<DateTime<Local>> {
    let stamp = line.get(..19)?;
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S").ok()?;
    Local.from_local_datetime(&naive).single()
}

/// End of `last` output: `wtmp begins Sun Sep 20 14:08:30 +05 2026`.
fn parse_wtmp_start(log: &str) -> Option<Ts> {
    let line = log.lines().find_map(|l| l.strip_prefix("wtmp begins "))?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    let [_, month, day, clock, _, year] = parts.as_slice() else {
        return None;
    };
    let text = format!("{day} {month} {year} {clock}");
    let naive = NaiveDateTime::parse_from_str(&text, "%d %b %Y %H:%M:%S").ok()?;
    Local.from_local_datetime(&naive).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn stats_are_read_by_name() {
        let text = "Sleep Count:670\nDark Wake Count:609\nUser Wake Count:88\n";
        assert_eq!(parse_stats(text), (Some(670), Some(609), Some(88)));
        assert_eq!(parse_stats("nonsense"), (None, None, None));
    }

    #[test]
    fn battery_share_weights_by_time() {
        let log = "\
2026-10-24 00:00:00 +0500 Sleep   Entering Sleep Using Batt (Charge:93%) 10 secs
2026-10-24 06:00:00 +0500 Wake    Wake Using AC (Charge:50%) 10 secs
2026-10-24 12:00:00 +0500 Other   no power word
2026-10-25 00:00:00 +0500 Sleep   Entering Sleep Using AC (Charge:90%)
";
        let (share, days) = battery_share(log).unwrap();
        assert!((share - 0.25).abs() < 1e-9, "{share}");
        assert!((days - 1.0).abs() < 1e-9, "{days}");
    }

    #[test]
    fn battery_share_needs_a_span() {
        assert_eq!(battery_share("nothing"), None);
        assert_eq!(
            battery_share("2026-10-24 00:00:00 +0500 Sleep Using AC"),
            None
        );
    }

    #[test]
    fn wtmp_start_is_parsed_in_c_locale() {
        let log = "reboot time Sun 18 Oct 18:25\n\nwtmp begins Sun Sep 20 14:08:30 +05 2026\n";
        let start = parse_wtmp_start(log).unwrap();
        assert_eq!((start.year(), start.month(), start.day()), (2026, 9, 20));
    }
}
