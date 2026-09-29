//! The conditions of conditional jumps.

use crate::{Flag, Machine};

/// A jump condition, named by what it means after `cmp a, b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cond {
    /// JE/JZ: zero.
    Equal,
    /// JNE/JNZ: not zero.
    NotEqual,
    /// JB/JC: unsigned below (carry).
    Below,
    /// JAE/JNC: unsigned above or equal.
    AboveOrEqual,
    /// JBE: unsigned below or equal.
    BelowOrEqual,
    /// JA: unsigned above.
    Above,
    /// JL: signed less.
    Less,
    /// JGE: signed greater or equal.
    GreaterOrEqual,
    /// JLE: signed less or equal.
    LessOrEqual,
    /// JG: signed greater.
    Greater,
    /// JS: negative.
    Sign,
    /// JNS: not negative.
    NotSign,
    /// JO: signed overflow.
    Overflow,
    /// JNO: no signed overflow.
    NotOverflow,
    /// JP: even parity.
    Parity,
    /// JNP: odd parity.
    NotParity,
    /// JCXZ: CX is zero.
    CxZero,
}

impl Machine {
    /// Whether `cond` holds for the current flags (or CX).
    pub const fn condition(&self, cond: Cond) -> bool {
        let zero = self.flag(Flag::Zero);
        let carry = self.flag(Flag::Carry);
        let less = self.flag(Flag::Sign) != self.flag(Flag::Overflow);
        match cond {
            Cond::Equal => zero,
            Cond::NotEqual => !zero,
            Cond::Below => carry,
            Cond::AboveOrEqual => !carry,
            Cond::BelowOrEqual => carry || zero,
            Cond::Above => !carry && !zero,
            Cond::Less => less,
            Cond::GreaterOrEqual => !less,
            Cond::LessOrEqual => zero || less,
            Cond::Greater => !zero && !less,
            Cond::Sign => self.flag(Flag::Sign),
            Cond::NotSign => !self.flag(Flag::Sign),
            Cond::Overflow => self.flag(Flag::Overflow),
            Cond::NotOverflow => !self.flag(Flag::Overflow),
            Cond::Parity => self.flag(Flag::Parity),
            Cond::NotParity => !self.flag(Flag::Parity),
            Cond::CxZero => self.regs.cx == 0,
        }
    }
}
