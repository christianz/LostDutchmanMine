//! `tests/qol.cpp`, its walking checks: a held diagonal on the world map keeps
//! both axes when a single key arrives, repeat or not, and leaves the pointer
//! alone; and held walking in the town, a mine and the saloon takes the
//! original collision steps in less time without speeding the survival clock,
//! returning to ordinary timing on release. The same C++ test's pointer and
//! scene checks are `qol_pointer.rs` and `qol_scenes.rs`.
//!
//! Not ported: the C++ test also wrote PPM captures to `captures/qol/`,
//! evidence for people rather than a check.

use dos::KEY_REPEAT;
use game::symbols::{
    BUILDING, Global, MAP_SCROLL_X, MAP_SCROLL_Y, PENDING_DIRECTION, POSITION_X, POSITION_Y,
    SCENE_CAVE, SCENE_MAP, SCENE_TOWN, SURVIVAL_TICKS, TOWN_PAGE, inventory,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL, SCENE_LIMIT};

/// The world map scene.
const MAP: Address = Address::new(0x0000, 0x04da);
/// Where the map's loop has moved the player but not yet checked for a
/// location or event.
const MOVED_ON_MAP: Address = Address::new(0x0000, 0x067e);
/// The map's walking step, set by the terrain.
const MAP_STEP: Global = Global(0x019a);
/// A mine scene.
const MINE: Address = Address::new(0x0bb4, 0x0358);
/// A building's entry.
const BUILDING_ENTRY: Address = Address::new(0x08c0, 0x0074);
/// Two words of the mine's state the C++ test cleared before entering.
const MINE_CLEARED: [Global; 2] = [Global(0x5b6a), Global(0x5b68)];
/// A lamp.
const LAMP: u16 = 0x12;
/// The lamp's oil.
const OIL: Global = Global(0x53e4);
/// The saloon.
const SALOON: u16 = 2;
/// Up as a held direction bit; down is 2.
const UP: u8 = 1;
/// Left as a held direction bit.
const LEFT: u8 = 4;
/// Right as a held direction bit.
const RIGHT: u8 = 8;
/// The Up key's scan.
const UP_SCAN: u16 = 0x48;
/// The Down key's scan.
const DOWN_SCAN: u16 = 0x50;
/// The Left key's scan.
const LEFT_SCAN: u16 = 0x4b;
/// The Right key's scan.
const RIGHT_SCAN: u16 = 0x4d;
/// The four diagonals.
const DIAGONALS: [u8; 4] = [5, 9, 6, 10];
/// Town pages are four pixels apart.
const PAGE_WIDTH: i32 = 4;
/// Loops walked, after one to settle into the held-input cadence.
const LOOPS: usize = 8;

/// Calls `routine` with fresh input, as the C++ scene tests did.
fn call(h: &mut Harness, routine: Address, args: &[u16]) {
    h.forget_input();
    h.set(PENDING_DIRECTION, 0);
    h.call(routine, args);
}

/// Steps until the next step starts at `address`.
fn until_at(h: &mut Harness, address: Address) {
    h.until(SCENE_LIMIT, "Scenario did not reach its original boundary", |h| h.at(address));
}

/// Runs the scene through one loop, back to its input poll.
fn next_input(h: &mut Harness) {
    h.step();
    until_at(h, INPUT_POLL);
}

/// The scan a single key event carries for `direction` on the vertical or
/// horizontal axis.
fn scan(direction: u8, vertical: bool) -> u16 {
    match (vertical, direction & UP != 0, direction & LEFT != 0) {
        (true, true, _) => UP_SCAN,
        (true, false, _) => DOWN_SCAN,
        (false, _, true) => LEFT_SCAN,
        (false, _, false) => RIGHT_SCAN,
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn map_diagonals_keep_both_axes_when_a_single_key_arrives() {
    let mut h = Harness::in_town();
    h.set(SCENE_TOWN, 0);
    h.set(SCENE_MAP, 1);
    for diagonal in DIAGONALS {
        for vertical in [true, false] {
            for repeat in [false, true] {
                h.set(POSITION_X, 160);
                h.set(POSITION_Y, 55);
                h.set(MAP_SCROLL_X, 0);
                h.set(MAP_SCROLL_Y, 0);
                call(&mut h, MAP, &[]);
                until_at(&mut h, INPUT_POLL);
                let step = i32::from(h.get(MAP_STEP));
                h.mouse().move_to(33, 44);
                h.set_movement(diagonal);
                let tag = if repeat { KEY_REPEAT } else { 0 };
                h.keyboard().push(u32::from(scan(diagonal, vertical)) << 8 | tag);
                until_at(&mut h, MOVED_ON_MAP);
                // A map step moves twice as far across as down.
                let dx = if diagonal & LEFT != 0 { -2 } else { 2 };
                let dy = if diagonal & UP != 0 { -1 } else { 1 };
                assert_eq!(
                    (i32::from(h.get(POSITION_X)), i32::from(h.get(POSITION_Y))),
                    (160 + dx * step, 55 + dy * step),
                    "World-map diagonal lost an axis when a single key event arrived"
                );
                let pointer = h.mouse().current();
                assert_eq!((pointer.x, pointer.y), (33, 44), "Map movement relocated the pointer");
            }
        }
    }
}

/// What eight held loops of walking right took and did.
#[derive(Clone, Copy, Debug)]
struct Walked {
    /// Timer ticks the loops took.
    ticks: u64,
    /// Steps of the survival clock they took.
    clock: i32,
    /// Pixels walked along the street.
    distance: i32,
}

/// Where the player walks.
#[derive(Clone, Copy)]
enum Scene {
    /// The town's street.
    Town,
    /// A mine.
    Mine,
    /// Inside the saloon.
    Saloon,
}

/// The player's position along the street, across town pages.
fn street(h: &Harness) -> i32 {
    i32::from(h.get(POSITION_X)) + PAGE_WIDTH * i32::from(h.get(TOWN_PAGE))
}

/// Walks right through eight loops of `scene`, then releases the key.
fn walk(scene: Scene, qol: bool) -> Walked {
    let mut h = Harness::in_town();
    h.game.set_qol_flag(qol);
    match scene {
        Scene::Town => {}
        Scene::Mine => {
            h.set(SCENE_TOWN, 0);
            h.set(SCENE_CAVE, 1);
            for cleared in MINE_CLEARED {
                h.set(cleared, 0);
            }
            h.set(inventory(1, 0), LAMP);
            h.set(OIL, 10);
            call(&mut h, MINE, &[]);
        }
        Scene::Saloon => {
            h.set(SCENE_TOWN, 0);
            h.set(BUILDING, SALOON);
            call(&mut h, BUILDING_ENTRY, &[]);
        }
    }
    until_at(&mut h, INPUT_POLL);
    h.set_movement(RIGHT);
    h.set(SURVIVAL_TICKS, 0);
    // One loop settles into the held-input cadence.
    next_input(&mut h);
    let (ticks, clock, start) = (h.timers, h.get(SURVIVAL_TICKS), street(&h));
    for _ in 0..LOOPS {
        next_input(&mut h);
    }
    let walked = Walked {
        ticks: h.timers - ticks,
        clock: i32::from(h.get(SURVIVAL_TICKS)) - i32::from(clock),
        distance: street(&h) - start,
    };
    h.set_movement(0);
    next_input(&mut h);
    assert!(
        !h.game.walk_is_fast(&h.m) && !h.game.walk_step_is_extra(&h.m),
        "Key release did not restore ordinary timing"
    );
    walked
}

/// Held walking in `scene`, classic and with QoL.
fn walking(scene: Scene) {
    let (classic, qol) = (walk(scene, false), walk(scene, true));
    assert!(
        classic.distance > 0 && classic.distance == qol.distance,
        "Faster walking changed original collision steps"
    );
    assert!(qol.ticks * 4 < classic.ticks * 3, "Held walking did not reduce frame delay");
    assert!(classic.clock == 8 && qol.clock == 4, "Faster walking accelerated the survival clock");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn held_walking_in_town_is_faster_without_speeding_time() {
    walking(Scene::Town);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn held_walking_in_a_mine_is_faster_without_speeding_time() {
    walking(Scene::Mine);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn held_walking_in_the_saloon_is_faster_without_speeding_time() {
    walking(Scene::Saloon);
}
