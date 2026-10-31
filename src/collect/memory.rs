//! Memory in the same categories that Activity Monitor shows.

use crate::model::Memory;
use crate::sys;

pub fn memory() -> Option<Memory> {
    let total = sys::sysctl_u64("hw.memsize")?;
    let stats = sys::run("vm_stat", &[])?;
    let swap = sys::run("sysctl", &["-n", "vm.swapusage"]).unwrap_or_default();
    parse(&stats, &swap, total)
}

pub fn parse(vm_stat: &str, swap: &str, total: u64) -> Option<Memory> {
    let page: u64 = vm_stat
        .lines()
        .next()?
        .split("page size of ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let pages = |key: &str| {
        vm_stat
            .lines()
            .find_map(|line| line.trim_start_matches('"').strip_prefix(key))
            .and_then(|rest| {
                rest.trim_start_matches(['"', ':'])
                    .trim()
                    .trim_end_matches('.')
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0)
    };
    let apps = pages("Anonymous pages").saturating_sub(pages("Pages purgeable")) * page;
    let wired = pages("Pages wired down") * page;
    let compressed = pages("Pages occupied by compressor") * page;
    let cached = (pages("File-backed pages") + pages("Pages purgeable")) * page;
    let free = total.saturating_sub(apps + wired + compressed + cached);
    let (swap_total, swap_used) = parse_swap(swap);
    Some(Memory {
        total,
        apps,
        wired,
        compressed,
        cached,
        free,
        swap_used,
        swap_total,
        compressions: pages("Compressions"),
        swapouts_bytes: pages("Swapouts") * page,
        top: Vec::new(),
    })
}

/// `total = 11264.00M  used = 10057.06M  free = 1206.94M  (encrypted)`.
fn parse_swap(text: &str) -> (u64, u64) {
    let value = |key: &str| {
        text.split(key)
            .nth(1)
            .and_then(|rest| {
                rest.trim_start_matches([' ', '='])
                    .split_whitespace()
                    .next()
            })
            .and_then(parse_size)
            .unwrap_or(0)
    };
    (value("total"), value("used"))
}

fn parse_size(text: &str) -> Option<u64> {
    let (number, unit) = text.split_at(text.len().checked_sub(1)?);
    let number: f64 = number.parse().ok()?;
    let scale = match unit {
        "K" => 1u64 << 10,
        "M" => 1 << 20,
        "G" => 1 << 30,
        _ => return None,
    };
    Some((number * scale as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VM_STAT: &str = r#"Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                     7844.
Pages wired down:                             204163.
Pages purgeable:                                3196.
"Translation faults":                    16958059211.
File-backed pages:                            217708.
Anonymous pages:                              378540.
Pages occupied by compressor:                 199849.
Compressions:                             3434498937.
Swapouts:                                   24714266.
"#;

    #[test]
    fn categories_add_up_to_total() {
        let total = 17_179_869_184;
        let m = parse(
            VM_STAT,
            "total = 11264.00M  used = 10057.06M  free = 1206.94M  (encrypted)",
            total,
        )
        .unwrap();
        assert_eq!(m.apps, (378540 - 3196) * 16384);
        assert_eq!(m.wired, 204163 * 16384);
        assert_eq!(m.compressed, 199849 * 16384);
        assert_eq!(m.cached, (217708 + 3196) * 16384);
        assert_eq!(m.apps + m.wired + m.compressed + m.cached + m.free, total);
        assert_eq!(m.swap_total, 11264 << 20);
        assert!((m.swap_used as f64 - 10057.06 * 1048576.0).abs() < 2.0);
        assert_eq!(m.compressions, 3434498937);
    }

    #[test]
    fn broken_input_gives_none_or_zero() {
        assert_eq!(parse("garbage", "", 1), None);
        assert_eq!(parse_swap("nonsense"), (0, 0));
        assert_eq!(parse_size("12X"), None);
        assert_eq!(parse_size("1.5G"), Some(3 << 29));
    }
}
