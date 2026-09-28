//! ALU results and flags, checked against independently computed arithmetic.

use machine::{AluOp, Flag, Machine, Width};

/// Parity by XOR-folding, independent of the implementation's bit count.
fn parity_even(value: u32) -> bool {
    let mut fold = value & 0xff;
    fold ^= fold >> 4;
    fold ^= fold >> 2;
    fold ^= fold >> 1;
    fold & 1 == 0
}

#[test]
fn add_and_subtract_with_carry_match_exact_arithmetic() {
    let mut m = Machine::new();
    let mut checked = 0;
    for width in [Width::Byte, Width::Word] {
        let (mask, sign) = (width.mask(), width.sign_bit());
        for a in 0..=mask {
            for b in [0, 1, sign - 1, sign, mask] {
                for carry in [false, true] {
                    for subtract in [false, true] {
                        m.regs.flags = 2 | Flag::Direction.mask();
                        m.set_flag(Flag::Carry, carry);
                        let op = if subtract { AluOp::Sbb } else { AluOp::Adc };
                        let result = u32::from(m.alu(op, a as u16, b as u16, width));

                        let c = i64::from(carry);
                        let exact = if subtract {
                            i64::from(a) - i64::from(b) - c
                        } else {
                            i64::from(a) + i64::from(b) + c
                        };
                        let signed = |v: u32| {
                            if v & sign == 0 {
                                i64::from(v)
                            } else {
                                i64::from(v) - i64::from(mask) - 1
                            }
                        };
                        let signed_result = if subtract {
                            signed(a) - signed(b) - c
                        } else {
                            signed(a) + signed(b) + c
                        };
                        let overflow =
                            signed_result < -i64::from(sign) || signed_result >= i64::from(sign);

                        assert_eq!(
                            i64::from(result),
                            exact & i64::from(mask),
                            "{op:?} {a} {b} {carry}"
                        );
                        assert_eq!(m.flag(Flag::Carry), exact < 0 || exact > i64::from(mask));
                        assert_eq!(m.flag(Flag::Overflow), overflow);
                        assert_eq!(m.flag(Flag::Zero), result == 0);
                        assert_eq!(m.flag(Flag::Sign), result & sign != 0);
                        assert_eq!(m.flag(Flag::Parity), parity_even(result));
                        assert!(m.flag(Flag::Direction), "unrelated flags must survive");
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 1_315_840);
}

#[test]
fn increment_and_decrement_preserve_carry() {
    let mut m = Machine::new();
    for carry in [false, true] {
        m.regs.flags = 2;
        m.set_flag(Flag::Carry, carry);
        m.alu(AluOp::Inc, 0xffff, 1, Width::Word);
        assert_eq!(m.flag(Flag::Carry), carry, "INC changed carry");
        m.alu(AluOp::Dec, 0, 1, Width::Word);
        assert_eq!(m.flag(Flag::Carry), carry, "DEC changed carry");
    }
}

#[test]
fn signed_multiply_overflows_at_the_boundary() {
    let mut m = Machine::new();
    m.regs.ax = (-32768_i16) as u16;
    m.multiply((-1_i16) as u16, Width::Word, true);
    assert_eq!((m.regs.dx, m.regs.ax), (0, 32768));
    assert!(m.flag(Flag::Overflow));
}

#[test]
fn signed_divide_truncates_towards_zero() {
    let mut m = Machine::new();
    m.regs.dx = 0xffff;
    m.regs.ax = (-1000_i16) as u16;
    m.divide(33, Width::Word, true).expect("-1000 / 33 fits");
    assert_eq!((m.regs.ax as i16, m.regs.dx as i16), (-30, -10));
}

#[test]
fn division_faults_are_reported() {
    let mut m = Machine::new();
    assert!(m.divide(0, Width::Word, false).is_err(), "division by zero");
    m.regs.dx = 1;
    m.regs.ax = 0;
    assert!(m.divide(1, Width::Word, false).is_err(), "quotient does not fit");
}
