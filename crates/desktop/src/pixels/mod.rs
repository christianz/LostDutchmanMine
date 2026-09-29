//! Everything between the game's frame and the picture on the display.
//!
//! The picture keeps its 4:3 shape at any resolution ([`picture_rect`]) and
//! maps the mouse back onto the game ([`picture_point`]). Its colours are
//! graded ([`colour_pixel`]), it is enlarged for the renderer
//! ([`display_pixels`]), and it may be given a CRT monitor's scanlines,
//! phosphors and glow ([`crt_mask`], [`crt_glow`]). All of this is spatial
//! processing only: it never changes the game's palette, pixels or timing.
//!
//! The floating-point arithmetic follows the C++ original operation for
//! operation, so every pixel matches it exactly; reordering it would not.

mod colour;
mod crt;
mod scale;
mod viewport;

pub use colour::colour_pixel;
pub use crt::{crt_glow, crt_mask};
pub use scale::display_pixels;
pub use viewport::{Rect, nearest_picture_point, picture_point, picture_rect};

/// The game's frame width, in pixels.
pub const WIDTH: usize = 320;

/// The game's frame height, in pixels.
pub const HEIGHT: usize = 200;

/// One frame of the game, row by row, as `0xAARRGGBB` pixels.
pub type Pixels = [u32; WIDTH * HEIGHT];

/// The alpha byte of an `0xAARRGGBB` pixel, and on its own an opaque black.
const ALPHA: u32 = 0xff00_0000;

/// A frame of nothing but `pixel`, built on the heap: at 256 KB it is too
/// large to build on the stack and move.
#[expect(clippy::missing_panics_doc, reason = "the vector is one frame long, so it always fits")]
pub fn filled(pixel: u32) -> Box<Pixels> {
    let frame = vec![pixel; WIDTH * HEIGHT].into_boxed_slice();
    frame.try_into().expect("the vector holds exactly one frame")
}

/// A pixel's red, green and blue levels, each 0 to 255.
fn channels(pixel: u32) -> [u32; 3] {
    [(pixel >> 16) & 0xff, (pixel >> 8) & 0xff, pixel & 0xff]
}

/// The colour bits of an `0xAARRGGBB` pixel with these red, green and blue levels.
fn rgb([red, green, blue]: [u32; 3]) -> u32 {
    (red << 16) | (green << 8) | blue
}
