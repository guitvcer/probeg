//! Running processes, app crashes and command-line habits.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::fmt::from_unix;
use crate::model::{Household, NamedAmount, Ts};
use crate::sys;

#[derive(Default)]
pub struct Activity {
    pub processes: u64,
    pub top_memory: Vec<NamedAmount>,
    pub top_cpu: Vec<NamedAmount>,
    pub crashes: u64,
    pub top_crasher: Option<NamedAmount>,
    pub panics: u64,
    pub shell_commands: Option<u64>,
    pub shell_since: Option<Ts>,
    pub top_commands: Vec<NamedAmount>,
}

pub fn activity() -> Activity {
    let mut activity = Activity::default();
    if let Some(text) = sys::run("ps", &["-axo", "rss=,time=,comm="]) {
        apply_processes(&mut activity, &text);
    }
    let reports: Vec<Report> = report_dirs()
        .iter()
        .flat_map(|dir| reports_in(dir))
        .collect();
    apply_crashes(&mut activity, &reports);
    if let Some(home) = std::env::var_os("HOME") {
        let history = std::fs::read(Path::new(&home).join(".zsh_history"))
            .or_else(|_| std::fs::read(Path::new(&home).join(".bash_history")));
        if let Ok(raw) = history {
            apply_shell(&mut activity, &String::from_utf8_lossy(&raw));
        }
    }
    activity
}

struct Report {
    name: String,
    bug_type: Option<String>,
}

fn reports_in(dir: &Path) -> Vec<Report> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".ips"))
        .map(|entry| Report {
            name: entry.file_name().to_string_lossy().into_owned(),
            bug_type: read_bug_type(&entry.path()),
        })
        .collect()
}

/// The report type is on the file's first line, so there's no need to read all of it.
fn read_bug_type(path: &Path) -> Option<String> {
    let mut head = [0u8; 1024];
    let size = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    parse_bug_type(&String::from_utf8_lossy(&head[..size])).map(str::to_string)
}

fn parse_bug_type(text: &str) -> Option<&str> {
    let rest = text.split("\"bug_type\":\"").nth(1)?;
    rest.split('"').next()
}

pub fn fill(household: &mut Household, activity: &Activity) {
    household.processes = Some(activity.processes);
    household.top_cpu = activity.top_cpu.clone();
    household.crashes = Some(activity.crashes);
    household.top_crasher = activity.top_crasher.clone();
    household.panics = Some(activity.panics);
    household.shell_commands = activity.shell_commands;
    household.shell_since = activity.shell_since;
    household.top_commands = activity.top_commands.clone();
}

fn apply_processes(activity: &mut Activity, ps: &str) {
    let mut memory: HashMap<String, u64> = HashMap::new();
    let mut cpu: HashMap<String, u64> = HashMap::new();
    for line in ps.lines() {
        let line = line.trim_start();
        let Some((rss, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Some((time, command)) = rest.trim_start().split_once(char::is_whitespace) else {
            continue;
        };
        let name = process_name(command.trim());
        activity.processes += 1;
        *memory.entry(name.clone()).or_default() += rss.parse::<u64>().unwrap_or(0) * 1024;
        *cpu.entry(name).or_default() += parse_cpu_time(time).unwrap_or(0);
    }
    activity.top_memory = top(memory, 3);
    activity.top_cpu = top(cpu, 3);
}

fn top(map: HashMap<String, u64>, limit: usize) -> Vec<NamedAmount> {
    let mut ranked: Vec<NamedAmount> = map
        .into_iter()
        .filter(|(_, amount)| *amount > 0)
        .map(|(name, amount)| NamedAmount { name, amount })
        .collect();
    ranked.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.name.cmp(&b.name)));
    ranked.truncate(limit);
    ranked
}

/// App helpers count as the app itself: everything inside Slack.app is Slack.
pub(super) fn process_name(command: &str) -> String {
    if let Some(index) = command.find(".app/") {
        let head = &command[..index];
        return head.rsplit('/').next().unwrap_or(head).to_string();
    }
    command.rsplit('/').next().unwrap_or(command).to_string()
}

/// `ps` format: `[[dd-]hh:]mm:ss.ss`. Returns seconds.
fn parse_cpu_time(text: &str) -> Option<u64> {
    let (days, rest) = match text.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().ok()?, rest),
        None => (0, text),
    };
    let mut secs = days * 86_400;
    let mut scale = 1;
    for part in rest.rsplit(':') {
        let value = part.split('.').next()?.parse::<u64>().ok()?;
        secs += value * scale;
        scale *= 60;
    }
    Some(secs)
}

fn report_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Library/Logs/DiagnosticReports")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(Path::new(&home).join("Library/Logs/DiagnosticReports"));
    }
    dirs
}

/// The reports folder holds plenty of housekeeping files. Only type 309 is a crash, and 210 a panic.
fn apply_crashes(activity: &mut Activity, reports: &[Report]) {
    const CRASH: &str = "309";
    const PANIC: &str = "210";
    let mut per_app: HashMap<String, u64> = HashMap::new();
    for report in reports {
        match report.bug_type.as_deref() {
            Some(PANIC) => activity.panics += 1,
            Some(CRASH) => {
                let app = crash_owner(&report.name);
                if !app.starts_with("ExcUserFault") {
                    activity.crashes += 1;
                    *per_app.entry(app).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    activity.top_crasher = top(per_app, 1).into_iter().next();
}

/// `ac1-2026-10-29-102240.ips` → `ac1`.
fn crash_owner(file: &str) -> String {
    let bytes = file.as_bytes();
    let cut = (0..bytes.len().saturating_sub(5)).find(|&i| {
        bytes[i] == b'-'
            && bytes[i + 1..i + 5].iter().all(u8::is_ascii_digit)
            && bytes.get(i + 5) == Some(&b'-')
    });
    file[..cut.unwrap_or(file.len())].to_string()
}

fn apply_shell(activity: &mut Activity, history: &str) {
    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut total = 0;
    let mut earliest: Option<i64> = None;
    let mut continued = false;
    for line in history.lines() {
        let was_continued = continued;
        continued = line.ends_with('\\');
        if was_continued {
            continue;
        }
        let (stamp, command) = split_history_line(line);
        if let Some(stamp) = stamp {
            earliest = Some(earliest.map_or(stamp, |e| e.min(stamp)));
        }
        if let Some(word) = first_word(command) {
            total += 1;
            *counts.entry(word.to_string()).or_default() += 1;
        }
    }
    activity.shell_commands = Some(total);
    activity.shell_since = earliest.and_then(from_unix);
    activity.top_commands = top(counts, 5);
}

/// `: 1700000000:0;git status` → (time, command).
fn split_history_line(line: &str) -> (Option<i64>, &str) {
    if let Some(rest) = line.strip_prefix(": ")
        && let Some((meta, command)) = rest.split_once(';')
    {
        return (meta.split(':').next().and_then(|s| s.parse().ok()), command);
    }
    (None, line)
}

/// Keep only the program itself, without arguments: those can contain passwords.
fn first_word(command: &str) -> Option<&str> {
    command
        .split_whitespace()
        .find(|word| !word.contains('=') && *word != "sudo")
        .map(|word| word.rsplit('/').next().unwrap_or(word))
        .filter(|word| {
            word.chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '.' || c == '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_add_up_under_their_app() {
        let ps = "\
1000  1:00.00 /Applications/Slack.app/Contents/MacOS/Slack
2000  0:30.50 /Applications/Slack.app/Contents/Frameworks/Slack Helper.app/Contents/MacOS/Slack Helper
500   2:00.00 /usr/sbin/cfprefsd
";
        let mut a = Activity::default();
        apply_processes(&mut a, ps);
        assert_eq!(a.processes, 3);
        assert_eq!(
            a.top_memory[0],
            NamedAmount {
                name: "Slack".into(),
                amount: 3000 * 1024
            }
        );
        assert_eq!(
            a.top_cpu[0],
            NamedAmount {
                name: "cfprefsd".into(),
                amount: 120
            }
        );
        assert_eq!(a.top_cpu[1].amount, 90);
    }

    #[test]
    fn cpu_time_formats() {
        assert_eq!(parse_cpu_time("0:30.50"), Some(30));
        assert_eq!(parse_cpu_time("24:03.63"), Some(1443));
        assert_eq!(parse_cpu_time("1:02:03.00"), Some(3723));
        assert_eq!(parse_cpu_time("2-01:00:00.00"), Some(176_400));
        assert_eq!(parse_cpu_time("abc"), None);
    }

    fn report(name: &str, bug_type: &str) -> Report {
        Report {
            name: name.into(),
            bug_type: Some(bug_type.into()),
        }
    }

    #[test]
    fn only_real_crashes_count() {
        let reports = [
            report("ac1-2026-10-29-102240.ips", "309"),
            report("ac1-2026-10-29-105952.ips", "309"),
            report("Safari-2026-10-01-000000.ips", "309"),
            report("ExcUserFault_agent-2026-10-01-000000.ips", "309"),
            report("JetsamEvent-2026-10-01-000000.ips", "298"),
            report("panic-full-2026-10-01-000000.ips", "210"),
        ];
        let mut a = Activity::default();
        apply_crashes(&mut a, &reports);
        assert_eq!(a.crashes, 3);
        assert_eq!(a.panics, 1);
        assert_eq!(
            a.top_crasher,
            Some(NamedAmount {
                name: "ac1".into(),
                amount: 2
            })
        );
    }

    #[test]
    fn bug_type_is_read_from_the_report_header() {
        let head =
            r#"{"app_name":"mds_stores","is_first_party":1,"bug_type":"309","os_version":"x"}"#;
        assert_eq!(parse_bug_type(head), Some("309"));
        assert_eq!(parse_bug_type("{}"), None);
    }

    #[test]
    fn crash_owner_keeps_dashes_in_names() {
        assert_eq!(
            crash_owner("Google-Chrome-Helper-2026-10-01-000000.ips"),
            "Google-Chrome-Helper"
        );
        assert_eq!(crash_owner("short.ips"), "short.ips");
    }

    #[test]
    fn shell_history_keeps_only_program_names() {
        let history = "\
: 1700000000:0;git status --token=SECRET
: 1600000000:0;sudo /usr/bin/git push
FOO=1 cargo test
echo one \\
two
ls
#comment
";
        let mut a = Activity::default();
        apply_shell(&mut a, history);
        assert_eq!(a.shell_commands, Some(5));
        assert_eq!(
            a.top_commands[0],
            NamedAmount {
                name: "git".into(),
                amount: 2
            }
        );
        assert_eq!(a.shell_since.unwrap().timestamp(), 1_600_000_000);
        let names: Vec<_> = a.top_commands.iter().map(|c| c.name.as_str()).collect();
        assert!(!names.iter().any(|n| n.contains("SECRET")));
        assert!(names.contains(&"cargo"));
    }
}
