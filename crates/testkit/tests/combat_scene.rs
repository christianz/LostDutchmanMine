//! The C++ `combat-scene` test: real encounters, Native Americans and a wanted
//! criminal, each entered from the hand cursor with a held panel click and from
//! the keys. The sight shows mouse aim at display rate without advancing the
//! encounter, matches the original sprite and blitter pixel for pixel, hides
//! under menus, and a click hits through the original ammunition and hit test.
//!
//! Not ported: the C++ test also wrote PPM captures of each step to
//! `captures/combat/`, evidence for people rather than a check.

use game::Frame;
use game::symbols::{
    BULLETS, ENCOUNTER_KIND, ENCOUNTER_WON, GUNS, Global, MOUSE_MODE, POSITION_X, POSITION_Y,
    SCENE_ENCOUNTER, SURVIVAL_TICKS, VGA_FRAMEBUFFER,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL};

/// The encounter, shared by bandits and Native Americans.
const ENCOUNTER: Address = Address::new(0x040a, 0x0002);
/// Where the encounter's shot returns to its loop.
const AFTER_SHOT: Address = Address::new(0x040a, 0x0300);
/// The encounter type: 1 for Native Americans, 0 for bandits. Not saved.
const NATIVE_AMERICANS: Global = Global(0x533c);
/// The wanted criminal's encounter kind.
const WANTED_CRIMINAL: u16 = 10;
/// A word the C++ fixture cleared before entering the encounter.
const CLEARED_BEFORE_ENCOUNTER: Global = Global(0x4eec);
/// Hits the encounter has registered.
const HITS: Global = Global(0x5312);
/// The encounter's local holding its 24-pixel target's x, below BP.
const TARGET_X_BELOW_BP: u16 = 0x16;
/// The local holding the target's y.
const TARGET_Y_BELOW_BP: u16 = 0x18;
/// The original sprite blitter the sight is drawn with.
const BLIT_SPRITE: Address = Address::new(0x0fc5, 0x0e8a);
/// The original page presenter that shows it.
const PRESENT: Address = Address::new(0x0505, 0x0254);
/// A stack for the comparison's calls, clear of the suspended encounter's.
const COMPARISON_STACK: u16 = 0x7000;
/// The sight's width and height.
const SIGHT: u16 = 16;
/// The sight's corner on the sprite page.
const SIGHT_SPRITE: (u16, u16) = (120, 160);
/// An encounter reaches its next input, or a shot returns, within this many steps.
const FRAME_LIMIT: u64 = 2_000_000;
/// A comparison call returns within this many steps.
const COMPARISON_LIMIT: u64 = 100_000;

/// Runs the encounter to its next input poll.
fn next_input(h: &mut Harness) {
    h.step();
    h.until(FRAME_LIMIT, "Encounter did not reach its next input", |h| h.at(INPUT_POLL));
}

/// The sight's committed corner.
fn sight(h: &Harness) -> (u16, u16) {
    (h.get(POSITION_X), h.get(POSITION_Y))
}

/// Repaints just the original sight on the displayed background, through the
/// original blitter and presenter on a stack of their own, and checks the
/// display-rate sight matches it; then resumes the suspended encounter.
fn compare_original_sight(h: &mut Harness) {
    let suspended = h.m.regs;
    let displayed = h.frame();
    let report = h.report();
    let (x, y) = (report.sight_x as u16, report.sight_y as u16);
    h.with_clock_stopped(|h| {
        let (sprite_x, sprite_y) = SIGHT_SPRITE;
        let blit = [1, 0, 12, 0, sprite_x, sprite_y, x, y, SIGHT, SIGHT];
        for (routine, args) in [(BLIT_SPRITE, &blit[..]), (PRESENT, &[][..])] {
            h.call_with_stack(COMPARISON_STACK, routine, args);
            h.until(
                COMPARISON_LIMIT,
                "Original sight comparison did not return",
                Harness::returned,
            );
        }
    });
    let screen = &h.m.memory.as_bytes()[VGA_FRAMEBUFFER..];
    for row in 0..SIGHT {
        for column in 0..SIGHT {
            let at = usize::from(y + row) * Frame::WIDTH + usize::from(x + column);
            assert_eq!(
                displayed.pixels()[at],
                h.m.vga.palette[usize::from(screen[at])],
                "Display-rate crosshair differs from original sprite/blitter"
            );
        }
    }
    h.m.regs = suspended;
}

/// Enters an armed encounter, a wanted criminal's or Native Americans', from
/// the hand cursor holding a panel click or from the keys, and aims, previews,
/// compares and fires.
fn encounter(wanted: bool, from_hand: bool) {
    let mut h = Harness::in_town();
    h.keyboard().clear();
    h.mouse().clear();
    h.set(NATIVE_AMERICANS, u16::from(!wanted));
    h.set(CLEARED_BEFORE_ENCOUNTER, 0);
    h.set(ENCOUNTER_WON, 0);
    h.set(ENCOUNTER_KIND, if wanted { WANTED_CRIMINAL } else { 0 });
    h.set(MOUSE_MODE, u16::from(!from_hand));
    if from_hand {
        h.mouse().move_to(100, 147);
        h.mouse().buttons(1);
    }
    h.set(SCENE_ENCOUNTER, 1);
    h.set(GUNS, 1);
    h.set(BULLETS, 20);
    h.call(ENCOUNTER, &[]);
    next_input(&mut h);
    assert!(h.game.combat_active(), "Original encounter hook did not activate");
    if from_hand {
        // Still holding the previous screen's panel click.
        next_input(&mut h);
    }
    h.mouse().move_to(80, 45);
    next_input(&mut h);
    assert_eq!(sight(&h), (72, 37), "Encounter did not accept mouse aim on its first input");
    assert_eq!(h.get(BULLETS), 20, "Entering the encounter fired without a new click");
    h.mouse().move_to(245, 75);
    next_input(&mut h);
    assert_eq!(sight(&h), (237, 67), "Original crosshair did not follow rightward aim");

    let (steps, bullets, clock) = (h.m.steps, h.get(BULLETS), h.get(SURVIVAL_TICKS));
    let before = h.frame();
    h.mouse().move_to(100, 40);
    let (moved, report) = (h.frame(), h.report());
    assert!(
        (report.sight_x, report.sight_y) == (92, 32) && before != moved,
        "Crosshair still waits for an encounter tick after mouse motion"
    );
    assert!(
        h.m.steps == steps
            && h.get(BULLETS) == bullets
            && h.get(SURVIVAL_TICKS) == clock
            && sight(&h) == (237, 67),
        "Display-rate motion advanced shots, aim simulation or the world clock"
    );
    h.game.open_menu();
    let before = h.frame();
    h.mouse().move_to(190, 30);
    assert!(before == h.frame(), "Crosshair drew over an open status menu");
    h.game.close_menu();
    h.mouse().move_to(245, 75);
    compare_original_sight(&mut h);

    h.mouse().buttons(0);
    next_input(&mut h);
    // The original hit test compares the sight's x - 8 with the 24-pixel
    // target's x, and the sight's y with its y. Aim there and fire for real.
    let local =
        |h: &Harness, below: u16| h.m.memory.read16(h.m.regs.ss, h.m.regs.bp.wrapping_sub(below));
    let target = (local(&h, TARGET_X_BELOW_BP), local(&h, TARGET_Y_BELOW_BP));
    assert_eq!(h.get(HITS), 0, "Encounter fixture already registered a hit");
    h.mouse().move_to(i32::from(target.0) + 16, i32::from(target.1) + 8);
    h.mouse().buttons(1);
    h.mouse().buttons(0);
    h.step();
    h.until(FRAME_LIMIT, "Original shot did not return", |h| h.at(AFTER_SHOT));
    assert_eq!(h.get(BULLETS), 19, "Mouse firing did not spend one original bullet");
    assert_eq!(h.get(HITS), 1, "Mouse aim did not hit through original hit testing");
    assert_eq!(h.get(MOUSE_MODE), 1, "Mouse shot unexpectedly entered hand mode");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn native_americans_entered_from_the_hand() {
    encounter(false, true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn native_americans_entered_from_the_keys() {
    encounter(false, false);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_wanted_criminal_entered_from_the_hand() {
    encounter(true, true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_wanted_criminal_entered_from_the_keys() {
    encounter(true, false);
}
