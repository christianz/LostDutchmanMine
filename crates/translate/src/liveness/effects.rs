//! What each instruction reads, and what it overwrites entirely.
//!
//! A write counts only when it replaces the whole register or flag whatever
//! the inputs: a byte register is half its word, a shift by CL may shift by
//! zero, and a repeated string instruction may run no times, so none of those
//! overwrite anything.

use machine::{AluOp, Cond, Flag, Reg16, Repeat, ShiftOp, StringOp, Width};

use super::Uses;
use crate::ir::{Ir, LoopKind, MemRef, Operand};

/// An instruction's reads and overwrites.
pub(super) struct Effect {
    pub(super) reads: Uses,
    pub(super) kills: Uses,
}

fn register(register: Reg16) -> Uses {
    Uses::of_register(register)
}

fn flags(flags: &[Flag]) -> Uses {
    flags.iter().fold(Uses::NONE, |uses, &flag| uses.with(Uses::of_flag(flag)))
}

/// The six flags arithmetic sets.
fn arithmetic() -> Uses {
    flags(&[Flag::Carry, Flag::Parity, Flag::Adjust, Flag::Zero, Flag::Sign, Flag::Overflow])
}

/// The flags LAHF and SAHF move.
fn low_flags() -> Uses {
    flags(&[Flag::Sign, Flag::Zero, Flag::Adjust, Flag::Parity, Flag::Carry])
}

fn all_flags() -> Uses {
    flags(&[
        Flag::Carry,
        Flag::Parity,
        Flag::Adjust,
        Flag::Zero,
        Flag::Sign,
        Flag::Trap,
        Flag::Interrupt,
        Flag::Direction,
        Flag::Overflow,
    ])
}

/// The registers forming a memory address.
fn address(mem: MemRef) -> Uses {
    [Some(mem.segment), mem.base, mem.index]
        .into_iter()
        .flatten()
        .fold(Uses::NONE, |uses, r| uses.with(register(r)))
}

/// Reading an operand's value.
fn read(operand: Operand) -> Uses {
    match operand {
        Operand::Reg(r) => register(r),
        Operand::Reg8(r) => register(r.parent().0),
        Operand::Imm(_) => Uses::NONE,
        Operand::Mem(mem) => address(mem),
    }
}

/// Writing an operand: what it reads to address, and what it overwrites.
fn write(operand: Operand) -> Effect {
    match operand {
        Operand::Reg(r) => Effect { reads: Uses::NONE, kills: register(r) },
        Operand::Mem(mem) => Effect { reads: address(mem), kills: Uses::NONE },
        Operand::Reg8(_) | Operand::Imm(_) => Effect { reads: Uses::NONE, kills: Uses::NONE },
    }
}

/// Reads `reads`, then writes `dst`, overwriting `also` too.
fn into(dst: Operand, reads: Uses, also: Uses) -> Effect {
    let written = write(dst);
    Effect { reads: reads.with(written.reads), kills: written.kills.with(also) }
}

fn condition(cond: Cond) -> Uses {
    match cond {
        Cond::Equal | Cond::NotEqual => flags(&[Flag::Zero]),
        Cond::Below | Cond::AboveOrEqual => flags(&[Flag::Carry]),
        Cond::BelowOrEqual | Cond::Above => flags(&[Flag::Carry, Flag::Zero]),
        Cond::Less | Cond::GreaterOrEqual => flags(&[Flag::Sign, Flag::Overflow]),
        Cond::LessOrEqual | Cond::Greater => flags(&[Flag::Zero, Flag::Sign, Flag::Overflow]),
        Cond::Sign | Cond::NotSign => flags(&[Flag::Sign]),
        Cond::Overflow | Cond::NotOverflow => flags(&[Flag::Overflow]),
        Cond::Parity | Cond::NotParity => flags(&[Flag::Parity]),
        Cond::CxZero => register(Reg16::Cx),
    }
}

fn stack() -> Uses {
    register(Reg16::Sp).with(register(Reg16::Ss))
}

/// The accumulator a byte or word operation uses, and DX with it for words.
fn accumulator(width: Width, with_dx: bool) -> Uses {
    let ax = register(Reg16::Ax);
    if width == Width::Word && with_dx { ax.with(register(Reg16::Dx)) } else { ax }
}

#[allow(clippy::too_many_lines, reason = "one arm per IR variant reads best as a table")]
pub(super) fn effect(ir: &Ir) -> Effect {
    let none = Uses::NONE;
    match *ir {
        Ir::Move { dst, src, .. } => into(dst, read(src), none),
        // XOR or SUB of a register with itself clears it, whatever it held.
        Ir::Alu {
            op: AluOp::Xor | AluOp::Sub, dst: Operand::Reg(a), src: Operand::Reg(b), ..
        } if a == b => Effect { reads: none, kills: register(a).with(arithmetic()) },
        Ir::Alu { op, dst, src, .. } => {
            let carry =
                if matches!(op, AluOp::Adc | AluOp::Sbb) { flags(&[Flag::Carry]) } else { none };
            into(dst, read(dst).with(read(src)).with(carry), arithmetic())
        }
        Ir::Compare { a, b, .. } => Effect { reads: read(a).with(read(b)), kills: arithmetic() },
        Ir::Increment { dst, .. } | Ir::Decrement { dst, .. } => {
            into(dst, read(dst), arithmetic().without(flags(&[Flag::Carry])))
        }
        Ir::Negate { dst, .. } => into(dst, read(dst), arithmetic()),
        Ir::Not { dst, .. } => into(dst, read(dst), none),
        Ir::Shift { op, dst, count, .. } => {
            let carry_in = if matches!(op, ShiftOp::Rcl | ShiftOp::Rcr) {
                flags(&[Flag::Carry])
            } else {
                none
            };
            let reads = read(dst).with(read(count)).with(carry_in);
            let written = match count {
                Operand::Imm(0) | Operand::Reg8(_) | Operand::Reg(_) | Operand::Mem(_) => none,
                Operand::Imm(n) => {
                    let shifted = if matches!(op, ShiftOp::Shl | ShiftOp::Shr | ShiftOp::Sar) {
                        flags(&[Flag::Carry, Flag::Parity, Flag::Zero, Flag::Sign])
                    } else {
                        flags(&[Flag::Carry])
                    };
                    if n == 1 { shifted.with(flags(&[Flag::Overflow])) } else { shifted }
                }
            };
            into(dst, reads, written)
        }
        Ir::Push(src) => Effect { reads: read(src).with(stack()), kills: none },
        Ir::Pop(dst) => into(dst, stack(), none),
        Ir::PushFlags => Effect { reads: all_flags().with(stack()), kills: none },
        Ir::PopFlags => Effect { reads: stack(), kills: all_flags() },
        Ir::Exchange { a, b, .. } => {
            let (first, second) = (write(a), write(b));
            let reads = read(a).with(read(b)).with(first.reads).with(second.reads);
            Effect { reads, kills: first.kills.with(second.kills) }
        }
        Ir::LoadAddress { dst, address: mem } => {
            let offset = [mem.base, mem.index].into_iter().flatten();
            let reads = offset.fold(none, |uses, r| uses.with(register(r)));
            Effect { reads, kills: register(dst) }
        }
        Ir::LoadFarPointer { dst, segment, address: mem } => {
            Effect { reads: address(mem), kills: register(dst).with(register(segment)) }
        }
        Ir::ConvertByteToWord => Effect { reads: register(Reg16::Ax), kills: register(Reg16::Ax) },
        Ir::ConvertWordToDouble => {
            Effect { reads: register(Reg16::Ax), kills: register(Reg16::Dx) }
        }
        Ir::Multiply { src, width, .. } => Effect {
            reads: read(src).with(register(Reg16::Ax)),
            kills: accumulator(width, true).with(flags(&[Flag::Carry, Flag::Overflow])),
        },
        Ir::Divide { src, width, .. } => Effect {
            reads: read(src).with(accumulator(width, true)),
            kills: accumulator(width, true),
        },
        Ir::Jump { cond, .. } => Effect { reads: cond.map_or(none, condition), kills: none },
        Ir::Loop { kind, .. } => {
            let zero = if kind == LoopKind::Always { none } else { flags(&[Flag::Zero]) };
            Effect { reads: register(Reg16::Cx).with(zero), kills: none }
        }
        Ir::JumpIndirect(target) | Ir::Call(target) => Effect { reads: read(target), kills: none },
        Ir::FarJumpIndirect(mem) | Ir::FarCallIndirect(mem) => {
            Effect { reads: address(mem), kills: none }
        }
        Ir::FarJump(_)
        | Ir::FarCall(_)
        | Ir::Return { .. }
        | Ir::ReturnFar { .. }
        | Ir::ReturnFromInterrupt
        | Ir::Interrupt(_)
        | Ir::Nop => Effect { reads: none, kills: none },
        Ir::In { dst, port, .. } => into(dst, read(port), none),
        Ir::Out { port, src, .. } => Effect { reads: read(port).with(read(src)), kills: none },
        Ir::SetFlag { flag, .. } => Effect { reads: none, kills: flags(&[flag]) },
        Ir::ComplementCarry => {
            Effect { reads: flags(&[Flag::Carry]), kills: flags(&[Flag::Carry]) }
        }
        Ir::LoadAhFromFlags => Effect { reads: low_flags().with(register(Reg16::Ax)), kills: none },
        Ir::StoreAhIntoFlags => Effect { reads: register(Reg16::Ax), kills: low_flags() },
        Ir::Translate => Effect {
            reads: register(Reg16::Ax).with(register(Reg16::Bx)).with(register(Reg16::Ds)),
            kills: none,
        },
        Ir::String { op, width, repeat, source } => string(op, width, repeat, source),
    }
}

fn string(op: StringOp, width: Width, repeat: Repeat, source: Reg16) -> Effect {
    let direction = flags(&[Flag::Direction]);
    let from = register(Reg16::Si).with(register(source));
    let to = register(Reg16::Di).with(register(Reg16::Es));
    let reads = direction.with(match op {
        StringOp::Movs | StringOp::Cmps => from.with(to),
        StringOp::Stos | StringOp::Scas => to.with(accumulator(width, false)),
        StringOp::Lods => from.with(accumulator(width, false)),
    });
    let once = repeat == Repeat::Once;
    let reads = if once { reads } else { reads.with(register(Reg16::Cx)) };
    let kills = match op {
        StringOp::Scas | StringOp::Cmps if once => arithmetic(),
        StringOp::Lods if once && width == Width::Word => register(Reg16::Ax),
        _ => Uses::NONE,
    };
    Effect { reads, kills }
}
