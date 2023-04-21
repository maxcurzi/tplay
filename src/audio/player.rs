use mpv::MpvHandler;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

pub struct AudioPlayer {
    mpv: MpvHandler,
}

impl AudioPlayer {
    pub fn new(input_path: &str) -> Self {
        let mut mpv_builder = mpv::MpvHandlerBuilder::new().expect("Failed to init MPV builder");
        mpv_builder.try_hardware_decoding();
        let mut mpv = mpv_builder.build().expect("Failed to build MPV handler");
        mpv.set_property("vid", "no")
            .expect("Failed to set no-video property");
        mpv.command(&["loadfile", input_path])
            .expect("Failed to load audio file");
        mpv.set_property("pause", false)
            .expect("Failed to set pause property");
        Self { mpv }
    }

    pub fn pause(&mut self) {
        self.mpv
            .set_property("pause", true)
            .expect("Failed to pause");
    }

    pub fn resume(&mut self) {
        self.mpv
            .set_property("pause", false)
            .expect("Failed to resume");
    }

    pub fn toggle(&mut self) {
        let paused = self
            .mpv
            .get_property::<bool>("pause")
            .expect("Failed to get pause property");
        if paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        self.mpv.command(&["stop"]).expect("Failed to stop");
    }
}

// pub fn demo(
//     input_path: &str,
//     barrier: std::sync::Arc<std::sync::Barrier>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     // let audio_path = extract_audio(input_path)?;

//     // play_audio_process(audio_path.to_str().unwrap())?;
//     play_audio_process(input_path, barrier)?;
//     Ok(())
// }

// pub fn play_audio_process(
//     audio_path: &str,
//     barrier: std::sync::Arc<std::sync::Barrier>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut mpv_builder = mpv::MpvHandlerBuilder::new().expect("Failed to init MPV builder");
//     let mut mpv = mpv_builder.build().expect("Failed to build MPV handler");

//     mpv.set_property("vid", "no")
//         .expect("Failed to set no-video property");
//     mpv.command(&["loadfile", audio_path])
//         .expect("Failed to load audio file");
//     mpv.set_property("pause", false)
//         .expect("Failed to set pause property");

//     barrier.wait();
//     // Wait for the "end-file" event to know when the playback has finished
//     loop {
//         if let Some(event) = mpv.wait_event(0.1) {
//             match event {
//                 mpv::Event::EndFile(_) => break,
//                 _ => {}
//             }
//         }
//     }

//     Ok(())
// }

// pub fn play_audio(audio_path: &str) -> Result<MpvHandler, Box<dyn std::error::Error>> {
//     let mut mpv_builder = mpv::MpvHandlerBuilder::new().expect("Failed to init MPV builder");
//     // mpv_builder.try_hardware_decoding();

//     let mut mpv = mpv_builder.build().expect("Failed to build MPV handler");
//     mpv.set_property("pause", true)
//         .expect("Failed to set pause property");
//     mpv.set_property("no-video", true)
//         .expect("Failed to set no-video property");
//     mpv.command(&["loadfile", audio_path])
//         .expect("Failed to load audio file");

//     Ok(mpv)
// }
