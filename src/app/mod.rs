pub mod config;
pub mod lyrics;
pub mod state;

pub mod cli;
pub mod library_helpers;
pub mod events;
pub mod keys;
pub mod inputs;
pub mod tmux;
pub mod lock;
pub use state::*;

#[cfg(feature = "mpd")]
pub fn with_mpd<F, R>(app: &mut App, args: &cli::Args, f: F) -> Option<R>
where
    F: FnOnce(&mut mpd::Client) -> R,
{
    let client = app.mpd_client.take().or_else(|| mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)).ok());
    
    if let Some(mut mpd) = client {
        if mpd.ping().is_err() {
             if let Ok(c) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) { mpd = c; }
        }
        
        let result = f(&mut mpd);
        app.mpd_client = Some(mpd);
        Some(result)
    } else {
        None
    }
}
