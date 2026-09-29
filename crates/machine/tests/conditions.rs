//! Conditional-jump conditions over every combination of the flags they read.

use machine::{Cond, Flag, Machine};

#[test]
fn conditions_follow_the_8086_definitions() {
    let mut m = Machine::new();
    for bits in 0..32_u16 {
        let (cf, zf, sf, of, pf) =
            (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0, bits & 16 != 0);
        for (flag, on) in [
            (Flag::Carry, cf),
            (Flag::Zero, zf),
            (Flag::Sign, sf),
            (Flag::Overflow, of),
            (Flag::Parity, pf),
        ] {
            m.set_flag(flag, on);
        }
        let expected = [
            (Cond::Equal, zf),
            (Cond::NotEqual, !zf),
            (Cond::Below, cf),
            (Cond::AboveOrEqual, !cf),
            (Cond::BelowOrEqual, cf || zf),
            (Cond::Above, !cf && !zf),
            (Cond::Less, sf != of),
            (Cond::GreaterOrEqual, sf == of),
            (Cond::LessOrEqual, zf || sf != of),
            (Cond::Greater, !zf && sf == of),
            (Cond::Sign, sf),
            (Cond::NotSign, !sf),
            (Cond::Overflow, of),
            (Cond::NotOverflow, !of),
            (Cond::Parity, pf),
            (Cond::NotParity, !pf),
        ];
        for (cond, want) in expected {
            assert_eq!(m.condition(cond), want, "{cond:?} with flags {bits:05b}");
        }
    }
}

#[test]
fn cx_zero_reads_cx_not_flags() {
    let mut m = Machine::new();
    m.regs.cx = 0;
    m.set_flag(Flag::Zero, false);
    assert!(m.condition(Cond::CxZero));
    m.regs.cx = 1;
    m.set_flag(Flag::Zero, true);
    assert!(!m.condition(Cond::CxZero));
}
