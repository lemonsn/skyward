//! What the recovery has done so far, kept where a phone that stops before
//! its screen comes up can still be asked about it: the room after the image
//! in either recovery partition, which the bootloader has already read, which
//! the image's AVB hash does not cover, and which flashing a working
//! recovery back into the booted slot leaves alone in the other slot.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Where the room starts, past any image this build makes.
const AT: u64 = 64 << 20;
/// How much room one marker takes, and how many there are.
const SLOT: u64 = 128;
const SLOTS: usize = 512;

static NEXT: AtomicUsize = AtomicUsize::new(0);
static DEVICES: OnceLock<Vec<PathBuf>> = OnceLock::new();

/// The recovery partitions the markers go to, as the phone names them.
fn devices() -> &'static [PathBuf] {
    DEVICES.get_or_init(|| {
        let by_name = PathBuf::from("/dev/block/by-name");
        ["recovery_a", "recovery_b"]
            .iter()
            .map(|n| by_name.join(n))
            .filter(|p| p.exists())
            .collect()
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
        let at = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut bytes = format!("{line}\n").into_bytes();
        bytes.truncate(SLOT as usize - 1);
        for device in devices() {
            let Ok(file) = OpenOptions::new().write(true).open(device) else { continue };
            // What an earlier run left is cleared, so no line outlives it.
            if at == 0 {
                let blank = vec![0u8; SLOTS * SLOT as usize];
                let _ = file.write_all_at(&blank, AT);
            }
            let _ = file.write_all_at(&bytes, AT + (at % SLOTS) as u64 * SLOT);
        }
    }
    #[cfg(not(target_os = "android"))]
    let _ = line;
}
