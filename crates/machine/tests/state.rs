//! Savestates of the machine: everything that identifies a moment survives.

use machine::state::{Persist, Reader, StateError, Writer};
use machine::{Machine, Width};

fn busy_machine() -> Machine {
    let mut m = Machine::new();
    (m.regs.ax, m.regs.cs, m.regs.ip, m.regs.sp, m.regs.flags) =
        (0x1234, 0x2000, 0x0100, 0xfff0, 0x0203);
    for i in 0..4096u16 {
        m.memory.write16(0x3000, i * 2, i.wrapping_mul(40_503));
    }
    m.port_out(0x43, 0x36, Width::Byte);
    m.port_out(0x40, 0x9c, Width::Byte);
    m.port_out(0x61, 3, Width::Byte);
    m.port_out(0x388, 0x20, Width::Byte);
    m.port_out(0x389, 0x21, Width::Byte);
    m.opl.advance(10_000);
    m.vga.palette[5] = 0xff12_3456;
    m.vga.attributes[3] = 7;
    m.steps = 987_654;
    m
}

fn saved(m: &Machine) -> Vec<u8> {
    let mut writer = Writer::new();
    m.save(&mut writer);
    writer.finish()
}

fn restored(state: &[u8]) -> Result<Machine, StateError> {
    let mut reader = Reader::new(state)?;
    let mut m = Machine::new();
    m.restore(&mut reader)?;
    reader.finish()?;
    Ok(m)
}

#[test]
fn a_restored_machine_is_the_same_moment() {
    let original = busy_machine();
    let copy = restored(&saved(&original)).expect("a valid state");
    assert_eq!(copy.state_hash(), original.state_hash());
    assert_eq!(copy.steps, original.steps);
    assert_eq!(copy.pit.timer_period(), original.pit.timer_period());
    assert_eq!(copy.ports.get(0x61), 3);
    assert_eq!(copy.vga.attributes, original.vga.attributes);
}

#[test]
fn the_timer_divisor_half_written_survives() {
    let mut original = Machine::new();
    original.port_out(0x43, 0x36, Width::Byte);
    original.port_out(0x40, 0x34, Width::Byte);
    let mut copy = restored(&saved(&original)).expect("a valid state");
    copy.port_out(0x40, 0x12, Width::Byte);
    assert_eq!(copy.pit.timer_divisor, 0x1234, "the high byte completes the divisor");
}

#[test]
fn the_opl_chip_continues_identically() {
    let mut original = busy_machine();
    let mut copy = restored(&saved(&original)).expect("a valid state");
    for _ in 0..3 {
        original.opl.advance(80_000);
        copy.opl.advance(80_000);
        assert_eq!(copy.port_in(0x388, Width::Byte), original.port_in(0x388, Width::Byte));
    }
}

#[test]
fn damaged_states_are_refused() {
    let state = saved(&busy_machine());
    assert!(matches!(restored(&state[..state.len() - 1]), Err(StateError::Truncated)));
    assert!(matches!(restored(b"not a state at all"), Err(StateError::NotAState)));
    let mut newer = state.clone();
    newer[8] = newer[8].wrapping_add(1);
    assert!(matches!(restored(&newer), Err(StateError::Version(_))));
    let mut longer = state;
    longer.push(0);
    assert!(matches!(restored(&longer), Err(StateError::Trailing)));
}
