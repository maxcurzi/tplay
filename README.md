<p align="center">
  <img src="https://user-images.githubusercontent.com/30084738/231727365-defc7606-59aa-48f5-b8c4-b7ec4664eac1.jpeg" alt="Image description" width="120" height="120">
</p>

# Terminal Media Player

[![Crates.io](https://img.shields.io/crates/v/tplay)](https://crates.io/crates/tplay)
[![Crates.io](https://img.shields.io/crates/d/tplay)](https://crates.io/crates/tplay)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Aur](https://img.shields.io/aur/version/tplay-git)](https://aur.archlinux.org/packages/tplay-git)

View images, videos (files or YouTube links), webcam, etc directly in the terminal as ASCII. All images you see [below](#features) are just made by characters on the terminal command line, drawn really fast.

<details>
  <summary><b>Table of Contents</b></summary>
  <p>

- [Terminal Media Player](#terminal-media-player)
- [Table of Contents](#table-of-contents)
  - [Who is it for?](#who-is-it-for)
  - [Features](#features)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
    - [Linux](#prerequisites-installation-on-linux)
    - [Windows](#prerequisites-installation-on-windows)
  - [Installation](#installation)
    - [For users](#for-users)
      - [Arch Linux](#arch-linux)
      - [NixOS](#nixos)
      - [Other Distros](#other-distros)
      - [Install Using Cargo](#install-using-cargo)
    - [For developers](#for-developers)
    - [Feature flags](#feature-flags)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)
- [Why](#why)
- [Credits](#credits)

  </p>
</details>

# Who is it for?
- You _really_ don't like graphical applications or work on a computer without graphical capabilities.
- You are looking for a quick way to convert visual media to ASCII art.
- You want to watch a video in the terminal, but you don't want to use `mpv` or `vlc` because they're too mainstream.
- You want to show off your terminal skills to your friends and make them think you're a hacker.

# Features
This crate is still in early development, but it already has a lot of features. Here's a list of what it can or can't do:
- [x] Converts and shows any media to ASCII art in the terminal
- [x] Supports images/gifs/videos/webcam and **YouTube** links
- [x] Any resolution, aspect ratio, and framerate
- [x] Use any character set as supported by your terminal
- [x] Handy pause/unpause and char map selection [controls](#playback-commands)
- [x] RGB Colors (on terminals that support RGB colors)
- [x] Play sounds
- [x] Spark joy
- [ ] Full media controls (forward, backwards, etc)
- [ ] Subtitles
- [ ] Replace a fully-fledged media player

## RGB Colors
![colors](https://user-images.githubusercontent.com/30084738/232452938-06de4ce6-343d-44de-85d9-5f0c99ab4f27.gif)
## Live update when updating character size
![font_size](https://user-images.githubusercontent.com/30084738/231709636-f764862a-d826-4a2e-b54d-6623d145ef41.gif)
## On-the-fly character map selection
![char_maps](https://user-images.githubusercontent.com/30084738/231709640-496b84ed-3807-4f62-b6b7-ebf9dcbb7bba.gif)
## Dynamic resize
![resize](https://user-images.githubusercontent.com/30084738/231709632-25af0fde-928e-46c2-bf42-f78a439e6594.gif)
## Emojis
![emojis](https://user-images.githubusercontent.com/30084738/231709625-084a496c-6557-4398-9361-0ba6ab41a02d.gif)
## Webcam support
![webcam](https://user-images.githubusercontent.com/30084738/231712280-d1fe42ae-f430-48f8-a561-83f5609357ee.gif)


# Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

# Prerequisites
Being a Rust crate, you will need to have Rust installed on your system. You can find the installation instructions [here](https://www.rust-lang.org/tools/install).

The following dependencies are also required:
- [OpenCV 4](https://github.com/twistedfall/opencv-rust#getting-opencv) Tested with OpenCV 4. It may work with OpenCV 3.4 and above.
- [LLVM](https://github.com/llvm/llvm-project/releases/tag/llvmorg-16.0.0)
- [ffmpeg](https://ffmpeg.org/download.html) Currently supported FFmpeg 6.1
- Optional dependency for YouTube playback support: [yt-dlp](https://github.com/yt-dlp/yt-dlp/wiki/installation)
- Optional dependency for audio playback via MPV: [MPV](https://mpv.io/installation/)

They can be simply installed on Linux with your package manager. See [below](#prerequisites-installation-on-linux) for more details.

## Prerequisites Installation on Ubuntu Linux
If you're on Linux (Ubuntu), you can install all dependencies with your package manager. First install Rust:

```bash
sudo apt install curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install `tplay`'s prerequisite dependencies:

```bash
sudo apt install libssl-dev libopencv-dev libstdc++-12-dev clang libclang-dev ffmpeg libavfilter-dev libavdevice-dev libasound2-dev yt-dlp
```

## Prerequisites installation on Windows
The crate can run on Windows and all prerequisites (opencv, ffmpeg) can be installed with vcpkg. However, the installation/setup process is lengthy and prone to errors. Performance is also very poor. Save yourself a headache: use WSL and follow the [Linux instructions](#prerequisites-installation-on-linux).

# Installation

## For users
### Arch Linux

You can install it on Arch Linux using [aur](https://aur.archlinux.org/packages/tplay-git) by running the following commands (using [paru](https://github.com/Morganamilo/paru)):

``` bash
paru -S tplay-git
```
### NixOS

https://search.nixos.org/packages?channel=24.05&show=tplay&from=0&size=50&sort=relevance&type=packages&query=tplay

### Other Distros

With your contribution and support it can be made available on other distros as well :).

### Install Using Cargo

You can install the `tplay` command line tool by running the following command:

```bash
# Install the tplay command line tool
cargo install tplay
```
So that you can run it from anywhere as
```bash
tplay <media> [options]
```

## For developers

```bash
# Clone the repository
git clone https://github.com/maxcurzi/tplay.git

# Change to the project directory
cd tplay

# (optional) Build the project
cargo build

# (optional) Run the tests
cargo test

# Run the project (use --release for faster performance)
cargo run --release -- <media> [options]
```

## Feature flags
By default, the crate uses [rodio](https://crates.io/crates/rodio) for audio playback. If you wish to use MPV (libmpv1 libmpv1-dev) as an audio playback backend, you can build/install the crate with:

`--features="mpv_0_35" --no-default-features`

or

`--features="mpv_0_34" --no-default-features`

within `cargo build`, `cargo run`, or `cargo install` commands.

MPV support may be dropped in future releases.

# Usage
`tplay <media> [options]`

| Argument | Description |
|--------|-------------|
| `media` | Name of the file or stream to be processed (required). |
| `-f`, `--fps` | Forces a specific frame rate (--fps 23.976). |
| `-c`, `--char-map` | Custom lookup character table to use for the output (default: ` .:-=+*#%@`). |
| `-g`, `--gray` | Start in grayscale mode |
| `-w`, `--w-mod` | Experimental width modifier for certain characters such as emojis (default: 1). Use a value of 2 if your char_map is composed of emojis. |
| `-a`, `--allow-frame-skip` | Experimental frame skip flag. Try to use it if the playback is too slow. |
| `-n`, `--new-lines` | Experimental flag. Adds newline and carriage return `\n\r` at the end of each line (except the last). Terminals wrap around and don't need new lines, but if you want to copy-paste the text outside the terminal you may want them. The output would be a single long string otherwise. Uses more CPU. |
| `-l`, `--loop-playback` | Loop video/gif forever (default: do not loop - play once) |
| `-b`, `--browser` | It's used when downloading videos from YouTube, maps to yt-dlp `cookies-from-browser` to prove YouTube you're not a robot. Defaults to "firefox". Supported browsers are: brave, chrome, chromium, edge, firefox, opera, safari, vivaldi, whale |

Substitute `tplay` with `cargo run --release --` if you plan to run from source.

```bash
# Run it
tplay <media> [options]

# Example: local image
tplay ./image.png

# Example: local gif
tplay ./image.gif

# Example: local video
tplay ./video.mp4

# Example: remote video (YouTube)
tplay https://www.youtube.com/watch?v=dQw4w9WgXcQ

# Example: remote video (Other)
tplay https://media.developer.dolby.com/Atmos/MP4/shattered-3Mb.mp4

# Example: YouTube video, with different char maps
tplay https://www.youtube.com/watch?v=fShlVhCfHig --char-map " ░▒▓█"

# Example: YouTube video, with different char maps (use w-mod to adjust width when using emoji-based char maps)
tplay https://www.youtube.com/watch?v=FtutLA63Cp8 --char-map "🍎🍏❤️😊" --w-mod 2

# Example: webcam on Linux (YMMV on other OSes)
tplay /dev/video0
```

# Playback commands
- `0-9` - change character map (with0 0
- `space` - toggle pause/unpause
- `g` - toggle grayscale/color
- `m` - toggle mute/unmute
- `q` - quit

# Known Issues
- Videos played through the Konsole terminal may have reduced performance. This is due to the way Konsole handles terminal output. If you experience this issue, try using a different terminal emulator. I recommend [Alacritty](https://alacritty.org/) for great performance.
- Media playback is CPU-intensive. To improve performance, increase the font size, reduce the terminal window size, or run with the `-a` / `--allow-frame-skip` flag.

# Alternatives
This is my ASCII media player: _there are many like it, but this one is mine._

For other ASCII media players, check out:
https://github.com/search?q=ascii+player&type=repositories

# Contributing
Contributions are welcome! Please open an issue or submit a pull request.
Some ideas:
- Reduce external dependencies and streamline the installation process.
- Investigate migration from OpenCV to FFmpeg.
- More media controls (jump forward, jump backwards, loop, etc.).
- Testing and feedback on installing and running it on other OSes.
- Let me know if you have any other ideas!

# License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

# Why?
_Your Scientists Were So Preoccupied With Whether Or Not They Could, They Didn’t Stop To Think If They Should_

Mostly did it for fun while learning Rust. I also wanted to see if it was possible to make a video player that could run in the terminal. I think it's pretty cool that you can play videos in the terminal now. I hope you enjoy it too!

# Credits

Thanks to the following people for their contributions and support:

<a href="https://github.com/maxcurzi/tplay/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=maxcurzi/tplay" />
</a>
