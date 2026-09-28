//! Preparing the frame for the renderer: grading it and enlarging it.

use std::borrow::Cow;
use std::collections::HashMap;

use super::colour::is_ungraded;
use super::{HEIGHT, Pixels, WIDTH, colour_pixel};
use crate::settings::{DisplaySettings, Scaling};

/// Room for every colour of the game's 256-colour palette, with headroom.
const PALETTE_ROOM: usize = 512;

/// The 2x2 block an enlarged pixel becomes: its top row, then its bottom row.
type Block = [[u32; 2]; 2];

/// Grades `input` and enlarges it as `settings.scaling` asks, into `output`,
/// row by row; returns the prepared picture's width and height.
///
/// Crisp stays 320x200 for the renderer to stretch with hard edges. Soft is a
/// 2x point enlargement the renderer then samples linearly, which softens pixel
/// edges without blurring each original pixel across its width. Pixel art is a
/// 2x Scale2x enlargement, which rounds diagonals.
pub fn display_pixels(
    input: &Pixels,
    settings: &DisplaySettings,
    output: &mut Vec<u32>,
) -> (u32, u32) {
    let graded = graded(input, *settings);
    output.clear();
    match settings.scaling {
        Scaling::Crisp => {
            output.extend_from_slice(&graded);
            (WIDTH as u32, HEIGHT as u32)
        }
        Scaling::Soft => enlarge(output, |x, y| [[graded[y * WIDTH + x]; 2]; 2]),
        Scaling::PixelArt => enlarge(output, |x, y| scale2x(input, &graded, x, y)),
    }
}

/// `input` in the chosen colours. Each distinct colour is graded once, not
/// each pixel or each enlarged subpixel: with the game's 256 colours, a lookup
/// is far cheaper than repeating the floating-point colour arithmetic.
fn graded(input: &Pixels, settings: DisplaySettings) -> Cow<'_, [u32]> {
    if is_ungraded(settings) {
        return Cow::Borrowed(input);
    }
    let mut colours = HashMap::with_capacity(PALETTE_ROOM);
    let mut grade =
        |pixel: u32| *colours.entry(pixel).or_insert_with(|| colour_pixel(pixel, &settings));
    Cow::Owned(input.iter().map(|&pixel| grade(pixel)).collect())
}

/// Fills `output` with a picture twice the frame's size in each direction,
/// taking the block for the frame pixel at (`x`, `y`) from `block`; returns
/// the picture's width and height.
fn enlarge(output: &mut Vec<u32>, block: impl Fn(usize, usize) -> Block) -> (u32, u32) {
    let (width, height) = (2 * WIDTH, 2 * HEIGHT);
    output.resize(width * height, 0);
    for (y, rows) in output.chunks_exact_mut(2 * width).enumerate() {
        let (top, bottom) = rows.split_at_mut(width);
        let pairs = top.chunks_exact_mut(2).zip(bottom.chunks_exact_mut(2));
        for (x, (top, bottom)) in pairs.enumerate() {
            let [upper, lower] = block(x, y);
            top.copy_from_slice(&upper);
            bottom.copy_from_slice(&lower);
        }
    }
    (width as u32, height as u32)
}

/// The Scale2x block for the frame pixel at (`x`, `y`), independently
/// implemented from the published algorithm: <https://www.scale2x.it/algorithm>.
///
/// Neighbours past the frame's edge repeat the edge. They are compared in the
/// source artwork, so grading never changes which diagonals are rounded; the
/// block's colours come from the `graded` frame.
fn scale2x(source: &Pixels, graded: &[u32], x: usize, y: usize) -> Block {
    let at = |x: usize, y: usize| y * WIDTH + x;
    let centre = graded[at(x, y)];
    let (up, down) = (at(x, y.saturating_sub(1)), at(x, (y + 1).min(HEIGHT - 1)));
    let (left, right) = (at(x.saturating_sub(1), y), at((x + 1).min(WIDTH - 1), y));
    if source[up] == source[down] || source[left] == source[right] {
        return [[centre; 2]; 2];
    }
    let corner =
        |side: usize, end: usize| if source[side] == source[end] { graded[side] } else { centre };
    [[corner(left, up), corner(right, up)], [corner(left, down), corner(right, down)]]
}
