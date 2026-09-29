//! `tests/cave.cpp`: a generated cave on a scrolled part of the world map,
//! under the whole original scene dispatcher. Clicks during the entrance or
//! inside never restart the scene, resuming a loaded cave keeps its position,
//! a rejected entry and every exit restore the exact map position and scroll,
//! and held Space mines at the original stroke rate, stopping on release.
//!
//! The C++ test also exported two fixture saves for `tools/fixtures.py`; here
//! the same moments are checked against the fixtures it wrote.

use game::symbols::{
    BUILDING, Global, LOADED_SCENE, MAP_SCROLL_X, MAP_SCROLL_Y, MINING_STROKES, MOUSE_MODE,
    POSITION_X, POSITION_Y, RETURN_X, RETURN_Y, SCENE_CAVE, SCENE_ENCOUNTER, SCENE_MAP,
    SCENE_RIVER, SCENE_TOWN, inventory,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL, SCENE_LIMIT};

/// The outer scene dispatcher, resumed with a fresh frame: it handles every
/// scene's return code, including 9.
const DISPATCHER: Address = Address::new(0x0000, 0x0095);
/// The dispatcher's fresh frame: SP and BP.
const DISPATCHER_FRAME: (u16, u16) = (0x8000, 0x8004);
/// The original timed wait.
const WAIT: Address = Address::new(0x0505, 0x000c);
/// The return into the cave code that marks a wait as the entrance's preview.
const ENTRANCE_RETURN: Address = Address::new(0x0bb4, 0x03a1);
/// A pick stroke begins here.
const PICK_STROKE: Address = Address::new(0x0bb4, 0x1047);
/// The mining loop's return address on the stack at its input poll.
const MINING_POLL_RETURN: u16 = 0x06bd;
/// The map's cave table: one byte per tile, nonzero where a cave is.
const CAVES: u16 = 0x0f60;
/// Tiles in each column of the cave table.
const TILES_PER_COLUMN: u16 = 18;
/// A lamp.
const LAMP: u16 = 0x12;
/// The lamp's oil.
const OIL: Global = Global(0x53e4);
/// A pick.
const PICK: u16 = 0x15;
/// Where the player stands on the map.
const MAP_POSITION: (u16, u16) = (135, 61);
/// Where inside the cave the player mines.
const MINING_POSITION: (u16, u16) = (160, 50);
/// Space: scan 39h, character 20h.
const SPACE: u32 = 0x3920;

/// The map beside a generated cave, driven through the original dispatcher.
struct Cave {
    /// The game.
    h: Harness,
    /// The map scroll that puts the cave beside the player.
    scroll: (u16, u16),
}

impl Cave {
    /// Stands the player on a scrolled part of the map beside a generated
    /// cave, with a lamp and oil, and resumes the dispatcher.
    fn new() -> Self {
        let mut h = Harness::in_town();
        let (x, y) = MAP_POSITION;
        let ds = h.m.regs.ds;
        let cave = (20..40)
            .flat_map(|column| (4..12).map(move |row| (column, row)))
            .find(|&(column, row)| {
                h.m.memory.read8(ds, CAVES + column * TILES_PER_COLUMN + row) != 0
            })
            .expect("No generated cave for the round-trip fixture");
        let scroll = (cave.0 - x / 16, cave.1 - (y - 8) / 16);
        for scene in [SCENE_TOWN, SCENE_RIVER, SCENE_CAVE, SCENE_ENCOUNTER, BUILDING, LOADED_SCENE]
        {
            h.set(scene, 0);
        }
        h.set(SCENE_MAP, 1);
        h.set(POSITION_X, x);
        h.set(POSITION_Y, y);
        h.set(MAP_SCROLL_X, scroll.0);
        h.set(MAP_SCROLL_Y, scroll.1);
        h.set(inventory(1, 0), LAMP);
        h.set(OIL, 10);
        h.forget_input();
        h.watch(PICK_STROKE);
        let mut cave = Cave { h, scroll };
        cave.dispatch();
        cave.until(|h| h.at(INPUT_POLL));
        cave
    }

    /// Steps until `done` holds.
    fn until(&mut self, done: impl FnMut(&Harness) -> bool) {
        self.h.until(SCENE_LIMIT, "Cave scenario timed out", done);
    }

    /// Whether the entrance's preview is showing.
    fn entrance(h: &Harness) -> bool {
        let (ss, sp) = (h.m.regs.ss, h.m.regs.sp);
        h.at(WAIT)
            && h.m.memory.read16(ss, sp) == ENTRANCE_RETURN.offset
            && h.m.memory.read16(ss, sp + 2) == ENTRANCE_RETURN.runtime_segment()
    }

    /// Runs to the next input poll, or the entrance's preview.
    fn next_input(&mut self) {
        self.h.step();
        self.until(|h| h.at(INPUT_POLL) || Cave::entrance(h));
    }

    /// Presses Space and steps into the cave.
    fn enter(&mut self) {
        self.h.keyboard().push(SPACE);
        self.h.step();
        self.until(|h| h.at(INPUT_POLL) && SCENE_CAVE.get(&h.m) != 0);
    }

    /// Clicks the scenery, holding the button or not.
    fn click(&mut self, held: bool) {
        self.h.mouse().move_to(180, 50);
        self.h.mouse().buttons(1);
        if !held {
            self.h.mouse().buttons(0);
        }
    }

    /// Resumes the dispatcher with a fresh frame.
    fn dispatch(&mut self) {
        (self.h.m.regs.sp, self.h.m.regs.bp) = DISPATCHER_FRAME;
        self.h.jump(DISPATCHER);
    }

    /// Walks left out of the cave onto the map.
    fn leave(&mut self) {
        self.h.mouse().buttons(0);
        self.h.set_movement(4);
        self.until(|h| h.at(INPUT_POLL) && SCENE_MAP.get(&h.m) != 0);
        self.h.set_movement(0);
        self.assert_position_restored();
    }

    /// Checks the player is back where they stood, with the map as it was.
    fn assert_position_restored(&self) {
        let h = &self.h;
        assert!(
            (h.get(POSITION_X), h.get(POSITION_Y)) == MAP_POSITION
                && (h.get(MAP_SCROLL_X), h.get(MAP_SCROLL_Y)) == self.scroll,
            "Cave exit changed the world-map position or scroll offsets"
        );
    }

    /// Where the player returns to on the map.
    fn return_position(&self) -> (u16, u16) {
        (self.h.get(RETURN_X), self.h.get(RETURN_Y))
    }
}

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
