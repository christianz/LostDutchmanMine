//! Arithmetic, logic, shifts, multiplication and division with 8086 flags.

use crate::{Fault, Flag, Machine};

/// The width of an operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Width {
    /// Eight bits.
    Byte,
    /// Sixteen bits.
    Word,
}

impl Width {
    /// The number of bits.
    pub const fn bits(self) -> u32 {
        match self {
            Width::Byte => 8,
            Width::Word => 16,
        }
    }

    /// All bits of the width set.
    pub const fn mask(self) -> u32 {
        (1 << self.bits()) - 1
    }

    /// The top bit of the width.
    pub const fn sign_bit(self) -> u32 {
        1 << (self.bits() - 1)
    }
}

/// A two-operand ALU operation. INC, DEC and NEG are expressed as `a op b`
/// with `b = 1`, and `0 - b` respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AluOp {
    /// `a + b`.
    Add,
    /// `a + b + carry`.
    Adc,
    /// `a - b`; also CMP.
    Sub,
    /// `a - b - carry`.
    Sbb,
    /// `a & b`; also TEST.
    And,
    /// `a | b`.
    Or,
    /// `a ^ b`.
    Xor,
    /// `a + 1`, leaving carry unchanged.
    Inc,
    /// `a - 1`, leaving carry unchanged.
    Dec,
    /// `0 - b`.
    Neg,
}

/// A shift or rotate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShiftOp {
    /// Shift left.
    Shl,
    /// Logical shift right.
    Shr,
    /// Arithmetic shift right.
    Sar,
    /// Rotate left.
    Rol,
    /// Rotate right.
    Ror,
    /// Rotate left through carry.
    Rcl,
    /// Rotate right through carry.
    Rcr,
}

impl Machine {
    /// Sets zero, sign and parity for a result.
    fn set_result_flags(&mut self, value: u32, width: Width) {
        let value = value & width.mask();
        self.set_flag(Flag::Zero, value == 0);
        self.set_flag(Flag::Sign, value & width.sign_bit() != 0);
        self.set_flag(Flag::Parity, (value & 0xff).count_ones().is_multiple_of(2));
    }

    /// Performs `op` on `a` and `b`, sets the flags the 8086 sets, and returns
    /// the result. Logic operations clear overflow and carry and leave adjust.
    pub fn alu(&mut self, op: AluOp, a: u16, b: u16, width: Width) -> u16 {
        let (mask, sign) = (width.mask(), width.sign_bit());
        let (a, b) = (u32::from(a) & mask, u32::from(b) & mask);
        let carry_in = u32::from(self.flag(Flag::Carry));
        let (result, carry) = match op {
            AluOp::Add | AluOp::Adc | AluOp::Inc => {
                let r = a + b + if op == AluOp::Adc { carry_in } else { 0 };
                self.set_flag(Flag::Overflow, !(a ^ b) & (a ^ r) & sign != 0);
                self.set_flag(Flag::Adjust, (a ^ b ^ r) & 0x10 != 0);
                (r, r > mask)
            }
            AluOp::Sub | AluOp::Sbb | AluOp::Dec | AluOp::Neg => {
                let borrow = if op == AluOp::Sbb { carry_in } else { 0 };
                let r = a.wrapping_sub(b).wrapping_sub(borrow);
                self.set_flag(Flag::Overflow, (a ^ b) & (a ^ r) & sign != 0);
                self.set_flag(Flag::Adjust, (a ^ b ^ r) & 0x10 != 0);
                (r, a < b + borrow)
            }
            AluOp::And | AluOp::Or | AluOp::Xor => {
                self.set_flag(Flag::Overflow, false);
                let r = match op {
                    AluOp::And => a & b,
                    AluOp::Or => a | b,
                    _ => a ^ b,
                };
                (r, false)
            }
        };
        if !matches!(op, AluOp::Inc | AluOp::Dec) {
            self.set_flag(Flag::Carry, carry);
        }
        self.set_result_flags(result, width);
        (result & mask) as u16
    }

    /// Shifts or rotates `value` by `count`, one bit at a time as the 8086 does
    /// (the count is not masked). Overflow is defined only for single-bit counts.
    pub fn shift(&mut self, op: ShiftOp, value: u16, count: u16, width: Width) -> u16 {
        let (mask, sign) = (width.mask(), width.sign_bit());
        let mut r = u32::from(value) & mask;
        if count == 0 {
            return r as u16;
        }
        let old_top = r & sign != 0;
        for _ in 0..count {
            let carry = self.flag(Flag::Carry);
            let (out, next) = match op {
                ShiftOp::Shl => (r & sign != 0, (r << 1) & mask),
                ShiftOp::Shr => (r & 1 != 0, r >> 1),
                ShiftOp::Sar => (r & 1 != 0, (r >> 1) | (r & sign)),
                ShiftOp::Rol => (r & sign != 0, ((r << 1) | u32::from(r & sign != 0)) & mask),
                ShiftOp::Ror => (r & 1 != 0, (r >> 1) | if r & 1 != 0 { sign } else { 0 }),
                ShiftOp::Rcl => (r & sign != 0, ((r << 1) | u32::from(carry)) & mask),
                ShiftOp::Rcr => (r & 1 != 0, (r >> 1) | if carry { sign } else { 0 }),
            };
            r = next;
            self.set_flag(Flag::Carry, out);
        }
        if matches!(op, ShiftOp::Shl | ShiftOp::Shr | ShiftOp::Sar) {
            self.set_result_flags(r, width);
        }
        if count == 1 {
            let overflow = match op {
                ShiftOp::Shr => old_top,
                ShiftOp::Sar => false,
                ShiftOp::Ror | ShiftOp::Rcr => {
                    ((r >> (width.bits() - 1)) ^ (r >> (width.bits() - 2))) & 1 != 0
                }
                ShiftOp::Shl | ShiftOp::Rol | ShiftOp::Rcl => {
                    (r & sign != 0) != self.flag(Flag::Carry)
                }
            };
            self.set_flag(Flag::Overflow, overflow);
        }
        r as u16
    }

    /// MUL or IMUL: AL×value into AX, or AX×value into DX:AX. Carry and
    /// overflow report whether the upper half is needed.
    pub fn multiply(&mut self, value: u16, width: Width, signed: bool) {
        let overflow = match width {
            Width::Byte => {
                let r = if signed {
                    i32::from(self.regs.ax as u8 as i8) * i32::from(value as u8 as i8)
                } else {
                    i32::from(self.regs.ax as u8) * i32::from(value as u8)
                };
                self.regs.ax = r as u16;
                if signed { r != i32::from(r as i8) } else { r & 0xff00 != 0 }
            }
            Width::Word => {
                let r = if signed {
                    i64::from(self.regs.ax as i16) * i64::from(value as i16)
                } else {
                    i64::from(self.regs.ax) * i64::from(value)
                };
                self.regs.ax = r as u16;
                self.regs.dx = (r as u64 >> 16) as u16;
                if signed { r != i64::from(r as i16) } else { r & 0xffff_0000 != 0 }
            }
        };
        self.set_flag(Flag::Carry, overflow);
        self.set_flag(Flag::Overflow, overflow);
    }

    /// DIV or IDIV of AX (byte) or DX:AX (word) by `value`, truncating towards
    /// zero. Flags are unchanged.
    ///
    /// # Errors
    ///
    /// [`Fault::DivideByZero`] or [`Fault::DivideOverflow`] when the 8086 would
    /// raise its divide-error interrupt.
    pub fn divide(&mut self, value: u16, width: Width, signed: bool) -> Result<(), Fault> {
        if value == 0 || (width == Width::Byte && value as u8 == 0) {
            return Err(Fault::DivideByZero);
        }
        let (numerator, denominator) = match width {
            Width::Byte if signed => (i64::from(self.regs.ax as i16), i64::from(value as u8 as i8)),
            Width::Byte => (i64::from(self.regs.ax), i64::from(value as u8)),
            Width::Word => {
                let n = (u32::from(self.regs.dx) << 16) | u32::from(self.regs.ax);
                if signed {
                    (i64::from(n as i32), i64::from(value as i16))
                } else {
                    (i64::from(n), i64::from(value))
                }
            }
        };
        let (quotient, remainder) = (numerator / denominator, numerator % denominator);
        let bits = width.bits();
        let (low, high) =
            if signed { (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) } else { (0, (1 << bits) - 1) };
        if quotient < low || quotient > high {
            return Err(Fault::DivideOverflow);
        }
        match width {
            Width::Byte => self.regs.ax = u16::from_le_bytes([quotient as u8, remainder as u8]),
            Width::Word => {
                self.regs.ax = quotient as u16;
                self.regs.dx = remainder as u16;
            }
        }
        Ok(())
    }
}
