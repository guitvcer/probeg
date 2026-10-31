//! Disk usage and the amount read and written since boot.

use plist::Value;

use crate::model::Disk;
use crate::sys;

const DATA_VOLUME: &str = "/System/Volumes/Data";

pub fn disk() -> Option<Disk> {
    let volume = sys::volume(DATA_VOLUME).or_else(|| sys::volume("/"))?;
    let (read, written) = sys::run_bytes("ioreg", &["-r", "-c", "IOBlockStorageDriver", "-a"])
        .and_then(|raw| plist::from_bytes::<Value>(&raw).ok())
        .map(|root| busiest_drive(&root))
        .unwrap_or_default();
    Some(Disk {
        total_bytes: volume.total,
        free_bytes: volume.free,
        file_count: volume.files,
        read_since_boot: read,
        written_since_boot: written,
    })
}

/// There are several drivers and nearly all count zeros. The real disk is the one that has read the most.
fn busiest_drive(node: &Value) -> (Option<u64>, Option<u64>) {
    let mut best = (None, None);
    visit(node, &mut best);
    best
}

fn visit(node: &Value, best: &mut (Option<u64>, Option<u64>)) {
    match node {
        Value::Array(items) => items.iter().for_each(|item| visit(item, best)),
        Value::Dictionary(dict) => {
            if let Some(stats) = dict.get("Statistics").and_then(Value::as_dictionary) {
                let number = |key: &str| stats.get(key).and_then(Value::as_unsigned_integer);
                let read = number("Bytes (Read)");
                if read.unwrap_or(0) >= best.0.unwrap_or(0) {
                    *best = (read, number("Bytes (Write)"));
                }
            }
            dict.values().for_each(|value| visit(value, best));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_drive_with_most_reads_wins() {
        let xml = br#"<?xml version="1.0"?><plist version="1.0"><array><dict>
            <key>Statistics</key><dict><key>Bytes (Read)</key><integer>0</integer><key>Bytes (Write)</key><integer>0</integer></dict>
            <key>IORegistryEntryChildren</key><array><dict>
              <key>Statistics</key><dict><key>Bytes (Read)</key><integer>4512</integer><key>Bytes (Write)</key><integer>1960</integer></dict>
            </dict></array>
        </dict></array></plist>"#;
        let root: Value = plist::from_bytes(xml).unwrap();
        assert_eq!(busiest_drive(&root), (Some(4512), Some(1960)));
    }
}
