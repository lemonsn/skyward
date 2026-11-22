//! What the recovery has done so far, kept where a phone that stops before
//! its screen comes up can still be asked about it: the room after the image
//! in the recovery partition, which the bootloader has already read and which
//! the image's AVB hash does not cover.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Where the room starts, past any image this build makes.
const AT: u64 = 64 << 20;
/// How much room one marker takes, and how many there are.
const SLOT: u64 = 128;
const SLOTS: usize = 512;

static NEXT: AtomicUsize = AtomicUsize::new(0);
static DEVICE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The recovery partition the phone started from: an A/B phone names it
/// with the slot's suffix and has no name without one.
fn device() -> Option<&'static Path> {
    DEVICE
        .get_or_init(|| {
            let by_name = Path::new("/dev/block/by-name");
            let suffix = std::fs::read_to_string("/proc/cmdline")
                .ok()
                .and_then(|line| {
                    line.split_ascii_whitespace()
                        .find_map(|w| w.strip_prefix("androidboot.slot_suffix=").map(str::to_string))
                })
                .unwrap_or_default();
            let mut names = vec![format!("recovery{suffix}"), "recovery_a".to_string(), "recovery".to_string()];
            names.dedup();
            names.into_iter().map(|n| by_name.join(n)).find(|p| p.exists())
        })
        .as_deref()
}

/// Keep `line` as one more marker, the oldest written over once the room
/// runs out.
pub fn mark(line: &str) {
    #[cfg(target_os = "android")]
    {
        use std::fs::OpenOptions;
        use std::os::unix::fs::FileExt;
        // The kernel log takes it as well, for whoever has adb on the phone.
        if let Ok(mut kmsg) = OpenOptions::new().write(true).open("/dev/kmsg") {
            use std::io::Write;
            let _ = writeln!(kmsg, "skyward-trace: {line}");
        }
        let Some(device) = device() else { return };
        let at = NEXT.fetch_add(1, Ordering::Relaxed) % SLOTS;
        let Ok(file) = OpenOptions::new().write(true).open(device) else { return };
        let mut bytes = format!("{line}\n").into_bytes();
        bytes.truncate(SLOT as usize - 1);
        let _ = file.write_all_at(&bytes, AT + at as u64 * SLOT);
    }
    #[cfg(not(target_os = "android"))]
    let _ = line;
}
