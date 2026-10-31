//! A believable made-up report for `--demo` and the rendering tests.

use chrono::{Local, TimeZone};

use crate::model::*;

fn at(y: i32, mo: u32, d: u32, h: u32) -> Ts {
    Local.with_ymd_and_hms(y, mo, d, h, 0, 0).unwrap()
}

fn named(name: &str, amount: u64) -> NamedAmount {
    NamedAmount {
        name: name.into(),
        amount,
    }
}

fn app(name: &str, hours: f64) -> AppUse {
    AppUse {
        bundle: format!("demo.{name}"),
        name: name.into(),
        secs: hours * 3600.0,
    }
}

pub fn report() -> Report {
    let mut week_hour = vec![vec![0.0; 24]; 7];
    for (day, row) in week_hour.iter_mut().enumerate() {
        let weekend = day >= 5;
        for (hour, cell) in row.iter_mut().enumerate() {
            let work = (9..19).contains(&hour) && !weekend;
            let evening = (20..24).contains(&hour);
            *cell = match (work, evening, weekend) {
                (true, _, _) => 2400.0 + 300.0 * ((hour * 7 + day * 3) % 5) as f64,
                (_, true, _) => 900.0 + 200.0 * ((hour + day) % 4) as f64,
                (_, _, true) if hour > 11 => 600.0,
                _ => 0.0,
            };
        }
    }
    Report {
        generated_at: at(2026, 10, 31, 3),
        identity: Identity {
            computer_name: Some("Alex's MacBook Pro".into()),
            model: Some("MacBook Pro".into()),
            model_id: Some("Mac16,1".into()),
            chip: Some("Apple M4".into()),
            cores_performance: Some(4),
            cores_efficiency: Some(6),
            memory_bytes: Some(16 << 30),
            ssd_model: Some("APPLE SSD AP0512Z".into()),
            ssd_bytes: Some(500_277_792_768),
            macos_version: Some("27.0.1".into()),
            macos_build: Some("26A434".into()),
        },
        timeline: Timeline {
            factory: Some(at(2025, 5, 17, 19)),
            first_setup: Some(at(2025, 7, 2, 12)),
            major_installed: Some(at(2026, 9, 20, 9)),
            major_version: Some("27".into()),
            last_update: Some(at(2026, 10, 18, 13)),
            macos_versions: vec![
                OsVersion {
                    version: "15.4.1".into(),
                    installed: at(2025, 5, 17, 19),
                },
                OsVersion {
                    version: "26.0".into(),
                    installed: at(2025, 9, 22, 5),
                },
                OsVersion {
                    version: "27.0".into(),
                    installed: at(2026, 9, 20, 9),
                },
                OsVersion {
                    version: "27.0.1".into(),
                    installed: at(2026, 10, 18, 13),
                },
            ],
            install_records: 380,
            security_updates: 26,
        },
        power: Power {
            boot: Some(at(2026, 10, 18, 18)),
            login: Some(at(2026, 10, 18, 18)),
            last_wake: Some(at(2026, 10, 30, 20)),
            last_sleep: Some(at(2026, 10, 30, 19)),
            awake_secs: Some(400_000),
            sleep_count: Some(670),
            user_wakes: Some(88),
            dark_wakes: Some(609),
            on_battery_share: Some(0.42),
            power_log_days: Some(7.0),
            reboots_logged: Some(9),
            log_since: Some(at(2026, 9, 20, 14)),
        },
        ssd: Some(Ssd {
            bytes_read: 102_602_000_000_000,
            bytes_written: 47_101_000_000_000,
            power_on_hours: 1351,
            power_cycles: 262,
            unsafe_shutdowns: 7,
            media_errors: 0,
            wear_percent: 2,
            spare_percent: 100,
            temperature_c: 43,
        }),
        disk: Some(Disk {
            total_bytes: 494_384_795_648,
            free_bytes: 101_000_000_000,
            file_count: 4_184_000,
            read_since_boot: Some(4_512_501_809_152),
            written_since_boot: Some(1_960_404_815_872),
        }),
        memory: Some(Memory {
            total: 16 << 30,
            apps: 6 << 30,
            wired: 3 << 30,
            compressed: 3 << 30,
            cached: 3 << 30,
            free: 1 << 30,
            swap_used: 9 << 30,
            swap_total: 11 << 30,
            compressions: 3_434_498_937,
            swapouts_bytes: 400_000_000_000,
            top: vec![
                named("qemu-system-aarch64", 1_850_000_000),
                named("Safari", 1_200_000_000),
                named("Xcode", 410_000_000),
            ],
        }),
        battery: Some(Battery {
            charge_percent: 100,
            charging: false,
            on_ac: true,
            cycles: 183,
            design_cycles: Some(1000),
            max_capacity_percent: Some(97),
            condition: Some("Good".into()),
            design_mah: Some(6249),
            full_mah: Some(5810),
            adapter: Some("140W USB-C Power Adapter".into()),
        }),
        network: Some(Network {
            down_since_boot: 66_921_412_061,
            up_since_boot: 8_475_229_714,
            top_live: vec![
                named("Safari", 3_100_000_000),
                named("Telegram", 900_000_000),
            ],
            known_wifi: Some(37),
            bluetooth_paired: Some(6),
            bluetooth_connected: Some(2),
        }),
        usage: Usage {
            status: UsageStatus::Ok,
            screen_days: 28.0,
            screen_secs: 163.4 * 3600.0,
            today_secs: 3.2 * 3600.0,
            week_hour,
            apps_days: 16.2,
            apps_total_secs: 85.9 * 3600.0,
            apps: vec![
                app("Terminal", 32.6),
                app("Safari", 13.1),
                app("Slack", 10.2),
                app("Xcode", 8.6),
                app("Figma", 6.1),
                app("Telegram", 4.5),
            ],
            switches: 10_063,
            longest_focus: Some(app("Terminal", 2.4)),
            notifications: 1629,
            notification_days: 27.5,
        },
        household: Household {
            apps_installed: Some(47),
            apps_system: Some(38),
            apps_unopened: Some(31),
            newest_app: Some("Notion".into()),
            newest_app_date: Some(at(2026, 10, 20, 12)),
            apps_new_30d: Some(3),
            brew_formulae: Some(188),
            brew_casks: Some(4),
            users: vec!["alex".into()],
            admins: Some(1),
            open_terminals: Some(3),
            processes: Some(742),
            top_cpu: vec![named("WindowServer", 95_000), named("Safari", 41_000)],
            crashes: Some(37),
            top_crasher: Some(named("Xcode", 21)),
            panics: Some(0),
            shell_commands: Some(11_546),
            shell_since: Some(at(2025, 7, 3, 9)),
            top_commands: vec![
                named("git", 3100),
                named("cd", 1900),
                named("cargo", 1200),
                named("npm", 800),
                named("ls", 700),
            ],
        },
    }
}
