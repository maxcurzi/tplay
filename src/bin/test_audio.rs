use serde_json::Value;
use std::process::{Command, Stdio};

fn extract_audio(input_path: &str, output_path: &str) -> std::io::Result<()> {
    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-vn") // Disable video
        .arg("-acodec")
        .arg("mp3") // Use the mp3 codec
        .arg("-y") // Overwrite output file if it exists
        // .arg("copy") // Copy audio codec to output without re-encoding
        .arg(output_path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to extract audio track",
        ))
    }
}

fn extract_fps(video_path: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=r_frame_rate")
        .arg("-of")
        .arg("json")
        .arg(video_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let output_str = String::from_utf8(output.stdout)?;
    let json_value: Value = serde_json::from_str(&output_str)?;
    let r_frame_rate = json_value["streams"][0]["r_frame_rate"]
        .as_str()
        .ok_or("Failed to get r_frame_rate")?;

    let (num, denom) = {
        let mut iter = r_frame_rate.split('/');
        (
            iter.next()
                .ok_or("Invalid r_frame_rate format")?
                .parse::<f64>()?,
            iter.next()
                .ok_or("Invalid r_frame_rate format")?
                .parse::<f64>()?,
        )
    };

    let fps = num / denom;
    Ok(fps)
}

fn play_audio(audio_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("mpv")
        .arg("--no-video")
        .arg(audio_path)
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to play audio track",
        )));
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = "assets/eva.webm";

    let audio_path = "/tmp/audio_track.mp3";
    if let Ok(fps) = extract_fps(input_path) {
        println!("FPS: {}", fps);
    }
    // Extract audio track from media file
    extract_audio(input_path, audio_path)?;

    // Play the extracted audio track
    let t = std::thread::spawn(move || {
        if let Err(e) = play_audio(audio_path) {
            println!("Error: {}", e);
        }
    });
    // Add your OpenCV frame iteration code here
    t.join().unwrap();
    Ok(())
}
