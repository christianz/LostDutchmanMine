//! A reference interpreter for the IR, used only by tests.
//!
//! It executes one instruction with exactly the semantics and evaluation order
//! of the translated code, so hardware test vectors can check the IR, and the
//! translator's output can be compared against it.

use machine::{AluOp, Fault, Flag, Machine, RESERVED};
use translate::Instruction;
use translate::ir::{Ir, LoopKind, MemRef, Operand};

/// What happens after an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Continue with the next instruction.
    Next,
    /// Continue at an offset in the same segment.
    Jump(u16),
    /// CS:IP have been set (far transfers, returns and indirect jumps).
    Transfer,
    /// An `INT`, which the interpreter leaves to the caller.
    Interrupt(u8),
}

/// Executes `instruction` on `m`. Far branch targets are image-relative and are
/// placed at `load_segment`; IP is left for the caller to advance from `Control`.
///
/// # Errors
///
/// A [`Fault`] from division.
#[allow(clippy::too_many_lines, reason = "one arm per IR variant reads best as a table")]
pub fn execute(
    m: &mut Machine,
    instruction: &Instruction,
    load_segment: u16,
) -> Result<Control, Fault> {
    let next = instruction.next();
    match &instruction.ir {
        Ir::Move { dst, src, .. } => {
            let value = read(m, *src);
            write(m, *dst, value);
        }
        Ir::Alu { op, dst, src, width } => {
            let result = m.alu(*op, read(m, *dst), read(m, *src), *width);
            write(m, *dst, result);
        }
        Ir::Compare { op, a, b, width } => {
            m.alu(*op, read(m, *a), read(m, *b), *width);
        }
        Ir::Increment { dst, width } => {
            let result = m.alu(AluOp::Inc, read(m, *dst), 1, *width);
            write(m, *dst, result);
        }
        Ir::Decrement { dst, width } => {
            let result = m.alu(AluOp::Dec, read(m, *dst), 1, *width);
            write(m, *dst, result);
        }
        Ir::Negate { dst, width } => {
            let result = m.alu(AluOp::Neg, 0, read(m, *dst), *width);
            write(m, *dst, result);
        }
        Ir::Not { dst, .. } => {
            let value = !read(m, *dst);
            write(m, *dst, value);
        }
        Ir::Shift { op, dst, count, width } => {
            let result = m.shift(*op, read(m, *dst), read(m, *count), *width);
            write(m, *dst, result);
        }
        Ir::Push(src) => {
            let value = read(m, *src);
            m.push(value);
        }
        Ir::Pop(dst) => {
            let value = m.pop();
            write(m, *dst, value);
        }
        Ir::PushFlags => m.push(m.regs.flags | RESERVED),
        Ir::PopFlags => m.regs.flags = m.pop() | RESERVED,
        Ir::Exchange { a, b, .. } => {
            let first = read(m, *a);
            let second = read(m, *b);
            write(m, *a, second);
            write(m, *b, first);
        }
        Ir::LoadAddress { dst, address } => m.regs.set(*dst, offset(m, *address)),
        Ir::LoadFarPointer { dst, segment, address } => {
            let (seg, at) = (m.regs.get(address.segment), offset(m, *address));
            let (pointer_offset, pointer_segment) =
                (m.memory.read16(seg, at), m.memory.read16(seg, at.wrapping_add(2)));
            m.regs.set(*dst, pointer_offset);
            m.regs.set(*segment, pointer_segment);
        }
        Ir::ConvertByteToWord => m.regs.ax = i16::from(m.regs.ax as u8 as i8) as u16,
        Ir::ConvertWordToDouble => m.regs.dx = if m.regs.ax & 0x8000 == 0 { 0 } else { 0xffff },
        Ir::Multiply { src, width, signed } => {
            let value = read(m, *src);
            m.multiply(value, *width, *signed);
        }
        Ir::Divide { src, width, signed } => {
            let value = read(m, *src);
            m.divide(value, *width, *signed)?;
        }
        Ir::Jump { cond, target } => {
            if cond.is_none_or(|cond| m.condition(cond)) {
                return Ok(Control::Jump(*target));
            }
        }
        Ir::JumpIndirect(target) => {
            m.regs.ip = read(m, *target);
            return Ok(Control::Transfer);
        }
        Ir::Loop { kind, target } => {
            m.regs.cx = m.regs.cx.wrapping_sub(1);
            let zero = m.flag(Flag::Zero);
            let taken = m.regs.cx != 0
                && match kind {
                    LoopKind::Always => true,
                    LoopKind::WhileEqual => zero,
                    LoopKind::WhileNotEqual => !zero,
                };
            if taken {
                return Ok(Control::Jump(*target));
            }
        }
        Ir::Call(target) => {
            let target = read(m, *target);
            m.push(next);
            m.regs.ip = target;
            return Ok(Control::Transfer);
        }
        Ir::FarJump(target) => {
            m.regs.cs = target.segment.wrapping_add(load_segment);
            m.regs.ip = target.offset;
            return Ok(Control::Transfer);
        }
        Ir::FarJumpIndirect(pointer) => {
            let (ip, cs) = far_pointer(m, *pointer);
            (m.regs.cs, m.regs.ip) = (cs, ip);
            return Ok(Control::Transfer);
        }
        Ir::FarCall(target) => {
            m.push(m.regs.cs);
            m.push(next);
            m.regs.cs = target.segment.wrapping_add(load_segment);
            m.regs.ip = target.offset;
            return Ok(Control::Transfer);
        }
        Ir::FarCallIndirect(pointer) => {
            let (ip, cs) = far_pointer(m, *pointer);
            m.push(m.regs.cs);
            m.push(next);
            (m.regs.cs, m.regs.ip) = (cs, ip);
            return Ok(Control::Transfer);
        }
        Ir::Return { release } => {
            m.regs.ip = m.pop();
            m.regs.sp = m.regs.sp.wrapping_add(*release);
            return Ok(Control::Transfer);
        }
        Ir::ReturnFar { release } => {
            m.regs.ip = m.pop();
            m.regs.cs = m.pop();
            m.regs.sp = m.regs.sp.wrapping_add(*release);
            return Ok(Control::Transfer);
        }
        Ir::ReturnFromInterrupt => {
            m.regs.ip = m.pop();
            m.regs.cs = m.pop();
            m.regs.flags = m.pop() | RESERVED;
            return Ok(Control::Transfer);
        }
        Ir::Interrupt(number) => return Ok(Control::Interrupt(*number)),
        Ir::In { dst, port, width } => {
            let port = read(m, *port);
            let value = m.port_in(port, *width);
            write(m, *dst, value);
        }
        Ir::Out { port, src, width } => {
            let (port, value) = (read(m, *port), read(m, *src));
            m.port_out(port, value, *width);
        }
        Ir::SetFlag { flag, on } => m.set_flag(*flag, *on),
        Ir::ComplementCarry => m.set_flag(Flag::Carry, !m.flag(Flag::Carry)),
        Ir::LoadAhFromFlags => {
            m.regs.ax = (m.regs.ax & 0x00ff) | (((m.regs.flags & 0xd5) | RESERVED) << 8);
        }
        Ir::StoreAhIntoFlags => {
            m.regs.flags = (m.regs.flags & !0xd5) | ((m.regs.ax >> 8) & 0xd5) | RESERVED;
        }
        Ir::Translate => {
            let at = m.regs.bx.wrapping_add(m.regs.ax & 0xff);
            m.regs.ax = (m.regs.ax & 0xff00) | u16::from(m.memory.read8(m.regs.ds, at));
        }
        Ir::String { op, width, repeat, source } => {
            let source = m.regs.get(*source);
            m.string_op(*op, *width, *repeat, source);
        }
        Ir::Nop => {}
    }
    Ok(Control::Next)
}

fn offset(m: &Machine, address: MemRef) -> u16 {
    let base = address.base.map_or(0, |register| m.regs.get(register));
    let index = address.index.map_or(0, |register| m.regs.get(register));
    base.wrapping_add(index).wrapping_add(address.displacement)
}

fn far_pointer(m: &Machine, pointer: MemRef) -> (u16, u16) {
    let (segment, at) = (m.regs.get(pointer.segment), offset(m, pointer));
    (m.memory.read16(segment, at), m.memory.read16(segment, at.wrapping_add(2)))
}

fn read(m: &Machine, operand: Operand) -> u16 {
    match operand {
        Operand::Reg(register) => m.regs.get(register),
        Operand::Reg8(register) => u16::from(m.regs.get8(register)),
        Operand::Imm(value) => value,
        Operand::Mem(address) => {
            m.read(m.regs.get(address.segment), offset(m, address), address.width)
        }
    }
}

fn write(m: &mut Machine, operand: Operand, value: u16) {
    match operand {
        Operand::Reg(register) => m.regs.set(register, value),
        Operand::Reg8(register) => m.regs.set8(register, value as u8),
        Operand::Imm(_) => unreachable!("lowering never writes to an immediate"),
        Operand::Mem(address) => {
            let (segment, at) = (m.regs.get(address.segment), offset(m, address));
            m.write(segment, at, value, address.width);
        }
    }
}
