# Terminal Media Player

[![Crates.io](https://img.shields.io/crates/v/terminal-media-player.svg)](https://crates.io/crates/terminal-media-player)
[![Docs.rs](https://docs.rs/terminal-media-player/badge.svg)](https://docs.rs/terminal-media-player)
[![License](https://img.shields.io/crates/l/terminal-media-player.svg)](https://github.com/maxcurzi/tplay/blob/main/LICENSE)

View images, videos, webcam, etc directly in the terminal as ASCII art

## Table of Contents

- [Project Title](#project-title)
  - [Table of Contents](#table-of-contents)
    - [Who is it for?](#who-is-it-for)
    - [Features](#features)
  - [Getting Started](#getting-started)
    - [Prerequisites](#prerequisites)
    - [Installation](#installation)
  - [Usage](#usage)
  - [Documentation](#documentation)
  - [Contributing](#contributing)
  - [License](#license)
  - [Why](#why)

## Who is it for?
- You _really_ don't like graphical applications or are working on a computer without graphical capabilities.
- You want to watch a video in the terminal, but you don't want to use `mpv` or `vlc` because they're too mainstream.
- You like ASCII art so much that you don't need sound to enjoy a good movie.
- Your screen is full of terminal windows and you want some entertainment.
- You are looking for a quick way to convert any visual media to ASCII art.
- You want to show off your terminal skills to your friends and make them think you're a hacker.

## Features
- [x] Converts and shows any media to ASCII art in the terminal
- [x] Play webcam video output in the terminal
- [x] Play images in the terminal
- [x] Play gifs in the terminal
- [x] Play any video (files/streams/devices) in the terminal
- [x] Play YouTube videos in the terminal
- [x] Play videos/gifs in the terminal at any frame rate
- [x] Play media in the terminal at any resolution
- [x] Play media in the terminal at any aspect ratio
- [x] Play media in the terminal with any character set supported by your terminal
- [x] Handy pause/unpause and char map selection controls
- [x] Supports all major video/image formats


## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

What things you need to install or have installed on your system to use this project.



## Installation

A step-by-step guide on how to set up the project for development or usage.

```bash
# Clone the repository
git clone https://github.com/maxcurzi/tplay.git

# Change to the project directory
cd tplay

# Build the project
cargo build

# Run the project (use --release for faster performance)
cargo run --release -- [FILE] [options]

# Run the tests
cargo test
```

or install it as a binary
You can install the `tplay` command line tool by running the following command:

```bash
# Install the tplay command line tool
cargo install tplay
```
## Usage
`tplay [file] [options]`

```bash
# Run it
tplay [media] [options]

# Example: local image
tplay ./image.png

# Example: local gif
tplay ./image.gif

# Example: local video
tplay ./video.mp4 --fps 60

# Example: remote video (YouTube)
tplay https://www.youtube.com/watch?v=dQw4w9WgXcQ --fps 30

# Example: remote video (Other)
tplay http://media.developer.dolby.com/Atmos/MP4/shattered-3Mb.mp4 --fps 30

# Example: YouTube video - 30fps, with different char maps
tplay https://www.youtube.com/watch?v=fShlVhCfHig --fps 30 --char-lookup " ░▒▓█"

# Example: YouTube video - 30fps, with different char maps (use w-mod to adjust width when using emoji-based char maps)
tplay https://www.youtube.com/watch?v=FtutLA63Cp8 --fps 30 --char-lookup "🍎🍏" --w-mod 2

# Example: webcam on Linux (YMMV on other OSes)
tplay /dev/video0 --fps 30
```

## Playback commands
- `space` - pause/unpause
- `q` - quit
- `0-9` - change char map

## Known Issues
known issues

## Documentation
documentation

## Contributing
contribution guidelines

## License
This project is licensed under the GNU GPLv3 License - see the [LICENSE](LICENSE) file for details.

## Why?
_Your Scientists Were So Preoccupied With Whether Or Not They Could, They Didn’t Stop To Think If They Should_




