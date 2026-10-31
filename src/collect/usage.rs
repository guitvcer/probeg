//! Screen time and apps from the macOS activity log. The log is protected: without
//! Full Disk Access for the terminal, this section says plainly that it is locked.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use rusqlite::{Connection, OpenFlags};

use crate::fmt::from_unix;
use crate::model::{AppUse, Usage, UsageStatus};

/// The log counts time from January 1, 2001.
const COCOA_EPOCH: f64 = 978_307_200.0;
/// An open interval longer than a day is a log glitch, not actual work.
const MAX_INTERVAL_SECS: f64 = 86_400.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Interval {
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Focus {
    pub bundle: String,
    pub start: f64,
    pub end: f64,
}

pub fn usage() -> Usage {
    let Some(path) = database_path() else {
        return Usage::default();
    };
    // Without permission, the system won't even let us check whether the file exists.
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Usage::default(),
        Err(_) => {
            return Usage {
                status: UsageStatus::NeedsFullDiskAccess,
                ..Usage::default()
            };
        }
    }
    let opened = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    );
    let Ok(db) = opened else {
        return Usage {
            status: UsageStatus::NeedsFullDiskAccess,
            ..Usage::default()
        };
    };
    match read(&db) {
        Ok((screen, focus, notifications)) => summarize(
            &screen,
            &focus,
            &notifications,
            Local::now().timestamp() as f64,
        ),
        Err(_) => Usage {
            status: UsageStatus::NeedsFullDiskAccess,
            ..Usage::default()
        },
    }
}

fn database_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Knowledge/knowledgeC.db"))
}

type Rows = (Vec<Interval>, Vec<Focus>, Vec<f64>);

fn read(db: &Connection) -> rusqlite::Result<Rows> {
    let mut query = db.prepare(
        "select ZSTARTDATE, ZENDDATE from ZOBJECT
         where ZSTREAMNAME = '/display/isBacklit' and ZVALUEINTEGER = 1",
    )?;
    let screen = query
        .query_map([], |row| {
            Ok(Interval {
                start: row.get::<_, f64>(0)? + COCOA_EPOCH,
                end: row.get::<_, f64>(1)? + COCOA_EPOCH,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut query = db.prepare(
        "select ZVALUESTRING, ZSTARTDATE, ZENDDATE from ZOBJECT
         where ZSTREAMNAME = '/app/usage' and ZVALUESTRING is not null",
    )?;
    let focus = query
        .query_map([], |row| {
            Ok(Focus {
                bundle: row.get(0)?,
                start: row.get::<_, f64>(1)? + COCOA_EPOCH,
                end: row.get::<_, f64>(2)? + COCOA_EPOCH,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut query =
        db.prepare("select ZSTARTDATE from ZOBJECT where ZSTREAMNAME = '/notification/usage'")?;
    let notifications = query
        .query_map([], |row| Ok(row.get::<_, f64>(0)? + COCOA_EPOCH))?
        .collect::<Result<_, _>>()?;
    Ok((screen, focus, notifications))
}

fn clipped(start: f64, end: f64, now: f64) -> Option<(f64, f64)> {
    let end = end.min(now);
    (end > start && end - start <= MAX_INTERVAL_SECS).then_some((start, end))
}

fn days_since(first: Option<f64>, now: f64) -> f64 {
    first.map_or(0.0, |first| ((now - first) / 86_400.0).max(0.0))
}

fn first_of(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |best: Option<f64>, value| {
        Some(best.map_or(value, |b| b.min(value)))
    })
}

pub fn summarize(screen: &[Interval], focus: &[Focus], notifications: &[f64], now: f64) -> Usage {
    let mut usage = Usage {
        status: UsageStatus::Ok,
        week_hour: vec![vec![0.0; 24]; 7],
        ..Usage::default()
    };

    let today_start = from_unix(now as i64)
        .and_then(|t| t.with_hour(0)?.with_minute(0)?.with_second(0))
        .map_or(now, |t| t.timestamp() as f64);
    for (start, end) in screen.iter().filter_map(|i| clipped(i.start, i.end, now)) {
        usage.screen_secs += end - start;
        usage.today_secs += (end - start.max(today_start)).max(0.0);
        spread_by_hour(start, end, &mut usage.week_hour);
    }
    usage.screen_days = days_since(first_of(screen.iter().map(|i| i.start)), now);

    let mut per_app: HashMap<&str, f64> = HashMap::new();
    for item in focus {
        let Some((start, end)) = clipped(item.start, item.end, now) else {
            continue;
        };
        let secs = end - start;
        *per_app.entry(&item.bundle).or_default() += secs;
        usage.apps_total_secs += secs;
        usage.switches += 1;
        if usage
            .longest_focus
            .as_ref()
            .is_none_or(|best| secs > best.secs)
        {
            usage.longest_focus = Some(AppUse {
                bundle: item.bundle.clone(),
                name: item.bundle.clone(),
                secs,
            });
        }
    }
    usage.apps_days = days_since(first_of(focus.iter().map(|f| f.start)), now);
    usage.apps = per_app
        .into_iter()
        .map(|(bundle, secs)| AppUse {
            bundle: bundle.to_string(),
            name: bundle.to_string(),
            secs,
        })
        .collect();
    usage.apps.sort_by(|a, b| {
        b.secs
            .total_cmp(&a.secs)
            .then_with(|| a.bundle.cmp(&b.bundle))
    });

    usage.notifications = notifications.len() as u64;
    usage.notification_days = days_since(first_of(notifications.iter().copied()), now);
    usage
}

/// Spreads an interval across weekday × hour cells, in local time.
fn spread_by_hour(start: f64, end: f64, grid: &mut [Vec<f64>]) {
    let Some(mut cursor) = from_unix(start as i64) else {
        return;
    };
    let mut position = start;
    while position < end {
        let hour_start = hour_floor(cursor);
        let next = (hour_start + Duration::hours(1)).timestamp() as f64;
        let until = next.min(end);
        let day = cursor.weekday().num_days_from_monday() as usize;
        grid[day][cursor.hour() as usize] += until - position;
        position = until;
        match from_unix(position as i64) {
            Some(t) => cursor = t,
            None => break,
        }
    }
}

fn hour_floor(t: DateTime<Local>) -> DateTime<Local> {
    t - Duration::minutes(t.minute() as i64) - Duration::seconds(t.second() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> f64 {
        Local
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .unwrap()
            .timestamp() as f64
    }

    #[test]
    fn interval_splits_across_hours_and_weekdays() {
        // October 26, 2026 is a Monday. 23:30 → 00:30 on Tuesday.
        let screen = [Interval {
            start: ts(2026, 10, 26, 23, 30),
            end: ts(2026, 10, 27, 0, 30),
        }];
        let u = summarize(&screen, &[], &[], ts(2026, 10, 28, 12, 0));
        assert_eq!(u.week_hour[0][23], 1800.0);
        assert_eq!(u.week_hour[1][0], 1800.0);
        assert_eq!(u.screen_secs, 3600.0);
        assert_eq!(u.today_secs, 0.0);
    }

    #[test]
    fn today_only_counts_after_midnight() {
        let screen = [Interval {
            start: ts(2026, 10, 27, 23, 0),
            end: ts(2026, 10, 28, 1, 0),
        }];
        let u = summarize(&screen, &[], &[], ts(2026, 10, 28, 12, 0));
        assert_eq!(u.today_secs, 3600.0);
    }

    #[test]
    fn apps_are_ranked_and_broken_rows_ignored() {
        let now = ts(2026, 10, 28, 12, 0);
        let focus = |bundle: &str, start: f64, secs: f64| Focus {
            bundle: bundle.into(),
            start,
            end: start + secs,
        };
        let rows = [
            focus("a", now - 9000.0, 600.0),
            focus("b", now - 8000.0, 1200.0),
            focus("a", now - 6000.0, 900.0),
            focus("broken", now - 5000.0, -10.0),
            focus("absurd", now - 300_000.0, 200_000.0),
        ];
        let u = summarize(&[], &rows, &[], now);
        assert_eq!(u.apps[0].bundle, "a");
        assert_eq!(u.apps[0].secs, 1500.0);
        assert_eq!(u.apps[1].bundle, "b");
        assert_eq!(u.apps.len(), 2);
        assert_eq!(u.switches, 3);
        assert_eq!(u.longest_focus.as_ref().unwrap().bundle, "b");
        assert_eq!(u.apps_total_secs, 2700.0);
    }

    #[test]
    fn notifications_have_their_own_window() {
        let now = ts(2026, 10, 28, 12, 0);
        let u = summarize(&[], &[], &[now - 86_400.0 * 2.0, now - 100.0], now);
        assert_eq!(u.notifications, 2);
        assert!((u.notification_days - 2.0).abs() < 1e-9);
    }

    #[test]
    fn empty_journal_is_not_an_error() {
        let u = summarize(&[], &[], &[], 1.0);
        assert_eq!(u.status, UsageStatus::Ok);
        assert_eq!(u.screen_days, 0.0);
        assert!(u.apps.is_empty());
    }

    #[test]
    fn real_schema_query_works_on_a_copy() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "create table ZOBJECT (ZSTREAMNAME text, ZVALUESTRING text, ZVALUEINTEGER int, ZSTARTDATE real, ZENDDATE real);
             insert into ZOBJECT values ('/display/isBacklit', null, 1, 100, 200);
             insert into ZOBJECT values ('/display/isBacklit', null, 0, 200, 300);
             insert into ZOBJECT values ('/app/usage', 'com.x', 5, 100, 150);
             insert into ZOBJECT values ('/notification/usage', 'Receive', 0, 100, 100);",
        )
        .unwrap();
        let (screen, focus, notifications) = read(&db).unwrap();
        assert_eq!(screen.len(), 1);
        assert_eq!(screen[0].start, 100.0 + COCOA_EPOCH);
        assert_eq!(focus[0].bundle, "com.x");
        assert_eq!(notifications.len(), 1);
    }
}
