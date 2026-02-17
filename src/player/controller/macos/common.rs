use anyhow::{Context, Result};
use std::process::Command;

/// Run a raw AppleScript command
pub fn run_script(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("Failed to execute AppleScript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("AppleScript error: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a macOS application is running via pgrep
pub fn is_app_running(app_name: &str) -> bool {
    let output = Command::new("pgrep").arg("-x").arg(app_name).output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}
