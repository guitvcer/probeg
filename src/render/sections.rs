//! What each section says. A row appears only when there is data for it.

use super::table::{Section, sparkline};
use super::theme::Tone;
use crate::i18n::{Locale, fill};
use crate::model::{OsVersion, Report, Ts, UsageStatus};

/// Rows of apps and commands shown before the list stops being useful.
const TOP_APPS: usize = 5;
const TOP_COMMANDS: usize = 4;
/// Long process names must not take over a row.
const NAME_CELLS: usize = 22;
/// Fits one column at the narrowest page.
const HINT_CELLS: usize = 40;

pub fn all(r: &Report, l: &Locale) -> Vec<Section> {
    [
        history(r, l),
        session(r, l),
        lifetime(r, l),
        battery(r, l),
        memory(r, l),
        storage(r, l),
        network(r, l),
        usage(r, l),
        top_apps(r, l),
        software(r, l),
    ]
    .into_iter()
    .filter(|section| !section.is_empty())
    .collect()
}

fn name(text: &str) -> String {
    super::layout::fit(text, NAME_CELLS)
}

fn percent(share: f64) -> String {
    format!("{}%", (share * 100.0).round() as u64)
}

fn days_alive(r: &Report) -> Option<f64> {
    r.timeline
        .first_setup
        .map(|born| (r.generated_at - born).num_days().max(1) as f64)
}

/// Years until `limit` at the current pace. Zero pace gives no forecast.
fn years_until(done: f64, limit: f64, days: f64) -> Option<f64> {
    (done > 0.0 && done < limit).then(|| (limit - done) / (done / days) / 365.25)
}

fn ago(r: &Report, l: &Locale, then: Ts) -> String {
    fill(
        l.ago,
        &[&l.duration((r.generated_at - then).num_seconds() as f64)],
    )
}

fn distinct_majors(versions: &[OsVersion]) -> usize {
    let mut majors: Vec<&str> = versions
        .iter()
        .filter_map(|v| v.version.split('.').next())
        .collect();
    majors.sort_unstable();
    majors.dedup();
    majors.len()
}

fn history(r: &Report, l: &Locale) -> Section {
    let t = &r.timeline;
    let mut s = Section::new(l.section_history);
    if let Some(factory) = t.factory {
        s.row(l.built, format!("~{}", l.month_year(factory)));
    }
    if let Some(setup) = t.first_setup {
        s.note(
            l.first_setup,
            l.date(setup),
            fill(l.ago, &[&l.age(setup, r.generated_at)]),
        );
    }
    if let (Some(installed), Some(major)) = (t.major_installed, &t.major_version) {
        s.note(
            fill(l.os_installed, &[major]),
            l.date(installed),
            fill(l.ago, &[&l.age(installed, r.generated_at)]),
        );
    }
    if let Some(update) = t.last_update {
        s.row(l.last_update, l.date(update));
    }
    if !t.macos_versions.is_empty() {
        let majors = distinct_majors(&t.macos_versions).to_string();
        s.note(
            l.os_versions,
            t.macos_versions.len().to_string(),
            fill(l.versions_major, &[&majors]),
        );
    }
    s
}

fn session(r: &Report, l: &Locale) -> Section {
    let power = &r.power;
    let mut s = Section::new(l.section_session);
    if let Some(boot) = power.boot {
        s.note(l.powered_on, l.date_time(boot), ago(r, l, boot));
    }
    if let Some(login) = power.login {
        s.note(l.logged_in, l.date_time(login), ago(r, l, login));
    }
    if let Some(wake) = power.last_wake {
        s.note(l.last_wake, l.date_time(wake), ago(r, l, wake));
    }
    if let (Some(awake), Some(boot)) = (power.awake_secs, power.boot) {
        let total = (r.generated_at - boot).num_seconds().max(1) as f64;
        s.note(
            l.awake,
            l.duration(awake as f64),
            percent(awake as f64 / total),
        );
    }
    if let Some(sleeps) = power.sleep_count {
        s.row(l.sleeps, l.group(sleeps));
    }
    if let (Some(user), Some(dark)) = (power.user_wakes, power.dark_wakes) {
        s.note(
            l.wakes,
            l.group(user + dark),
            fill(l.wakes_note, &[&l.group(user), &l.group(dark)]),
        );
    }
    if let (Some(reboots), Some(since)) = (power.reboots_logged, power.log_since) {
        s.note(
            l.reboots,
            l.group(reboots),
            fill(l.since, &[&l.date(since)]),
        );
    }
    s
}

fn lifetime(r: &Report, l: &Locale) -> Section {
    let mut s = Section::new(l.section_lifetime);
    let Some(d) = &r.ssd else {
        return s;
    };
    let days = days_alive(r);
    let rate = |bytes: u64| {
        days.map_or_else(String::new, |days| {
            fill(l.per_day, &[&l.bytes((bytes as f64 / days) as u64)])
        })
    };
    s.note(l.written, l.bytes(d.bytes_written), rate(d.bytes_written));
    s.note(l.read, l.bytes(d.bytes_read), rate(d.bytes_read));
    s.note(
        l.hours_on,
        l.group(d.power_on_hours),
        l.count(d.power_cycles, &l.power_ons),
    );
    let left = days
        .and_then(|days| years_until(d.wear_percent as f64, 100.0, days))
        .map_or_else(String::new, |years| {
            fill(l.years_left, &[&(years.round() as u64).to_string()])
        });
    let wear = format!("{}%", d.wear_percent);
    if d.wear_percent >= 80 {
        s.toned(l.wear, wear, left, Tone::Red);
    } else {
        s.note(l.wear, wear, left);
    }
    if d.spare_percent < 100 {
        s.row(l.spare, format!("{}%", d.spare_percent));
    }
    s.row(l.unsafe_shutdowns, l.group(d.unsafe_shutdowns));
    if d.media_errors > 0 {
        s.toned(l.media_errors, l.group(d.media_errors), "", Tone::Red);
    }
    s.row(l.temperature, format!("{} °C", d.temperature_c));
    s
}

fn battery(r: &Report, l: &Locale) -> Section {
    let mut s = Section::new(l.section_battery);
    let Some(b) = &r.battery else {
        return s;
    };
    let state = match (b.charging, b.on_ac) {
        (true, _) => l.state_charging,
        (false, true) => l.state_plugged,
        _ => l.state_on_battery,
    };
    let tone = match b.charge_percent {
        0..=15 => Tone::Red,
        16..=35 => Tone::Yellow,
        _ => Tone::Green,
    };
    s.bar(
        b.charge_percent as f64 / 100.0,
        tone,
        format!("{}%, {state}", b.charge_percent),
    );
    let cycles = match b.design_cycles {
        Some(limit) => format!("{} / {}", l.group(b.cycles), l.group(limit)),
        None => l.group(b.cycles),
    };
    let pace = days_alive(r)
        .filter(|_| b.cycles > 0)
        .map_or_else(String::new, |days| {
            fill(
                l.per_month,
                &[&l.decimal(b.cycles as f64 / days * 30.44, 1)],
            )
        });
    s.note(l.cycles, cycles, pace);
    if let Some(capacity) = b.max_capacity_percent {
        let note = match (b.full_mah, b.design_mah) {
            (Some(full), Some(design)) => {
                format!("{} / {} {}", l.group(full), l.group(design), l.mah)
            }
            _ => String::new(),
        };
        s.note(l.capacity, format!("{capacity}%"), note);
    }
    if let (Some(share), Some(days)) = (r.power.on_battery_share, r.power.power_log_days) {
        let window = l.count(days.round() as u64, &l.days);
        s.note(l.on_battery, percent(share), fill(l.last_n, &[&window]));
    }
    if let Some(adapter) = &b.adapter {
        s.row(l.charger, adapter.clone());
    }
    s
}

fn memory(r: &Report, l: &Locale) -> Section {
    let mut s = Section::new(l.section_memory);
    let Some(m) = &r.memory else {
        return s;
    };
    let used = m.apps + m.wired + m.compressed;
    let share = used as f64 / m.total.max(1) as f64;
    s.bar(
        share,
        Tone::Green,
        format!("{} / {}", l.ram(used), l.ram(m.total)),
    );
    s.row(l.mem_apps, l.ram(m.apps));
    s.row(l.mem_wired, l.ram(m.wired));
    s.row(l.mem_compressed, l.ram(m.compressed));
    s.row(l.mem_cached, l.ram(m.cached));
    s.row(l.free, l.ram(m.free));
    if m.swap_total > 0 {
        let value = format!("{} / {}", l.ram(m.swap_used), l.ram(m.swap_total));
        if m.swap_used as f64 / m.total.max(1) as f64 > 0.25 {
            s.toned(l.swap, value, "", Tone::Yellow);
        } else {
            s.row(l.swap, value);
        }
    }
    if let Some(top) = m.top.first() {
        s.note(l.largest, name(&top.name), l.bytes(top.amount));
    }
    s
}

fn storage(r: &Report, l: &Locale) -> Section {
    let mut s = Section::new(l.section_storage);
    let Some(d) = &r.disk else {
        return s;
    };
    let used = d.total_bytes.saturating_sub(d.free_bytes);
    let share = used as f64 / d.total_bytes.max(1) as f64;
    let tone = if share > 0.9 {
        Tone::Red
    } else if share > 0.75 {
        Tone::Yellow
    } else {
        Tone::Green
    };
    s.bar(
        share,
        tone,
        format!("{} / {}", l.bytes(used), l.bytes(d.total_bytes)),
    );
    s.row(l.free, l.bytes(d.free_bytes));
    s.row(l.files, l.group(d.file_count));
    if let (Some(read), Some(written)) = (d.read_since_boot, d.written_since_boot) {
        s.row(l.read_since_boot, l.bytes(read));
        s.row(l.written_since_boot, l.bytes(written));
    }
    s
}

fn network(r: &Report, l: &Locale) -> Section {
    let mut s = Section::new(l.section_network);
    let Some(n) = &r.network else {
        return s;
    };
    let since = r
        .power
        .boot
        .map_or_else(String::new, |boot| fill(l.since, &[&l.date(boot)]));
    s.note(l.down, l.bytes(n.down_since_boot), since);
    s.row(l.up, l.bytes(n.up_since_boot));
    if let Some(top) = n.top_live.first() {
        s.note(l.top_traffic, name(&top.name), l.bytes(top.amount));
    }
    if let Some(wifi) = n.known_wifi {
        s.row(l.known_wifi, l.group(wifi));
    }
    if let (Some(paired), Some(connected)) = (n.bluetooth_paired, n.bluetooth_connected) {
        s.note(
            l.bluetooth,
            l.group(paired),
            fill(l.bluetooth_note, &[&l.group(connected)]),
        );
    }
    s
}

fn usage(r: &Report, l: &Locale) -> Section {
    let u = &r.usage;
    let mut s = Section::new(l.section_usage);
    match u.status {
        UsageStatus::Unavailable => return s,
        UsageStatus::NeedsFullDiskAccess => {
            for line in super::layout::wrap(l.full_disk_access_hint, HINT_CELLS) {
                s.text(line);
            }
            return s;
        }
        UsageStatus::Ok => {}
    }
    if u.screen_days >= 1.0 {
        s.row(l.screen_today, l.duration(u.today_secs));
        let window = l.count(u.screen_days.round() as u64, &l.days);
        s.note(
            l.screen_avg,
            l.duration(u.screen_secs / u.screen_days),
            fill(l.last_n, &[&window]),
        );
        if u.week_hour.iter().flatten().any(|secs| *secs > 0.0) {
            let mut hours = [0.0; 24];
            for day in &u.week_hour {
                for (hour, secs) in day.iter().enumerate() {
                    hours[hour] += secs;
                }
            }
            s.row(format!("{} 00-23", l.by_hour), sparkline(&hours));
        }
    }
    if u.apps_days >= 1.0 && u.switches > 0 {
        s.row(
            l.switches_per_day,
            l.group((u.switches as f64 / u.apps_days).round() as u64),
        );
    }
    if u.notification_days >= 1.0 {
        s.row(
            l.notifications_per_day,
            l.group((u.notifications as f64 / u.notification_days).round() as u64),
        );
    }
    s
}

fn top_apps(r: &Report, l: &Locale) -> Section {
    let u = &r.usage;
    let window = l.count(u.apps_days.round() as u64, &l.days);
    let mut s = Section::new(&fill(l.top_apps, &[&window]));
    let total = u.apps_total_secs.max(1.0);
    for app in u.apps.iter().take(TOP_APPS) {
        s.note(
            name(&app.name),
            l.hours(app.secs),
            percent(app.secs / total),
        );
    }
    s
}

fn software(r: &Report, l: &Locale) -> Section {
    let h = &r.household;
    let mut s = Section::new(l.section_software);
    if let Some(apps) = h.apps_installed {
        let note = h
            .apps_system
            .map_or_else(String::new, |system| fill(l.apps_note, &[&l.group(system)]));
        s.note(l.apps, l.group(apps), note);
    }
    if let Some(unopened) = h.apps_unopened {
        let window = l.count(r.usage.apps_days.round() as u64, &l.days);
        s.note(l.unopened, l.group(unopened), fill(l.last_n, &[&window]));
    }
    if let (Some(formulae), Some(casks)) = (h.brew_formulae, h.brew_casks) {
        s.row(
            l.homebrew,
            fill(l.homebrew_value, &[&l.group(formulae), &l.group(casks)]),
        );
    }
    if !h.users.is_empty() {
        s.note(l.users, h.users.len().to_string(), h.users.join(", "));
    }
    if let Some(processes) = h.processes {
        s.row(l.processes, l.group(processes));
    }
    if let Some(crashes) = h.crashes {
        let note = h
            .top_crasher
            .as_ref()
            .filter(|_| crashes > 0)
            .map_or_else(String::new, |top| fill(l.crashes_note, &[&name(&top.name)]));
        s.note(l.crashes, l.group(crashes), note);
    }
    if let Some(panics) = h.panics.filter(|n| *n > 0) {
        s.row(l.panics, l.group(panics));
    }
    if let Some(commands) = h.shell_commands {
        s.row(l.shell_commands, l.group(commands));
    }
    if !h.top_commands.is_empty() {
        let top: Vec<&str> = h
            .top_commands
            .iter()
            .take(TOP_COMMANDS)
            .map(|c| c.name.as_str())
            .collect();
        s.row(l.top_commands, top.join(", "));
    }
    s
}

/// The machine in one line, the way `neofetch` opens.
pub fn spec_line(r: &Report, l: &Locale) -> String {
    let id = &r.identity;
    let model = id.model.as_ref().map(|model| match &id.model_id {
        Some(code) => format!("{model} ({code})"),
        None => model.clone(),
    });
    let chip = id
        .chip
        .as_ref()
        .map(|chip| match (id.cores_performance, id.cores_efficiency) {
            (Some(perf), Some(eff)) => format!("{chip} {perf}+{eff}"),
            _ => chip.clone(),
        });
    let macos = id
        .macos_version
        .as_ref()
        .map(|version| match &id.macos_build {
            Some(build) => format!("macOS {version} ({build})"),
            None => format!("macOS {version}"),
        });
    [
        model,
        chip,
        id.memory_bytes
            .map(|bytes| fill(l.spec_ram, &[&l.ram(bytes)])),
        id.ssd_bytes
            .map(|bytes| fill(l.spec_ssd, &[&l.bytes(bytes)])),
        macos,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_resource_gives_no_forecast() {
        assert_eq!(years_until(1000.0, 1000.0, 100.0), None);
        assert_eq!(years_until(0.0, 1000.0, 100.0), None);
        assert_eq!(years_until(50.0, 100.0, 365.25), Some(1.0));
    }
}
