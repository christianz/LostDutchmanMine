//! The C++ `cave` test: a generated cave on a scrolled part of the world map,
//! under the whole original scene dispatcher. Clicks during the entrance or
//! inside never restart the scene, resuming a loaded cave keeps its position,
//! a rejected entry and every exit restore the exact map position and scroll,
//! and held Space mines at the original stroke rate, stopping on release.
//!
//! The moments it starts from are fixtures too; `cargo xtask fixtures`
//! builds them, and they are checked against them here.

use game::symbols::{
    Global, LOADED_SCENE, MINING_STROKES, MOUSE_MODE, POSITION_X, POSITION_Y, SCENE_CAVE,
    SCENE_MAP, inventory,
};
use testkit::harness::{Harness, INPUT_POLL};
use testkit::moments::cave::{
    Cave, MAP_POSITION, MINING_POLL_RETURN, MINING_POSITION, OIL, PICK, SPACE,
};

/// Enters the cave with Space, clicks the scenery during the entrance or
/// inside, quickly or held, and walks out again.
fn round_trip(during_preview: bool, held: bool) {
    let mut cave = Cave::new();
    cave.h.keyboard().push(SPACE);
    cave.until(Cave::entrance);
    if during_preview {
        cave.click(held);
    }
    cave.h.step();
    cave.until(|h| h.at(INPUT_POLL));
    assert!(
        cave.h.get(SCENE_CAVE) != 0 && cave.h.get(SCENE_MAP) == 0,
        "Space did not enter the generated cave"
    );
    assert_eq!(cave.return_position(), MAP_POSITION, "Cave entry lost its map return position");
    if !during_preview {
        cave.click(held);
    }
    // The whole scene dispatcher stays live: a leaked command 9 would restart
    // the entrance and later save indoor coordinates as the map position.
    for _ in 0..8 {
        cave.next_input();
        assert!(!Cave::entrance(&cave.h), "Scenery click restarted the cave entrance");
        assert_eq!(
            cave.return_position(),
            MAP_POSITION,
            "Scenery click overwrote the map return position"
        );
    }
    cave.leave();
}

/// Resumes the cave as a loaded game would, with an indoor position.
fn resume(qol: bool) {
    let mut cave = Cave::new();
    cave.h.game.set_qol_flag(qol);
    cave.enter();
    // As a loaded game: the loaded-scene flag, and an indoor position.
    cave.h.set(LOADED_SCENE, 1);
    cave.h.set(POSITION_X, 90);
    cave.dispatch();
    cave.until(|h| h.at(INPUT_POLL) || Cave::entrance(h));
    assert!(
        !Cave::entrance(&cave.h) && cave.h.get(POSITION_X) == 90,
        "Resuming cave replayed its entrance or reset indoor position"
    );
    if qol {
        cave.click(false);
        cave.next_input();
        assert!(!Cave::entrance(&cave.h), "Click restarted a resumed cave");
    }
    cave.leave();
}

/// Tries to enter the cave with `missing` cleared: the lamp, or its oil.
fn without_light(missing: Global) {
    let mut cave = Cave::new();
    cave.h.set(missing, 0);
    cave.h.keyboard().push(SPACE);
    cave.h.step();
    cave.until(Cave::entrance);
    cave.click(false);
    cave.h.step();
    cave.until(|h| h.at(INPUT_POLL) && SCENE_MAP.get(&h.m) != 0);
    cave.assert_position_restored();
}

/// Mines with held Space: five strokes, none after release, one per tap.
fn held_pick(qol: bool) {
    let mut cave = Cave::new();
    cave.enter();
    cave.h.game.set_qol_flag(qol);
    // A pick, leaving the lamp in its slot.
    cave.h.set(inventory(2, 0), PICK);
    cave.h.set(POSITION_X, MINING_POSITION.0);
    cave.h.set(POSITION_Y, MINING_POSITION.1);
    cave.h.set(MOUSE_MODE, 1);
    cave.h.set(MINING_STROKES, 0);
    cave.h.game.set_mining_space_held(true);
    cave.h.keyboard().push(SPACE);
    cave.h.step();
    cave.until(|h| h.visits() >= 5);
    cave.h.game.set_mining_space_held(false);
    let mining_poll = |h: &Harness| {
        h.at(INPUT_POLL) && h.m.memory.read16(h.m.regs.ss, h.m.regs.sp) == MINING_POLL_RETURN
    };
    cave.until(mining_poll);
    let strokes = cave.h.visits();
    assert_eq!(strokes, 5, "Releasing Space allowed an extra pick stroke");
    for _ in 0..3 {
        cave.next_input();
    }
    assert_eq!(cave.h.visits(), strokes, "Mining continued after release");
    cave.h.keyboard().push(SPACE);
    cave.h.step();
    cave.until(mining_poll);
    assert_eq!(cave.h.visits(), strokes + 1, "A Space tap must still make one stroke");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn clicks_inside_the_cave_keep_the_scene_and_the_way_back() {
    for during_preview in [false, true] {
        for held in [false, true] {
            round_trip(during_preview, held);
        }
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_resumed_cave_keeps_its_position_and_the_way_back() {
    resume(false);
    resume(true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_rejected_entry_without_a_lamp_or_oil_keeps_the_map_position() {
    without_light(inventory(1, 0));
    without_light(OIL);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn held_space_mines_five_strokes_and_stops_on_release() {
    held_pick(false);
    held_pick(true);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_map_and_mining_moments_match_the_cpp_fixtures() {
    let mut cave = Cave::new();
    cave.h.assert_fixture("map");
    cave.enter();
    cave.h.set(inventory(2, 0), PICK);
    cave.h.set(POSITION_X, MINING_POSITION.0);
    cave.h.set(POSITION_Y, MINING_POSITION.1);
    cave.h.assert_fixture("mining");
}
