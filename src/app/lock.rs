use fs2::FileExt;
use std::fs::File;
use std::io::Write;

const LOCK_FILE_PATH: &str = "/tmp/vyom_audio.lock";

pub fn try_acquire_audio_lock() -> Option<File> {
    // 1. Create/Open lock file
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(LOCK_FILE_PATH)
        .ok()?;

    // 2. Try to acquire exclusive lock 🔒
    // If this fails, another process holds the lock
    if file.try_lock_exclusive().is_ok() {
        // We got the lock!
        // Truncate file and write our PID (informational)
        if file.set_len(0).is_ok() {
             let pid = std::process::id();
             let _ = write!(file, "{}", pid);
        }
        
        // Return the file handle. The lock is released when the file is closed (dropped).
        return Some(file);
    }

    None
}

pub fn release_audio_lock() {
    let _ = std::fs::remove_file(LOCK_FILE_PATH);
}

