//! The machine's passport. The serial number is deliberately not read, so the report can be shared.

use serde_json::Value;

use crate::model::Identity;
use crate::sys;

pub fn identity() -> Identity {
    let mut identity = Identity {
        chip: sys::sysctl_string("machdep.cpu.brand_string"),
        model_id: sys::sysctl_string("hw.model"),
        cores_performance: sys::sysctl_u64("hw.perflevel0.physicalcpu").map(|n| n as u32),
        cores_efficiency: sys::sysctl_u64("hw.perflevel1.physicalcpu").map(|n| n as u32),
        memory_bytes: sys::sysctl_u64("hw.memsize"),
        computer_name: line("scutil", &["--get", "ComputerName"]),
        macos_version: line("sw_vers", &["-productVersion"]),
        macos_build: line("sw_vers", &["-buildVersion"]),
        ..Identity::default()
    };
    let profile = sys::run(
        "system_profiler",
        &["SPHardwareDataType", "SPNVMeDataType", "-json"],
    )
    .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if let Some(profile) = profile {
        apply_profile(&mut identity, &profile);
    }
    identity
}

fn line(program: &str, args: &[&str]) -> Option<String> {
    let out = sys::run(program, args)?;
    let out = out.trim();
    (!out.is_empty()).then(|| out.to_string())
}

fn apply_profile(identity: &mut Identity, profile: &Value) {
    let hardware = &profile["SPHardwareDataType"][0];
    identity.model = hardware["machine_name"].as_str().map(str::to_string);
    if let Some(chip) = hardware["chip_type"].as_str() {
        identity.chip = Some(chip.to_string());
    }
    let drive = &profile["SPNVMeDataType"][0]["_items"][0];
    identity.ssd_model = drive["device_model"].as_str().map(|s| s.trim().to_string());
    identity.ssd_bytes = drive["size_in_bytes"].as_u64();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_fills_model_and_drive() {
        let profile: Value = serde_json::from_str(
            r#"{"SPHardwareDataType":[{"machine_name":"MacBook Pro","chip_type":"Apple M4"}],
                "SPNVMeDataType":[{"_items":[{"device_model":"APPLE SSD AP0512Z ","size_in_bytes":500277792768}]}]}"#,
        )
        .unwrap();
        let mut identity = Identity::default();
        apply_profile(&mut identity, &profile);
        assert_eq!(identity.model.as_deref(), Some("MacBook Pro"));
        assert_eq!(identity.chip.as_deref(), Some("Apple M4"));
        assert_eq!(identity.ssd_model.as_deref(), Some("APPLE SSD AP0512Z"));
        assert_eq!(identity.ssd_bytes, Some(500277792768));
    }

    #[test]
    fn missing_profile_fields_are_tolerated() {
        let mut identity = Identity::default();
        apply_profile(&mut identity, &serde_json::json!({}));
        assert_eq!(identity.model, None);
    }
}
