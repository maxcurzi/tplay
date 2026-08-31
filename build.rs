use std::env;

fn main() {
    // On macOS, add Homebrew's lib paths so the linker can find ffmpeg/mpv.
    // Some formulae (mpv) don't symlink their libs into <brew prefix>/lib, so
    // also probe each keg's own lib directory.
    #[cfg(target_os = "macos")]
    {
        let brew_prefix = |args: &[&str]| -> Option<String> {
            let output = std::process::Command::new("brew")
                .arg("--prefix")
                .args(args)
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let prefix = String::from_utf8(output.stdout).ok()?;
            let prefix = prefix.trim().to_owned();
            if prefix.is_empty() { None } else { Some(prefix) }
        };

        for args in [&[][..], &["mpv"][..], &["ffmpeg"][..]] {
            if let Some(prefix) = brew_prefix(args) {
                println!("cargo:rustc-link-search={prefix}/lib");
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
