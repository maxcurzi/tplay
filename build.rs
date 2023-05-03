use std::env;
// use std::process::Command;

fn main() {
    let user_mpv_0_34 = env::var("CARGO_FEATURE_MPV_0_34").is_ok();
    let user_mpv_0_35 = env::var("CARGO_FEATURE_MPV_0_35").is_ok();
    let user_rodio_audio = env::var("CARGO_FEATURE_RODIO_AUDIO").is_ok();

    let enabled_count = [user_mpv_0_34, user_mpv_0_35, user_rodio_audio]
        .iter()
        .filter(|&&x| x)
        .count();

    if enabled_count > 1 {
        eprintln!("Error: At most one of the following features can be enabled at a time: mpv_0_34, mpv_0_35, rodio_audio.");
        std::process::exit(1);
    }

    if user_mpv_0_34 || user_mpv_0_35 || user_rodio_audio {
        if user_mpv_0_34 {
            println!("cargo:rustc-cfg=feature=\"mpv_0_34\"");
        } else if user_mpv_0_35 {
            println!("cargo:rustc-cfg=feature=\"mpv_0_35\"");
        } else {
            println!("cargo:rustc-cfg=feature=\"rodio_audio\"");
        }
    }
    // else {
    //     let output = Command::new("mpv").arg("--version").output();
    //     if let Ok(output) = output {
    //         let version_string = String::from_utf8_lossy(&output.stdout);
    //         let version = version_string
    //             .lines()
    //             .next()
    //             .unwrap_or("")
    //             .split_whitespace()
    //             .nth(1)
    //             .unwrap_or("");

    //         if version.starts_with("0.34") {
    //             println!("cargo:rustc-cfg=feature=\"mpv_0_34\"");
    //         } else if version.starts_with("0.35") {
    //             println!("cargo:rustc-cfg=feature=\"mpv_0_35\"");
    //         } else {
    //             // fallback to rodio
    //             println!("cargo:rustc-cfg=feature=\"rodio_audio\"");
    //         }
    //     } else {
    //         // fallback to rodio
    //         println!("cargo:rustc-cfg=feature=\"rodio_audio\"");
    //     }
    // }
}
