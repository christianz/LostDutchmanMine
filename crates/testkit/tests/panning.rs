//! `tests/panning.cpp`: the original Pan routine pans only with a pan the
//! player or an owned mule actually carries, whatever the stale pan counter
//! says; every grade animates before any gold is awarded, a full pack goes to
//! the original message, and a rejected Pan leaves the caller's stack intact,
//! with QoL on and off.

use game::symbols::{
    GOLD_BAGS, Global, INVENTORY, INVENTORY_SLOTS, ITEM_NONE, ITEM_PAN, MULES_OWNED, PANS,
    VIDEO_MODE_VGA, inventory,
};
use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The original Pan routine, taking the river's gold grade.
const PAN: Address = Address::new(0x033f, 0x02cc);
/// Where panning's first frame reaches the original background blitter.
const BLITTER: Address = Address::new(0x0fc5, 0x0d12);
/// The original message a full pack shows instead.
const PACK_FULL: Address = Address::new(0x0652, 0x01c2);
/// The food table's first slot, beside the inventory but never a tool.
const FOOD_SLOT: Global = Global(0x5bdc);
/// An ordinary item that fills a slot.
const ITEM_ORDINARY: u16 = 0x10;
/// Panning reaches one of its boundaries well within this many steps.
const PAN_LIMIT: u64 = 10_000;

/// What the pack holds when Pan begins. Every mule is unowned.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pack {
    /// Nothing but a pan in the player's last slot; the counter says one.
    OnePan,
    /// Nothing; the counter says none.
    NoPan,
    /// Ordinary items in every other slot beside that pan.
    Full,
}

/// The loaded game about to pan at grade `grade`, with QoL `qol`.
fn pan(qol: bool, pack: Pack, grade: u16) -> Harness {
    let mut h = Harness::loaded();
    h.game.dos.video.mode = VIDEO_MODE_VGA;
    h.game.set_qol_flag(qol);
    h.set(PANS, u16::from(pack != Pack::NoPan));
    h.set(GOLD_BAGS, 0);
    let filler = if pack == Pack::Full { ITEM_ORDINARY } else { ITEM_NONE };
    for row in 0..4 {
        if row > 0 {
            h.set(MULES_OWNED.nth(row - 1), 0);
        }
        for slot in 1..INVENTORY_SLOTS {
            h.set(inventory(slot, row), filler);
        }
    }
    if pack != Pack::NoPan {
        h.set(inventory(INVENTORY_SLOTS - 1, 0), ITEM_PAN);
    }
    h.call(PAN, &[grade]);
    h
}

/// Runs Pan to its animation, its full-pack message or its return.
fn run(h: &mut Harness) {
    let start = h.m.steps;
    loop {
        h.step();
        assert!(h.m.steps - start < PAN_LIMIT, "Original panning path did not reach its boundary");
        if h.at(BLITTER) || h.at(PACK_FULL) || h.returned() {
            break;
        }
    }
}

/// Whether Pan returned without animating or awarding gold.
fn refused(h: &Harness) -> bool {
    h.returned() && !h.report().panning && h.get(GOLD_BAGS) == 0
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn only_a_carried_pan_pans() {
    for qol in [false, true] {
        let mut stale = pan(qol, Pack::OnePan, 0);
        stale.set(inventory(INVENTORY_SLOTS - 1, 0), ITEM_NONE);
        run(&mut stale);
        assert!(refused(&stale), "Stale pan counter permits panning without an inventory pan");
        assert_eq!(
            stale.m.regs.sp,
            STACK_TOP - 2,
            "Rejected Pan did not preserve the caller argument"
        );

        let mut unrelated = pan(qol, Pack::NoPan, 0);
        unrelated.set(INVENTORY, ITEM_PAN);
        unrelated.set(FOOD_SLOT, ITEM_PAN);
        run(&mut unrelated);
        assert!(
            refused(&unrelated),
            "Carrier icons or the food table must not count as an inventory pan"
        );

        for row in 0..4 {
            for slot in [1, 10] {
                for owned in [false, true] {
                    let mut h = pan(qol, Pack::NoPan, 0);
                    h.set(inventory(slot, row), ITEM_PAN);
                    if row > 0 {
                        h.set(MULES_OWNED.nth(row - 1), u16::from(owned));
                    }
                    run(&mut h);
                    let allowed = row == 0 || owned;
                    assert!(
                        h.report().panning == allowed && h.get(GOLD_BAGS) == 0,
                        "Pan eligibility does not match an actual player/owned-mule inventory slot"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_grade_animates_before_awarding_gold() {
    for grade in 0..3 {
        for qol in [false, true] {
            let mut h = pan(qol, Pack::OnePan, grade);
            run(&mut h);
            assert!(
                h.report().panning && h.get(GOLD_BAGS) == 0 && h.at(BLITTER),
                "Every grade must animate before awarding gold with QoL on or off"
            );
        }
    }
    let mut animated = pan(true, Pack::OnePan, 0);
    run(&mut animated);
    assert!(
        animated.report().panning && animated.get(GOLD_BAGS) == 0 && animated.at(BLITTER),
        "Pan must reach the original background blitter before awarding gold"
    );
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn no_pan_or_a_full_pack_neither_animates_nor_awards_gold() {
    let mut missing = pan(true, Pack::NoPan, 0);
    run(&mut missing);
    assert!(refused(&missing), "No pan must mean no animation or gold");
    assert_eq!(missing.m.regs.sp, STACK_TOP - 2, "No-pan return must preserve the caller argument");
    let mut full = pan(true, Pack::Full, 0);
    run(&mut full);
    assert!(
        !full.report().panning && full.get(GOLD_BAGS) == 0 && full.at(PACK_FULL),
        "Full pack must reach the original message without animating or awarding gold"
    );
}
