//! Walking and the keys that steer it.
//!
//! With QoL a held direction walks twice as many steps, each with half the
//! original delay, while every other step skips the survival clock and the
//! mine's hazards, so time and danger pass at the original rate per distance.

use dos::direction_scan;
use machine::Machine;
use machine::state::{Reader, StateError, Writer};
use patches::After;
use patches::resume::{AFTER_HAZARD_CHECK, WALKING};

use crate::Game;
use crate::symbols::joystick;

/// The joystick port's four direction bits.
const DIRECTIONS: u8 = 15;

/// Whether this walking step is fast, and whether it is the extra one.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Walk {
    fast: bool,
    extra_tick: bool,
}

impl Walk {
    pub(crate) fn save(self, w: &mut Writer) {
        w.bool(self.fast);
        w.bool(self.extra_tick);
    }

    pub(crate) fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        (self.fast, self.extra_tick) = (r.bool()?, r.bool()?);
        Ok(())
    }
}

/// A walking step begins.
pub(super) fn begin_tick(m: &Machine, g: &mut Game) -> After {
    g.walk.fast = g.qol && joystick(m) & DIRECTIONS != 0;
    g.walk.extra_tick = g.walk.fast && !g.walk.extra_tick;
    After::Continue
}

/// The extra fast step leaves the survival clock alone.
pub(super) fn skip_extra_tick(g: &Game, next: u16) -> After {
    if g.walk.extra_tick { After::Goto(next) } else { After::Continue }
}

/// The extra fast step in the mine leaves its hazards alone.
pub(super) fn skip_extra_hazard_check(g: &Game) -> After {
    if g.walk.extra_tick { After::Goto(AFTER_HAZARD_CHECK) } else { After::Continue }
}

/// A fast step waits half the original delay, the argument on top of the stack.
pub(super) fn halve_delay(m: &mut Machine, g: &Game) -> After {
    if g.walk.fast {
        let (ss, sp) = (m.regs.ss, m.regs.sp);
        let delay = m.memory.read16(ss, sp);
        m.memory.write16(ss, sp, delay.div_ceil(2));
    }
    After::Continue
}

/// A movement reader (true) or any other reader (false) polls the keyboard:
/// only movement readers see movement scans, so text fields get letters.
pub(super) fn movement_keys(g: &mut Game, movement: bool) -> After {
    g.dos.keyboard.movement_aliases = movement;
    After::Continue
}

/// SI already holds the combined held directions; a single key's scan, often
/// an auto-repeat, must not replace them with one axis. Taps without held
/// input, Space and menu keys keep their paths.
pub(super) fn keep_held_directions(m: &mut Machine) -> After {
    if direction_scan(m.regs.di) && m.regs.si & u16::from(DIRECTIONS) != 0 {
        m.regs.di = 0;
    }
    After::Continue
}

/// In the hand-cursor loop a direction is already forwarded, but the original
/// waits for a right click before walking again. Take its own return path.
pub(super) fn return_to_walking(m: &Machine) -> After {
    if direction_scan(m.regs.ax) { After::Goto(WALKING) } else { After::Continue }
}
