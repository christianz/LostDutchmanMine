//! The OPL chip through its safe wrapper.

use ymfm_sys::Chip;

fn write(chip: &mut Chip, register: u8, value: u8) {
    chip.write(0, register);
    chip.write(1, value);
}

#[test]
fn adlib_detection_sees_timer_one_expire() {
    let mut chip = Chip::new();
    write(&mut chip, 4, 0x60);
    write(&mut chip, 4, 0x80);
    assert_eq!(chip.read(0) & 0xe0, 0x00, "timers reset");
    write(&mut chip, 2, 0xff);
    write(&mut chip, 4, 0x21);
    chip.advance(10_000);
    assert_eq!(chip.read(0) & 0xe0, 0xc0, "timer 1 expired and raised IRQ");
}

#[test]
fn saved_state_restores_timers_and_registers() {
    let mut chip = Chip::new();
    write(&mut chip, 2, 0xff);
    write(&mut chip, 4, 0x21);
    let saved = chip.save();
    let mut copy = Chip::new();
    copy.restore(&saved);
    chip.advance(10_000);
    copy.advance(10_000);
    assert_eq!(copy.read(0), chip.read(0));
    assert_eq!(copy.save(), chip.save());
}

#[test]
fn clones_are_independent() {
    let mut chip = Chip::new();
    write(&mut chip, 2, 0xff);
    write(&mut chip, 4, 0x21);
    let copy = chip.clone();
    chip.advance(10_000);
    assert_eq!(chip.read(0) & 0xe0, 0xc0);
    let mut copy = copy;
    assert_eq!(copy.read(0) & 0xe0, 0x00, "the clone's timer has not advanced");
}
