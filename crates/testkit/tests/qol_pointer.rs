//! The C++ `qol` test, its pointer and hover checks: walking shows the QoL
//! pointer and keeps a quick click for the toolbar, the selector parks only
//! the classic hand, and menus and the saloon's sleep hide every hover target
//! beneath them until they close. The same C++ test's scene and walking
//! checks are `qol_scenes.rs` and `qol_walking.rs`.
//!
//! Not ported: the C++ test also wrote PPM captures to `captures/qol/`,
//! evidence for people rather than a check.

use game::symbols::{
    BUILDING, Global, INVENTORY_SLOTS, ITEM_PAN, LOADED_SCENE, MOUSE_MODE, PANS, PENDING_DIRECTION,
    POSITION_X, POSITION_Y, RETURN_X, SCENE_RIVER, SCENE_TOWN, inventory,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL, SCENE_LIMIT};

/// The world command poll, taking one argument.
const COMMAND_POLL: Address = Address::new(0x0000, 0x07f2);
/// Where it dispatches the toolbar's Game button: the Game menu.
const GAME_MENU: Address = Address::new(0x0652, 0x1a52);
/// The shared selector's cursor setup.
const SELECTOR_SETUP: Address = Address::new(0x0505, 0x030e);
/// The shared selector, taking a click's x and y.
const SELECTOR: Address = Address::new(0x0000, 0x093e);
/// The status menus by key, taking the key's scan and two zeros.
const MENU_BY_KEY: Address = Address::new(0x0000, 0x0ae4);
/// The original mouse helper, polled by open menus.
const READ_MOUSE: Address = Address::new(0x0fc5, 0x0038);
/// The river scene.
const RIVER: Address = Address::new(0x033f, 0x000e);
/// The town scene.
const TOWN: Address = Address::new(0x0000, 0x0238);
/// A building's entry.
const BUILDING_ENTRY: Address = Address::new(0x08c0, 0x0074);
/// Where the saloon's Sleep command begins.
const SLEEP: Address = Address::new(0x08c0, 0x29b6);
/// The original timed wait, which Sleep sleeps in.
const WAIT: Address = Address::new(0x0505, 0x000c);
/// A supply the river's buttons check; the C++ test gave the player one.
const RIVER_SUPPLY: Global = Global(0x53e8);
/// The hour of the day.
const HOUR: Global = Global(0x5328);
/// The saloon.
const SALOON: u16 = 2;
/// The toolbar's Cash button, on the original panel's icon.
const CASH_BUTTON: (i32, i32) = (69, 181);
/// The saloon's Sleep button.
const SLEEP_BUTTON: (i32, i32) = (100, 147);
/// Enter.
const ENTER: u32 = 0x1c0d;
/// S, Sleep's key.
const S: u32 = 0x1f73;
/// The F1 key's scan; F2 to F6 follow it.
const F1_SCAN: u16 = 0x3b;

/// A point inside toolbar button `i`.
fn tool(i: i32) -> (i32, i32) {
    (52 + 42 * i + 5, 167 + 5)
}

/// A point inside a scene's action button `i`.
fn action(i: i32) -> (i32, i32) {
    (if i < 2 { 72 } else { 152 } + 5, if i % 2 == 1 { 138 } else { 118 } + 5)
}

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

/// Whether pointing at (`x`, `y`) changes the frame: a hover highlight.
fn hover(h: &mut Harness, (x, y): (i32, i32)) -> bool {
    h.mouse().move_to(0, 0);
    let normal = h.frame();
    h.mouse().move_to(x, y);
    normal != h.frame()
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn walking_shows_the_pointer_and_keeps_one_quick_click() {
    let mut h = Harness::in_town();
    let report = h.report();
    assert!(report.mouse_visibility < 0 && report.pointer_visible, "Walking hides the QoL pointer");
    h.game.set_qol_flag(false);
    assert!(!h.report().pointer_visible, "QoL off shows a walking pointer");
    h.game.set_qol_flag(true);
    call(&mut h, INPUT_POLL, &[]);
    h.set_movement(8);
    h.mouse().move_to(275, 180);
    h.mouse().buttons(1);
    h.mouse().buttons(0);
    h.finish();
    assert!(
        h.m.regs.ax == 8 && h.game.world_click_pending(&h.m) && h.get(MOUSE_MODE) == 1,
        "A quick click interrupted held movement or was lost"
    );
    // The real world command poll, keeping the pending click.
    h.call(COMMAND_POLL, &[0]);
    until_at(&mut h, GAME_MENU);
    assert!(!h.game.world_click_pending(&h.m), "Single toolbar click was not consumed");
    h.mouse().clear();
    h.game.reset_world_pointer();
    assert!(!h.game.world_click_pending(&h.m), "Focus reset retained a click");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_selector_parks_only_the_classic_hand() {
    let mut h = Harness::in_town();
    for qol in [false, true] {
        h.game.set_qol_flag(qol);
        h.mouse().move_to(33, 44);
        call(&mut h, SELECTOR_SETUP, &[]);
        h.finish();
        let current = h.mouse().current();
        assert_eq!(
            (current.x, current.y),
            if qol { (33, 44) } else { (240, 140) },
            "Selector setup must park only the classic hand cursor"
        );
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn menus_hide_the_hover_targets_beneath_them() {
    let mut h = Harness::in_town();
    h.game.dos.mouse.custom_cursor = false;
    h.set(SCENE_TOWN, 0);
    h.set(SCENE_RIVER, 1);
    h.set(PANS, 1);
    h.set(RIVER_SUPPLY, 1);
    h.set(inventory(INVENTORY_SLOTS - 1, 0), ITEM_PAN);
    call(&mut h, RIVER, &[]);
    until_at(&mut h, INPUT_POLL);
    let river_button = (100, 125);
    assert!(hover(&mut h, river_button), "River button does not highlight before opening a menu");
    // The Health and Tools menus, by a click on their panel icons or by key.
    for menu in [1, 3] {
        for by_mouse in [false, true] {
            if by_mouse {
                let (x, y) = CASH_BUTTON;
                call(&mut h, SELECTOR, &[(x + 42 * menu) as u16, y as u16]);
            } else {
                call(&mut h, MENU_BY_KEY, &[F1_SCAN + menu as u16, 0, 0]);
            }
            until_at(&mut h, READ_MOUSE);
            for i in 0..4 {
                assert!(!hover(&mut h, action(i)), "A menu highlights a hidden river button");
            }
            assert!(
                !hover(&mut h, CASH_BUTTON),
                "An inactive toolbar button highlights behind a menu"
            );
            h.keyboard().push(ENTER);
            h.finish();
            assert!(
                hover(&mut h, river_button),
                "Closing a menu did not restore river button hover"
            );
        }
    }
}

/// Whether pointing at a point changes the frame, the game's own cursor
/// aside: entering a building installs it.
fn hover_in_building(h: &mut Harness, point: (i32, i32)) -> bool {
    h.game.dos.mouse.custom_cursor = false;
    hover(h, point)
}

/// Sleeps at the saloon at 22:00, by mouse or by key.
fn sleep(by_mouse: bool) {
    let mut h = Harness::in_town();
    h.set(BUILDING, SALOON);
    h.set(SCENE_TOWN, 0);
    h.set(LOADED_SCENE, 1);
    h.set(POSITION_X, 150);
    h.set(POSITION_Y, 63);
    h.set(RETURN_X, 80);
    h.set(HOUR, 22);
    call(&mut h, BUILDING_ENTRY, &[]);
    until_at(&mut h, INPUT_POLL);
    assert!(
        hover_in_building(&mut h, SLEEP_BUTTON),
        "Saloon Sleep button does not highlight before sleeping"
    );
    if by_mouse {
        h.mouse().move_to(SLEEP_BUTTON.0, SLEEP_BUTTON.1);
        h.mouse().buttons(1);
        h.mouse().buttons(0);
    } else {
        h.keyboard().push(S);
    }
    until_at(&mut h, SLEEP);
    h.step();
    until_at(&mut h, WAIT);
    for i in 0..4 {
        assert!(
            !hover_in_building(&mut h, action(i)),
            "Sleep highlights an inactive saloon button"
        );
    }
    for i in 0..6 {
        assert!(!hover_in_building(&mut h, tool(i)), "Sleep highlights an inactive toolbar button");
    }
    h.finish();
    assert!(
        h.get(HOUR) == 9 && h.get(BUILDING) == 0 && h.get(POSITION_X) == 80,
        "Sleep did not wake at the original town doorway"
    );
    call(&mut h, TOWN, &[]);
    until_at(&mut h, INPUT_POLL);
    assert!(hover_in_building(&mut h, CASH_BUTTON), "Waking up did not restore toolbar hover");
    assert!(
        !hover_in_building(&mut h, SLEEP_BUTTON),
        "Waking up restored a stale saloon Sleep target"
    );
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn sleeping_by_mouse_or_key_suspends_hover_until_waking() {
    sleep(false);
    sleep(true);
}
