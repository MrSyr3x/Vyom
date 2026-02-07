use std::fs::File;
use std::io::{Read, Write};

const LOCK_FILE_PATH: &str = "/tmp/vyom_audio.lock";

/// Try to acquire the audio lock.
/// Returns Some(File) if we acquired the lock (and thus should play audio).
/// Returns None if another instance holds the lock (we should be UI-only).
pub fn try_acquire_audio_lock() -> Option<File> {
    // 1. Check if lock file exists
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(LOCK_FILE_PATH)
    {
        let mut pid_str = String::new();
        if file.read_to_string(&mut pid_str).is_ok() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                // 2. Check if process is alive
                unsafe {
                    // kill(pid, 0) checks existence without sending a signal
                    if libc::kill(pid, 0) == 0 {
                        // Process is alive! We are secondary.
                        return None;
                    }
                }
            }
        }
    }

    // 3. Create/Overwrite lock file
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(LOCK_FILE_PATH)
    {
        let pid = std::process::id();
        let _ = write!(file, "{}", pid);
        return Some(file);
    }


    None
}

pub fn release_audio_lock() {
    let _ = std::fs::remove_file(LOCK_FILE_PATH);
}

