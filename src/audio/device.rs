//! Audio device information module
//!
//! Uses cpal to list audio output devices and SwitchAudioSource (macOS) to switch.

#[cfg(feature = "eq")]
use cpal::traits::{DeviceTrait, HostTrait};
use std::process::Command;

/// Audio device with name
#[derive(Clone, Debug)]
pub struct AudioDevice {
    pub name: String,
    #[allow(dead_code)]
    pub is_default: bool,
}

/// Get all available audio output devices
#[cfg(feature = "eq")]
pub fn get_output_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                devices.push(AudioDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }

    // Deduplicate by name (macOS sometimes lists duplicates)
    devices.dedup_by(|a, b| a.name == b.name);

    devices
}

/// Get the name of the default audio output device
#[cfg(feature = "eq")]
pub fn get_output_device_name() -> String {
    let host = cpal::default_host();

    match host.default_output_device() {
        Some(device) => device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string()),
        None => "No Output Device".to_string(),
    }
}

/// Switch to a specific audio output device (macOS)
/// Returns true if successful
#[cfg(target_os = "macos")]
pub fn switch_audio_device(device_name: &str) -> bool {
    // Try using SwitchAudioSource (must be installed via: brew install switchaudio-osx)
    let result = Command::new("SwitchAudioSource")
        .arg("-s")
        .arg(device_name)
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn switch_audio_device(_device_name: &str) -> bool {
    // Not supported on other platforms yet
    false
}

/// Get list of devices using SwitchAudioSource (for more reliable device names)
#[cfg(target_os = "macos")]
pub fn get_devices_from_system() -> Vec<String> {
    let result = Command::new("SwitchAudioSource")
        .arg("-a") // List all devices
        .arg("-t") // Type: output
        .arg("output")
        .output();

    match result {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_devices_from_system() -> Vec<String> {
    Vec::new()
}

/// Fallback when eq feature is not enabled
#[cfg(not(feature = "eq"))]
pub fn get_output_device_name() -> String {
    "Audio Device".to_string()
}

#[cfg(not(feature = "eq"))]
pub fn get_output_devices() -> Vec<AudioDevice> {
    vec![AudioDevice {
        name: "Audio Device".to_string(),
        is_default: true,
    }]
}
