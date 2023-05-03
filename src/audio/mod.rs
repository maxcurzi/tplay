//! The `audio` module contains the necessary components for playing audio files.
//!
//! It consists of the following sub-modules:
//! - `mpv_player`: Defines an `MpvPlayer` struct and related functionality for playing audio files
//!   via the mpv player.
//! - `player`: Defines an `AudioPlayer` struct and related functionality for playing audio files,
//!   it also defines the trait AudioPlayerControls which an audio player backend should implement.
//! - `rodio_player`: Defines a `RodioPlayer` struct and related functionality for playing audio via
//!   the rodio crate.
//! - `runner`: Implements the main functionality for running the audio playback.
//! - `utils`: Contains utility functions for working with audio files.
#[cfg(not(feature = "rodio_audio"))]
pub mod mpv_player;
pub mod player;
#[cfg(feature = "rodio_audio")]
pub mod rodio_player;
pub mod runner;
pub mod utils;
