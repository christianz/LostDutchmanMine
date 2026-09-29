//! The town about to enter the assay office as a loaded game, with eight bags
//! of gold across the pack and three mules, as the C++ `assay` test set it
//! up: the assay tests start here, and so does the `assay` fixture.

use game::symbols::{
    BUILDING, CASH, GOLD_BAGS, Global, INVENTORY_SLOTS, ITEM_NONE, LOADED_SCENE, MULES_OWNED,
    RETURN_X, SCENE_TOWN, TOWN_PAGE, inventory,
};

use crate::harness::Harness;

/// The assay office.
const ASSAY_OFFICE: u16 = 4;
/// What loading restores the wallet from: three times the cash.
pub const SAVED_CASH: Global = Global(0x5b72);
/// Equipment that is not ore: a lamp.
pub const LAMP: u16 = 0x12;
/// The gold bags in slot 6: 20h to 22h, of one to three pounds.
const SMALL_BAG: u16 = 0x20;
/// The gold bags in slot 7: 23h to 25h, of five to seven pounds.
const LARGE_BAG: u16 = 0x23;

/// The town, with eight bags of gold and three mules, about to enter the
/// assay office as a loaded game.
pub fn office(qol: bool) -> Harness {
    let mut h = Harness::in_town();
    h.game.set_qol_flag(qol);
    h.set(SCENE_TOWN, 0);
    h.set(BUILDING, ASSAY_OFFICE);
    h.set(LOADED_SCENE, 1);
    h.set(RETURN_X, 80);
    h.set(TOWN_PAGE, 40);
    h.set(GOLD_BAGS, 8);
    set_cash(&mut h, 1000);
    // Loading restores the wallet from here, not from the cash itself.
    h.set(SAVED_CASH, 3000);
    for row in 0..4 {
        if row > 0 {
            h.set(MULES_OWNED.nth(row - 1), 1);
        }
        for slot in 1..INVENTORY_SLOTS {
            h.set(inventory(slot, row), ITEM_NONE);
        }
        h.set(inventory(1, row), LAMP);
        h.set(inventory(6, row), SMALL_BAG + row % 3);
        h.set(inventory(7, row), LARGE_BAG + row % 3);
    }
    h
}

/// The cash, from its low and high words.
pub fn cash(h: &Harness) -> u32 {
    u32::from(h.get(CASH)) | u32::from(h.get(CASH.nth(1))) << 16
}

/// Sets the cash's low and high words.
fn set_cash(h: &mut Harness, cash: u32) {
    h.set(CASH, cash as u16);
    h.set(CASH.nth(1), (cash >> 16) as u16);
}
