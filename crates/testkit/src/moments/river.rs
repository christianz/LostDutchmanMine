//! The town with an empty pack, and the river as a loaded game shows it, as
//! the C++ `tests/pan-inventory.cpp` set them up: the pan tests start here,
//! and so do the `pan-missing` and `pan-owned` fixtures.

use game::symbols::{
    BUILDING, CASH, GOLD_BAGS, INVENTORY_SLOTS, ITEM_NONE, LOADED_SCENE, MULES_OWNED, PANS,
    POSITION_X, POSITION_Y, SCENE_CAVE, SCENE_ENCOUNTER, SCENE_MAP, SCENE_RIVER, SCENE_TOWN,
    SCENE_VARIANT, inventory,
};

use super::assay::SAVED_CASH;
use crate::harness::Harness;

/// The town with an empty pack, no mules, no pans and $1,000.
pub fn town(qol: bool) -> Harness {
    let mut h = Harness::in_town();
    h.game.set_qol_flag(qol);
    for row in 0..4 {
        if row > 0 {
            h.set(MULES_OWNED.nth(row - 1), 0);
        }
        for slot in 1..INVENTORY_SLOTS {
            h.set(inventory(slot, row), ITEM_NONE);
        }
    }
    h.set(PANS, 0);
    h.set(GOLD_BAGS, 0);
    h.set(CASH, 1000);
    h.set(CASH.nth(1), 0);
    h.set(SAVED_CASH, 3000);
    h
}

/// The river, as a loaded game shows it.
pub fn river_state(h: &mut Harness) {
    for scene in [SCENE_TOWN, SCENE_MAP, SCENE_CAVE, SCENE_ENCOUNTER, BUILDING] {
        h.set(scene, 0);
    }
    h.set(SCENE_RIVER, 1);
    h.set(LOADED_SCENE, 1);
    h.set(POSITION_X, 40);
    h.set(POSITION_Y, 55);
    h.set(SCENE_VARIANT, 0);
}
