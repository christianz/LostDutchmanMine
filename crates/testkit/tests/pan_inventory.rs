//! `tests/pan-inventory.cpp`: the original shop and discard routines leave
//! the pan counter stale, so the river's Pan must follow the actual inventory.
//! A full-pack purchase, a real purchase and discarding the last pan, even
//! while the river's old button is still drawn, never let the mouse or P pan
//! without one, with QoL on and off.
//!
//! The C++ test also exported the river with and without a pan as fixtures for
//! `tools/fixtures.py`; here those moments are checked against the fixtures.

use game::symbols::{
    CASH, GOLD_BAGS, INVENTORY_SLOTS, ITEM_PAN, MOUSE_MODE, MULES_OWNED, PANS, PENDING_DIRECTION,
    inventory,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL, SCENE_LIMIT};
use testkit::moments::river::{river_state, town};

/// The shop's purchase, taking the item bought.
const BUY: Address = Address::new(0x08c0, 0x2718);
/// The inventory's discard, taking a slot and a row.
const DISCARD: Address = Address::new(0x0652, 0x17e0);
/// The river scene.
const RIVER: Address = Address::new(0x033f, 0x000e);
/// The original mouse helper.
const READ_MOUSE: Address = Address::new(0x0fc5, 0x0038);
/// The return into the classic hand's release drain: a mouse read with it
/// on the stack waits for the selection click.
const RELEASE_DRAIN_RETURN: u16 = 0x0876;
/// An ordinary item that fills a slot.
const LAMP: u16 = 0x12;
/// The river's Pan button is context button 1.
const PAN_BUTTON: u8 = 0b10;
/// Where the river's Pan button is drawn.
const PAN_CLICK: (i32, i32) = (100, 147);
/// P, Pan's key.
const P: u32 = 0x1970;

/// Whether the player or an owned mule actually carries a pan.
fn has_pan(h: &Harness) -> bool {
    (0..4)
        .filter(|&row| row == 0 || h.get(MULES_OWNED.nth(row - 1)) != 0)
        .any(|row| (1..INVENTORY_SLOTS).any(|slot| h.get(inventory(slot, row)) == ITEM_PAN))
}

/// Calls `routine` with fresh input, and runs it to its return.
fn run_to_return(h: &mut Harness, routine: Address, args: &[u16]) {
    h.forget_input();
    h.set(PENDING_DIRECTION, 0);
    h.call(routine, args);
    h.finish();
}

/// Enters the river and runs it to its input poll.
fn river(h: &mut Harness) {
    river_state(h);
    h.forget_input();
    h.set(PENDING_DIRECTION, 0);
    h.call(RIVER, &[]);
    h.until(SCENE_LIMIT, "Pan inventory scenario timed out", |h| h.at(INPUT_POLL));
}

/// P, and a click where Pan is drawn, must neither animate nor award gold.
fn blocked_pan(h: &mut Harness) {
    assert!(h.at(INPUT_POLL), "Pan must start from the river input poll");
    let bags = h.get(GOLD_BAGS);
    for click in [false, true] {
        if click && !h.game.qol() {
            // Classic input spends the first click opening the hand: wait past
            // its release drain before the selection click.
            h.set(MOUSE_MODE, 0);
            h.step();
            h.until(SCENE_LIMIT, "Pan inventory scenario timed out", |h| {
                h.at(READ_MOUSE)
                    && h.m.memory.read16(h.m.regs.ss, h.m.regs.sp) == RELEASE_DRAIN_RETURN
            });
        }
        if click {
            h.mouse().move_to(PAN_CLICK.0, PAN_CLICK.1);
            h.mouse().buttons(1);
            h.mouse().buttons(0);
        } else {
            h.keyboard().push(P);
        }
        h.step();
        h.until(SCENE_LIMIT, "Pan inventory scenario timed out", |h| {
            h.at(INPUT_POLL) || h.report().panning
        });
        assert!(
            !h.report().panning && h.get(GOLD_BAGS) == bags,
            "Missing pan still animates or awards gold"
        );
        h.mouse().clear();
        h.game.reset_world_pointer();
        h.set(MOUSE_MODE, 1);
    }
}

/// Whether the river offers its Pan button.
fn pan_button(h: &Harness) -> bool {
    h.game.context_buttons() & PAN_BUTTON != 0
}

/// Buys a pan with a full pack, then for real, and discards it, trying to pan
/// without one at each step.
fn scenario(qol: bool) {
    let mut h = town(qol);
    for slot in 1..INVENTORY_SLOTS {
        h.set(inventory(slot, 0), LAMP);
    }
    run_to_return(&mut h, BUY, &[ITEM_PAN]);
    assert!(
        h.get(PANS) == 1 && h.get(CASH) == 1000 && !has_pan(&h),
        "Full-pack purchase did not reproduce the stale pan counter"
    );
    // Discard an ordinary item to make room.
    run_to_return(&mut h, DISCARD, &[1, 0]);
    river(&mut h);
    assert!(!pan_button(&h), "A phantom pan enables the river's Pan button");
    blocked_pan(&mut h);

    run_to_return(&mut h, BUY, &[ITEM_PAN]);
    assert!(
        has_pan(&h) && h.get(inventory(1, 0)) == ITEM_PAN && h.get(CASH) < 1000,
        "Successful purchase did not put a paid pan in the pack"
    );
    river(&mut h);
    assert!(pan_button(&h), "A purchased pan did not enable Pan");

    // Discard the last pan through the original routine on the river's own
    // stack, so its already drawn Pan button is exercised too.
    let river_at = (h.m.regs.cs, h.m.regs.ip, h.m.regs.sp);
    h.call_with_stack(h.m.regs.sp, DISCARD, &[1, 0]);
    h.finish();
    (h.m.regs.cs, h.m.regs.ip, h.m.regs.sp) = river_at;
    assert!(
        !has_pan(&h) && h.get(PANS) == 1,
        "Last-pan discard did not retain the phantom count fixture"
    );
    blocked_pan(&mut h);
    river(&mut h);
    assert!(!pan_button(&h), "Discarding the last pan left Pan available on entry");
    blocked_pan(&mut h);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn pan_follows_the_actual_inventory_with_qol() {
    scenario(true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn pan_follows_the_actual_inventory_without_qol() {
    scenario(false);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_river_with_and_without_a_pan_matches_the_cpp_fixtures() {
    let mut h = town(true);
    river_state(&mut h);
    h.set(PANS, 1);
    h.assert_fixture("pan-missing");
    h.set(inventory(1, 0), ITEM_PAN);
    h.assert_fixture("pan-owned");
}
