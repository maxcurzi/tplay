//! The `audio` module contains the necessary components for playing audio files.
//!
//! It consists of the following sub-modules:
//! - `mpv_player`: Defines an `MpvPlayer` struct and related functionality for playing audio files
//! - `player`: Defines an `AudioPlayer` struct and related functionality for playing audio files.
//! - `rodio_player`: Defines a `RodioPlayer` struct and related functionality for playing audio
//! - `runner`: Implements the main functionality for running the audio playback, including frame
//!   rate control and output.
//! - `utils`: Contains utility functions for working with audio files.
#[cfg(not(feature = "rodio_audio"))]
pub mod mpv_player;
pub mod player;
#[cfg(feature = "rodio_audio")]
pub mod rodio_player;
pub mod runner;
pub mod utils;
