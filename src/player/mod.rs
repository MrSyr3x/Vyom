pub mod controller;
pub mod mpd;
pub mod traits;

// Re-export common types
pub use traits::{PlayerState, PlayerTrait, QueueItem, RepeatMode, TrackInfo};

// Re-export specific players if needed, but mainly we use get_player()
pub use controller::get_player;

#[cfg(feature = "mpd")]
pub use self::mpd::MpdPlayer;

use crate::app::cli::Args;
use crate::app::config::UserConfig;
use std::sync::Arc;

pub struct PlayerFactory;

impl PlayerFactory {
    pub fn create(args: &Args, user_config: &UserConfig) -> Arc<dyn PlayerTrait> {
        #[cfg(feature = "mpd")]
        {
            if !args.controller {
                return Arc::new(MpdPlayer::new(
                    args.mpd_host.clone(),
                    args.mpd_port,
                    user_config.music_directory.clone(),
                ));
            }
        }

        // Default or Fallback: Apple Music Native Controller
        Arc::from(get_player())
    }
}
