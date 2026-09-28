//! Mouse aiming in encounters.
//!
//! The encounter's six-tick loop still owns enemies, shots and hit tests. With
//! QoL, while armed and aiming, the sight follows the pointer and a left press
//! in the shooting area fires once where it was pressed. Hand mode, status
//! menus, victory choices and fights without a gun keep their original input.

use dos::MouseSample;
use machine::Machine;
use machine::state::{Reader, StateError, Writer};
use patches::After;

use crate::Game;
use crate::symbols::{AIM_FLOOR, ENCOUNTER_WON, GUNS, MOUSE_MODE, POSITION_X, POSITION_Y};

/// What the original shooting routine reads as a shot.
const SHOT: u16 = 0x80;
/// Rows above this are the shooting area; below is the status panel.
const SHOOTING_AREA_END: i32 = 112;
/// The sight is 16x16 and positioned by its top-left corner.
const SIGHT_HALF: i32 = 8;
/// The sight's top-left corner stays within these bounds.
const AIM_LEFT: i32 = 20;
const AIM_RIGHT: i32 = 290;
const AIM_TOP: i32 = 20;
/// The lowest corner row, before the encounter raises it by [`AIM_FLOOR`].
const AIM_BOTTOM: i32 = 94;

/// The encounter's aiming read, while it runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AimingRead {
    /// Not reading, or reading without the mouse.
    #[default]
    Off,
    /// The mouse aims.
    Aiming,
    /// The mouse aims, and a fresh press fires.
    Firing,
}

/// The encounter's state as the port sees it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Combat {
    pub(crate) active: bool,
    read: AimingRead,
    sight_pending: bool,
    pub(crate) sight_visible: bool,
    pointer: MouseSample,
}

impl Combat {
    pub(crate) fn save(&self, w: &mut Writer) {
        w.bool(self.active);
        w.u8(match self.read {
            AimingRead::Off => 0,
            AimingRead::Aiming => 1,
            AimingRead::Firing => 2,
        });
        w.bool(self.sight_pending);
        w.bool(self.sight_visible);
        crate::state::save_sample(w, self.pointer);
    }

    pub(crate) fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        self.active = r.bool()?;
        self.read = match r.u8()? {
            0 => AimingRead::Off,
            1 => AimingRead::Aiming,
            2 => AimingRead::Firing,
            other => return Err(StateError::Invalid(format!("aiming read {other}"))),
        };
        self.sight_pending = r.bool()?;
        self.sight_visible = r.bool()?;
        self.pointer = crate::state::restore_sample(r)?;
        Ok(())
    }

    /// Baselines on `current`: only motion from here moves the sight, and a
    /// press made before it is no shot.
    pub(crate) fn reset_pointer(&mut self, current: MouseSample) {
        self.pointer = current;
        if self.read == AimingRead::Firing {
            self.read = AimingRead::Aiming;
        }
    }
}

/// Clamps like the C++ oracle's `std::clamp`, which never panics.
fn bound(value: i32, low: i32, high: i32) -> i32 {
    if value < low {
        low
    } else if high < value {
        high
    } else {
        value
    }
}

/// The sight's top-left corner that keeps its centre under `point`.
fn sight_corner(point: MouseSample, aim_floor: u16) -> (i32, i32) {
    (
        bound(point.x - SIGHT_HALF, AIM_LEFT, AIM_RIGHT),
        bound(point.y - SIGHT_HALF, AIM_TOP, AIM_BOTTOM - i32::from(aim_floor)),
    )
}

/// Whether the encounter is aiming with a gun and QoL, reading globals `read`.
fn armed_and_aiming(g: &Game, read: impl Fn(crate::symbols::Global) -> u16) -> bool {
    g.qol && read(MOUSE_MODE) == 1 && read(ENCOUNTER_WON) == 0 && read(GUNS) > 0
}

/// A fight starts, perhaps from a map or building hand cursor: aim at once
/// when armed. The click that entered the encounter is not a shot, so a
/// release and a fresh press are needed.
pub(super) fn begin(m: &mut Machine, g: &mut Game) -> After {
    if g.qol && ENCOUNTER_WON.get(m) == 0 && GUNS.get(m) > 0 {
        MOUSE_MODE.set(m, 1);
        g.dos.mouse.input.discard_pending();
    }
    g.combat = Combat { active: true, ..Combat::default() };
    g.combat.reset_pointer(g.dos.mouse.input.current());
    After::Continue
}

/// Only the encounter's aiming read opts in.
pub(super) fn begin_input(m: &Machine, g: &mut Game) -> After {
    let aiming = armed_and_aiming(g, |global| global.get(m));
    g.combat.read = if aiming { AimingRead::Aiming } else { AimingRead::Off };
    After::Continue
}

/// The original mouse read would open the hand. A press carried in from the
/// previous screen must not, even over its old panel button; a fresh left
/// press in the shooting area fires instead.
pub(super) fn filter_mouse(m: &mut Machine, g: &mut Game) -> After {
    let point = g.dos.mouse.input.sample();
    if g.combat.read == AimingRead::Off {
        return After::Continue;
    }
    if point.buttons != 0 && point.buttons == g.combat.pointer.buttons {
        m.regs.ax = 0;
    } else if point.buttons == 1 && point.y < SHOOTING_AREA_END {
        let fresh = g.combat.pointer.buttons & 1 == 0;
        g.combat.read = if fresh { AimingRead::Firing } else { AimingRead::Aiming };
        m.regs.ax = 0;
    }
    After::Continue
}

/// The aiming read ends: a moved pointer places the sight, taking priority
/// over a simultaneous direction, and a fresh press shoots. A stationary mouse
/// never undoes keyboard aiming.
pub(super) fn finish_input(m: &mut Machine, g: &mut Game) -> After {
    let point = g.dos.mouse.input.sample();
    let combat = &mut g.combat;
    let fire = combat.read == AimingRead::Firing;
    if combat.read != AimingRead::Off && point.y < SHOOTING_AREA_END && point.buttons & 2 == 0 {
        let moved = (point.x, point.y) != (combat.pointer.x, combat.pointer.y);
        if fire || moved {
            let (x, y) = sight_corner(point, AIM_FLOOR.get(m));
            POSITION_X.set(m, x as u16);
            POSITION_Y.set(m, y as u16);
            if m.regs.ax < SHOT {
                m.regs.ax = 0;
            }
        }
        if fire {
            m.regs.ax = SHOT;
        }
    }
    combat.pointer = point;
    combat.read = AimingRead::Off;
    After::Continue
}

/// The encounter has drawn its scene: the sight may show.
pub(super) fn show_sight(g: &mut Game) -> After {
    g.combat.sight_visible = g.combat.sight_pending;
    After::Continue
}

/// The fight is over.
pub(super) fn end(g: &mut Game) -> After {
    g.combat.active = false;
    g.combat.read = AimingRead::Off;
    After::Continue
}

/// With QoL the frame draws the sight at display cadence, not this blit.
pub(super) fn defer_sight(g: &mut Game, next: u16) -> After {
    g.combat.sight_pending = g.qol;
    if g.combat.sight_pending { After::Goto(next) } else { After::Continue }
}

/// Where the sight shows now. Motion previews every display frame; only the
/// original input path commits aim, so a stationary mouse yields to the keys.
/// Frames are composed between runs, so globals are read at rest.
pub(crate) fn aim(m: &Machine, g: &Game) -> (i32, i32) {
    let point = g.dos.mouse.input.current();
    let moved = (point.x, point.y) != (g.combat.pointer.x, g.combat.pointer.y);
    if g.combat.active
        && armed_and_aiming(g, |global| global.at_rest(m))
        && point.y < SHOOTING_AREA_END
        && point.buttons & 2 == 0
        && moved
    {
        return sight_corner(point, AIM_FLOOR.at_rest(m));
    }
    (i32::from(POSITION_X.at_rest(m)), i32::from(POSITION_Y.at_rest(m)))
}
