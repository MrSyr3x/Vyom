pub mod traits;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(target_os = "macos"))]
pub mod generic;

use crate::player::traits::PlayerTrait;

#[cfg(target_os = "macos")]
pub use macos::MacOsPlayer;

#[cfg(not(target_os = "macos"))]
pub use generic::DummyPlayer;

/// Factory to get the correct player for the current OS
pub fn get_player() -> Box<dyn PlayerTrait> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsPlayer::new()) // Updated to call new()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(DummyPlayer)
    }
}
