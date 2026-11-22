//! What the recovery has done so far, kept where a phone that stops before
//! its screen comes up can still be asked about it: the recovery partition
//! of the slot the phone did not start from, which is empty and which
//! flashing a working recovery into the booted slot leaves alone. The room
//! after the image in the booted slot is written as well.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Where the room starts in a booted slot's partition, past any image this
/// build makes.
const AT: u64 = 64 << 20;
/// How much room one marker takes, and how many there are.
const SLOT: u64 = 128;
const SLOTS: usize = 512;

static NEXT: AtomicUsize = AtomicUsize::new(0);
static DEVICES: OnceLock<Vec<(PathBuf, u64)>> = OnceLock::new();

fn suffix() -> String {
    std::fs::read_to_string("/proc/cmdline")
        .ok()
        .and_then(|line| {
            line.split_ascii_whitespace()
                .find_map(|w| w.strip_prefix("androidboot.slot_suffix=").map(str::to_string))
        })
        .unwrap_or_default()
}

/// The partitions the markers go to, and where in each: the other slot's
/// recovery partition from its start, and the room after the image in the
/// one the phone started from.
fn devices() -> &'static [(PathBuf, u64)] {
    DEVICES.get_or_init(|| {
        let by_name = Path::new("/dev/block/by-name");
        let mut out = Vec::new();
        let suffix = suffix();
        let other = match suffix.as_str() {
            "_a" => "_b".to_string(),
            "_b" => "_a".to_string(),
            _ => String::new(),
        };
        for (name, at) in [
            (format!("recovery{other}"), 0),
            (format!("recovery{suffix}"), AT),
            ("recovery".to_string(), AT),
        ] {
            let path = by_name.join(name);
            if path.exists() {
                out.push((path, at));
            }
        }
        out
    })
}

/// Keep `line` as one more marker, the oldest written over once the room
/// runs out.
pub fn mark(line: &str) {
    #[cfg(target_os = "android")]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::FileExt;
        // The kernel log takes it as well, for whoever has adb on the phone.
        if let Ok(mut kmsg) = OpenOptions::new().write(true).open("/dev/kmsg") {
            let _ = writeln!(kmsg, "skyward-trace: {line}");
        }
        let at = NEXT.fetch_add(1, Ordering::Relaxed) % SLOTS;
        for (device, base) in devices() {
            let Ok(file) = OpenOptions::new().write(true).open(device) else { continue };
            let mut bytes = format!("{line}\n").into_bytes();
            bytes.truncate(SLOT as usize - 1);
            let _ = file.write_all_at(&bytes, base + at as u64 * SLOT);
        }
    }
    #[cfg(not(target_os = "android"))]
    let _ = line;
}
