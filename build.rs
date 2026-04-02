use std::env;

fn main() {
    // On macOS, add Homebrew's lib path so the linker can find ffmpeg/mpv
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("brew").arg("--prefix").output() {
            if let Ok(prefix) = String::from_utf8(output.stdout) {
                println!("cargo:rustc-link-search={}/lib", prefix.trim());
            }
        }
    }

    let user_mpv = env::var("CARGO_FEATURE_MPV").is_ok();
    let user_rodio_audio = env::var("CARGO_FEATURE_RODIO_AUDIO").is_ok();

    if user_mpv && user_rodio_audio {
        eprintln!("Error: At most one of the following features can be enabled at a time: mpv, rodio_audio.");
        std::process::exit(1);
    }

    if user_mpv {
        println!("cargo:rustc-cfg=feature=\"mpv\"");
    } else if user_rodio_audio {
        println!("cargo:rustc-cfg=feature=\"rodio_audio\"");
    }
}
