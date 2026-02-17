pub mod controller;
pub mod mpd;
pub mod traits;

// Re-export common types
pub use traits::{PlayerState, PlayerTrait, QueueItem, RepeatMode, TrackInfo};

// Re-export specific players if needed, but mainly we use get_player()
pub use controller::get_player;

#[cfg(feature = "mpd")]
pub use self::mpd::MpdPlayer;
