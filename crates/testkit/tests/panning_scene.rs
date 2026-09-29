//! `tests/panning-scene.cpp`: the restored panning animation on the game's
//! own sprites, page buffers and timer handler, with the synthetic clock. For
//! every grade, with QoL on and off, it shows the original three poses for
//! five to nine cycles at the original delays, awards exactly one graded bag
//! only once it ends, ignores input and a QoL change made during it, and
//! returns cleanly. The desktop scenarios exercise the river's real Pan button.
//!
//! Not ported: the C++ test also wrote the three poses as PPM captures to
//! `captures/panning-original/`, evidence for people rather than a check.

use game::symbols::{GOLD_BAGS, INVENTORY_SLOTS, ITEM_NONE, ITEM_PAN, PANS, POSITION_Y, inventory};
use machine::Address;
use testkit::harness::{Harness, RETURN_OFFSET, STACK_TOP};

/// The original Pan routine, taking the river's gold grade.
const PAN: Address = Address::new(0x033f, 0x02cc);
/// Where the animation has just presented its left or right pose.
const SIDE_POSE: Address = Address::new(0x033f, 0x0395);
/// Where it has just presented the middle pose on its way back.
const MIDDLE_POSE: Address = Address::new(0x033f, 0x040d);
/// The animation's local holding the presented sprite's x, below BP.
const SPRITE_X_BELOW_BP: u16 = 4;
/// Where the first pose's sprite starts on the sprite page.
const FIRST_POSE_X: i32 = 240;
/// How far apart the three poses' sprites are.
const POSE_WIDTH: i32 = 24;
/// The middle pose.
const MIDDLE: i32 = 1;
/// A graded bag of gold is item 20h plus the grade.
const GOLD_BAG: u16 = 0x20;
/// The animation finishes within this many steps.
const ANIMATION_LIMIT: u64 = 4_000_000;
/// Each pose waits at least this many timer ticks.
const TICKS_PER_POSE: u64 = 12;
/// Space: scan 39h, character 20h.
const SPACE: u32 = 0x3920;

/// Pans at `grade` with QoL `qol` from the town, and checks the animation.
fn animate(h: &mut Harness, qol: bool, grade: u16) {
    h.game.set_qol_flag(qol);
    h.set(PANS, 1);
    h.set(GOLD_BAGS, 0);
    for slot in 1..INVENTORY_SLOTS {
        h.set(inventory(slot, 0), ITEM_NONE);
    }
    // An actual pan, leaving the first slot free for the reward.
    h.set(inventory(INVENTORY_SLOTS - 1, 0), ITEM_PAN);
    h.keyboard().clear();
    h.mouse().clear();
    h.set_movement(0);
    h.call(PAN, &[grade]);
    let (start, start_ticks) = (h.m.steps, h.timers);
    let mut poses = [0; 3];
    let mut switched = false;
    while !h.returned() {
        assert!(h.m.steps - start < ANIMATION_LIMIT, "Restored animation did not finish");
        // After each original presentation call its pose remains in a local.
        if h.at(SIDE_POSE) || h.at(MIDDLE_POSE) {
            let sprite_x = h.m.memory.read16(h.m.regs.ss, h.m.regs.bp - SPRITE_X_BELOW_BP);
            let pose = if h.at(MIDDLE_POSE) {
                MIDDLE
            } else {
                (i32::from(sprite_x) - FIRST_POSE_X) / POSE_WIDTH
            };
            assert!(
                (0..3).contains(&pose),
                "Animation used a sprite outside the original three poses"
            );
            poses[pose as usize] += 1;
            assert!(
                h.report().panning && h.get(GOLD_BAGS) == 0,
                "Reward must wait for the entire original animation"
            );
            if !switched {
                // Changing presentation preferences cannot interrupt it.
                h.game.set_qol_flag(!qol);
                switched = true;
                h.keyboard().push(SPACE);
                h.mouse().buttons(1);
                h.set_movement(8);
            }
        }
        h.step();
    }
    let shown: u64 = poses.iter().sum();
    let cycles = poses[0];
    assert!(
        (5..=9).contains(&cycles) && poses[2] == cycles && poses[MIDDLE as usize] == 2 * cycles,
        "Original three-frame loop and middle return did not complete 5-9 cycles"
    );
    assert!(
        h.timers - start_ticks >= shown * TICKS_PER_POSE,
        "Original animation delays were bypassed"
    );
    assert!(
        h.m.regs.ip == RETURN_OFFSET && h.m.regs.sp == STACK_TOP - 2 && !h.report().panning,
        "Animation did not restore the original stack and return"
    );
    assert!(
        h.get(GOLD_BAGS) == 1
            && h.get(inventory(1, 0)) == GOLD_BAG + grade
            && h.get(inventory(2, 0)) == ITEM_NONE,
        "Animation must award exactly one original graded bag"
    );
    assert!(
        h.get(POSITION_Y) == 55
            && h.game.dos.keyboard.is_empty()
            && h.mouse().current().buttons == 0,
        "Animation left a displaced player or stale input"
    );
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_grade_animates_then_awards_one_bag_with_qol_on_and_off() {
    let mut h = Harness::in_town();
    for qol in [false, true] {
        for grade in 0..3 {
            animate(&mut h, qol, grade);
        }
    }
}
