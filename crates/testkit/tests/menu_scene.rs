//! The C++ `menu-scene` test: the QoL toolbar over the real town panel. Without
//! QoL the frame is the original VGA screen, pixel for pixel; with it the
//! labelled toolbar stays within its buttons and the original 16 colours,
//! never writes game memory, and keeps the live health and food warnings: the
//! 64 and 32 thresholds, the critical flash and the recovery.
//!
//! Not ported: the C++ test also wrote PPM captures to `captures/menu/`,
//! evidence for people rather than a check.

use game::Frame;
use game::symbols::{Global, VGA_FRAMEBUFFER};
use machine::Address;
use testkit::harness::{Harness, RETURN_OFFSET, STACK_TOP};

/// The original health and food update.
const STATUS_UPDATE: Address = Address::new(0x0652, 0x1d5e);
/// Survival counters the C++ test cleared before each update.
const CLEARED_BY_FIXTURE: [Global; 3] = [Global(0x5314), Global(0x5316), Global(0x5318)];
/// The two supplies the update derives health from; the C++ test set both.
const SUPPLIES: [Global; 2] = [Global(0x531a), Global(0x531c)];
/// The health it computes.
const HEALTH: Global = Global(0x531e);
/// Health at its best.
const FULL_HEALTH: u16 = 96;
/// The update returns within this many steps.
const UPDATE_LIMIT: u64 = 100_000;
/// A clear background pixel of the Life portrait in the original panel.
const LIFE_PORTRAIT: (usize, usize) = (100, 171);
/// Where the labelled toolbar shows that pixel.
const LIFE_ICON: (usize, usize) = (104, 174);
/// The labelled Food icon's columns.
const FOOD_ICON_COLUMNS: std::ops::Range<usize> = 144..168;
/// The labelled Food icon's rows.
const FOOD_ICON_ROWS: std::ops::Range<usize> = 170..188;

/// Whether (`x`, `y`) lies on one of the six toolbar buttons.
fn on_toolbar(x: usize, y: usize) -> bool {
    (0..6).any(|i| (52 + 42 * i..92 + 42 * i).contains(&x) && (167..199).contains(&y))
}

/// The pixel of `frame` at (`x`, `y`).
fn at(frame: &Frame, (x, y): (usize, usize)) -> u32 {
    frame.pixels()[y * Frame::WIDTH + x]
}

/// The frame with QoL off and then on.
fn frames(h: &mut Harness) -> (Frame, Frame) {
    h.game.set_qol_flag(false);
    let original = h.frame();
    h.game.set_qol_flag(true);
    (original, h.frame())
}

/// What the labelled toolbar shows of the player's condition.
struct Status {
    /// The Life icon's warning colour.
    life: u32,
    /// The Food icon's pixels.
    food: Vec<u32>,
}

/// Runs the original health and food update with both supplies at
/// `supplies`, then reads the labelled toolbar's Life and Food icons.
fn status(h: &mut Harness, supplies: u16) -> Status {
    for counter in CLEARED_BY_FIXTURE {
        h.set(counter, 0);
    }
    for supply in SUPPLIES {
        h.set(supply, supplies);
    }
    h.with_clock_stopped(|h| {
        h.call(STATUS_UPDATE, &[]);
        h.until(UPDATE_LIMIT, "Original health update did not return", Harness::returned);
    });
    assert!(
        h.m.regs.ip == RETURN_OFFSET && h.m.regs.sp == STACK_TOP,
        "Original health update corrupted its return"
    );
    assert_eq!(
        h.get(HEALTH),
        (supplies + 1).min(FULL_HEALTH),
        "Fixture did not produce the intended original health value"
    );
    let unchanged = h.m.memory.clone();
    let (original, labelled) = frames(h);
    assert!(
        h.m.memory.as_bytes()[..] == unchanged.as_bytes()[..],
        "Toolbar rendering changed original health or framebuffer data"
    );
    // A clear background pixel of the original portrait must keep its warning
    // colour once the icon is fitted above its label.
    assert_eq!(
        at(&labelled, LIFE_ICON),
        at(&original, LIFE_PORTRAIT),
        "Labelled Life icon hides the original health warning colour"
    );
    let food = FOOD_ICON_ROWS
        .flat_map(|y| FOOD_ICON_COLUMNS.map(move |x| (x, y)))
        .map(|point| at(&labelled, point))
        .collect();
    Status { life: at(&labelled, LIFE_ICON), food }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_toolbar_labels_the_original_panel_and_keeps_its_warnings() {
    let mut h = Harness::in_town();
    // The panel skin is the overlay's own; the QoL pointer shows over the
    // hidden driver cursor only once the skin is loaded and on screen.
    let report = h.report();
    assert!(
        report.mouse_visibility < 0 && report.pointer_visible,
        "Original initialization did not prepare the panel skin"
    );
    h.game.dos.mouse.custom_cursor = false;
    h.mouse().move_to(0, 0);
    let (original, labelled) = frames(&mut h);
    let screen = &h.m.memory.as_bytes()[VGA_FRAMEBUFFER..];
    assert!(
        original
            .pixels()
            .iter()
            .zip(screen)
            .all(|(&pixel, &index)| pixel == h.m.vga.palette[usize::from(index)]),
        "QoL off changed the original VGA frame"
    );
    let memory = h.m.memory.clone();
    let mut changed = 0;
    for (i, (&labelled, &original)) in labelled.pixels().iter().zip(original.pixels()).enumerate() {
        if labelled == original {
            continue;
        }
        let (x, y) = (i % Frame::WIDTH, i / Frame::WIDTH);
        assert!(on_toolbar(x, y), "Toolbar repaint covered original scenery or context actions");
        assert!(y >= 167, "Toolbar repaint covered the logo or weekday");
        assert!(
            h.m.vga.palette[..16].contains(&labelled),
            "Panel atlas escaped the original 16-colour blitter palette"
        );
        changed += 1;
    }
    assert!(changed > 500, "Toolbar enhancement did not render");
    h.game.dos.mouse.visibility = 0;
    h.mouse().move_to(281, 196);
    h.frame();
    assert!(
        h.m.memory.as_bytes()[..] == memory.as_bytes()[..],
        "Native menu rendering wrote into original game memory"
    );
    h.mouse().move_to(0, 0);

    // The original update's thresholds and critical flash: an overlay showing
    // the healthy artwork stored in PANL_VGA would hide them.
    let healthy = status(&mut h, 96);
    let green_edge = status(&mut h, 63);
    let amber = status(&mut h, 62);
    let amber_edge = status(&mut h, 31);
    let red = status(&mut h, 30);
    let flashing_a = status(&mut h, 3);
    let flashing_b = status(&mut h, 3);
    let recovered = status(&mut h, 96);
    assert!(
        healthy.life == green_edge.life && healthy.life == recovered.life,
        "Healthy colour did not return after recovery"
    );
    assert!(
        healthy.life != amber.life && amber.life == amber_edge.life && amber.life != red.life,
        "Original green/warning/danger thresholds are hidden"
    );
    assert_ne!(flashing_a.life, flashing_b.life, "Critical health no longer flashes");
    assert!(
        healthy.food != amber.food && amber.food != red.food,
        "Labelled Food icon hides the original food warnings"
    );
    assert!(healthy.food == recovered.food, "Food icon did not recover with original supplies");
}
