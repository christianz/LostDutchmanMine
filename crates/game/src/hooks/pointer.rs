//! The walking pointer: with QoL, walking and the mouse coexist.
//!
//! The original walking poll either walks with the keys or, after a click,
//! enters a modal hand cursor. With QoL the poll keeps walking; a fresh left
//! press is latched and later dispatched through the original selector, as if
//! the hand had chosen it.

use dos::MouseSample;
use machine::Machine;
use machine::state::{Reader, StateError, Writer};
use patches::After;
use patches::resume::{SELECTION, SELECTOR_CALL, WALKING};

use crate::Game;

/// The selector's result for a click on nothing: it would restart the scene.
const RESTART_SCENE: u16 = 9;

/// The last pointer sample the walking poll saw, and a click waiting for the
/// selector.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WorldPointer {
    last: MouseSample,
    click: Option<MouseSample>,
}

impl WorldPointer {
    pub(crate) fn save(&self, w: &mut Writer) {
        crate::state::save_sample(w, self.last);
        w.bool(self.click.is_some());
        crate::state::save_sample(w, self.click.unwrap_or_default());
    }

    pub(crate) fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        self.last = crate::state::restore_sample(r)?;
        let pending = r.bool()?;
        let click = crate::state::restore_sample(r)?;
        self.click = pending.then_some(click);
        Ok(())
    }

    /// Baselines on `current` and forgets a waiting click.
    pub(crate) fn reset(&mut self, current: MouseSample) {
        self.last = current;
        self.click = None;
    }
}

/// One desktop mouse sample for the whole original mouse read.
pub(super) fn poll(g: &mut Game) -> After {
    g.dos.mouse.input.poll();
    After::Continue
}

/// The walking poll's mouse read: latch a fresh left press and keep walking.
pub(super) fn filter(m: &mut Machine, g: &mut Game) -> After {
    let point = g.dos.mouse.input.sample();
    if g.qol && !g.combat.active {
        if point.buttons & 1 != 0 && g.pointer.last.buttons & 1 == 0 {
            g.pointer.click = Some(point);
        }
        m.regs.ax = 0;
    }
    g.pointer.last = point;
    After::Continue
}

/// The selector's setup: the physical pointer stays where it is.
pub(super) fn reset(g: &mut Game) -> After {
    g.pointer.reset(g.dos.mouse.input.current());
    After::Continue
}

/// A latched click goes straight to the selector.
pub(super) fn dispatch_pending(g: &Game) -> After {
    if g.qol && g.pointer.click.is_some() { After::Goto(SELECTOR_CALL) } else { After::Continue }
}

/// With QoL a click on nothing keeps the scene, and so the cave's return position.
pub(super) fn keep_scene(m: &mut Machine, g: &Game) -> After {
    if g.qol && !g.combat.active && m.regs.ax == RESTART_SCENE {
        m.regs.ax = 0;
    }
    After::Continue
}

/// Hands the latched click to the selector through its locals: x, y and a
/// press, just below BP.
pub(super) fn dispatch(m: &mut Machine, g: &mut Game) -> After {
    let Some(click) = g.pointer.click.take().filter(|_| g.qol) else {
        return After::Continue;
    };
    let (ss, bp) = (m.regs.ss, m.regs.bp);
    m.memory.write16(ss, bp.wrapping_sub(2), click.x as u16);
    m.memory.write16(ss, bp.wrapping_sub(4), click.y as u16);
    m.memory.write16(ss, bp.wrapping_sub(6), 1);
    After::Goto(SELECTION)
}

/// With QoL, no selection means walking on instead of entering the hand.
pub(super) fn keep_walking(m: &Machine, g: &Game) -> After {
    if g.qol && !g.combat.active && m.regs.ax == 0 { After::Goto(WALKING) } else { After::Continue }
}
