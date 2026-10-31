//! Thin wrappers over system calls. All `unsafe` except the drive code lives here.

use std::ffi::{CStr, CString};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Runs a system utility. A hung utility must not hang the report.
pub fn run(program: &str, args: &[&str]) -> Option<String> {
    run_bytes(program, args).map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn run_bytes(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    const LIMIT: Duration = Duration::from_secs(15);
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        stdout.read_to_end(&mut buffer).ok().map(|_| buffer)
    });
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => return reader.join().ok()?,
            Some(_) => return None,
            None if started.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

pub fn sysctl_string(name: &str) -> Option<String> {
    let key = CString::new(name).ok()?;
    let mut size = 0usize;
    // SAFETY: the first call only gets the size, the second writes to a buffer of that size.
    unsafe {
        if libc::sysctlbyname(
            key.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
        let mut buffer = vec![0u8; size];
        if libc::sysctlbyname(
            key.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
        buffer.truncate(size);
        let text = CStr::from_bytes_until_nul(&buffer).ok()?;
        Some(text.to_string_lossy().into_owned())
    }
}

pub fn sysctl_u64(name: &str) -> Option<u64> {
    let key = CString::new(name).ok()?;
    let mut raw = [0u8; 8];
    let mut size = raw.len();
    // SAFETY: the buffer is 8 bytes; the kernel writes at most `size` and reports how much it wrote.
    let code = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            raw.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if code != 0 {
        return None;
    }
    match size {
        4 => Some(u32::from_ne_bytes(raw[..4].try_into().ok()?) as u64),
        8 => Some(u64::from_ne_bytes(raw)),
        _ => None,
    }
}

/// Values like `kern.boottime`, in Unix seconds.
pub fn sysctl_time(name: &str) -> Option<i64> {
    let key = CString::new(name).ok()?;
    let mut value = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut size = std::mem::size_of::<libc::timeval>();
    // SAFETY: `value` is a timeval and its size is passed in `size`.
    let code = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            (&raw mut value).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (code == 0 && value.tv_sec > 0).then_some(value.tv_sec)
}

fn clock_secs(id: libc::clockid_t) -> Option<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid pointer to a timespec.
    let code = unsafe { libc::clock_gettime(id, &mut time) };
    (code == 0).then_some(time.tv_sec as u64)
}

/// Seconds since boot, excluding sleep.
pub fn awake_secs() -> Option<u64> {
    clock_secs(libc::CLOCK_UPTIME_RAW)
}

pub fn terminal_width() -> Option<usize> {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    // SAFETY: TIOCGWINSZ fills in the winsize passed to it.
    let code = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) };
    (code == 0 && size.ws_col > 0).then_some(size.ws_col as usize)
}

pub fn stdout_is_tty() -> bool {
    // SAFETY: isatty only reads the descriptor's state.
    unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
}

pub struct VolumeStats {
    pub total: u64,
    pub free: u64,
    pub files: u64,
}

pub fn volume(path: &str) -> Option<VolumeStats> {
    let path = CString::new(path).ok()?;
    let mut stats: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `path` is a C string and `stats` is filled in by the kernel.
    if unsafe { libc::statfs(path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let block = stats.f_bsize as u64;
    Some(VolumeStats {
        total: stats.f_blocks * block,
        free: stats.f_bavail * block,
        files: stats.f_files.saturating_sub(stats.f_ffree),
    })
}

pub struct Session {
    pub user: String,
    pub line: String,
    pub since: i64,
}

/// Current logins.
pub fn sessions() -> Vec<Session> {
    let mut out = Vec::new();
    // SAFETY: getutxent returns a pointer to a static record; we copy it
    // before the next call and finish iterating with endutxent.
    unsafe {
        libc::setutxent();
        loop {
            let entry = libc::getutxent();
            if entry.is_null() {
                break;
            }
            let entry = &*entry;
            if entry.ut_type == libc::USER_PROCESS {
                out.push(Session {
                    user: c_chars(&entry.ut_user),
                    line: c_chars(&entry.ut_line),
                    since: entry.ut_tv.tv_sec,
                });
            }
        }
        libc::endutxent();
    }
    out
}

fn c_chars(raw: &[libc::c_char]) -> String {
    let bytes: Vec<u8> = raw
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn uid() -> u32 {
    // SAFETY: getuid has no preconditions.
    unsafe { libc::getuid() }
}
