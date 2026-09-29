//! Colour profiles and brightness, graded pixel by pixel.

use super::{ALPHA, channels, rgb};
use crate::settings::{Colour, DisplaySettings};

/// Rec. 709 luma weights: how bright red, green and blue each look.
const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// The level contrast pivots on: the middle of 0 to 255.
const MID_LEVEL: f64 = 127.5;

/// The brightness that leaves the picture as it is, in percent.
const NEUTRAL_BRIGHTNESS: u8 = 100;

/// How a colour profile adjusts the artwork.
struct Grade {
    /// Red, green and blue multipliers, applied first.
    tint: [f64; 3],
    /// How far colours move from grey: below 1 mutes them, above 1 deepens them.
    saturation: f64,
    /// How far levels move from the middle: below 1 flattens, above 1 sharpens.
    contrast: f64,
}

impl Grade {
    /// Changes nothing.
    const NEUTRAL: Self = Self { tint: [1.0; 3], saturation: 1.0, contrast: 1.0 };

    /// The adjustments `colour` makes.
    const fn of(colour: Colour) -> Self {
        match colour {
            Colour::Original => Self::NEUTRAL,
            Colour::Warm => Self { tint: [1.035, 0.985, 0.90], ..Self::NEUTRAL },
            Colour::Vivid => Self { saturation: 1.14, contrast: 1.04, ..Self::NEUTRAL },
            Colour::Gentle => Self { saturation: 0.82, contrast: 0.90, ..Self::NEUTRAL },
        }
    }
}

/// Whether `settings` leave every colour exactly as it is.
pub(super) fn is_ungraded(settings: DisplaySettings) -> bool {
    settings.colour == Colour::Original && settings.brightness == NEUTRAL_BRIGHTNESS
}

/// `pixel` in the chosen colour profile and brightness, keeping its alpha.
pub fn colour_pixel(pixel: u32, settings: &DisplaySettings) -> u32 {
    if is_ungraded(*settings) {
        return pixel;
    }
    let grade = Grade::of(settings.colour);
    let [red, green, blue] = channels(pixel).map(f64::from);
    let grey = LUMA[0] * red + LUMA[1] * green + LUMA[2] * blue;
    let tinted = [red * grade.tint[0], green * grade.tint[1], blue * grade.tint[2]];
    let level = |tinted: f64| {
        let saturated = grey + (tinted - grey) * grade.saturation;
        let contrasted = (saturated - MID_LEVEL) * grade.contrast + MID_LEVEL;
        let lit = contrasted * f64::from(settings.brightness) / 100.0;
        lit.round().clamp(0.0, f64::from(u8::MAX)) as u32
    };
    (pixel & ALPHA) | rgb(tinted.map(level))
}
