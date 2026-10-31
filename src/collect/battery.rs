//! Battery: the controller's registry entry plus macOS's own capacity estimate.

use plist::Value;

use crate::model::Battery;
use crate::sys;

pub fn battery() -> Option<Battery> {
    let raw = sys::run_bytes("ioreg", &["-r", "-n", "AppleSmartBattery", "-a"])?;
    let root: Value = plist::from_bytes(&raw).ok()?;
    let mut battery = from_registry(&root)?;
    if let Some(profile) = sys::run("system_profiler", &["SPPowerDataType", "-json"])
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&profile)
    {
        apply_health(&mut battery, &json);
    }
    Some(battery)
}

pub fn from_registry(root: &Value) -> Option<Battery> {
    let dict = root
        .as_array()
        .and_then(|items| items.first())
        .unwrap_or(root)
        .as_dictionary()?;
    let number = |key: &str| dict.get(key).and_then(Value::as_unsigned_integer);
    let flag = |key: &str| dict.get(key).and_then(Value::as_boolean).unwrap_or(false);
    let data = dict.get("BatteryData").and_then(Value::as_dictionary);
    let data_number = |key: &str| {
        data.and_then(|d| d.get(key))
            .and_then(Value::as_unsigned_integer)
    };
    Some(Battery {
        charge_percent: number("CurrentCapacity")?.min(100) as u8,
        charging: flag("IsCharging"),
        on_ac: flag("ExternalConnected"),
        cycles: number("CycleCount")?,
        design_cycles: number("DesignCycleCount9C"),
        max_capacity_percent: None,
        condition: None,
        design_mah: data_number("DesignCapacity"),
        full_mah: data_number("FullChargeCapacity"),
        adapter: dict
            .get("AdapterDetails")
            .and_then(Value::as_dictionary)
            .and_then(|d| d.get("Name"))
            .and_then(Value::as_string)
            .map(str::to_string),
    })
}

fn apply_health(battery: &mut Battery, profile: &serde_json::Value) {
    let Some(items) = profile["SPPowerDataType"].as_array() else {
        return;
    };
    for item in items {
        let health = &item["sppower_battery_health_info"];
        if let Some(text) = health["sppower_battery_health_maximum_capacity"].as_str() {
            battery.max_capacity_percent = text.trim_end_matches('%').parse().ok();
        }
        if let Some(text) = health["sppower_battery_health"].as_str() {
            battery.condition = Some(text.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_fields_map_to_battery() {
        let xml = br#"<?xml version="1.0"?><plist version="1.0"><array><dict>
          <key>CurrentCapacity</key><integer>87</integer>
          <key>CycleCount</key><integer>183</integer>
          <key>DesignCycleCount9C</key><integer>1000</integer>
          <key>ExternalConnected</key><true/>
          <key>IsCharging</key><false/>
          <key>BatteryData</key><dict><key>DesignCapacity</key><integer>6249</integer><key>FullChargeCapacity</key><integer>5810</integer></dict>
          <key>AdapterDetails</key><dict><key>Name</key><string>140W USB-C Power Adapter</string></dict>
        </dict></array></plist>"#;
        let root: Value = plist::from_bytes(xml).unwrap();
        let b = from_registry(&root).unwrap();
        assert_eq!(
            (b.charge_percent, b.cycles, b.design_cycles),
            (87, 183, Some(1000))
        );
        assert!(b.on_ac && !b.charging);
        assert_eq!((b.design_mah, b.full_mah), (Some(6249), Some(5810)));
        assert_eq!(b.adapter.as_deref(), Some("140W USB-C Power Adapter"));
    }

    #[test]
    fn desktop_without_battery_gives_none() {
        let empty: Value = plist::from_bytes(br#"<plist version="1.0"><dict/></plist>"#).unwrap();
        assert!(from_registry(&empty).is_none());
    }

    #[test]
    fn health_comes_from_system_profiler() {
        let mut b = Battery::default();
        let json = serde_json::json!({"SPPowerDataType":[{"sppower_battery_health_info":{"sppower_battery_health":"Good","sppower_battery_health_maximum_capacity":"97%"}},{"x":1}]});
        apply_health(&mut b, &json);
        assert_eq!(b.max_capacity_percent, Some(97));
        assert_eq!(b.condition.as_deref(), Some("Good"));
    }
}
