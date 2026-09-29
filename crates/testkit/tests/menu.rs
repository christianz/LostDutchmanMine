//! The C++ `menu` test: the original shared scene and building selector. With QoL
//! the enlarged toolbar buttons and the context buttons' bevels dispatch
//! through the original status menus and actions; gaps, absent buttons and
//! scene clicks keep their original routing, the far return and caller stack
//! survive, and without QoL the click reaches the original unchanged. (The
//! desktop's settings menu is `crates/desktop/tests/menu.rs`.)

use game::symbols::VIDEO_MODE_VGA;
use machine::Address;
use testkit::harness::{Harness, RETURN_OFFSET, STACK_TOP};

/// The shared selector, taking a click's x and y.
const SELECTOR: Address = Address::new(0x0000, 0x093e);
/// The segment of the six status menus.
const MENUS: u16 = 0x0652;
/// Where each status menu begins, in toolbar order: Cash, Health, Food,
/// Tools, Ammunition and Game.
const MENU_ENTRIES: [u16; 6] = [0x00aa, 0x21c2, 0x0d60, 0x036a, 0x00c2, 0x1a52];
/// The selector dispatches or returns well within this many steps.
const SELECT_LIMIT: usize = 2000;
/// The selector's result for a click on the scene: select an item.
const ITEM_SELECTION: u16 = 9;
/// Every one of the four context buttons.
const ALL_CONTEXT_BUTTONS: u8 = 15;

/// Where a click led.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selected {
    /// Into toolbar menu `i`.
    Menu(usize),
    /// Back to the caller with this result: a context action, 9 for the
    /// scene, or 0 for nothing.
    Result(u16),
}

/// A scene the selector runs in.
#[derive(Clone, Copy)]
struct Scene {
    /// Whether QoL is on.
    qol: bool,
    /// The context buttons the scene offers.
    context_buttons: u8,
    /// Whether an encounter is running.
    combat: bool,
}

/// A scene offering every context button, with QoL.
const QOL: Scene = Scene { qol: true, context_buttons: ALL_CONTEXT_BUTTONS, combat: false };
/// The same scene without QoL.
const CLASSIC: Scene = Scene { qol: false, ..QOL };

/// Runs the selector on a click at (`x`, `y`) in `scene`, stopping at a status
/// menu's entry or the far return.
fn select(x: u16, y: u16, scene: Scene) -> Selected {
    let mut h = Harness::loaded();
    h.game.set_qol_flag(scene.qol);
    h.game.dos.video.mode = VIDEO_MODE_VGA;
    h.game.set_combat_active(scene.combat);
    h.install_mouse_driver();
    h.game.set_context_buttons(scene.context_buttons);
    h.call(SELECTOR, &[x, y]);
    for _ in 0..SELECT_LIMIT {
        h.step();
        if let Some(menu) = MENU_ENTRIES.iter().position(|&entry| h.at(Address::new(MENUS, entry)))
        {
            return Selected::Menu(menu);
        }
        if h.returned() {
            let (ss, sp) = (h.m.regs.ss, h.m.regs.sp);
            assert!(
                sp == STACK_TOP - 4 && h.m.regs.ip == RETURN_OFFSET,
                "World selector damaged its far return or caller stack"
            );
            if !scene.qol {
                assert!(
                    h.m.memory.read16(ss, sp) == x && h.m.memory.read16(ss, sp + 2) == y,
                    "QoL off rewrote original input"
                );
            }
            return Selected::Result(h.m.regs.ax);
        }
    }
    panic!("World selector failed to dispatch or return");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn toolbar_corners_dispatch_through_the_original_menus() {
    for (i, left) in (0..6).map(|i| (i, 52 + 42 * i as u16)) {
        for x in [left, left + 39] {
            for y in [167, 198] {
                assert_eq!(
                    select(x, y, QOL),
                    Selected::Menu(i),
                    "Toolbar corner did not reach its original menu"
                );
            }
        }
        let nothing = Selected::Result(0);
        assert_eq!(
            select(left + 17, 181, CLASSIC),
            Selected::Menu(i),
            "Original icon interior no longer dispatches"
        );
        assert_eq!(select(left, 198, CLASSIC), nothing, "QoL off retained an enlarged target");
        assert_eq!(
            select(left + 40, 180, QOL),
            nothing,
            "Gap between toolbar buttons must not select anything"
        );
        assert_eq!(
            select(left + 17, 165, QOL),
            nothing,
            "Logo below the title became a toolbar target"
        );
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn context_button_bevels_reach_their_original_actions() {
    for action in 0..4 {
        let (x, y) = (if action < 2 { 72 } else { 152 }, if action % 2 == 1 { 138 } else { 118 });
        for px in [x, x + 74] {
            for py in [y, y + 18] {
                assert_eq!(
                    select(px, py, QOL),
                    Selected::Result(action + 1),
                    "Context-button bevel did not reach its original action"
                );
            }
        }
        let nothing = Selected::Result(0);
        assert_eq!(select(x, y, CLASSIC), nothing, "QoL off changed a contextual edge");
        let absent = Scene { context_buttons: 0, ..QOL };
        assert_eq!(
            select(x, y, absent),
            nothing,
            "An absent contextual button gained a new target"
        );
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn scene_clicks_keep_their_original_routing() {
    let item = Selected::Result(ITEM_SELECTION);
    assert_eq!(select(100, 100, QOL), item, "Shared selector lost the item-selection command");
    let combat = Scene { combat: true, ..QOL };
    assert!(
        select(100, 100, CLASSIC) == item && select(100, 100, combat) == item,
        "Classic or combat scene click routing changed"
    );
    assert_eq!(
        select(150, 150, QOL),
        Selected::Result(0),
        "Space between context columns became clickable"
    );
}
