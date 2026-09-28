//! A 320x200 frame of 0xAARRGGBB pixels, and drawing in native game pixels.

/// Fully opaque black.
pub const BLACK: u32 = rgb(0, 0, 0);
/// Fully opaque white.
pub const WHITE: u32 = rgb(255, 255, 255);

/// An opaque colour.
pub const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    255 << 24 | (red as u32) << 16 | (green as u32) << 8 | blue as u32
}

/// Inverts the colour channels, keeping alpha.
pub const fn invert(colour: u32) -> u32 {
    colour ^ (WHITE & !(255 << 24))
}

/// Whether the colour channels are all zero.
pub const fn is_black(colour: u32) -> bool {
    colour & !(255 << 24) == 0
}

/// What the game shows: 320x200 pixels, row by row.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame(Box<[u32]>);

impl Frame {
    /// Pixels per row.
    pub const WIDTH: usize = 320;
    /// Rows.
    pub const HEIGHT: usize = 200;

    /// A frame of one colour.
    pub fn filled(colour: u32) -> Self {
        Frame(vec![colour; Self::WIDTH * Self::HEIGHT].into_boxed_slice())
    }

    /// The pixels, row by row.
    pub fn pixels(&self) -> &[u32] {
        &self.0
    }

    /// The pixels, for writing whole rows.
    pub(crate) fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.0
    }

    /// The pixel at `x, y`, if on the frame.
    pub(crate) fn get(&self, x: i32, y: i32) -> Option<u32> {
        Self::index(x, y).map(|i| self.0[i])
    }

    /// The pixel at `x, y`, if on the frame.
    pub(crate) fn at_mut(&mut self, x: i32, y: i32) -> Option<&mut u32> {
        Self::index(x, y).map(|i| &mut self.0[i])
    }

    fn index(x: i32, y: i32) -> Option<usize> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        (x < Self::WIDTH && y < Self::HEIGHT).then_some(y * Self::WIDTH + x)
    }

    /// Sets one pixel; off the frame nothing happens.
    pub(crate) fn dot(&mut self, x: i32, y: i32, colour: u32) {
        if let Some(pixel) = self.at_mut(x, y) {
            *pixel = colour;
        }
    }

    /// Fills a rectangle, clipped to the frame.
    pub(crate) fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, colour: u32) {
        for row in y..y + height {
            for column in x..x + width {
                self.dot(column, row, colour);
            }
        }
    }

    /// A one-pixel line from `from` to `to`, both ends included (Bresenham).
    pub(crate) fn line(&mut self, from: (i32, i32), to: (i32, i32), colour: u32) {
        let ((mut x, mut y), (x1, y1)) = (from, to);
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (step_x, step_y) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
        let mut error = dx + dy;
        loop {
            self.dot(x, y, colour);
            if (x, y) == (x1, y1) {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x += step_x;
            }
            if doubled <= dx {
                error += dx;
                y += step_y;
            }
        }
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Frame({}x{})", Self::WIDTH, Self::HEIGHT)
    }
}

/// Hand-authored 5x7 capitals and digits: native game pixels, never a scaled
/// desktop font. Used only if the game's own font is not loaded.
const GLYPHS: [[u8; 7]; 36] = [
    [14, 17, 19, 21, 25, 17, 14],
    [4, 12, 4, 4, 4, 4, 14],
    [14, 17, 1, 2, 4, 8, 31],
    [30, 1, 1, 14, 1, 1, 30],
    [2, 6, 10, 18, 31, 2, 2],
    [31, 16, 16, 30, 1, 1, 30],
    [14, 16, 16, 30, 17, 17, 14],
    [31, 1, 2, 4, 8, 8, 8],
    [14, 17, 17, 14, 17, 17, 14],
    [14, 17, 17, 15, 1, 1, 14],
    [14, 17, 17, 31, 17, 17, 17],
    [30, 17, 17, 30, 17, 17, 30],
    [14, 17, 16, 16, 16, 17, 14],
    [30, 17, 17, 17, 17, 17, 30],
    [31, 16, 16, 30, 16, 16, 31],
    [31, 16, 16, 30, 16, 16, 16],
    [14, 17, 16, 23, 17, 17, 15],
    [17, 17, 17, 31, 17, 17, 17],
    [14, 4, 4, 4, 4, 4, 14],
    [7, 2, 2, 2, 18, 18, 12],
    [17, 18, 20, 24, 20, 18, 17],
    [16, 16, 16, 16, 16, 16, 31],
    [17, 27, 21, 21, 17, 17, 17],
    [17, 25, 21, 19, 17, 17, 17],
    [14, 17, 17, 17, 17, 17, 14],
    [30, 17, 17, 30, 16, 16, 16],
    [14, 17, 17, 17, 21, 18, 13],
    [30, 17, 17, 30, 20, 18, 17],
    [15, 16, 16, 14, 1, 1, 30],
    [31, 4, 4, 4, 4, 4, 4],
    [17, 17, 17, 17, 17, 17, 14],
    [17, 17, 17, 17, 17, 10, 4],
    [17, 17, 17, 21, 21, 21, 10],
    [17, 17, 10, 4, 10, 17, 17],
    [17, 17, 10, 4, 4, 4, 4],
    [31, 1, 2, 4, 8, 16, 31],
];

impl Frame {
    /// Writes `text` in the 5x7 fallback capitals, six pixels per character.
    pub(crate) fn fallback_text(&mut self, x: i32, y: i32, text: &str, colour: u32, centre: bool) {
        let width = i32::try_from(text.len()).unwrap_or(i32::MAX / 6) * 6 - 1;
        let mut x = if centre { x - width / 2 } else { x };
        for character in text.bytes().map(|c| c.to_ascii_uppercase()) {
            let glyph = match character {
                b'0'..=b'9' => Some(usize::from(character - b'0')),
                b'A'..=b'Z' => Some(usize::from(character - b'A') + 10),
                _ => None,
            };
            if let Some(glyph) = glyph {
                for (row, bits) in (0..).zip(GLYPHS[glyph]) {
                    for column in 0..5 {
                        if bits & (16 >> column) != 0 {
                            self.dot(x + column, y + row, colour);
                        }
                    }
                }
            } else {
                self.fallback_punctuation(x, y, character, colour);
            }
            x += 6;
        }
    }

    fn fallback_punctuation(&mut self, x: i32, y: i32, character: u8, colour: u32) {
        match character {
            b'.' => self.dot(x + 2, y + 6, colour),
            b':' => {
                self.dot(x + 2, y + 2, colour);
                self.dot(x + 2, y + 5, colour);
            }
            b'!' => {
                self.rect(x + 2, y, 1, 4, colour);
                self.dot(x + 2, y + 6, colour);
            }
            b'/' => self.line((x + 4, y), (x, y + 6), colour),
            b'-' => self.rect(x, y + 3, 5, 1, colour),
            b'<' => {
                self.line((x + 3, y + 1), (x, y + 3), colour);
                self.line((x, y + 3), (x + 3, y + 5), colour);
            }
            b'>' => {
                self.line((x, y + 1), (x + 3, y + 3), colour);
                self.line((x + 3, y + 3), (x, y + 5), colour);
            }
            _ => {}
        }
    }
}
