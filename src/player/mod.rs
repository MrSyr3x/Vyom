pub mod controller;
pub mod mpd;
pub mod traits;

pub use controller::{DummyPlayer, MacOsPlayer};
#[cfg(feature = "mpd")]
pub use mpd::MpdPlayer;
pub use traits::{PlayerState, PlayerTrait, TrackInfo, RepeatMode, QueueItem};

/// Factory to get the correct player for the current OS
pub fn get_player() -> Box<dyn PlayerTrait> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsPlayer)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Placeholder for Linux/Windows
        Box::new(DummyPlayer)
    }
}
