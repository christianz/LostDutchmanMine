//! MOVS, STOS, LODS, SCAS and CMPS, with their REP prefixes.

use crate::{AluOp, Flag, Machine, Width};

/// A string instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StringOp {
    /// Copy from the source to ES:DI.
    Movs,
    /// Store AL or AX at ES:DI.
    Stos,
    /// Load AL or AX from the source.
    Lods,
    /// Compare AL or AX with ES:DI.
    Scas,
    /// Compare the source with ES:DI.
    Cmps,
}

/// A string instruction's repeat prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Repeat {
    /// No prefix: one iteration.
    Once,
    /// REP: CX iterations.
    Rep,
    /// REPE: up to CX iterations while equal.
    WhileEqual,
    /// REPNE: up to CX iterations while not equal.
    WhileNotEqual,
}

impl Machine {
    /// Executes a string instruction. `source` is the segment of the SI operand
    /// (DS unless overridden); the destination is always ES:DI.
    pub fn string_op(&mut self, op: StringOp, width: Width, repeat: Repeat, source: u16) {
        let iterations = if repeat == Repeat::Once { 1 } else { self.regs.cx };
        let step: u16 = if width == Width::Byte { 1 } else { 2 };
        let delta: u16 = if self.flag(Flag::Direction) { step.wrapping_neg() } else { step };
        for _ in 0..iterations {
            let (es, si, di) = (self.regs.es, self.regs.si, self.regs.di);
            match op {
                StringOp::Movs => {
                    let value = self.read(source, si, width);
                    self.write(es, di, value, width);
                    self.regs.si = si.wrapping_add(delta);
                    self.regs.di = di.wrapping_add(delta);
                }
                StringOp::Stos => {
                    self.write(es, di, self.regs.ax, width);
                    self.regs.di = di.wrapping_add(delta);
                }
                StringOp::Lods => {
                    let value = self.read(source, si, width);
                    self.regs.ax = match width {
                        Width::Byte => (self.regs.ax & 0xff00) | value,
                        Width::Word => value,
                    };
                    self.regs.si = si.wrapping_add(delta);
                }
                StringOp::Scas => {
                    let value = self.read(es, di, width);
                    self.alu(AluOp::Sub, self.regs.ax, value, width);
                    self.regs.di = di.wrapping_add(delta);
                }
                StringOp::Cmps => {
                    let (a, b) = (self.read(source, si, width), self.read(es, di, width));
                    self.alu(AluOp::Sub, a, b, width);
                    self.regs.si = si.wrapping_add(delta);
                    self.regs.di = di.wrapping_add(delta);
                }
            }
            if repeat != Repeat::Once {
                self.regs.cx = self.regs.cx.wrapping_sub(1);
            }
            let zero = self.flag(Flag::Zero);
            if (repeat == Repeat::WhileEqual && !zero) || (repeat == Repeat::WhileNotEqual && zero)
            {
                break;
            }
        }
    }

    /// Reads a byte (zero-extended) or word.
    pub fn read(&self, segment: u16, offset: u16, width: Width) -> u16 {
        match width {
            Width::Byte => u16::from(self.memory.read8(segment, offset)),
            Width::Word => self.memory.read16(segment, offset),
        }
    }

    /// Writes the low byte or the whole word of `value`.
    pub fn write(&mut self, segment: u16, offset: u16, value: u16, width: Width) {
        match width {
            Width::Byte => self.memory.write8(segment, offset, value as u8),
            Width::Word => self.memory.write16(segment, offset, value),
        }
    }
}
