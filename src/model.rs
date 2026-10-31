//! A snapshot of the machine. Anything that might fail to collect is an `Option`: the report must
//! still render however much data is missing.

use chrono::{DateTime, Local};
use serde::Serialize;

pub type Ts = DateTime<Local>;

#[derive(Serialize, Default, Clone, Debug)]
pub struct Report {
    pub generated_at: Ts,
    pub identity: Identity,
    pub timeline: Timeline,
    pub power: Power,
    pub ssd: Option<Ssd>,
    pub disk: Option<Disk>,
    pub memory: Option<Memory>,
    pub battery: Option<Battery>,
    pub network: Option<Network>,
    pub usage: Usage,
    pub household: Household,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Identity {
    pub computer_name: Option<String>,
    pub model: Option<String>,
    pub model_id: Option<String>,
    pub chip: Option<String>,
    pub cores_performance: Option<u32>,
    pub cores_efficiency: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub ssd_model: Option<String>,
    pub ssd_bytes: Option<u64>,
    pub macos_version: Option<String>,
    pub macos_build: Option<String>,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Timeline {
    /// The first install record: on a new Mac, it's made at the factory.
    pub factory: Option<Ts>,
    pub first_setup: Option<Ts>,
    pub major_installed: Option<Ts>,
    pub major_version: Option<String>,
    pub last_update: Option<Ts>,
    pub macos_versions: Vec<OsVersion>,
    pub install_records: usize,
    pub security_updates: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct OsVersion {
    pub version: String,
    pub installed: Ts,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Power {
    pub boot: Option<Ts>,
    pub login: Option<Ts>,
    pub last_wake: Option<Ts>,
    pub last_sleep: Option<Ts>,
    /// Seconds since boot that the machine was awake.
    pub awake_secs: Option<u64>,
    pub sleep_count: Option<u64>,
    pub user_wakes: Option<u64>,
    pub dark_wakes: Option<u64>,
    /// Share of time on battery from the power log, and the log's depth in days.
    pub on_battery_share: Option<f64>,
    pub power_log_days: Option<f64>,
    pub reboots_logged: Option<u64>,
    pub log_since: Option<Ts>,
}

/// Lifetime counters from the drive itself.
#[derive(Serialize, Default, Clone, Debug, PartialEq)]
pub struct Ssd {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub power_on_hours: u64,
    pub power_cycles: u64,
    pub unsafe_shutdowns: u64,
    pub media_errors: u64,
    pub wear_percent: u8,
    pub spare_percent: u8,
    pub temperature_c: i32,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Disk {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub file_count: u64,
    pub read_since_boot: Option<u64>,
    pub written_since_boot: Option<u64>,
}

#[derive(Serialize, Default, Clone, Debug, PartialEq)]
pub struct Memory {
    pub total: u64,
    pub apps: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
    pub free: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub compressions: u64,
    pub swapouts_bytes: u64,
    pub top: Vec<NamedAmount>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NamedAmount {
    pub name: String,
    pub amount: u64,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Battery {
    pub charge_percent: u8,
    pub charging: bool,
    pub on_ac: bool,
    pub cycles: u64,
    pub design_cycles: Option<u64>,
    pub max_capacity_percent: Option<u8>,
    pub condition: Option<String>,
    pub design_mah: Option<u64>,
    pub full_mah: Option<u64>,
    pub adapter: Option<String>,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Network {
    pub down_since_boot: u64,
    pub up_since_boot: u64,
    pub top_live: Vec<NamedAmount>,
    pub known_wifi: Option<u64>,
    pub bluetooth_paired: Option<u64>,
    pub bluetooth_connected: Option<u64>,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Usage {
    pub status: UsageStatus,
    pub screen_days: f64,
    pub screen_secs: f64,
    pub today_secs: f64,
    /// Seconds with the screen on: \[weekday starting from Monday\]\[hour\].
    pub week_hour: Vec<Vec<f64>>,
    pub apps_days: f64,
    pub apps_total_secs: f64,
    pub apps: Vec<AppUse>,
    pub switches: u64,
    pub longest_focus: Option<AppUse>,
    pub notifications: u64,
    pub notification_days: f64,
}

#[derive(Serialize, Default, Clone, Copy, Debug, PartialEq)]
pub enum UsageStatus {
    #[default]
    Unavailable,
    /// The terminal app needs Full Disk Access.
    NeedsFullDiskAccess,
    Ok,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AppUse {
    pub bundle: String,
    pub name: String,
    pub secs: f64,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Household {
    pub apps_installed: Option<u64>,
    pub apps_system: Option<u64>,
    pub apps_unopened: Option<u64>,
    pub newest_app: Option<String>,
    pub newest_app_date: Option<Ts>,
    pub apps_new_30d: Option<u64>,
    pub brew_formulae: Option<u64>,
    pub brew_casks: Option<u64>,
    pub users: Vec<String>,
    pub admins: Option<u64>,
    pub open_terminals: Option<u64>,
    pub processes: Option<u64>,
    pub top_cpu: Vec<NamedAmount>,
    pub crashes: Option<u64>,
    pub top_crasher: Option<NamedAmount>,
    pub panics: Option<u64>,
    pub shell_commands: Option<u64>,
    pub shell_since: Option<Ts>,
    pub top_commands: Vec<NamedAmount>,
}
