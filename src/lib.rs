pub mod app;
pub mod artwork;
pub mod audio;
pub mod inputs;
// pub mod audio_device; // Deprecated
// pub mod audio_pipeline; // Deprecated
// pub mod config; // Deprecated
// pub mod dsp_eq; // Deprecated
// pub mod lyrics; // Deprecated
pub mod player;
// pub mod theme; // Deprecated
pub mod ui;
// pub mod visualizer; // Deprecated

#[cfg(feature = "mpd")]
pub mod mpd_player;
