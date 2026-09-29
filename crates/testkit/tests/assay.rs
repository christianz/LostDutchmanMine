//! The C++ `assay` test: selling gold at the assay office with the mouse. Eight
//! bags across the player's pack and all three mules' rows are each assayed
//! once, with the original weight and grade rules and exactly one payout;
//! equipment and emptied slots pay nothing, and Done and Exit return to the
//! saved town doorway, with QoL on and off.
//!
//! The moments it starts from are fixtures too; `cargo xtask fixtures`
//! builds them, and they are checked against them here.

use game::symbols::{
    ASSAY_GRADE, ASSAY_POUNDS, BUILDING, GOLD_BAGS, ITEM_NONE, POSITION_X, POSITION_Y, SCENE_TOWN,
    inventory,
};
use machine::Address;
use testkit::harness::{Harness, RETURN_OFFSET, SCENE_LIMIT};
use testkit::moments::assay::{LAMP, cash, office};

/// A building's entry.
const BUILDING_ENTRY: Address = Address::new(0x08c0, 0x0074);
/// A building's input poll.
const BUILDING_POLL: Address = Address::new(0x08c0, 0x01c4);
/// The button that opens the bag selection.
const SELECT_BAG: (i32, i32) = (108, 147);
/// While selecting, Next: the next mule's row.
const NEXT_ROW: (i32, i32) = (108, 127);
/// While selecting, Done, where the selection's button was.
const DONE: (i32, i32) = (108, 147);
/// Exit.
const EXIT: (i32, i32) = (188, 147);
/// The selection offers Next, Done and Exit: context buttons 0, 1 and 3.
const NEXT_DONE_EXIT: u8 = 0b1011;
/// The row the inventory's slots are clicked on.
const SLOTS_Y: i32 = 98;
/// The lamp's slot in that row.
const LAMP_SLOT_X: i32 = 42;

/// Enters the office and runs to its input poll.
fn enter(h: &mut Harness) {
    h.forget_input();
    h.call(BUILDING_ENTRY, &[]);
    h.until(SCENE_LIMIT, "Assay scenario timed out", |h| h.at(BUILDING_POLL));
}

/// Clicks at (`x`, `y`) and waits for the office's next input poll, or its return.
fn click(h: &mut Harness, (x, y): (i32, i32)) {
    assert!(h.at(BUILDING_POLL), "Click must start at the building's input poll");
    h.mouse().move_to(x, y);
    h.mouse().buttons(1);
    h.mouse().buttons(0);
    // Both mouse edges are read, then the office polls again.
    for _ in 0..3 {
        h.step();
        h.until(SCENE_LIMIT, "Assay scenario timed out", |h| h.at(BUILDING_POLL) || h.returned());
        if h.returned() {
            break;
        }
    }
}

/// Sells every bag, row by row, then leaves through Done and Exit.
fn assay(qol: bool) {
    let mut h = office(qol);
    enter(&mut h);
    click(&mut h, SELECT_BAG);
    assert_eq!(h.game.context_buttons(), NEXT_DONE_EXIT, "Assay did not open Next/Done/Exit");
    let mut remaining = 8;
    for row in 0..4 {
        let before = cash(&h);
        click(&mut h, (LAMP_SLOT_X, SLOTS_Y));
        assert!(
            h.get(GOLD_BAGS) == remaining && cash(&h) == before && h.get(inventory(1, row)) == LAMP,
            "Assay sold non-ore equipment"
        );
        for slot in [6, 7] {
            let (before, bag) = (cash(&h), h.get(inventory(slot, row)));
            let at = (i32::from(slot) * 29 + 14, SLOTS_Y);
            click(&mut h, at);
            remaining -= 1;
            assert!(
                h.get(GOLD_BAGS) == remaining && h.get(inventory(slot, row)) == ITEM_NONE,
                "Clicking a gold bag did not assay exactly that bag"
            );
            let (pounds, grades) = if slot == 6 { (bag - 31, 0..=4) } else { (bag - 30, 5..=10) };
            let grade = h.get(ASSAY_GRADE);
            assert!(
                h.get(ASSAY_POUNDS) == pounds && grades.contains(&grade),
                "Assay changed the original weight or grade rules"
            );
            assert_eq!(
                cash(&h),
                before + u32::from(pounds * grade * 10),
                "Assay payout was not credited exactly once"
            );
            let before = cash(&h);
            click(&mut h, at);
            assert!(
                cash(&h) == before && h.get(GOLD_BAGS) == remaining,
                "Empty slot paid for a bag again"
            );
        }
        if row < 3 {
            click(&mut h, NEXT_ROW);
        }
    }
    click(&mut h, DONE);
    click(&mut h, EXIT);
    assert!(
        h.returned()
            && h.m.regs.ip == RETURN_OFFSET
            && h.get(SCENE_TOWN) != 0
            && h.get(BUILDING) == 0,
        "Done/Exit did not return from the assay office"
    );
    assert_eq!(
        (h.get(POSITION_X), h.get(POSITION_Y)),
        (80, 59),
        "Assay exit lost the saved doorway"
    );
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_mouse_assays_eight_bags_across_all_rows_with_qol() {
    assay(true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_mouse_assays_eight_bags_across_all_rows_without_qol() {
    assay(false);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_office_matches_the_cpp_fixture() {
    office(true).assert_fixture("assay");
}
