<p align="center">
  <img src="https://user-images.githubusercontent.com/30084738/231727365-defc7606-59aa-48f5-b8c4-b7ec4664eac1.jpeg" alt="Image description" width="120" height="120">
</p>

# Terminal Media Player

[![Crates.io](https://img.shields.io/crates/v/tplay)](https://crates.io/crates/tplay)
[![Crates.io](https://img.shields.io/crates/d/tplay)](https://crates.io/crates/tplay)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

View images, videos (files or YouTube links), webcam, etc directly in the terminal as ASCII. All images you see [below](#features) are just made by characters on the terminal command line, drawn really fast.

# Table of Contents

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
    - [For developers](#for-developers)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)
- [Why](#why)

# Who is it for?
- You _really_ don't like graphical applications or are working on a computer without graphical capabilities.
- You are looking for a quick way to convert any visual media to ASCII art.
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
- [ ] Full media controls (forward, backward, loop, etc)
- [ ] Subtitles
- [ ] Replace a fully fledged media player

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

[OpenCV 4](https://github.com/twistedfall/opencv-rust#getting-opencv), [LLVM](https://github.com/llvm/llvm-project/releases/tag/llvmorg-16.0.0), [MPV](https://mpv.io/installation/), [ffmpeg](https://ffmpeg.org/download.html)
They are  simply installed on linux with your package manager. See [below](#prerequisites-installation-on-linux) for more details.

 Optional dependency for YouTube playback support: [yt-dlp](https://github.com/yt-dlp/yt-dlp/wiki/installation)

## Prerequisites Installation on Linux
If you're on Linux, you can install all dependencies with your package manager. First install Rust:

```bash
sudo apt install curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install `tplay`'s prerequisite dependencies:

```bash
sudo apt install libopencv-dev clang libclang-dev libmpv1 libmpv-dev ffmpeg libavfilter-dev libavdevice-dev
```

## Prerequisites installation on Windows
**Installing all prerequisited on Windows is NOT RECOMMENDED as it's a lengthy process prone to errors. I leave a partial description that should work up until crate version 0.2.1**

**If you are on Windows, use WSL (Windows Subsystem for Linux) and follow the [Linux instructions](#prerequisites-installation-on-linux)**

-- Old instructions for Windows (up until crate version 0.2.1) --
- Download OpenCV prebuilt binaries (I used this [one](https://sourceforge.net/projects/opencvlibrary/)) and it was 4.6.0 at the time of writing.
- Open the package and extract the `opencv` folder to `C:\opencv` or any other location you prefer.
- Set the following environment variables (update the paths if you extracted the package to a different location):
  - OPENCV_INCLUDE_PATHS = `C:\opencv\build\include`
  - OPENCV_LINK_LIBS = `opencv_world460` (or whatever version you have, for OpenCV 4.7.0 you want `opencv_world470`)
  - OPENCV_LINK_PATHS = `C:\opencv\build\x64\vc15\lib`
  - Also add this to your PATH variable : `C:\opencv\build\x64\vc15\bin`

- Install [LLVM](https://github.com/llvm/llvm-project/releases/tag/llvmorg-16.0.0) from binary, you'll likely want to use the 64-bit version on a modern computer.
  - Add this to your PATH variable (or whatever corresponding directory you have on your computer): `C:\Program Files\LLVM\bin`
- Install [yt-dlp](https://github.com/yt-dlp/yt-dlp/wiki/installation) from binary, you'll likely want to use the 64-bit version on a modern computer.
  - Add this to your PATH variable (or whatever corresponding directory you have on your computer): `C:\Program Files\yt-dlp\bin`

# Installation

## For users
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

# Usage
`tplay <media> [options]`

| Argument | Description |
|--------|-------------|
| `media` | Name of the file or stream to be processed (required). |
| `-f`, `--fps` | Maximum frames per second for the output (default: 60). |
| `-c`, `--char-map` | Custom lookup character table to use for the output (default: ` .:-=+*#%@`). |
| `-g`, `--gray` | Start in grayscale mode |
| `-w`, `--w-mod` | Experimental width modifier for certain characters such as emojis (default: 1). Use a value of 2 if your char_map is composed of emojis. |
| `-a`, `--allow-frame-skip` | Experimental frame skip flag. Try to use if the playback is too slow. |
| `-n`, `--new-lines` | Experimental flag. Adds newline and carriage return `\n\r` at the end of each line (except the last). Terminals wrap around and don't need new lines, but if you want to copy-paste the text outside the terminal you may want them. The output would be a single long string otherwise. Uses more CPU. |

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
tplay http://media.developer.dolby.com/Atmos/MP4/shattered-3Mb.mp4

# Example: YouTube video, with different char maps
tplay https://www.youtube.com/watch?v=fShlVhCfHig --char-map " ░▒▓█"

# Example: YouTube video, with different char maps (use w-mod to adjust width when using emoji-based char maps)
tplay https://www.youtube.com/watch?v=FtutLA63Cp8 --char-map "🍎🍏❤️😊" --w-mod 2

# Example: webcam on Linux (YMMV on other OSes)
tplay /dev/video0
```

# Playback commands
- `0-9` - change character map
- `space` - toggle pause/unpause
- `g` - toggle grayscale/color
- `m` - toggle mute/unmute
- `q` - quit

# Known Issues
- Videos played through the Konsole terminal may have reduced performance. This is due to the way Konsole handles terminal output. If you experience this issue, try using a different terminal emulator. I recommend [Alacritty](https://alacritty.org/) which has great performance on all operative systems I tested tplay on (Linux, Windows).
- Media playback is cpu-intensive. To improve performance, try lowering the `fps` value, increase font size, reduce the terminal window size, or open with the `--allow-frame-skip` flag.
- If you get this error: `panicked at 'Failed to init MPV builder: VersionMismatch { linked: 65644, loaded: 131072 }'` see [this issue](https://github.com/maxcurzi/tplay/issues/3) for a workaround.

# Alternatives
This is my ASCII media player: _there are many like it, but this one is mine._

For other ASCII media players, check out:
https://github.com/search?q=ascii+player&type=repositories

# Contributing
Contributions are welcome! Please open an issue or submit a pull request.
Some ideas:
- Reduce external dependencies and streamline installation process.
- Investigate migration from OpenCV to ffmpeg.
- More media controls (jump forward, jump backward, loop, etc.).
- Testing and feedback on installing and running it on other OSes.
- Let me know if you have any other ideas!

# License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

# Why?
_Your Scientists Were So Preoccupied With Whether Or Not They Could, They Didn’t Stop To Think If They Should_

Mostly did it for fun while learning Rust. I also wanted to see if it was possible to make a video player that could run in the terminal. I think it's pretty cool that you can play videos in the terminal now. I hope you enjoy it too!
