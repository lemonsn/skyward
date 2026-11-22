//! What the recovery has done so far, kept where a phone that stops before
//! its screen comes up can still be asked about it: the room after the image
//! in the recovery partition, which the bootloader has already read and which
//! the image's AVB hash does not cover.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Where the room starts, past any image this build makes.
const AT: u64 = 64 << 20;
/// How much room one marker takes, and how many there are.
const SLOT: u64 = 128;
const SLOTS: usize = 512;

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Keep `line` as one more marker, the oldest written over once the room
/// runs out.
pub fn mark(line: &str) {
    #[cfg(target_os = "android")]
    {
        use std::fs::OpenOptions;
        use std::os::unix::fs::FileExt;
        let at = NEXT.fetch_add(1, Ordering::Relaxed) % SLOTS;
        let Ok(file) = OpenOptions::new().write(true).open("/dev/block/by-name/recovery") else {
            return;
        };
        let mut bytes = format!("{line}\n").into_bytes();
        bytes.truncate(SLOT as usize - 1);
        let _ = file.write_all_at(&bytes, AT + at as u64 * SLOT);
    }
    #[cfg(not(target_os = "android"))]
    let _ = line;
}
