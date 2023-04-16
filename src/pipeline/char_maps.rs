//! This module defines various character maps used for rendering the media.
//!
//! The character maps are strings that represent different sets of characters
//! that can be used to approximate the grayscale levels of the media being rendered.
//!
//! The available character maps are:
//! * `SHORT1`: A short character map with 10 ASCII-characters.
//! * `SHORT2`: A short character map with 5 ASCII-extended characters.
//! * `LONG1`: A longer character map with 67 characters.
//! * `LONG2`: An even longer character map with 92 characters.

pub const CHARS1: &str = " .:-=+*#%@"; // 10 chars
pub const CHARS2: &str = " .'`^\",:;Il!i~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"; // 67 chars
pub const CHARS3: &str =
    " `.-':_,^=;><+!rc*/z?sLTv)J7(|Fi{C}fI31tlu[neoZ5Yxjya]2ESwqkP6h9d4VpOGbUAKXHm8RD#$Bg0MNWQ%&@"; // 92 chars
pub const SOLID: &str = "█"; // 1 Solid block
pub const GRADIENT: &str = " ░▒▓█"; // 5 chars
pub const BLACKWHITE: &str = " █"; // 2 chars
