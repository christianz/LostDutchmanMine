//! The FLAGS register.

/// Bit 1 of FLAGS, which always reads as one.
pub const RESERVED: u16 = 0x0002;

/// A flag in the FLAGS register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flag {
    /// Carry or borrow out of the result.
    Carry,
    /// The low byte of the result has an even number of set bits.
    Parity,
    /// Carry or borrow out of bit 3 (auxiliary carry).
    Adjust,
    /// The result is zero.
    Zero,
    /// The result's top bit is set.
    Sign,
    /// Single-step trap.
    Trap,
    /// Maskable interrupts are enabled.
    Interrupt,
    /// String operations move downwards.
    Direction,
    /// Signed overflow.
    Overflow,
}

impl Flag {
    /// The flag's bit in FLAGS.
    pub const fn mask(self) -> u16 {
        match self {
            Flag::Carry => 0x0001,
            Flag::Parity => 0x0004,
            Flag::Adjust => 0x0010,
            Flag::Zero => 0x0040,
            Flag::Sign => 0x0080,
            Flag::Trap => 0x0100,
            Flag::Interrupt => 0x0200,
            Flag::Direction => 0x0400,
            Flag::Overflow => 0x0800,
        }
    }
}
