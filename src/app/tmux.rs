use anyhow::Result;
use crate::app::cli::Args;

pub fn handle_tmux_split(
    args: &Args,
    exe_path: &str,
    is_tmux: bool,
    is_standalone: bool,
    want_lyrics: bool,
) -> Result<bool> { // Returns true if split occurred and we should exit
    // Only auto-split if we WANT full UI (default) and aren't already the child process
    if is_tmux && !is_standalone && want_lyrics {
        // Build child command with all necessary flags
        let mut child_args = vec!["--standalone".to_string()];

        // Pass controller flag if present
        if args.controller {
            child_args.push("--controller".to_string());
        } else {
            // Default is MPD, pass args if needed
            #[cfg(feature = "mpd")]
            {
                child_args.push("--mpd-host".to_string());
                child_args.push(args.mpd_host.clone());
                child_args.push("--mpd-port".to_string());
                child_args.push(args.mpd_port.to_string());
            }
        }

        let child_cmd = format!("{} {}", exe_path, child_args.join(" "));

        // Auto-split logic (Tmux)
        let status = std::process::Command::new("tmux")
            .arg("split-window")
            .arg("-h")
            .arg("-p")
            .arg("22")
            .arg(child_cmd)
            .status();

        match status {
            Ok(_) => return Ok(true), // Split successful, parent should exit
            Err(e) => {
                eprintln!("Failed to create tmux split: {}", e);
                // Continue as single pane if split fails
            }
        }
    }
    Ok(false)
}
