//! The C++ `mouse` test: the original mouse helper reads position and buttons in
//! three BIOS calls. Quick clicks, their positions, holds and their order
//! survive it; focus loss and stale clicks do not; and the classic cursor
//! parking happens only without QoL. How the driver queues edges on its own is
//! `crates/dos/tests/mouse.rs`.

use std::path::PathBuf;

use dos::MouseSample;
use game::Game;
use machine::{Address, Machine};
use testkit::harness::{Harness, STACK_TOP};

/// The original mouse helper: it stores x, y and buttons through three pointers.
const READ_MOUSE: Address = Address::new(0x0fc5, 0x0038);
/// Where the test has it store x.
const SAMPLE_X: u16 = 0x6000;
/// Where the test has it store y.
const SAMPLE_Y: u16 = 0x6002;
/// Where the test has it store the buttons.
const SAMPLE_BUTTONS: u16 = 0x6004;
/// The helper returns well within this many steps.
const READ_LIMIT: u64 = 1000;
/// Where the desktop pointer moves while the helper runs.
const MOVING: (i32, i32) = (300, 190);

/// Reads the mouse through the original helper. Between each of its steps the
/// desktop pointer moves: a queued click must keep its own coordinates.
fn read(h: &mut Harness) -> MouseSample {
    h.call(READ_MOUSE, &[SAMPLE_X, SAMPLE_Y, SAMPLE_BUTTONS]);
    let start = h.m.steps;
    while !h.returned() {
        assert!(h.m.steps - start < READ_LIMIT, "Original mouse helper failed to return");
        h.step();
        h.mouse().move_to(MOVING.0, MOVING.1);
    }
    h.m.regs.sp += 6;
    assert_eq!(h.m.regs.sp, STACK_TOP, "Mouse polling corrupted stack");
    let word = |at| i32::from(h.m.memory.read16(h.m.regs.ds, at));
    MouseSample { x: word(SAMPLE_X), y: word(SAMPLE_Y), buttons: word(SAMPLE_BUTTONS) as u8 }
}

/// The desktop's buttons change through `sequence`.
fn buttons(h: &mut Harness, sequence: &[u8]) {
    for &mask in sequence {
        h.mouse().buttons(mask);
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_original_helper_keeps_clicks_coherent_holds_and_order() {
    let mut h = Harness::loaded();
    h.install_mouse_driver();

    h.mouse().move_to(123, 87);
    buttons(&mut h, &[1, 0]);
    let (down, up, moved) = (read(&mut h), read(&mut h), read(&mut h));
    let point = |sample: MouseSample| (sample.x, sample.y);
    assert!(
        down.buttons == 1 && point(down) == (123, 87),
        "Quick press or click position was lost"
    );
    assert!(up.buttons == 0 && point(up) == (123, 87), "Quick release was lost");
    assert!(moved.buttons == 0 && point(moved) == MOVING, "Current movement did not resume");

    buttons(&mut h, &[2, 2]);
    assert!(
        read(&mut h).buttons == 2 && read(&mut h).buttons == 2,
        "Held right button was not preserved"
    );
    buttons(&mut h, &[0, 1, 0]);
    h.mouse().clear();
    assert!(
        read(&mut h).buttons == 0 && read(&mut h).buttons == 0,
        "Focus loss retained a queued click"
    );
    h.mouse().warp(25, 40);
    buttons(&mut h, &[1]);
    let warped = read(&mut h);
    assert!(
        point(warped) == (25, 40) && warped.buttons == 1,
        "Original pointer positioning failed"
    );

    // Clicks expire after one second of emulated time, never host time.
    h.mouse().clear();
    h.mouse().set_time(1000);
    buttons(&mut h, &[1, 0]);
    h.mouse().set_time(3000);
    assert_eq!(read(&mut h).buttons, 0, "A click made during loading was replayed later");
    h.mouse().set_time(1000);
    buttons(&mut h, &[1]);
    h.mouse().set_time(3000);
    assert_eq!(read(&mut h).buttons, 1, "A physically held button expired");
    h.mouse().clear();
    h.mouse().set_time(1000);
    buttons(&mut h, &[1, 0]);
    h.mouse().set_time(1999);
    assert_eq!(read(&mut h).buttons, 1, "A click younger than one second was dropped");

    h.mouse().clear();
    buttons(&mut h, &[1, 0, 2, 0]);
    for mask in [1, 0, 2, 0] {
        assert_eq!(read(&mut h).buttons, mask, "Consecutive clicks were reordered");
    }
}

#[test]
fn legacy_cursor_parking_is_ignored_only_with_qol() {
    for qol in [false, true] {
        let mut game = Game::new(PathBuf::new(), PathBuf::new(), qol);
        let mut m = Machine::new();
        game.dos.mouse.input.move_to(33, 44);
        (m.regs.ax, m.regs.cx, m.regs.dx) = (4, 480, 140);
        game.dos.interrupt(&mut m, 0x33).expect("INT 33h positions the pointer");
        let current = game.dos.mouse.input.current();
        assert_eq!(
            (current.x, current.y),
            if qol { (33, 44) } else { (240, 140) },
            "Legacy cursor parking must be ignored only with QoL enabled"
        );
    }
}
