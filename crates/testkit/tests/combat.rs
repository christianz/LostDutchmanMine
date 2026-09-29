//! `tests/combat.cpp`: the original encounter input loop with QoL mouse aim.
//! The sight follows the pointer and a press fires once at its own position;
//! keys and Space still aim and fire; aim stays in bounds; focus loss resets;
//! and the hand cursor, status panel, victory choices, fights without a gun
//! and QoL off keep the original input.
//!
//! Not ported: the C++ test also asserted after every poll that the hooks'
//! "reading combat input" flag was clear. In Rust that flag is private to the
//! combat hook, which clears it as each read finishes; nothing outside the
//! hook can observe it.

use game::symbols::{ENCOUNTER_WON, GUNS, MOUSE_MODE, POSITION_X, POSITION_Y};
use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The encounter's aiming read, entered mid-routine.
const AIMING_READ: Address = Address::new(0x040a, 0x0288);
/// The encounter's frame the read runs in: its BP.
const FRAME: u16 = STACK_TOP;
/// The stack below that frame when the read begins.
const FRAME_STACK: u16 = 0x7fc0;
/// Where a read that shoots ends: the original shot.
const SHOT: Address = Address::new(0x040a, 0x09e2);
/// Where any other read ends: the encounter's delay.
const DELAY: Address = Address::new(0x0505, 0x008c);
/// The stack a shot leaves.
const SHOT_STACK: u16 = 0x7fb6;
/// The stack the delay leaves.
const DELAY_STACK: u16 = 0x7fba;
/// A read finishes well within this many steps.
const READ_LIMIT: u64 = 2000;
/// Space: scan 39h, character 20h.
const SPACE: u32 = 0x3920;

/// An armed encounter with its sight at (160, 64), aiming with the keys.
fn encounter() -> Harness {
    let mut h = Harness::loaded();
    h.install_mouse_driver();
    h.attach_joystick();
    h.set(MOUSE_MODE, 1);
    h.set(GUNS, 1);
    h.set(POSITION_X, 160);
    h.set(POSITION_Y, 64);
    h.game.begin_combat(&mut h.m).expect("the encounter begins");
    h
}

/// Runs one aiming read; true when it ends in the original shot.
fn poll(h: &mut Harness) -> bool {
    h.jump(AIMING_READ);
    (h.m.regs.bp, h.m.regs.sp) = (FRAME, FRAME_STACK);
    let start = h.m.steps;
    loop {
        h.step();
        assert!(h.m.steps - start < READ_LIMIT, "Combat input did not finish");
        let (shot, delay) = (h.at(SHOT), h.at(DELAY));
        if shot || delay {
            let stack = if shot { SHOT_STACK } else { DELAY_STACK };
            assert_eq!(h.m.regs.sp, stack, "Combat input corrupted the original call stack");
            return shot;
        }
    }
}

/// Focus loss: buttons, queued clicks, held directions and keys are dropped,
/// and aiming is back on the keys.
fn clear(h: &mut Harness) {
    h.mouse().clear();
    h.game.reset_combat_pointer();
    h.set_movement(0);
    h.keyboard().clear();
    h.set(MOUSE_MODE, 1);
}

/// The sight's corner.
fn sight(h: &Harness) -> (u16, u16) {
    (h.get(POSITION_X), h.get(POSITION_Y))
}

/// The desktop pointer moves to (`x`, `y`).
fn point(h: &mut Harness, x: i32, y: i32) {
    h.mouse().move_to(x, y);
}

/// The desktop holds `buttons`.
fn press(h: &mut Harness, buttons: u8) {
    h.mouse().buttons(buttons);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn an_entry_click_from_the_hand_is_no_shot() {
    let mut h = encounter();
    for held in [false, true] {
        clear(&mut h);
        h.set(MOUSE_MODE, 0);
        point(&mut h, 100, 147);
        press(&mut h, 1);
        if !held {
            press(&mut h, 0);
        }
        h.game.begin_combat(&mut h.m).expect("the encounter begins");
        assert!(
            !poll(&mut h) && h.get(MOUSE_MODE) == 1,
            "Entry click held over the old panel reopened the hand cursor"
        );
        point(&mut h, 70, 45);
        assert!(
            !poll(&mut h) && sight(&h) == (62, 37) && h.get(MOUSE_MODE) == 1,
            "Hand-mode encounter entry did not enable immediate aim or replayed its entry click"
        );
        assert!(!poll(&mut h), "Holding the entry click fired a shot");
        press(&mut h, 0);
        assert!(!poll(&mut h), "Releasing the entry click fired a shot");
        press(&mut h, 1);
        assert!(poll(&mut h), "Fresh press after encounter entry did not fire");
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_mouse_aims_and_fires_beside_the_keys() {
    let mut h = encounter();
    clear(&mut h);
    point(&mut h, 160, 100);
    h.game.reset_combat_pointer();
    assert!(
        !poll(&mut h) && sight(&h) == (160, 64),
        "Entering combat must not jump to a stationary mouse"
    );
    point(&mut h, 70, 45);
    assert!(
        !poll(&mut h) && sight(&h) == (62, 37),
        "Mouse motion must position the original sight's centre"
    );
    h.set_movement(8);
    assert!(
        !poll(&mut h) && sight(&h) == (72, 37),
        "Stationary mouse must not undo keyboard aiming"
    );
    assert!(
        !poll(&mut h) && sight(&h) == (82, 37),
        "Held aiming must keep working without mouse motion"
    );
    point(&mut h, 240, 80);
    assert!(
        !poll(&mut h) && sight(&h) == (232, 72),
        "Fresh mouse aim must win over a simultaneous direction"
    );
    h.set_movement(0);
    point(&mut h, 80, 60);
    press(&mut h, 1);
    press(&mut h, 0);
    point(&mut h, 245, 80);
    assert!(
        poll(&mut h) && sight(&h) == (72, 52),
        "Quick click must fire at the latched click position"
    );
    assert!(!poll(&mut h), "Release must not fire a second shot");
    assert!(
        !poll(&mut h) && sight(&h) == (237, 72),
        "Latest mouse motion must resume after queued click edges"
    );
    assert_eq!(h.get(MOUSE_MODE), 1, "Firing must not enter hand mode");
    press(&mut h, 1);
    assert!(poll(&mut h), "Left press must fire");
    assert!(!poll(&mut h) && !poll(&mut h), "Holding the mouse must not empty the ammunition");

    clear(&mut h);
    point(&mut h, 100, 70);
    press(&mut h, 1);
    clear(&mut h);
    assert!(
        !poll(&mut h) && sight(&h) == (237, 72),
        "Focus loss must clear queued shots and stale aim"
    );
    point(&mut h, 0, 0);
    assert!(!poll(&mut h) && sight(&h) == (20, 20), "Aim must stop at the upper-left bounds");
    point(&mut h, 319, 111);
    assert!(!poll(&mut h) && sight(&h) == (290, 84), "Aim must stop at the lower-right bounds");
    point(&mut h, 90, 140);
    assert!(!poll(&mut h) && sight(&h) == (290, 84), "Status panel motion must not move the sight");
    press(&mut h, 1);
    assert!(!poll(&mut h) && h.get(MOUSE_MODE) == 0, "Status clicks must retain the hand cursor");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn hand_victory_unarmed_and_classic_input_stay_original() {
    let mut h = encounter();
    point(&mut h, 319, 111);
    assert!(!poll(&mut h) && sight(&h) == (290, 84), "Aim must stop at the lower-right bounds");
    clear(&mut h);
    point(&mut h, 90, 60);
    press(&mut h, 2);
    assert!(
        !poll(&mut h) && sight(&h) == (290, 84) && h.get(MOUSE_MODE) == 0,
        "Right click must open hand mode without aiming or firing"
    );
    clear(&mut h);
    h.set(MOUSE_MODE, 0);
    point(&mut h, 50, 30);
    press(&mut h, 1);
    assert!(
        !poll(&mut h) && sight(&h) == (290, 84),
        "Hand menus must retain their original selection input"
    );
    clear(&mut h);
    h.set(ENCOUNTER_WON, 1);
    point(&mut h, 80, 40);
    press(&mut h, 1);
    assert!(!poll(&mut h) && sight(&h) == (290, 84), "Victory choices must not fire or aim");
    clear(&mut h);
    h.set(ENCOUNTER_WON, 0);
    h.set(GUNS, 0);
    point(&mut h, 110, 60);
    press(&mut h, 1);
    assert!(
        !poll(&mut h) && sight(&h) == (290, 84),
        "No gun must preserve the original input path"
    );

    clear(&mut h);
    h.set(GUNS, 1);
    h.game.set_qol_flag(false);
    point(&mut h, 70, 45);
    h.set(MOUSE_MODE, 0);
    h.game.begin_combat(&mut h.m).expect("the encounter begins");
    assert_eq!(h.get(MOUSE_MODE), 0, "QoL off changed the encounter entry mode");
    h.set(MOUSE_MODE, 1);
    assert!(!poll(&mut h) && sight(&h) == (290, 84), "QoL off must retain original aiming");
    press(&mut h, 1);
    assert!(!poll(&mut h) && h.get(MOUSE_MODE) == 0, "QoL off must retain mouse selection");
    clear(&mut h);
    h.keyboard().push(SPACE);
    assert!(poll(&mut h), "Space must still call the original shooting routine");
}
