//! `tests/clock.cpp`: DOS date and time derive from the clock base and emulated
//! time. `crates/dos/tests/services.rs` checks the default date and time, the
//! emulated time and the leap day; the one date it lacks, the day after
//! February in a common year, is here.

use dos::Dos;
use machine::Machine;

/// 2027-03-01 00:00:00, seconds since 1970-01-01.
const FIRST_OF_MARCH_2027: i64 = 1_803_859_200;

#[test]
fn the_first_of_march_2027_is_a_monday() {
    let (mut dos, mut m) = (Dos::default(), Machine::new());
    dos.clock.base = FIRST_OF_MARCH_2027;
    m.regs.ax = 0x2a00;
    dos.interrupt(&mut m, 0x21).expect("DOS reports the date");
    assert_eq!(
        (m.regs.cx, m.regs.dx, m.regs.al()),
        (2027, 0x0301, 1),
        "1 March 2027 must be a Monday"
    );
}
