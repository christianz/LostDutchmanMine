//! Drawing the overlay onto a composed frame.

use machine::Machine;

use super::{ATLAS_SIZE, Button, TOOLS};
use crate::Game;
use crate::hooks::mule_available;
use crate::pixels::{BLACK, Frame, is_black, rgb};
use crate::symbols::{MULES_OWNED, VGA_FRAMEBUFFER, VIDEO_MODE_VGA};

const INK: u32 = BLACK;
const CREAM: u32 = rgb(0xee, 0xe2, 0xbb);
const GOLD: u32 = rgb(0xff, 0xd3, 0x4e);
const SHADOW: u32 = rgb(0x55, 0x55, 0x55);
/// A button face before the artwork has loaded.
const PLAIN: u32 = rgb(0xaa, 0xaa, 0xaa);

const NAMES: [&str; 6] = ["Cash", "Life", "Food", "Tools", "Ammo", "Game"];
const HINTS: [&str; 6] = [
    "Cash - F1",
    "Health - F2",
    "Food / Drink - F3",
    "Tools / Pack - F4",
    "Ammunition - F5",
    "Save / Load / Quit - F6",
];

/// The original screen, as colour indices.
fn screen(m: &Machine) -> &[u8] {
    &m.memory.as_bytes()[VGA_FRAMEBUFFER..VGA_FRAMEBUFFER + ATLAS_SIZE]
}

/// Whether the original panel is on screen. Modal screens replace all or part
/// of it, so its six button borders are compared before painting.
fn visible(m: &Machine, g: &Game) -> bool {
    let Some(atlas) = &g.overlay.atlas else { return false };
    let screen = screen(m);
    g.dos.video.mode == VIDEO_MODE_VGA
        && (0..6usize).all(|i| {
            [167usize, 180, 191].iter().all(|&y| {
                [54usize, 84].iter().all(|&x| {
                    let at = y * Frame::WIDTH + x + i * 42;
                    screen[at] == atlas[at]
                })
            })
        })
}

/// Whether a pointer shows: the game's own, or with QoL wherever the panel is
/// on screen outside fights, panning and close-ups.
pub(crate) fn pointer_visible(m: &Machine, g: &Game) -> bool {
    g.dos.mouse.visibility >= 0
        || (g.qol && !g.combat.active && !g.supplies.panning && !g.desert_view && visible(m, g))
}

/// The toolbar, hover outlines and sold-out marks.
pub(crate) fn draw(frame: &mut Frame, m: &Machine, g: &Game) {
    if !g.qol || !visible(m, g) {
        return;
    }
    let pointer = g.dos.mouse.input.current();
    let interactive = pointer_visible(m, g);
    let palette = &m.vga.palette;
    for (i, (name, hint)) in (0..TOOLS).zip(NAMES.iter().zip(HINTS)) {
        let button = Button::tool(i);
        let hover = interactive && !g.overlay.menu_open() && button.contains(pointer.x, pointer.y);
        let pressed = hover && pointer.buttons & 1 != 0;
        let press = i32::from(pressed);
        draw_button(frame, m, g, button, hover, pressed);
        // The live icons: health and food change with the player's condition,
        // and critical health flashes; the atlas holds only the defaults.
        for y in 0..18 {
            for x in 0..24 {
                let source = (166 + y * 26 / 18) * 320 + 56 + i * 42 + x * 28 / 24;
                let colour = palette[usize::from(screen(m)[source as usize])];
                frame.dot(button.x + 8 + x + press, button.y + 3 + y + press, colour);
            }
        }
        text(frame, g, (button.x + 20, button.y + 22 + press), name, INK, true);
        // The top strip belongs to the game's status messages: a hint may use
        // it only while it is entirely blank, never covering game text.
        if hover && frame.pixels()[..Frame::WIDTH * 10].iter().all(|&c| is_black(c)) {
            text(frame, g, (160, 1), hint, CREAM, true);
        }
    }
    for i in 0..4 {
        let action = Button::action(i);
        if interactive
            && g.overlay.context_buttons & 1 << i != 0
            && action.contains(pointer.x, pointer.y)
        {
            outline(frame, action, if pointer.buttons & 1 != 0 { CREAM } else { GOLD });
        }
    }
    if g.overlay.mule_shop_visible {
        let owned = |i: u8| MULES_OWNED.nth(u16::from(i)).at_rest(m) != 0;
        for i in (0..3).filter(|&i| !mule_available(g, i, owned)) {
            // The names start above row 89; row 110 is the shop's border.
            let x = i32::from(i) * 100;
            frame.rect(26 + x, 85, 68, 25, palette[0]);
            text(frame, g, (60 + x, 96), "SOLD OUT", CREAM, true);
        }
    }
}

/// Where a resized button's pixel comes from in the original, keeping the
/// corner bevels.
fn slice(at: i32, size: i32, source: i32, edge: i32) -> i32 {
    if at < edge {
        at
    } else if at >= size - edge {
        source - (size - at)
    } else {
        edge + (at - edge) * (source - 2 * edge) / (size - 2 * edge)
    }
}

/// The original blank 75x19 action button, as Fish, Water and Pan use it.
fn draw_button(
    frame: &mut Frame,
    m: &Machine,
    g: &Game,
    button: Button,
    hover: bool,
    pressed: bool,
) {
    for y in 0..button.height {
        for x in 0..button.width {
            let (sx, sy) = (slice(x, button.width, 75, 6), slice(y, button.height, 19, 4));
            let colour = g.overlay.atlas.as_ref().map_or(PLAIN, |atlas| {
                m.vga.palette[usize::from(atlas[((64 + sy) * 320 + 181 + sx) as usize])]
            });
            frame.dot(button.x + x, button.y + y, colour);
        }
    }
    if hover {
        outline(frame, button, if pressed { CREAM } else { GOLD });
    }
    if pressed {
        let (x, y, right, bottom) =
            (button.x, button.y, button.x + button.width, button.y + button.height);
        frame.line((x + 3, y + 1), (right - 4, y + 1), SHADOW);
        frame.line((x + 1, y + 3), (x + 1, bottom - 4), SHADOW);
    }
}

fn outline(frame: &mut Frame, b: Button, colour: u32) {
    let (right, bottom) = (b.x + b.width - 1, b.y + b.height - 1);
    frame.line((b.x + 2, b.y), (right - 2, b.y), colour);
    frame.line((b.x, b.y + 2), (b.x, bottom - 2), colour);
    frame.line((b.x + 2, bottom), (right - 2, bottom), colour);
    frame.line((right, b.y + 2), (right, bottom - 2), colour);
}

/// A glyph's used columns, and how far it advances: blank glyphs are spaces.
fn span(font: &[u8], character: u8) -> (i32, i32, i32) {
    let rows = &font[usize::from(character) * 8..][..8];
    let bits = rows.iter().fold(0, |bits, &row| bits | row);
    if bits == 0 {
        return (8, 7, 4);
    }
    let (first, last) = (bits.leading_zeros() as i32, 7 - bits.trailing_zeros() as i32);
    (first, last, last - first + 2)
}

/// Text in the game's own font, proportionally spaced; the 5x7 fallback when
/// the font is not loaded.
fn text(frame: &mut Frame, g: &Game, (x, y): (i32, i32), text: &str, colour: u32, centre: bool) {
    let font = &g.overlay.font;
    if font.get(usize::from(b'A') * 8 + 1).is_none_or(|&row| row == 0) {
        frame.fallback_text(x, y, text, colour, centre);
        return;
    }
    let width: i32 = text.bytes().map(|c| span(font, c).2).sum();
    let mut x = if centre { x - (width - 1) / 2 } else { x };
    for character in text.bytes() {
        let (first, last, advance) = span(font, character);
        for (row, &bits) in (0..).zip(&font[usize::from(character) * 8..][..8]) {
            for column in first..=last {
                if bits & (128 >> column) != 0 {
                    frame.dot(x + column - first, y + row, colour);
                }
            }
        }
        x += advance;
    }
}
