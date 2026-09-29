//! The settings menu's font metrics, for laying out text before drawing it.
//!
//! The menu's text is a trusted pre-baked bitmap, so there is no font parser
//! or system font at runtime: `tools/bake_font.py` bakes DejaVu Sans into
//! `ui-font.bmp` at [`BAKED_SIZE`] pixels (see `resources/FONT-LICENSE.txt`),
//! and [`GLYPHS`] is the table it writes alongside.

/// Where one character sits in the font atlas and how it is placed, in pixels
/// at [`BAKED_SIZE`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// The left edge of the character's image in the atlas.
    pub x: i32,
    /// The top edge of the character's image in the atlas.
    pub y: i32,
    /// The image's width.
    pub w: i32,
    /// The image's height.
    pub h: i32,
    /// From the pen position to the image's left edge.
    pub left: i32,
    /// From the baseline to the image's top edge; negative is above it.
    pub top: i32,
    /// How far the pen moves on after the character.
    pub advance: f32,
}

/// The size, in pixels, the atlas was baked at.
pub const BAKED_SIZE: f32 = 88.0;

mod glyphs;

pub use glyphs::GLYPHS;

/// The glyph for `byte` if it is printable ASCII; other bytes are not drawn.
pub fn glyph(byte: u8) -> Option<&'static Glyph> {
    GLYPHS.get(usize::from(byte.checked_sub(b' ')?))
}

/// How wide `text` is when drawn at `size` pixels, in the same units as `size`.
/// Bytes the font lacks take no space, as they are not drawn.
pub fn text_width(text: &str, size: f32) -> f32 {
    text.bytes()
        .filter_map(glyph)
        .fold(0.0, |width, glyph| width + glyph.advance * size / BAKED_SIZE)
}
