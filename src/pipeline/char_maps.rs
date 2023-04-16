//! This module defines various character maps used for rendering the media.
//!
//! The character maps are strings that represent different sets of characters
//! that can be used to approximate the grayscale levels of the media being rendered.
//!
//! The available character maps are:
//! * `CHARS1`: 10 characters, ASCII-127 only.
//! * `CHARS2`: 67 characters, ASCII-127 only.
//! * `CHARS3`: 92 characters, ASCII-255.
//! * `SOLID`: 1 character, a solid block.
//! * `DOTTED`: 1 character, a dotted block.
//! * `GRADIENT`: 5 characters, a gradient of solid blocks.
//! * `BLACKWHITE`: 2 characters, a solid block and a space.
//! * `BW_DOTTED`: 2 characters, a dotted block and a space.
//! * `BRAILLE`: 16 characters, a braille-based gradient of solid blocks.

// ASCII-127 Only
pub const CHARS1: &str = r##" .:-=+*#%@"##; // 10 chars
pub const CHARS2: &str = r##" .'`^",:;Il!i~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"##; // 67 chars
pub const CHARS3: &str = r##" `.-':_,^=;><+!rc*/z?sLTv)J7(|Fi{C}fI31tlu[neoZ5Yxjya]2ESwqkP6h9d4VpOGbUAKXHm8RD#$Bg0MNWQ%&@"##; // 92 chars

// ASCII-255
pub const SOLID: &str = r#"█"#; // 1 Solid block
pub const DOTTED: &str = r#"⣿"#; // 1 dotted block
pub const GRADIENT: &str = r#" ░▒▓█"#; // 5 chars
pub const BLACKWHITE: &str = r#" █"#; // 2 chars
pub const BW_DOTTED: &str = r#" ⣿"#; // 2 dotted block
pub const BRAILLE: &str = r#" ··⣀⣀⣤⣤⣤⣀⡀⢀⠠⠔⠒⠑⠊⠉⠁"#; // 16 chars (braille-based)
