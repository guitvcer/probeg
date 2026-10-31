//! Lifetime counters of the NVMe drive. macOS exposes them without admin rights through
//! the `NVMeSMARTLib` plugin, which has a COM-style C interface. This is the only `unsafe`
//! outside `sys`, because the plugin's method table has to be described by hand.

use std::ffi::{c_char, c_void};
use std::ptr;

use crate::model::Ssd;

/// In NVMe, a "data unit" is a thousand 512-byte sectors.
const DATA_UNIT: u64 = 512_000;

#[repr(C)]
#[derive(Clone, Copy)]
struct Uuid([u8; 16]);

type Allocator = *const c_void;
type CfUuid = *const c_void;
type Service = u32;

#[repr(C)]
struct PluginVtable {
    reserved: *const c_void,
    query_interface: unsafe extern "C" fn(*mut c_void, Uuid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    release: unsafe extern "C" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct SmartVtable {
    base: PluginVtable,
    version: u16,
    revision: u16,
    read: unsafe extern "C" fn(*mut c_void, *mut u8) -> i32,
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *const c_void;
    fn IOServiceGetMatchingService(port: u32, matching: *const c_void) -> Service;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOCreatePlugInInterfaceForService(
        service: Service,
        plugin_type: CfUuid,
        interface_type: CfUuid,
        plugin: *mut *mut *const PluginVtable,
        score: *mut i32,
    ) -> i32;
    fn IODestroyPlugInInterface(plugin: *mut *const PluginVtable) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFUUIDCreateFromUUIDBytes(allocator: Allocator, bytes: Uuid) -> CfUuid;
    fn CFRelease(object: *const c_void);
}

const SMART_CLIENT: Uuid = Uuid([
    0xAA, 0x0F, 0xA6, 0xF9, 0xC2, 0xD6, 0x45, 0x7F, 0xB1, 0x0B, 0x59, 0xA1, 0x32, 0x53, 0x29, 0x2F,
]);
const SMART_INTERFACE: Uuid = Uuid([
    0xCC, 0xD1, 0xDB, 0x19, 0xFD, 0x9A, 0x4D, 0xAF, 0xBF, 0x95, 0x12, 0x45, 0x4B, 0x23, 0x0A, 0xB6,
]);
const PLUGIN_INTERFACE: Uuid = Uuid([
    0xC2, 0x44, 0xE8, 0x58, 0x10, 0x9C, 0x11, 0xD4, 0x91, 0xD4, 0x00, 0x50, 0xE4, 0xC6, 0x42, 0x6F,
]);

pub fn read() -> Option<Ssd> {
    let log = read_log()?;
    parse_log(&log)
}

fn read_log() -> Option<[u8; 512]> {
    let mut log = [0u8; 512];
    // SAFETY: the call sequence follows the one documented in
    // NVMeSMARTLibExternal.h; every pointer is checked and objects are released.
    unsafe {
        let matching = IOServiceMatching(c"IONVMeBlockStorageDevice".as_ptr());
        if matching.is_null() {
            return None;
        }
        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return None;
        }
        let client = CFUUIDCreateFromUUIDBytes(ptr::null(), SMART_CLIENT);
        let plugin_id = CFUUIDCreateFromUUIDBytes(ptr::null(), PLUGIN_INTERFACE);
        let mut plugin: *mut *const PluginVtable = ptr::null_mut();
        let mut score = 0;
        let created =
            IOCreatePlugInInterfaceForService(service, client, plugin_id, &mut plugin, &mut score);
        CFRelease(client);
        CFRelease(plugin_id);
        IOObjectRelease(service);
        if created != 0 || plugin.is_null() {
            return None;
        }

        let mut smart: *mut *const SmartVtable = ptr::null_mut();
        let query =
            ((**plugin).query_interface)(plugin.cast(), SMART_INTERFACE, (&raw mut smart).cast());
        let ok = if query == 0 && !smart.is_null() {
            let code = ((**smart).read)(smart.cast(), log.as_mut_ptr());
            ((**smart).base.release)(smart.cast());
            code == 0
        } else {
            false
        };
        IODestroyPlugInInterface(plugin);
        ok.then_some(log)
    }
}

fn le_u64(log: &[u8], at: usize) -> u64 {
    // Log counters are 128-bit, but the upper half won't fill up in the drive's lifetime.
    let raw: [u8; 8] = log[at..at + 8].try_into().unwrap_or_default();
    u64::from_le_bytes(raw)
}

/// Parses the NVMe health log (log page 02h).
pub fn parse_log(log: &[u8]) -> Option<Ssd> {
    if log.len() < 512 {
        return None;
    }
    let kelvin = u16::from_le_bytes([log[1], log[2]]) as i32;
    Some(Ssd {
        temperature_c: kelvin - 273,
        spare_percent: log[3],
        wear_percent: log[5],
        bytes_read: le_u64(log, 32).saturating_mul(DATA_UNIT),
        bytes_written: le_u64(log, 48).saturating_mul(DATA_UNIT),
        power_cycles: le_u64(log, 112),
        power_on_hours: le_u64(log, 128),
        unsafe_shutdowns: le_u64(log, 144),
        media_errors: le_u64(log, 160),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_log_offsets_follow_the_nvme_spec() {
        let mut log = [0u8; 512];
        log[1..3].copy_from_slice(&316u16.to_le_bytes());
        log[3] = 100;
        log[5] = 2;
        log[32..40].copy_from_slice(&200_395_703u64.to_le_bytes());
        log[48..56].copy_from_slice(&91_994_467u64.to_le_bytes());
        log[112..120].copy_from_slice(&262u64.to_le_bytes());
        log[128..136].copy_from_slice(&1351u64.to_le_bytes());
        log[144..152].copy_from_slice(&7u64.to_le_bytes());
        let ssd = parse_log(&log).unwrap();
        assert_eq!(ssd.temperature_c, 43);
        assert_eq!(ssd.spare_percent, 100);
        assert_eq!(ssd.wear_percent, 2);
        assert_eq!(ssd.bytes_written, 91_994_467 * 512_000);
        assert_eq!(ssd.bytes_read, 200_395_703 * 512_000);
        assert_eq!(ssd.power_cycles, 262);
        assert_eq!(ssd.power_on_hours, 1351);
        assert_eq!(ssd.unsafe_shutdowns, 7);
        assert_eq!(ssd.media_errors, 0);
    }

    #[test]
    fn short_log_is_rejected() {
        assert_eq!(parse_log(&[0u8; 16]), None);
    }
}
