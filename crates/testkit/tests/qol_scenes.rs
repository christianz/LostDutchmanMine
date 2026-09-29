//! `tests/qol.cpp`, its scene checks: a loaded saloon exits to its saved
//! doorway, mules are sold cheapest first with QoL and marked sold out without
//! erasing the shop, and the desert close-up waits for a fresh dismissal that
//! never reaches the map. The same C++ test's pointer and walking checks are
//! `qol_pointer.rs` and `qol_walking.rs`.
//!
//! Not ported: the C++ test also wrote PPM captures to `captures/qol/`,
//! evidence for people rather than a check.

use dos::KEY_REPEAT;
use game::Frame;
use game::symbols::{
    BUILDING, CASH, LOADED_SCENE, MULES_OWNED, PENDING_DIRECTION, POSITION_X, POSITION_Y, RETURN_X,
    SCENE_MAP, SCENE_TOWN, TOWN_PAGE, VGA_FRAMEBUFFER,
};
use machine::Address;
use testkit::harness::{Harness, INPUT_POLL, SCENE_LIMIT};

/// A building's entry.
const BUILDING_ENTRY: Address = Address::new(0x08c0, 0x0074);
/// A building's input poll.
const BUILDING_POLL: Address = Address::new(0x08c0, 0x01c4);
/// Where a normal building entry has recorded the street position.
const STREET_RECORDED: Address = Address::new(0x08c0, 0x09d4);
/// The town scene.
const TOWN: Address = Address::new(0x0000, 0x0238);
/// The mule shop.
const MULE_SHOP: Address = Address::new(0x08c0, 0x1de2);
/// A mule's purchase, taking its number from 1.
const BUY_MULE: Address = Address::new(0x08c0, 0x220c);
/// The Cash menu, which F1 opens.
const CASH_MENU: Address = Address::new(0x0652, 0x00aa);
/// The Space close-up on the map, taking one argument.
const DESERT_CLOSE_UP: Address = Address::new(0x05d6, 0x05de);
/// The saloon.
const SALOON: u16 = 2;
/// The three mules' prices.
const MULE_PRICES: [u32; 3] = [800, 1200, 2000];
/// E, a building's Exit.
const E: u32 = 0x1265;
/// Space.
const SPACE: u32 = 0x3920;
/// F1, the Cash menu.
const F1: u32 = 0x3b00;
/// Timer ticks a held Space's repeats keep arriving: longer than the classic
/// preview's timeout.
const HELD_TICKS: u64 = 200;

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

/// Enters the saloon, with fresh input.
fn enter_saloon(h: &mut Harness) {
    h.set(BUILDING, SALOON);
    call(h, BUILDING_ENTRY, &[]);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_loaded_saloon_exits_to_its_saved_doorway() {
    for qol in [false, true] {
        let mut h = Harness::in_town();
        h.game.set_qol_flag(qol);
        h.set(SCENE_TOWN, 0);
        h.set(LOADED_SCENE, 1);
        h.set(POSITION_X, 150);
        h.set(POSITION_Y, 63);
        h.set(RETURN_X, 80);
        h.set(TOWN_PAGE, 40);
        enter_saloon(&mut h);
        until_at(&mut h, INPUT_POLL);
        assert_eq!(h.get(RETURN_X), 80, "Loading saloon overwrote the saved street position");
        h.keyboard().push(E);
        h.finish();
        assert!(
            (h.get(POSITION_X), h.get(POSITION_Y)) == (80, 59)
                && h.get(SCENE_TOWN) == 1
                && h.get(BUILDING) == 0,
            "Saloon exit did not restore the saved doorway"
        );
        call(&mut h, TOWN, &[]);
        until_at(&mut h, INPUT_POLL);
        h.set(LOADED_SCENE, 0);
        h.set(POSITION_X, 240);
        enter_saloon(&mut h);
        until_at(&mut h, STREET_RECORDED);
        assert_eq!(
            h.get(RETURN_X),
            240,
            "Normal building entry did not remember the street position"
        );
    }
}

/// Owns the mules whose bits are set in `owned`.
fn own_mules(h: &mut Harness, owned: u16) {
    for i in 0..3 {
        h.set(MULES_OWNED.nth(i), owned >> i & 1);
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn mules_sell_cheapest_first_and_show_sold_out() {
    let mut h = Harness::in_town();
    for qol in [false, true] {
        for owned in 0..8 {
            for (pick, price) in (0..3).zip(MULE_PRICES) {
                h.game.set_qol_flag(qol);
                own_mules(&mut h, owned);
                h.set(CASH, 10_000);
                h.set(CASH.nth(1), 0);
                let cheaper = (1 << pick) - 1;
                let allowed = owned & 1 << pick == 0 && (!qol || owned & cheaper == cheaper);
                call(&mut h, BUY_MULE, &[pick + 1]);
                h.finish();
                assert_eq!(
                    u32::from(h.get(CASH)),
                    10_000 - if allowed { price } else { 0 },
                    "Mule selection charged the wrong price or sold a blocked mule"
                );
                for i in 0..3 {
                    let expected = owned >> i & 1 != 0 || (allowed && i == pick);
                    assert_eq!(
                        h.get(MULES_OWNED.nth(i)),
                        u16::from(expected),
                        "Mule purchase changed the wrong inventory row"
                    );
                }
            }
        }
    }
    h.game.set_qol_flag(true);
    for owned in [0, 1, 3, 7] {
        own_mules(&mut h, owned);
        call(&mut h, MULE_SHOP, &[]);
        until_at(&mut h, BUILDING_POLL);
        assert!(h.game.mule_shop_visible(), "Mule availability labels missing at shop input");
        h.game.dos.mouse.custom_cursor = false;
        let labels = h.frame();
        let pixel = |x: usize, y: usize| labels.pixels()[y * Frame::WIDTH + x];
        for i in (0..3).filter(|&i| !h.game.mule_available(&h.m, i)) {
            let left = 26 + usize::from(i) * 100;
            for y in 85..96 {
                for x in left..left + 68 {
                    assert_eq!(
                        pixel(x, y),
                        h.m.vga.palette[0],
                        "Old mule lettering remains above SOLD OUT"
                    );
                }
            }
        }
        let border = VGA_FRAMEBUFFER + 110 * Frame::WIDTH;
        for x in 0..Frame::WIDTH {
            let index = h.m.memory.as_bytes()[border + x];
            assert_eq!(
                pixel(x, 110),
                h.m.vga.palette[usize::from(index)],
                "SOLD OUT erases the shop's lower border"
            );
        }
    }
    h.keyboard().push(F1);
    until_at(&mut h, CASH_MENU);
    assert!(!h.game.mule_shop_visible(), "Mule labels cover a keyboard-opened status dialog");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_desert_close_up_waits_for_a_fresh_dismissal() {
    let mut h = Harness::in_town();
    h.set(SCENE_TOWN, 0);
    h.set(SCENE_MAP, 1);
    h.set(POSITION_X, 40);
    h.set(POSITION_Y, 55);
    call(&mut h, DESERT_CLOSE_UP, &[1]);
    h.until(SCENE_LIMIT, "Scenario did not reach its original boundary", |h| {
        h.report().desert_view
    });
    let start = h.timers;
    h.keyboard().push(KEY_REPEAT | SPACE);
    while h.timers - start < HELD_TICKS {
        h.step();
    }
    assert!(h.report().desert_view, "Desert closed on a held Space repeat or its old timeout");
    h.keyboard().push(SPACE);
    h.finish();
    assert!(
        !h.report().desert_view && h.game.dos.keyboard.is_empty() && h.get(PENDING_DIRECTION) == 0,
        "Desert dismissal leaked into the next map input"
    );
    call(&mut h, INPUT_POLL, &[]);
    h.finish();
    assert_eq!(h.m.regs.ax, 0, "Second Space reopened the desert");
    h.game.set_qol_flag(false);
    call(&mut h, DESERT_CLOSE_UP, &[1]);
    h.finish();
    assert!(!h.report().desert_view, "QoL off replaced the original timed preview");
}
