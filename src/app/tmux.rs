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
        // Auto-split logic (Tmux)
        let mut cmd = std::process::Command::new("tmux");
        cmd.arg("split-window")
           .arg("-h")
           .arg("-p")
           .arg("22")
           .arg(exe_path)
           .arg("--standalone");

        // Pass controller flag if present
        if args.controller {
            cmd.arg("--controller");
        } else {
            // Default is MPD, pass args if needed
            #[cfg(feature = "mpd")]
            {
                cmd.arg("--mpd-host").arg(&args.mpd_host);
                cmd.arg("--mpd-port").arg(args.mpd_port.to_string());
            }
        }

        let status = cmd.status();

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
