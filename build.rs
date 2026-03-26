use std::env;

fn main() {
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
