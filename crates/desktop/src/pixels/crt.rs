//! The CRT monitor effect, as two layers over the picture: a steady mask of
//! scanlines and phosphor stripes the renderer multiplies in, and a blurred
//! highlight layer it adds for the phosphors' glow.

use std::f64::consts::{PI, TAU};

use super::{ALPHA, HEIGHT, Pixels, WIDTH, channels, rgb};
use crate::settings::Crt;

/// The mask that leaves the picture as it is.
const WHITE: u32 = 0xffff_ffff;

/// The beam draws the game's 200 lines, whatever the display's height.
const SCANLINES: f64 = 200.0;

/// Display lines per pixel of phosphor stripe width: stripes are one pixel
/// wide at 1080 lines and two at 2160, so they stay visible on large displays.
const LINES_PER_STRIPE_PIXEL: f64 = 1080.0;

/// The phosphor stripes' colours, repeating across the display.
const PHOSPHORS: usize = 3;

/// Levels above this glow; darker ones, and black, stay dark.
const GLOW_THRESHOLD: u32 = 64;

/// A 3x3 binomial blur's weights along each axis.
const BLUR: [u32; 3] = [1, 2, 1];

/// The nine blur weights' total.
const BLUR_TOTAL: u32 = (BLUR[0] + BLUR[1] + BLUR[2]).pow(2);

/// How strongly a CRT style shades the picture.
struct Strength {
    /// How much the gaps between scanlines darken.
    beam: f64,
    /// How brightly a phosphor stripe passes the two colours it does not emit.
    phosphor: f64,
    /// How much the edges and corners darken.
    edge: f64,
}

/// A light effect.
const SUBTLE: Strength = Strength { beam: 0.15, phosphor: 0.96, edge: 0.06 };

/// A bolder effect.
const STRONG: Strength = Strength { beam: 0.28, phosphor: 0.88, edge: 0.14 };

/// The mask for a `width` x `height` picture, row by row, into `output`: white
/// for [`Crt::Off`], otherwise scanlines, coloured phosphor stripes and gently
/// darkened edges. It depends only on the size and strength, so the renderer
/// can keep it until either changes. An empty size gives an empty mask.
pub fn crt_mask(effect: Crt, width: i32, height: i32, output: &mut Vec<u32>) {
    output.clear();
    if width <= 0 || height <= 0 {
        return;
    }
    let area = width as usize * height as usize;
    let strength = match effect {
        Crt::Off => {
            output.resize(area, WHITE);
            return;
        }
        Crt::Subtle => SUBTLE,
        Crt::Strong => STRONG,
    };
    let stripe = ((f64::from(height) / LINES_PER_STRIPE_PIXEL).round() as i32).max(1);
    let columns: Vec<_> = (0..width).map(|x| column(x, width, stripe, &strength)).collect();
    // Average each output pixel's slice of the 200-line beam pattern. Small
    // previews fade towards the average instead of aliasing into dark bands.
    let span = PI * SCANLINES / f64::from(height);
    let average = span.sin() / span;
    output.reserve(area);
    for y in 0..height {
        let light = row_light(y, height, average, &strength);
        let shade =
            |column: &[f64; 3]| ALPHA | rgb(column.map(|level| (level * light).round() as u32));
        output.extend(columns.iter().map(shade));
    }
}

/// The glow layer for `picture`, a `width` x `height` result of
/// [`display_pixels`](super::display_pixels): its highlights, blurred, at the
/// frame's 320x200 for the renderer to stretch and add.
///
/// # Panics
///
/// If `picture` is not 320x200 or 640x400 pixels, of `width` x `height`.
pub fn crt_glow(picture: &[u32], width: u32, height: u32, output: &mut Pixels) {
    let (width, height) = (width as usize, height as usize);
    let factor = width / WIDTH;
    assert!(
        matches!(factor, 1 | 2)
            && (width, height) == (factor * WIDTH, factor * HEIGHT)
            && picture.len() == width * height,
        "a CRT glow source is 320x200 or 640x400, not {width}x{height} with {} pixels",
        picture.len(),
    );
    // Extract a small highlight image before blurring. Blacks stay black and
    // the bloom follows the artwork rather than brightening the whole picture.
    let highlights: Vec<_> =
        (0..WIDTH * HEIGHT).map(|at| highlight(picture, factor, at % WIDTH, at / WIDTH)).collect();
    for (at, glow) in output.iter_mut().enumerate() {
        *glow = blurred(&highlights, at % WIDTH, at / WIDTH);
    }
}

/// Column `x`'s mask colour before scanlines: its phosphor stripe, darkened
/// towards the sides.
fn column(x: i32, width: i32, stripe: i32, strength: &Strength) -> [f64; 3] {
    let vignette = vignette(x, width, strength.edge);
    let lit = (x / stripe) as usize % PHOSPHORS;
    let full = f64::from(u8::MAX) * vignette;
    std::array::from_fn(|c| full * (if c == lit { 1.0 } else { strength.phosphor }))
}

/// Row `y`'s brightness: the averaged scanline pattern, darkened towards the
/// top and bottom.
fn row_light(y: i32, height: i32, average: f64, strength: &Strength) -> f64 {
    let scanline = (TAU * (f64::from(y) + 0.5) * SCANLINES / f64::from(height)).cos();
    (1.0 - strength.beam * (0.5 + 0.5 * scanline * average)) * vignette(y, height, strength.edge)
}

/// How much a curved screen's `edge` darkening leaves of pixel `i` of `size`:
/// all of it at the centre, falling away steeply only near the edges.
fn vignette(i: i32, size: i32, edge: f64) -> f64 {
    let from_centre = (f64::from(i) + 0.5) * 2.0 / f64::from(size) - 1.0;
    1.0 - edge * 0.5 * from_centre * from_centre * from_centre * from_centre
}

/// How far frame pixel (`x`, `y`) rises above [`GLOW_THRESHOLD`] in each
/// channel, averaged over the `factor` x `factor` block it became in `picture`.
fn highlight(picture: &[u32], factor: usize, x: usize, y: usize) -> [u32; 3] {
    let width = factor * WIDTH;
    let mut sum = [0; 3];
    for row in y * factor..(y + 1) * factor {
        let block = &picture[row * width + x * factor..][..factor];
        for &pixel in block {
            for (total, level) in sum.iter_mut().zip(channels(pixel)) {
                *total += level.saturating_sub(GLOW_THRESHOLD);
            }
        }
    }
    sum.map(|total| total / (factor * factor) as u32)
}

/// The blurred highlight at frame pixel (`x`, `y`), as an opaque pixel.
/// Neighbours past the frame's edge repeat the edge.
fn blurred(highlights: &[[u32; 3]], x: usize, y: usize) -> u32 {
    let mut sum = [0; 3];
    for (dy, y_weight) in BLUR.into_iter().enumerate() {
        let row = (y + dy).saturating_sub(1).min(HEIGHT - 1);
        for (dx, x_weight) in BLUR.into_iter().enumerate() {
            let column = (x + dx).saturating_sub(1).min(WIDTH - 1);
            for (total, level) in sum.iter_mut().zip(highlights[row * WIDTH + column]) {
                *total += level * x_weight * y_weight;
            }
        }
    }
    ALPHA | rgb(sum.map(|total| total / BLUR_TOTAL))
}
