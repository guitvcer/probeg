//! The system's biography: the install log and the file marking first-time setup as done.

use std::time::SystemTime;

use chrono::{DateTime, Duration, Local};
use serde::Deserialize;

use crate::model::{OsVersion, Timeline, Ts};

const HISTORY: &str = "/Library/Receipts/InstallHistory.plist";
const SETUP_MARKER: &str = "/var/db/.AppleSetupDone";

#[derive(Deserialize)]
struct RawRecord {
    date: plist::Date,
    #[serde(rename = "displayName", default)]
    name: String,
    #[serde(rename = "displayVersion", default)]
    version: String,
    #[serde(rename = "processName", default)]
    process: String,
}

pub struct Record {
    pub date: Ts,
    pub name: String,
    pub version: String,
    pub process: String,
}

pub fn timeline() -> Timeline {
    let records = plist::from_file::<_, Vec<RawRecord>>(HISTORY)
        .unwrap_or_default()
        .into_iter()
        .map(|raw| Record {
            date: DateTime::<Local>::from(SystemTime::from(raw.date)),
            name: raw.name,
            version: raw.version,
            process: raw.process,
        })
        .collect();
    let setup = std::fs::metadata(SETUP_MARKER)
        .and_then(|meta| meta.created())
        .ok()
        .map(DateTime::<Local>::from);
    let current = crate::sys::run("sw_vers", &["-productVersion"]).map(|v| v.trim().to_string());
    summarize(records, setup, current.as_deref())
}

pub fn summarize(mut records: Vec<Record>, setup: Option<Ts>, current: Option<&str>) -> Timeline {
    records.sort_by_key(|r| r.date);

    let mut versions: Vec<OsVersion> = Vec::new();
    for record in records.iter().filter(|r| r.name.starts_with("macOS ")) {
        if !record.version.is_empty() && !versions.iter().any(|v| v.version == record.version) {
            versions.push(OsVersion {
                version: record.version.clone(),
                installed: record.date,
            });
        }
    }

    // An entry older than first-time setup came from the factory. If it matches setup instead,
    // the system was reinstalled and the factory date is unknown.
    let factory = records
        .first()
        .map(|r| r.date)
        .filter(|first| setup.is_some_and(|s| *first < s - Duration::days(1)));

    let major = current
        .and_then(|c| c.split('.').next())
        .map(str::to_string);
    let major_installed = major.as_ref().and_then(|major| {
        versions
            .iter()
            .find(|v| v.version.split('.').next() == Some(major))
            .map(|v| v.installed)
    });

    Timeline {
        factory,
        first_setup: setup,
        major_installed,
        major_version: major,
        last_update: versions.last().map(|v| v.installed),
        install_records: records.len(),
        security_updates: records
            .iter()
            .filter(|r| r.process == "XProtectUpdateService")
            .count(),
        macos_versions: versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, mo: u32, d: u32) -> Ts {
        Local.with_ymd_and_hms(y, mo, d, 12, 0, 0).unwrap()
    }

    fn os(date: Ts, version: &str) -> Record {
        Record {
            date,
            name: format!("macOS {version}"),
            version: version.into(),
            process: "softwareupdated".into(),
        }
    }

    #[test]
    fn factory_record_precedes_setup() {
        let t = summarize(
            vec![
                os(at(2025, 5, 17), "15.4.1"),
                os(at(2025, 9, 22), "26.0"),
                os(at(2026, 9, 20), "27.0"),
                os(at(2026, 10, 18), "27.0.1"),
            ],
            Some(at(2025, 7, 2)),
            Some("27.0.1"),
        );
        assert_eq!(t.factory, Some(at(2025, 5, 17)));
        assert_eq!(t.major_installed, Some(at(2026, 9, 20)));
        assert_eq!(t.major_version.as_deref(), Some("27"));
        assert_eq!(t.last_update, Some(at(2026, 10, 18)));
        assert_eq!(t.macos_versions.len(), 4);
    }

    #[test]
    fn reinstall_hides_factory_date() {
        let t = summarize(vec![os(at(2025, 7, 2), "15.5")], Some(at(2025, 7, 2)), None);
        assert_eq!(t.factory, None);
    }

    #[test]
    fn repeated_versions_count_once_and_empty_history_is_fine() {
        let t = summarize(
            vec![os(at(2025, 7, 23), "26.0"), os(at(2025, 7, 30), "26.0")],
            None,
            None,
        );
        assert_eq!(t.macos_versions.len(), 1);
        assert_eq!(t.macos_versions[0].installed, at(2025, 7, 23));
        let empty = summarize(vec![], None, None);
        assert_eq!(empty.factory, None);
        assert_eq!(empty.install_records, 0);
    }

    #[test]
    fn security_updates_are_counted_by_process() {
        let record = Record {
            date: at(2026, 1, 1),
            name: "XProtectCloudKitUpdate".into(),
            version: "1".into(),
            process: "XProtectUpdateService".into(),
        };
        let t = summarize(vec![record], None, None);
        assert_eq!(t.security_updates, 1);
        assert!(t.macos_versions.is_empty());
    }
}
