//! Network. The system only counts traffic since the last boot.

use std::collections::HashMap;

use serde_json::Value;

use super::activity::process_name;
use crate::model::{NamedAmount, Network};
use crate::sys;

pub fn network() -> Option<Network> {
    let (down, up) = sys::run("netstat", &["-ibn"])
        .as_deref()
        .map(physical_traffic)?;
    let bluetooth = sys::run("system_profiler", &["SPBluetoothDataType", "-json"])
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    Some(Network {
        down_since_boot: down,
        up_since_boot: up,
        top_live: sys::run(
            "nettop",
            &["-P", "-L", "1", "-x", "-n", "-J", "bytes_in,bytes_out"],
        )
        .map(|csv| top_processes(&csv, 3, &full_process_names()))
        .unwrap_or_default(),
        known_wifi: known_wifi(),
        bluetooth_paired: bluetooth.as_ref().map(|b| bluetooth_count(b, true)),
        bluetooth_connected: bluetooth.as_ref().map(|b| bluetooth_count(b, false)),
    })
}

/// Total across physical interfaces. VPN tunnels are skipped: their traffic already went through Wi-Fi.
pub fn physical_traffic(netstat: &str) -> (u64, u64) {
    let (mut down, mut up) = (0, 0);
    for line in netstat.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 9 || !fields[2].starts_with("<Link#") {
            continue;
        }
        let name = fields[0].trim_end_matches('*');
        let is_physical = name
            .strip_prefix("en")
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
        if !is_physical {
            continue;
        }
        // From the end: Ibytes Opkts Oerrs Obytes Coll. The line may have no address.
        let tail = &fields[fields.len() - 5..];
        down += tail[0].parse::<u64>().unwrap_or(0);
        up += tail[3].parse::<u64>().unwrap_or(0);
    }
    (down, up)
}

/// Which currently running process has moved the most data.
pub fn top_processes(
    csv: &str,
    limit: usize,
    full_names: &HashMap<String, String>,
) -> Vec<NamedAmount> {
    let mut totals: HashMap<String, u64> = HashMap::new();
    for line in csv.lines().skip(1) {
        let mut fields = line.split(',');
        let (Some(process), Some(down), Some(up)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // nettop truncates names, so the full one is looked up by process ID.
        let (short, pid) = process.rsplit_once('.').unwrap_or((process, ""));
        let name = full_names.get(pid).map_or(short, String::as_str);
        let amount = down.parse::<u64>().unwrap_or(0) + up.parse::<u64>().unwrap_or(0);
        *totals.entry(name.to_string()).or_default() += amount;
    }
    let mut ranked: Vec<_> = totals
        .into_iter()
        .filter(|(_, amount)| *amount > 0)
        .map(|(name, amount)| NamedAmount { name, amount })
        .collect();
    ranked.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.name.cmp(&b.name)));
    ranked.truncate(limit);
    ranked
}

fn full_process_names() -> HashMap<String, String> {
    let listing = sys::run("ps", &["-axo", "pid=,comm="]).unwrap_or_default();
    listing
        .lines()
        .filter_map(|line| {
            let (pid, command) = line.trim_start().split_once(char::is_whitespace)?;
            Some((pid.to_string(), process_name(command.trim())))
        })
        .collect()
}

fn known_wifi() -> Option<u64> {
    let ports = sys::run("networksetup", &["-listallhardwareports"])?;
    let device = wifi_device(&ports)?;
    let list = sys::run("networksetup", &["-listpreferredwirelessnetworks", &device])?;
    Some(
        list.lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count() as u64,
    )
}

fn wifi_device(ports: &str) -> Option<String> {
    let mut lines = ports.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("Hardware Port: Wi-Fi") {
            return lines.next()?.strip_prefix("Device: ").map(str::to_string);
        }
    }
    None
}

fn bluetooth_count(profile: &Value, include_offline: bool) -> u64 {
    let root = &profile["SPBluetoothDataType"][0];
    let size = |key: &str| root[key].as_array().map_or(0, Vec::len) as u64;
    size("device_connected")
        + if include_offline {
            size("device_not_connected")
        } else {
            0
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETSTAT: &str = "\
Name       Mtu   Network       Address            Ipkts Ierrs     Ibytes    Opkts Oerrs     Obytes  Coll
lo0        16384 <Link#1>                      16708129     0 9091737923 16708129     0 9091737923     0
en4        1500  <Link#7>    ee:da:f5:75:6a:a5        0     0          0        0     0          0     0
en0        1500  <Link#14>   62:8b:12:80:b6:c1 68121251     0 66921412061 22408593     0 8475229714     0
en0        1500  fe80::1%en0 fe80:e::1         68121251     - 66921412061 22408593     - 8475229714     -
en7*       1500  <Link#20>   62:8b:12:80:b6:c2      100     0       5000      100     0        700     0
utun1      1380  <Link#19>                            0     0     123456       28     0       2280     0
awdl0      1500  <Link#16>   42:1f:aa:09:a9:67      128     0      49393      167     0      56836     0
";

    #[test]
    fn only_physical_link_rows_count() {
        assert_eq!(
            physical_traffic(NETSTAT),
            (66921412061 + 5000, 8475229714 + 700)
        );
    }

    #[test]
    fn nettop_groups_by_process_name() {
        let csv = ",bytes_in,bytes_out,\nkernel_task.0,615352,407404,\nSafari.1.5,10,10,\nSafari.1.6,5,5,\nidle.3,0,0,\n";
        let full = HashMap::from([("7".to_string(), "qemu-system-aarch64".to_string())]);
        assert_eq!(
            top_processes("x,\nqemu-system-aar.7,5,5,\n", 1, &full)[0].name,
            "qemu-system-aarch64"
        );
        let top = top_processes(csv, 2, &HashMap::new());
        assert_eq!(
            top[0],
            NamedAmount {
                name: "kernel_task".into(),
                amount: 1022756
            }
        );
        assert_eq!(top[1].name, "Safari.1");
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn wifi_device_is_found_by_port_name() {
        let ports = "Hardware Port: Ethernet\nDevice: en4\nEthernet Address: x\n\nHardware Port: Wi-Fi\nDevice: en0\nEthernet Address: y\n";
        assert_eq!(wifi_device(ports).as_deref(), Some("en0"));
        assert_eq!(wifi_device("nothing"), None);
    }

    #[test]
    fn bluetooth_counts_online_and_all() {
        let json = serde_json::json!({"SPBluetoothDataType":[{"device_connected":[{"a":{}}, {"b":{}}],"device_not_connected":[{"c":{}}]}]});
        assert_eq!(bluetooth_count(&json, true), 3);
        assert_eq!(bluetooth_count(&json, false), 2);
    }
}
