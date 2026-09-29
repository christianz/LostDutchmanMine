//! Panning, mining and mules.

use machine::{AluOp, Cond, Machine, Width};
use patches::After;
use patches::resume::{
    AFTER_PANNING_FRAMES, AFTER_PANNING_REWARD, PANNING_FRAME_TEST, PANNING_REWARD_LOOP,
};

use crate::Game;
use crate::hooks::forget_input;
use crate::symbols::{
    INVENTORY_SLOTS, ITEM_NONE, ITEM_PAN, MOUSE_MODE, MULES_OWNED, inventory, set_joystick,
};

/// What pick strokes and the original shooting routine read as Space.
const SPACE_COMMAND: u16 = 0x80;
/// The player's inventory row, then one row per mule.
const ROWS: u16 = 4;
const MULES: u8 = 3;

/// Whether the player is panning, and whether Space is held for mining.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Supplies {
    pub(crate) panning: bool,
    pub(crate) mining_space_held: bool,
}

/// The supplied executable jumps over the panning reward loop. Restored, it
/// runs while panning and the loop's comparison says "less".
pub(super) fn panning_reward_loop(m: &Machine, g: &Game) -> After {
    if g.supplies.panning && m.condition(Cond::Less) {
        After::YieldAt(PANNING_REWARD_LOOP)
    } else {
        After::YieldAt(AFTER_PANNING_REWARD)
    }
}

/// The supplied executable skips all three panning frames. Reconnect the
/// surviving loop: its comparison decides between another frame and the end.
pub(super) fn panning_frame_loop(m: &Machine) -> After {
    if m.condition(Cond::GreaterOrEqual) {
        After::Goto(AFTER_PANNING_FRAMES)
    } else {
        After::Goto(PANNING_FRAME_TEST)
    }
}

/// Whether the player or an owned mule carries a pan. The separate pan counter
/// can be stale after a rejected purchase with a full pack; slot 0 of each row
/// is its carrier's icon.
pub(crate) fn has_pan(m: &Machine) -> bool {
    (0..ROWS)
        .filter(|&row| row == 0 || MULES_OWNED.nth(row - 1).get(m) != 0)
        .any(|row| (1..INVENTORY_SLOTS).any(|slot| inventory(slot, row).get(m) == ITEM_PAN))
}

/// The Pan label and command compare the pan counter with zero; compare
/// actual ownership instead.
pub(super) fn pan_ownership(m: &mut Machine) -> After {
    m.alu(AluOp::Sub, u16::from(has_pan(m)), 0, Width::Word);
    After::Continue
}

/// The translated original owns the animation, delays, inventory and cleanup.
/// With a full pack it goes straight to its message instead of making the
/// player wait; otherwise input made before panning must not act during it.
pub(super) fn begin_panning(m: &mut Machine, g: &mut Game) -> After {
    let room = (1..INVENTORY_SLOTS).any(|slot| inventory(slot, 0).get(m) == ITEM_NONE);
    g.supplies.panning = room && has_pan(m);
    if g.supplies.panning {
        forget_panning_input(m, g);
    }
    After::Continue
}

/// Panning ends; input made during it is dropped.
pub(super) fn finish_panning(m: &mut Machine, g: &mut Game) -> After {
    if g.supplies.panning {
        forget_panning_input(m, g);
    }
    g.supplies.panning = false;
    After::Continue
}

fn forget_panning_input(m: &mut Machine, g: &mut Game) {
    forget_input(m, g);
    set_joystick(m, 0);
}

/// Whether mule `index` is for sale: not owned, and with QoL only once every
/// cheaper mule is sold.
pub(crate) fn mule_available(g: &Game, index: u8, owned: impl Fn(u8) -> bool) -> bool {
    index < MULES && !owned(index) && (!g.qol || (0..index).all(&owned))
}

/// The shop compares a mule's ownership with zero; compare its availability.
pub(super) fn mule_for_sale(m: &mut Machine, g: &Game, index: u8) -> After {
    let available = mule_available(g, index, |i| MULES_OWNED.nth(u16::from(i)).get(m) != 0);
    m.alu(AluOp::Sub, u16::from(!available), 0, Width::Word);
    After::Continue
}

/// Holding Space keeps the pick swinging at its original stroke rate.
pub(super) fn continue_mining(m: &mut Machine, g: &Game) -> After {
    if m.regs.ax == 0 && g.supplies.mining_space_held && MOUSE_MODE.get(m) == 1 {
        m.regs.ax = SPACE_COMMAND;
    }
    After::Continue
}
