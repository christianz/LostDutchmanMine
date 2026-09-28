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

/// The printable ASCII characters' glyphs, from space to tilde.
pub static GLYPHS: [Glyph; 95] = [
    Glyph { x: 2, y: 2, w: 28, h: 1, left: 0, top: 0, advance: 27.9688 }, // space
    Glyph { x: 33, y: 2, w: 35, h: 64, left: 0, top: -64, advance: 35.2812 }, // !
    Glyph { x: 71, y: 2, w: 40, h: 64, left: 0, top: -64, advance: 40.4844 }, // "
    Glyph { x: 114, y: 2, w: 74, h: 64, left: 0, top: -64, advance: 73.7344 }, // #
    Glyph { x: 191, y: 2, w: 56, h: 80, left: 0, top: -67, advance: 55.9844 }, // $
    Glyph { x: 250, y: 2, w: 84, h: 66, left: 0, top: -65, advance: 83.6250 }, // %
    Glyph { x: 337, y: 2, w: 69, h: 66, left: 0, top: -65, advance: 68.6250 }, // &
    Glyph { x: 409, y: 2, w: 24, h: 64, left: 0, top: -64, advance: 24.1875 }, // '
    Glyph { x: 436, y: 2, w: 34, h: 79, left: 0, top: -67, advance: 34.3281 }, // (
    Glyph { x: 473, y: 2, w: 34, h: 79, left: 0, top: -67, advance: 34.3281 }, // )
    Glyph { x: 510, y: 2, w: 44, h: 65, left: 0, top: -65, advance: 44.0000 }, // *
    Glyph { x: 557, y: 2, w: 74, h: 55, left: 0, top: -55, advance: 73.7344 }, // +
    Glyph { x: 634, y: 2, w: 28, h: 21, left: 0, top: -11, advance: 27.9688 }, // ,
    Glyph { x: 665, y: 2, w: 32, h: 28, left: 0, top: -28, advance: 31.7500 }, // -
    Glyph { x: 700, y: 2, w: 28, h: 11, left: 0, top: -11, advance: 27.9688 }, // .
    Glyph { x: 731, y: 2, w: 30, h: 72, left: 0, top: -64, advance: 29.6562 }, // /
    Glyph { x: 764, y: 2, w: 56, h: 66, left: 0, top: -65, advance: 55.9844 }, // 0
    Glyph { x: 823, y: 2, w: 56, h: 64, left: 0, top: -64, advance: 55.9844 }, // 1
    Glyph { x: 882, y: 2, w: 56, h: 65, left: 0, top: -65, advance: 55.9844 }, // 2
    Glyph { x: 941, y: 2, w: 56, h: 66, left: 0, top: -65, advance: 55.9844 }, // 3
    Glyph { x: 2, y: 112, w: 56, h: 64, left: 0, top: -64, advance: 55.9844 }, // 4
    Glyph { x: 61, y: 112, w: 56, h: 65, left: 0, top: -64, advance: 55.9844 }, // 5
    Glyph { x: 120, y: 112, w: 56, h: 66, left: 0, top: -65, advance: 55.9844 }, // 6
    Glyph { x: 179, y: 112, w: 56, h: 64, left: 0, top: -64, advance: 55.9844 }, // 7
    Glyph { x: 238, y: 112, w: 56, h: 66, left: 0, top: -65, advance: 55.9844 }, // 8
    Glyph { x: 297, y: 112, w: 56, h: 66, left: 0, top: -65, advance: 55.9844 }, // 9
    Glyph { x: 356, y: 112, w: 30, h: 46, left: 0, top: -46, advance: 29.6562 }, // :
    Glyph { x: 389, y: 112, w: 30, h: 56, left: 0, top: -46, advance: 29.6562 }, // ;
    Glyph { x: 422, y: 112, w: 74, h: 51, left: 0, top: -51, advance: 73.7344 }, // <
    Glyph { x: 499, y: 112, w: 74, h: 39, left: 0, top: -39, advance: 73.7344 }, // =
    Glyph { x: 576, y: 112, w: 74, h: 51, left: 0, top: -51, advance: 73.7344 }, // >
    Glyph { x: 653, y: 112, w: 47, h: 65, left: 0, top: -65, advance: 46.7031 }, // ?
    Glyph { x: 703, y: 112, w: 88, h: 77, left: 0, top: -62, advance: 88.0000 }, // @
    Glyph { x: 794, y: 112, w: 60, h: 64, left: 0, top: -64, advance: 60.2031 }, // A
    Glyph { x: 857, y: 112, w: 60, h: 64, left: 0, top: -64, advance: 60.3750 }, // B
    Glyph { x: 920, y: 112, w: 61, h: 66, left: 0, top: -65, advance: 61.4531 }, // C
    Glyph { x: 2, y: 222, w: 68, h: 64, left: 0, top: -64, advance: 67.7656 }, // D
    Glyph { x: 73, y: 222, w: 56, h: 64, left: 0, top: -64, advance: 55.6094 }, // E
    Glyph { x: 132, y: 222, w: 51, h: 64, left: 0, top: -64, advance: 50.6250 }, // F
    Glyph { x: 186, y: 222, w: 68, h: 66, left: 0, top: -65, advance: 68.1875 }, // G
    Glyph { x: 257, y: 222, w: 66, h: 64, left: 0, top: -64, advance: 66.1719 }, // H
    Glyph { x: 326, y: 222, w: 26, h: 64, left: 0, top: -64, advance: 25.9531 }, // I
    Glyph { x: 355, y: 222, w: 31, h: 82, left: -5, top: -64, advance: 25.9531 }, // J
    Glyph { x: 389, y: 222, w: 60, h: 64, left: 0, top: -64, advance: 57.7031 }, // K
    Glyph { x: 452, y: 222, w: 49, h: 64, left: 0, top: -64, advance: 49.0312 }, // L
    Glyph { x: 504, y: 222, w: 76, h: 64, left: 0, top: -64, advance: 75.9219 }, // M
    Glyph { x: 583, y: 222, w: 66, h: 64, left: 0, top: -64, advance: 65.8281 }, // N
    Glyph { x: 652, y: 222, w: 69, h: 66, left: 0, top: -65, advance: 69.2656 }, // O
    Glyph { x: 724, y: 222, w: 53, h: 64, left: 0, top: -64, advance: 53.0625 }, // P
    Glyph { x: 780, y: 222, w: 69, h: 76, left: 0, top: -65, advance: 69.2656 }, // Q
    Glyph { x: 852, y: 222, w: 61, h: 64, left: 0, top: -64, advance: 61.1406 }, // R
    Glyph { x: 916, y: 222, w: 56, h: 66, left: 0, top: -65, advance: 55.8594 }, // S
    Glyph { x: 2, y: 332, w: 56, h: 64, left: -1, top: -64, advance: 53.7500 }, // T
    Glyph { x: 61, y: 332, w: 64, h: 65, left: 0, top: -64, advance: 64.4062 }, // U
    Glyph { x: 128, y: 332, w: 60, h: 64, left: 0, top: -64, advance: 60.2031 }, // V
    Glyph { x: 191, y: 332, w: 87, h: 64, left: 0, top: -64, advance: 87.0156 }, // W
    Glyph { x: 281, y: 332, w: 60, h: 64, left: 0, top: -64, advance: 60.2812 }, // X
    Glyph { x: 344, y: 332, w: 55, h: 64, left: -1, top: -64, advance: 53.7500 }, // Y
    Glyph { x: 402, y: 332, w: 60, h: 64, left: 0, top: -64, advance: 60.2812 }, // Z
    Glyph { x: 465, y: 332, w: 34, h: 79, left: 0, top: -67, advance: 34.3281 }, // [
    Glyph { x: 502, y: 332, w: 30, h: 72, left: 0, top: -64, advance: 29.6562 }, // backslash
    Glyph { x: 535, y: 332, w: 34, h: 79, left: 0, top: -67, advance: 34.3281 }, // ]
    Glyph { x: 572, y: 332, w: 74, h: 64, left: 0, top: -64, advance: 73.7344 }, // ^
    Glyph { x: 649, y: 332, w: 46, h: 21, left: -1, top: 0, advance: 44.0000 }, // _
    Glyph { x: 698, y: 332, w: 44, h: 70, left: 0, top: -70, advance: 44.0000 }, // `
    Glyph { x: 745, y: 332, w: 54, h: 50, left: 0, top: -49, advance: 53.9219 }, // a
    Glyph { x: 802, y: 332, w: 56, h: 68, left: 0, top: -67, advance: 55.8594 }, // b
    Glyph { x: 861, y: 332, w: 48, h: 50, left: 0, top: -49, advance: 48.3906 }, // c
    Glyph { x: 912, y: 332, w: 56, h: 68, left: 0, top: -67, advance: 55.8594 }, // d
    Glyph { x: 2, y: 442, w: 54, h: 50, left: 0, top: -49, advance: 54.1406 }, // e
    Glyph { x: 59, y: 442, w: 33, h: 67, left: 0, top: -67, advance: 30.9844 }, // f
    Glyph { x: 95, y: 442, w: 56, h: 67, left: 0, top: -49, advance: 55.8594 }, // g
    Glyph { x: 154, y: 442, w: 56, h: 67, left: 0, top: -67, advance: 55.7812 }, // h
    Glyph { x: 213, y: 442, w: 24, h: 67, left: 0, top: -67, advance: 24.4531 }, // i
    Glyph { x: 240, y: 442, w: 26, h: 85, left: -2, top: -67, advance: 24.4531 }, // j
    Glyph { x: 269, y: 442, w: 51, h: 67, left: 0, top: -67, advance: 50.9688 }, // k
    Glyph { x: 323, y: 442, w: 24, h: 67, left: 0, top: -67, advance: 24.4531 }, // l
    Glyph { x: 350, y: 442, w: 86, h: 49, left: 0, top: -49, advance: 85.7188 }, // m
    Glyph { x: 439, y: 442, w: 56, h: 49, left: 0, top: -49, advance: 55.7812 }, // n
    Glyph { x: 498, y: 442, w: 54, h: 50, left: 0, top: -49, advance: 53.8438 }, // o
    Glyph { x: 555, y: 442, w: 56, h: 67, left: 0, top: -49, advance: 55.8594 }, // p
    Glyph { x: 614, y: 442, w: 56, h: 67, left: 0, top: -49, advance: 55.8594 }, // q
    Glyph { x: 673, y: 442, w: 37, h: 49, left: 0, top: -49, advance: 36.1875 }, // r
    Glyph { x: 713, y: 442, w: 46, h: 50, left: 0, top: -49, advance: 45.8438 }, // s
    Glyph { x: 762, y: 442, w: 35, h: 62, left: 0, top: -62, advance: 34.5000 }, // t
    Glyph { x: 800, y: 442, w: 56, h: 50, left: 0, top: -49, advance: 55.7812 }, // u
    Glyph { x: 859, y: 442, w: 52, h: 48, left: 0, top: -48, advance: 52.0781 }, // v
    Glyph { x: 914, y: 442, w: 72, h: 48, left: 0, top: -48, advance: 71.9688 }, // w
    Glyph { x: 2, y: 552, w: 52, h: 48, left: 0, top: -48, advance: 52.0781 }, // x
    Glyph { x: 57, y: 552, w: 52, h: 66, left: 0, top: -48, advance: 52.0781 }, // y
    Glyph { x: 112, y: 552, w: 46, h: 48, left: 0, top: -48, advance: 46.1875 }, // z
    Glyph { x: 161, y: 552, w: 56, h: 81, left: 0, top: -67, advance: 55.9844 }, // {
    Glyph { x: 220, y: 552, w: 30, h: 88, left: 0, top: -67, advance: 29.6562 }, // |
    Glyph { x: 253, y: 552, w: 56, h: 81, left: 0, top: -67, advance: 55.9844 }, // }
    Glyph { x: 312, y: 552, w: 74, h: 36, left: 0, top: -36, advance: 73.7344 }, // ~
];

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
