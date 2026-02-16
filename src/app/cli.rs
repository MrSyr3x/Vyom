use clap::Parser;

/// Vyom - A beautiful music companion for your terminal 🎵
#[derive(Parser, Debug)]
#[command(name = "vyom", version, about)]
pub struct Args {
    /// Run inside tmux split (internal)
    #[arg(long)]
    pub standalone: bool,

    /// Start in mini player mode (defaults to full UI)
    #[arg(long, short = 'm')]
    pub mini: bool,

    /// Play MP3/FLAC music (Defaults to MPD Client mode)
    #[arg(long, short = 'c')]
    pub controller: bool,

    /// MPD host (default: localhost)
    #[cfg(feature = "mpd")]
    #[arg(long, default_value = "localhost")]
    pub mpd_host: String,

    /// MPD port (default: 6600)
    #[cfg(feature = "mpd")]
    #[arg(long, default_value_t = 6600)]
    pub mpd_port: u16,

    /// Generate default config.toml to stdout
    #[arg(long)]
    pub generate_config: bool,
}
