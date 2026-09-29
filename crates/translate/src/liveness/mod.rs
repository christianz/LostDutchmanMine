//! Which registers and flags are read after a routine returns, before they are
//! overwritten: the part of a routine's result its callers can observe.
//!
//! A backward dataflow over each segment's recovered code. Anything whose next
//! reader is unknown counts as read: at calls, interrupts, returns, indirect or
//! far jumps and exits from recovered code everything is live. The analysis is
//! therefore conservative: it may keep a register the callers never read, but
//! never drops one they do.

mod effects;

use std::collections::BTreeMap;

use machine::{Address, Flag, Reg16};

use crate::Instruction;
use crate::ir::{Ir, Operand};
use crate::translation::{Segment, Translation};

use effects::effect;

/// The registers a set can hold, in the bit order of [`Uses::of_register`].
const REGISTERS: [Reg16; 12] = [
    Reg16::Ax,
    Reg16::Cx,
    Reg16::Dx,
    Reg16::Bx,
    Reg16::Sp,
    Reg16::Bp,
    Reg16::Si,
    Reg16::Di,
    Reg16::Es,
    Reg16::Cs,
    Reg16::Ss,
    Reg16::Ds,
];
/// The flags a set can hold, in the bit order of [`Uses::of_flag`].
const FLAGS: [Flag; 9] = [
    Flag::Carry,
    Flag::Parity,
    Flag::Adjust,
    Flag::Zero,
    Flag::Sign,
    Flag::Trap,
    Flag::Interrupt,
    Flag::Direction,
    Flag::Overflow,
];

/// A set of registers and flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Uses(u32);

impl Uses {
    /// Nothing.
    pub const NONE: Uses = Uses(0);
    /// Every register and flag.
    pub const ALL: Uses = Uses((1 << (REGISTERS.len() + FLAGS.len())) - 1);

    /// The set as bits, for generated code.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The set of [`Uses::bits`].
    pub const fn from_bits(bits: u32) -> Self {
        Uses(bits & Uses::ALL.0)
    }

    /// One register.
    pub const fn of_register(register: Reg16) -> Self {
        let bit = match register {
            Reg16::Ax => 0,
            Reg16::Cx => 1,
            Reg16::Dx => 2,
            Reg16::Bx => 3,
            Reg16::Sp => 4,
            Reg16::Bp => 5,
            Reg16::Si => 6,
            Reg16::Di => 7,
            Reg16::Es => 8,
            Reg16::Cs => 9,
            Reg16::Ss => 10,
            Reg16::Ds => 11,
        };
        Uses(1 << bit)
    }

    /// One flag.
    pub const fn of_flag(flag: Flag) -> Self {
        let bit = match flag {
            Flag::Carry => 0,
            Flag::Parity => 1,
            Flag::Adjust => 2,
            Flag::Zero => 3,
            Flag::Sign => 4,
            Flag::Trap => 5,
            Flag::Interrupt => 6,
            Flag::Direction => 7,
            Flag::Overflow => 8,
        };
        Uses(1 << (REGISTERS.len() + bit))
    }

    /// Whether the set holds `register`.
    pub fn register(self, register: Reg16) -> bool {
        self.0 & Uses::of_register(register).0 != 0
    }

    /// Whether the set holds `flag`.
    pub fn flag(self, flag: Flag) -> bool {
        self.0 & Uses::of_flag(flag).0 != 0
    }

    /// The registers in the set.
    pub fn registers(self) -> impl Iterator<Item = Reg16> {
        REGISTERS.into_iter().filter(move |&register| self.register(register))
    }

    /// The flags in the set, as a mask of the flags register.
    pub fn flags_mask(self) -> u16 {
        FLAGS.into_iter().filter(|&flag| self.flag(flag)).fold(0, |mask, flag| mask | flag.mask())
    }

    /// Both sets.
    #[must_use]
    pub const fn with(self, other: Uses) -> Self {
        Uses(self.0 | other.0)
    }

    /// This set without `other`.
    #[must_use]
    pub const fn without(self, other: Uses) -> Self {
        Uses(self.0 & !other.0)
    }
}

/// What a routine's callers may read once it returns: the union, over every
/// known call site, of what is live at the return address. A routine with no
/// known call sites keeps everything.
pub fn routine_live_out(translation: &Translation, routine: Address) -> Uses {
    let mut live = Uses::NONE;
    let mut callers = 0;
    for segment in translation.segments.values() {
        let live_in = live_in(segment);
        for instruction in segment.instructions.values() {
            let calls_routine = match instruction.ir {
                Ir::Call(Operand::Imm(target)) => {
                    segment.number == routine.segment && target == routine.offset
                }
                Ir::FarCall(target) => target == routine,
                _ => false,
            };
            if calls_routine {
                callers += 1;
                live = live.with(live_in.get(&instruction.next()).copied().unwrap_or(Uses::ALL));
            }
        }
    }
    if callers == 0 { Uses::ALL } else { live }
}

/// What is live on entry to each instruction of a segment.
fn live_in(segment: &Segment) -> BTreeMap<u16, Uses> {
    let mut live: BTreeMap<u16, Uses> =
        segment.instructions.keys().map(|&offset| (offset, Uses::NONE)).collect();
    loop {
        let mut changed = false;
        for (&offset, instruction) in segment.instructions.iter().rev() {
            let effect = effect(&instruction.ir);
            let out = live_out(segment, instruction, &live);
            let entering = effect.reads.with(out.without(effect.kills));
            if live.insert(offset, entering) != Some(entering) {
                changed = true;
            }
        }
        if !changed {
            return live;
        }
    }
}

/// What is live after an instruction: the union over its successors, or
/// everything where the next reader is unknown.
fn live_out(segment: &Segment, instruction: &Instruction, live: &BTreeMap<u16, Uses>) -> Uses {
    let next = instruction.next();
    let successors: &[u16] = match instruction.ir {
        Ir::Jump { cond: None, target } => &[target],
        Ir::Jump { cond: Some(_), target } | Ir::Loop { target, .. } => &[target, next],
        Ir::JumpIndirect(_)
        | Ir::Call(_)
        | Ir::FarJump(_)
        | Ir::FarJumpIndirect(_)
        | Ir::FarCall(_)
        | Ir::FarCallIndirect(_)
        | Ir::Return { .. }
        | Ir::ReturnFar { .. }
        | Ir::ReturnFromInterrupt
        | Ir::Interrupt(_) => return Uses::ALL,
        _ => &[next],
    };
    successors.iter().fold(Uses::NONE, |out, successor| {
        let known = segment.instructions.contains_key(successor);
        out.with(if known { live[successor] } else { Uses::ALL })
    })
}
