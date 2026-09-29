//! The world map beside a generated cave, under the whole original scene
//! dispatcher, as the C++ `tests/cave.cpp` set it up: the cave tests start
//! here, and so do the `map` and `mining` fixtures.

use game::symbols::{
    BUILDING, Global, LOADED_SCENE, MAP_SCROLL_X, MAP_SCROLL_Y, POSITION_X, POSITION_Y, RETURN_X,
    RETURN_Y, SCENE_CAVE, SCENE_ENCOUNTER, SCENE_MAP, SCENE_RIVER, SCENE_TOWN, inventory,
};
use machine::Address;

use crate::harness::{Harness, INPUT_POLL, SCENE_LIMIT};

/// The outer scene dispatcher, resumed with a fresh frame: it handles every
/// scene's return code, including 9.
pub const DISPATCHER: Address = Address::new(0x0000, 0x0095);
/// The dispatcher's fresh frame: SP and BP.
pub const DISPATCHER_FRAME: (u16, u16) = (0x8000, 0x8004);
/// The original timed wait.
pub const WAIT: Address = Address::new(0x0505, 0x000c);
/// The return into the cave code that marks a wait as the entrance's preview.
pub const ENTRANCE_RETURN: Address = Address::new(0x0bb4, 0x03a1);
/// A pick stroke begins here.
pub const PICK_STROKE: Address = Address::new(0x0bb4, 0x1047);
/// The mining loop's return address on the stack at its input poll.
pub const MINING_POLL_RETURN: u16 = 0x06bd;
/// The map's cave table: one byte per tile, nonzero where a cave is.
pub const CAVES: u16 = 0x0f60;
/// Tiles in each column of the cave table.
pub const TILES_PER_COLUMN: u16 = 18;
/// A lamp.
pub const LAMP: u16 = 0x12;
/// The lamp's oil.
pub const OIL: Global = Global(0x53e4);
/// A pick.
pub const PICK: u16 = 0x15;
/// Where the player stands on the map.
pub const MAP_POSITION: (u16, u16) = (135, 61);
/// Where inside the cave the player mines.
pub const MINING_POSITION: (u16, u16) = (160, 50);
/// Space: scan 39h, character 20h.
pub const SPACE: u32 = 0x3920;

/// The map beside a generated cave, driven through the original dispatcher.
pub struct Cave {
    /// The game.
    pub h: Harness,
    /// The map scroll that puts the cave beside the player.
    pub scroll: (u16, u16),
}

impl Cave {
    /// Stands the player on a scrolled part of the map beside a generated
    /// cave, with a lamp and oil, and resumes the dispatcher.
    ///
    /// # Panics
    ///
    /// When the game does not boot, or its map has no cave in reach.
    #[allow(clippy::new_without_default, reason = "it runs the game to the map, no default")]
    pub fn new() -> Self {
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
    pub fn until(&mut self, done: impl FnMut(&Harness) -> bool) {
        self.h.until(SCENE_LIMIT, "Cave scenario timed out", done);
    }

    /// Whether the entrance's preview is showing.
    pub fn entrance(h: &Harness) -> bool {
        let (ss, sp) = (h.m.regs.ss, h.m.regs.sp);
        h.at(WAIT)
            && h.m.memory.read16(ss, sp) == ENTRANCE_RETURN.offset
            && h.m.memory.read16(ss, sp + 2) == ENTRANCE_RETURN.runtime_segment()
    }

    /// Runs to the next input poll, or the entrance's preview.
    pub fn next_input(&mut self) {
        self.h.step();
        self.until(|h| h.at(INPUT_POLL) || Cave::entrance(h));
    }

    /// Presses Space and steps into the cave.
    pub fn enter(&mut self) {
        self.h.keyboard().push(SPACE);
        self.h.step();
        self.until(|h| h.at(INPUT_POLL) && SCENE_CAVE.get(&h.m) != 0);
    }

    /// Clicks the scenery, holding the button or not.
    pub fn click(&mut self, held: bool) {
        self.h.mouse().move_to(180, 50);
        self.h.mouse().buttons(1);
        if !held {
            self.h.mouse().buttons(0);
        }
    }

    /// Resumes the dispatcher with a fresh frame.
    pub fn dispatch(&mut self) {
        (self.h.m.regs.sp, self.h.m.regs.bp) = DISPATCHER_FRAME;
        self.h.jump(DISPATCHER);
    }

    /// Walks left out of the cave onto the map.
    pub fn leave(&mut self) {
        self.h.mouse().buttons(0);
        self.h.set_movement(4);
        self.until(|h| h.at(INPUT_POLL) && SCENE_MAP.get(&h.m) != 0);
        self.h.set_movement(0);
        self.assert_position_restored();
    }

    /// Checks the player is back where they stood, with the map as it was.
    ///
    /// # Panics
    ///
    /// When they are not.
    pub fn assert_position_restored(&self) {
        let h = &self.h;
        assert!(
            (h.get(POSITION_X), h.get(POSITION_Y)) == MAP_POSITION
                && (h.get(MAP_SCROLL_X), h.get(MAP_SCROLL_Y)) == self.scroll,
            "Cave exit changed the world-map position or scroll offsets"
        );
    }

    /// Where the player returns to on the map.
    pub fn return_position(&self) -> (u16, u16) {
        (self.h.get(RETURN_X), self.h.get(RETURN_Y))
    }
}
