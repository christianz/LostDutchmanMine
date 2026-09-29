//! Scene changes the port corrects: the desert close-up, a loaded saloon's
//! doorway, and the graphics selector at startup.

use dos::{KEY_REPEAT, bios_key};
use machine::Machine;
use patches::After;
use patches::resume::VGA_CHOSEN;

use crate::Game;
use crate::hooks::forget_input;
use crate::symbols::{LOADED_SCENE, RETURN_X, VGA_CHOICE};

/// The scans that dismiss a close-up: Space, Enter and Escape.
const DISMISS: [u16; 3] = [0x39, 0x1c, 0x01];

/// The Space close-up on the map was a timed preview, so a held Space opened
/// the next one. With QoL, or once one is showing, it waits for a fresh
/// dismissal, which belongs to the close-up and never to the next map poll.
pub(super) fn desert_close_up(m: &mut Machine, g: &mut Game, next: u16) -> After {
    if !g.qol && !g.desert_view {
        return After::Original;
    }
    g.desert_view = true;
    let dismissed = g
        .dos
        .keyboard
        .iter()
        .any(|key| key & KEY_REPEAT == 0 && DISMISS.contains(&(bios_key(key, false) >> 8)));
    if !dismissed {
        g.dos.waiting = true;
        return After::Yield;
    }
    forget_input(m, g);
    g.desert_view = false;
    g.dos.waiting = false;
    After::Goto(next)
}

/// Leaving a saloon returns to its doorway, which a loaded game already set.
pub(super) fn saloon_return_position(m: &mut Machine) -> After {
    if LOADED_SCENE.get(m) == 0 {
        RETURN_X.set(m, m.regs.ax);
    }
    After::Continue
}

/// Choose VGA as if the player had, without showing the selector.
pub(super) fn skip_graphics_selector(m: &mut Machine) -> After {
    m.regs.ax = VGA_CHOICE;
    After::Goto(VGA_CHOSEN)
}
