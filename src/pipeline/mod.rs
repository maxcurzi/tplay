//! The `pipeline` module contains the necessary components for processing images and creating ASCII art animations.
//!
//! It consists of the following sub-modules:
//! - `char_maps`: Provides character lookup tables used for converting image pixels to ASCII characters.
//! - `frames`: Defines a `Frame` struct and related functionality for representing individual frames in an ASCII animation.
//! - `image_pipeline`: Contains a pipeline for processing images, resizing them, and converting them to ASCII art.
//! - `runner`: Implements the main functionality for running the ASCII animation, including frame rate control and output.
//! - `sound`: Contains functionality for playing audio tracks in the background while the animation is running.
pub mod char_maps;
pub mod frames;
pub mod image_pipeline;
pub mod runner;
pub mod sound;
