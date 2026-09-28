//! Where the picture sits on the display, and which game pixel a point is over.

use super::{HEIGHT, WIDTH};

/// A rectangle of display pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width; zero or less is empty.
    pub w: i32,
    /// The height; zero or less is empty.
    pub h: i32,
}

impl Rect {
    /// Whether the rectangle covers no pixels.
    pub const fn is_empty(self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Whether the display point (`px`, `py`) lies inside the rectangle.
    pub fn contains(self, px: i32, py: i32) -> bool {
        spans(self.x, self.w, px) && spans(self.y, self.h, py)
    }
}

/// The picture's width in units of its 4:3 shape.
const ASPECT_WIDTH: f64 = 4.0;

/// The picture's height in units of its 4:3 shape.
const ASPECT_HEIGHT: f64 = 3.0;

/// The largest 4:3 rectangle on a `width` x `height` display, shrunk to
/// `percent` of that size (clamped to 1 to 100) and centred. Empty for an empty
/// display.
pub fn picture_rect(width: i32, height: i32, percent: u8) -> Rect {
    if width <= 0 || height <= 0 {
        return Rect::default();
    }
    // VGA's rectangular pixels are presented at their original 4:3 aspect,
    // not the 16:10 their count suggests.
    let largest = (f64::from(width) / ASPECT_WIDTH).min(f64::from(height) / ASPECT_HEIGHT);
    let unit = largest * f64::from(percent.clamp(1, 100)) / 100.0;
    let w = ((unit * ASPECT_WIDTH).floor() as i32).max(1);
    let h = ((unit * ASPECT_HEIGHT).floor() as i32).max(1);
    Rect { x: (width - w) / 2, y: (height - h) / 2, w, h }
}

/// The game pixel under display point (`px`, `py`) on `picture`, or `None`
/// outside it: clicks on the letterbox never reach the game.
pub fn picture_point(picture: Rect, px: i32, py: i32) -> Option<(i32, i32)> {
    nearest_picture_point(picture, px, py).filter(|_| picture.contains(px, py))
}

/// The game pixel nearest display point (`px`, `py`), clamped to `picture`'s
/// edge so pointer motion over the letterbox still tracks it. `None` only when
/// `picture` is empty.
pub fn nearest_picture_point(picture: Rect, px: i32, py: i32) -> Option<(i32, i32)> {
    if picture.is_empty() {
        return None;
    }
    Some((onto(px, picture.x, picture.w, WIDTH), onto(py, picture.y, picture.h, HEIGHT)))
}

/// Whether `point` lies in the `extent` pixels from `start`, in wide arithmetic
/// so no rectangle can overflow.
fn spans(start: i32, extent: i32, point: i32) -> bool {
    let start = i64::from(start);
    (start..start + i64::from(extent)).contains(&i64::from(point))
}

/// Scales `point`, on a picture edge `extent` pixels long from `start`, onto
/// the game's `pixels` along that edge, clamped to them.
fn onto(point: i32, start: i32, extent: i32, pixels: usize) -> i32 {
    let pixels = pixels as i64;
    let scaled = (i64::from(point) - i64::from(start)) * pixels / i64::from(extent);
    scaled.clamp(0, pixels - 1) as i32
}
