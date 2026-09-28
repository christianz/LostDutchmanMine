//! DOS and BIOS services on a bare machine: no game code involved.

use dos::{Dos, KEY_REPEAT};
use machine::{Flag, Machine};

fn call(dos: &mut Dos, m: &mut Machine, number: u8, ax: u16) {
    m.regs.ax = ax;
    dos.interrupt(m, number).expect("supported service");
}

#[test]
fn date_and_time_come_from_the_clock_base_and_emulated_time() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x21, 0x2a00);
    assert_eq!((m.regs.cx, m.regs.dx, m.regs.al()), (2026, 0x0101, 4), "Thursday 1 January 2026");
    call(&mut dos, &mut m, 0x21, 0x2c00);
    assert_eq!((m.regs.cx, m.regs.dx), (0x0c00, 0), "12:00:00.00");
    dos.clock.emulated_ms = 3_723_450;
    call(&mut dos, &mut m, 0x21, 0x2c00);
    assert_eq!((m.regs.cx, m.regs.dx), (0x0d02, 0x032d), "13:02:03.45");
    (dos.clock.emulated_ms, dos.clock.base) = (0, 1_835_395_200);
    call(&mut dos, &mut m, 0x21, 0x2a00);
    assert_eq!((m.regs.cx, m.regs.dx, m.regs.al()), (2028, 0x021d, 2), "Tuesday 29 February 2028");
}

#[test]
fn keyboard_reads_wait_peeks_do_not() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x16, 0x0100);
    assert!(m.flag(Flag::Zero) && !dos.waiting, "peeking an empty buffer does not wait");
    call(&mut dos, &mut m, 0x16, 0x0000);
    assert!(dos.waiting, "reading an empty buffer waits");
    dos.keyboard.push(0x1e61);
    call(&mut dos, &mut m, 0x16, 0x0100);
    assert!(!m.flag(Flag::Zero) && m.regs.ax == 0x1e61 && dos.keyboard.len() == 1);
    call(&mut dos, &mut m, 0x16, 0x0000);
    assert!(!dos.waiting && m.regs.ax == 0x1e61 && dos.keyboard.is_empty());
}

#[test]
fn movement_readers_see_the_movement_scan_of_a_tagged_key() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    let w_moving_up = 0x1177 | (0x48 << dos::KEY_MOVEMENT_SHIFT) | KEY_REPEAT;
    dos.keyboard.push(w_moving_up);
    call(&mut dos, &mut m, 0x16, 0x0100);
    assert_eq!(m.regs.ax, 0x1177, "text readers get W");
    dos.keyboard.movement_aliases = true;
    call(&mut dos, &mut m, 0x16, 0x0100);
    assert_eq!(m.regs.ax, 0x4800, "movement readers get Up");
}

#[test]
fn kbhit_and_getch_use_the_raw_character() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x21, 0x0b00);
    assert_eq!(m.regs.al(), 0);
    dos.keyboard.push(0x011b);
    call(&mut dos, &mut m, 0x21, 0x0b00);
    assert_eq!((m.regs.al(), dos.keyboard.len()), (0xff, 1), "kbhit does not consume");
    call(&mut dos, &mut m, 0x21, 0x0800);
    assert_eq!((m.regs.al(), dos.keyboard.len()), (0x1b, 0));
}

#[test]
fn mouse_driver_reports_samples_and_parks_only_classically() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x33, 0x0000);
    assert_eq!((m.regs.ax, m.regs.bx, dos.mouse.visibility), (0xffff, 2, -1));
    dos.mouse.input.move_to(100, 50);
    dos.mouse.input.poll();
    call(&mut dos, &mut m, 0x33, 0x0003);
    assert_eq!((m.regs.cx, m.regs.dx), (200, 50), "horizontal mickeys are doubled");
    (m.regs.cx, m.regs.dx) = (480, 140);
    call(&mut dos, &mut m, 0x33, 0x0004);
    assert_eq!(dos.mouse.input.current().x, 100, "QoL keeps the physical pointer");
    dos.park_cursor = true;
    (m.regs.cx, m.regs.dx) = (480, 140);
    call(&mut dos, &mut m, 0x33, 0x0004);
    assert_eq!((dos.mouse.input.current().x, dos.mouse.input.current().y), (240, 140));
}

#[test]
fn palette_services_scale_six_bit_colours() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    (m.regs.bx, m.regs.dx, m.regs.cx) = (7, 0x3f00, 0x2010);
    call(&mut dos, &mut m, 0x10, 0x1010);
    assert_eq!(m.vga.palette[7], 0xffff_8140, "x * 255 / 63, truncated");
    call(&mut dos, &mut m, 0x10, 0x1015);
    assert_eq!((m.regs.dx, m.regs.cx), (0x3f00, 0x1f0f), "reading back truncates again");
}

#[test]
fn setting_a_video_mode_updates_the_bios_data_area() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x10, 0x0013);
    assert_eq!(
        (dos.video.mode, m.memory.read8(0x40, 0x49), m.memory.read16(0x40, 0x4a)),
        (0x13, 0x13, 40)
    );
    call(&mut dos, &mut m, 0x10, 0x0f00);
    assert_eq!(m.regs.ax, 0x2813);
}

#[test]
fn memory_allocation_stops_below_the_video_area() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    m.regs.bx = 0x100;
    call(&mut dos, &mut m, 0x21, 0x4800);
    assert!(!m.flag(Flag::Carry) && m.regs.ax == 0x9000);
    m.regs.bx = 0x1000;
    call(&mut dos, &mut m, 0x21, 0x4800);
    assert!(m.flag(Flag::Carry) && m.regs.ax == 8 && m.regs.bx == 0x9fff - 0x9101);
}

#[test]
fn exit_and_unsupported_services() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    call(&mut dos, &mut m, 0x21, 0x4c00);
    assert!(!dos.running);
    m.regs.ax = 0x5100;
    assert!(
        dos.interrupt(&mut m, 0x21).is_err(),
        "an unimplemented service stops with a diagnostic"
    );
}
