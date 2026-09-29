//! Composing what the player sees: the original screen, the encounter's sight,
//! the overlay and the pointer. Frames are composed between runs and change
//! nothing in the machine.

use machine::Machine;

pub use crate::pixels::Frame;
use crate::pixels::{BLACK, WHITE, invert, rgb};
use crate::symbols::{
    CGA_FRAMEBUFFER, ENCOUNTER_WON, GUNS, SCENE_ENCOUNTER, SPRITE_PAGE, VGA_FRAMEBUFFER,
    VIDEO_MODE_CGA, VIDEO_MODE_CGA_GREY, VIDEO_MODE_VGA,
};
use crate::{Game, hooks, overlay};

/// The CGA version's four colours.
const CGA_COLOURS: [u32; 4] = [BLACK, rgb(0x55, 0xff, 0xff), rgb(0xff, 0x55, 0xff), WHITE];
/// Bytes per CGA row, and the offset of the odd rows' bank.
const CGA_ROW: usize = 80;
const CGA_ODD_BANK: usize = 8192;

/// The sight: 16x16 at this corner of the resident sprite page, drawn only
/// within the shooting area's rows.
const SIGHT_SIZE: i32 = 16;
const SIGHT_SPRITE: (i32, i32) = (120, 160);
const SHOOTING_ROWS: std::ops::Range<i32> = 11..111;
/// The original transparent blitter skips indices from 128 up, except this one,
/// which draws colour 0.
const DRAWS_COLOUR_ZERO: u8 = 255;

/// The QoL pointer: a small outlined arrow, its tip at the hit-test point.
const ARROW: [&[u8; 12]; 16] = [
    b"X           ",
    b"XX          ",
    b"X.X         ",
    b"X..X        ",
    b"X...X       ",
    b"X....X      ",
    b"X.....X     ",
    b"X......X    ",
    b"X.......X   ",
    b"X....XXXXX  ",
    b"X..X..X     ",
    b"X.X X..X    ",
    b"XX  X..X    ",
    b"X    X..X   ",
    b"     X..X   ",
    b"      XX    ",
];

impl Game {
    /// The frame the player sees now.
    pub fn frame(&self, m: &Machine) -> Frame {
        let mut frame = match self.dos.video.mode {
            VIDEO_MODE_VGA => vga(m),
            VIDEO_MODE_CGA | VIDEO_MODE_CGA_GREY => cga(m),
            _ => Frame::filled(BLACK),
        };
        if self.dos.video.mode == VIDEO_MODE_VGA && self.sight_shows(m) {
            draw_sight(&mut frame, m, hooks::aim(m, self));
        }
        overlay::draw(&mut frame, m, self);
        if self.dos.mouse.custom_cursor && overlay::pointer_visible(m, self) {
            self.draw_pointer(&mut frame);
        }
        frame
    }

    /// The encounter's six-tick loop still owns enemies, shots and hit tests,
    /// but with QoL its sight is drawn here at display cadence, so pointer
    /// motion need not wait for the next simulation step.
    fn sight_shows(&self, m: &Machine) -> bool {
        self.combat.active
            && self.combat.sight_visible
            && SCENE_ENCOUNTER.at_rest(m) != 0
            && !self.overlay.menu_open()
            && ENCOUNTER_WON.at_rest(m) == 0
            && GUNS.at_rest(m) != 0
    }

    fn draw_pointer(&self, frame: &mut Frame) {
        let mouse = &self.dos.mouse;
        let pointer = mouse.input.current();
        let origin = if self.qol {
            (pointer.x, pointer.y)
        } else {
            (pointer.x - mouse.hotspot.0, pointer.y - mouse.hotspot.1)
        };
        for (y, arrow_row) in (0..16).zip(ARROW) {
            for x in 0..16 {
                let Some(pixel) = frame.at_mut(origin.0 + x, origin.1 + y) else { continue };
                if self.qol {
                    match arrow_row.get(x as usize) {
                        Some(b'X') => *pixel = BLACK,
                        Some(b'.') => *pixel = WHITE,
                        _ => {}
                    }
                } else {
                    // The game's own cursor: a screen mask, then a cursor mask,
                    // with bit 15 the leftmost pixel.
                    let bit = 1 << (15 - x);
                    if mouse.shape[y as usize] & bit == 0 {
                        *pixel = BLACK;
                    }
                    if mouse.shape[y as usize + 16] & bit != 0 {
                        *pixel = invert(*pixel);
                    }
                }
            }
        }
    }
}

/// Mode 13h: one palette index per pixel.
fn vga(m: &Machine) -> Frame {
    let mut frame = Frame::filled(BLACK);
    let screen = &m.memory.as_bytes()[VGA_FRAMEBUFFER..];
    for (pixel, &index) in frame.pixels_mut().iter_mut().zip(screen) {
        *pixel = m.vga.palette[usize::from(index)];
    }
    frame
}

/// Modes 4 and 5: four pixels per byte, even and odd rows in separate banks.
fn cga(m: &Machine) -> Frame {
    let mut frame = Frame::filled(BLACK);
    let memory = m.memory.as_bytes();
    for (y, row) in frame.pixels_mut().chunks_mut(Frame::WIDTH).enumerate() {
        let bank = CGA_FRAMEBUFFER + (y % 2) * CGA_ODD_BANK + (y / 2) * CGA_ROW;
        for (x, pixel) in row.iter_mut().enumerate() {
            let byte = memory[bank + x / 4];
            *pixel = CGA_COLOURS[usize::from((byte >> (6 - 2 * (x % 4))) & 3)];
        }
    }
    frame
}

/// Composites the sight from the resident original sprite, matching the
/// original transparent VGA blitter at 1265:0427: below 128 an index is a
/// 16-colour index; above, it is transparent, except [`DRAWS_COLOUR_ZERO`].
fn draw_sight(frame: &mut Frame, m: &Machine, (x, y): (i32, i32)) {
    let sprites = m.memory.read16(SPRITE_PAGE.runtime_segment(), SPRITE_PAGE.offset);
    for row in 0..SIGHT_SIZE {
        for column in 0..SIGHT_SIZE {
            let (px, py) = (x + column, y + row);
            if !SHOOTING_ROWS.contains(&py) || frame.get(px, py).is_none() {
                continue;
            }
            let source = (SIGHT_SPRITE.1 + row) * 320 + SIGHT_SPRITE.0 + column;
            let colour = m.memory.read8(sprites, source as u16);
            if colour < 128 || colour == DRAWS_COLOUR_ZERO {
                let index = if colour == DRAWS_COLOUR_ZERO { 0 } else { colour & 15 };
                frame.dot(px, py, m.vga.palette[usize::from(index)]);
            }
        }
    }
}
