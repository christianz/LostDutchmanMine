//! Lowering decoded 8086 instructions into IR.
//!
//! The rules match the original C++ emitter exactly (`tools/translate.py`), so
//! the Rust and C++ builds execute the same semantics. Anything the 8086 lacks,
//! or the game never uses, is rejected rather than approximated.

use std::collections::HashSet;

use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction as Decoded, IntelFormatter, MemorySize,
    Mnemonic, OpKind, Register,
};
use machine::{
    Address, AluOp, Cond, Flag, LOAD_SEGMENT, Reg8, Reg16, Repeat, ShiftOp, StringOp, Width,
};

use crate::ir::{Ir, LoopKind, MemRef, Operand};

/// Image offsets of the words the loader relocates by the load segment.
#[derive(Clone, Debug, Default)]
pub struct Relocations(HashSet<u32>);

impl Relocations {
    /// The relocations at these image offsets.
    pub fn from_offsets(offsets: impl IntoIterator<Item = u32>) -> Self {
        Relocations(offsets.into_iter().collect())
    }

    /// Whether any relocated word starts within `length` bytes from `start`.
    pub fn touch(&self, start: u32, length: u32) -> bool {
        (start..start + length).any(|at| self.0.contains(&at))
    }
}

/// A decoded and lowered instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    /// Where it is in the program image.
    pub address: Address,
    /// Its length in bytes.
    pub length: u16,
    /// Its disassembly.
    pub text: String,
    /// What it means.
    pub ir: Ir,
}

impl Instruction {
    /// The offset of the instruction that follows.
    pub const fn next(&self) -> u16 {
        self.address.offset.wrapping_add(self.length)
    }
}

/// Why an instruction cannot be lowered.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LowerError {
    /// The bytes are not a valid instruction.
    #[error("{0}: the bytes do not decode as an 8086 instruction")]
    Invalid(Address),
    /// A valid instruction the translator does not support.
    #[error("{address}: unsupported instruction `{text}`")]
    Unsupported {
        /// Where it is.
        address: Address,
        /// Its disassembly.
        text: String,
    },
}

/// Decodes the instruction at the start of `bytes` and lowers it.
///
/// # Errors
///
/// [`LowerError`] when the bytes are not an instruction the translator supports.
pub fn lower(
    address: Address,
    bytes: &[u8],
    relocations: &Relocations,
) -> Result<Instruction, LowerError> {
    let decoded =
        Decoder::with_ip(16, bytes, u64::from(address.offset), DecoderOptions::NONE).decode();
    if decoded.is_invalid() {
        return Err(LowerError::Invalid(address));
    }
    let text = disassemble(&decoded);
    let start = u32::from(address.segment) * 16 + u32::from(address.offset);
    let length = decoded.len() as u32;
    // Far branch targets name their segment explicitly; every other immediate in
    // a relocated instruction is a segment the loader moved by LOAD_SEGMENT.
    let far_branch = matches!(decoded.mnemonic(), Mnemonic::Call | Mnemonic::Jmp)
        && (decoded.op0_kind() == OpKind::FarBranch16
            || decoded.memory_size() == MemorySize::SegPtr16);
    let lowering =
        Lowering { i: &decoded, relocated: !far_branch && relocations.touch(start, length) };
    match lowering.ir() {
        Some(ir) => Ok(Instruction { address, length: length as u16, text, ir }),
        None => Err(LowerError::Unsupported { address, text }),
    }
}

fn disassemble(decoded: &Decoded) -> String {
    if decoded.op_count() > 0 && decoded.op0_kind() == OpKind::FarBranch16 {
        let mnemonic = format!("{:?}", decoded.mnemonic()).to_lowercase();
        let (segment, offset) = (decoded.far_branch_selector(), decoded.far_branch16());
        return format!("{mnemonic} far {segment:04x}:{offset:04x}");
    }
    let mut formatter = IntelFormatter::new();
    let options = formatter.options_mut();
    options.set_hex_prefix("0x");
    options.set_hex_suffix("");
    options.set_uppercase_hex(false);
    options.set_space_after_operand_separator(true);
    options.set_show_branch_size(false);
    let mut text = String::new();
    formatter.format(decoded, &mut text);
    text
}

struct Lowering<'a> {
    i: &'a Decoded,
    relocated: bool,
}

impl Lowering<'_> {
    fn ir(&self) -> Option<Ir> {
        use Mnemonic as M;
        let i = self.i;
        let ir = match i.mnemonic() {
            M::Mov => Ir::Move { dst: self.op(0)?, src: self.op(1)?, width: self.width(0)? },
            M::Add | M::Adc | M::Sub | M::Sbb | M::And | M::Or | M::Xor => Ir::Alu {
                op: alu_op(i.mnemonic())?,
                dst: self.op(0)?,
                src: self.op(1)?,
                width: self.width(0)?,
            },
            M::Cmp | M::Test => Ir::Compare {
                op: if i.mnemonic() == M::Cmp { AluOp::Sub } else { AluOp::And },
                a: self.op(0)?,
                b: self.op(1)?,
                width: self.width(0)?,
            },
            M::Inc => Ir::Increment { dst: self.op(0)?, width: self.width(0)? },
            M::Dec => Ir::Decrement { dst: self.op(0)?, width: self.width(0)? },
            M::Neg => Ir::Negate { dst: self.op(0)?, width: self.width(0)? },
            M::Not => Ir::Not { dst: self.op(0)?, width: self.width(0)? },
            M::Shl | M::Shr | M::Sar | M::Rol | M::Ror | M::Rcl | M::Rcr => Ir::Shift {
                op: shift_op(i.mnemonic())?,
                dst: self.op(0)?,
                count: self.op(1)?,
                width: self.width(0)?,
            },
            M::Push => Ir::Push(self.op(0)?),
            M::Pop => Ir::Pop(self.op(0)?),
            M::Pushf => Ir::PushFlags,
            M::Popf => Ir::PopFlags,
            M::Xchg => Ir::Exchange { a: self.op(0)?, b: self.op(1)?, width: self.width(0)? },
            M::Lea => Ir::LoadAddress { dst: self.reg16(0)?, address: self.mem(Width::Word) },
            M::Lds | M::Les => Ir::LoadFarPointer {
                dst: self.reg16(0)?,
                segment: if i.mnemonic() == M::Lds { Reg16::Ds } else { Reg16::Es },
                address: self.mem(Width::Word),
            },
            M::Cbw => Ir::ConvertByteToWord,
            M::Cwd => Ir::ConvertWordToDouble,
            M::Mul | M::Imul | M::Div | M::Idiv if i.op_count() == 1 => {
                let (src, width) = (self.op(0)?, self.width(0)?);
                let signed = matches!(i.mnemonic(), M::Imul | M::Idiv);
                if matches!(i.mnemonic(), M::Mul | M::Imul) {
                    Ir::Multiply { src, width, signed }
                } else {
                    Ir::Divide { src, width, signed }
                }
            }
            M::Jcxz => Ir::Jump { cond: Some(Cond::CxZero), target: i.near_branch16() },
            M::Loop | M::Loope | M::Loopne => Ir::Loop {
                kind: match i.mnemonic() {
                    M::Loope => LoopKind::WhileEqual,
                    M::Loopne => LoopKind::WhileNotEqual,
                    _ => LoopKind::Always,
                },
                target: i.near_branch16(),
            },
            M::Jmp => self.branch(false)?,
            M::Call => self.branch(true)?,
            M::Ret => Ir::Return { release: self.release() },
            M::Retf => Ir::ReturnFar { release: self.release() },
            M::Iret => Ir::ReturnFromInterrupt,
            M::Int => Ir::Interrupt(i.immediate8()),
            M::In => Ir::In { dst: self.op(0)?, port: self.op(1)?, width: self.width(0)? },
            M::Out => Ir::Out { port: self.op(0)?, src: self.op(1)?, width: self.width(1)? },
            M::Clc | M::Stc | M::Cld | M::Std | M::Cli | M::Sti => {
                let (flag, on) = match i.mnemonic() {
                    M::Clc => (Flag::Carry, false),
                    M::Stc => (Flag::Carry, true),
                    M::Cld => (Flag::Direction, false),
                    M::Std => (Flag::Direction, true),
                    M::Cli => (Flag::Interrupt, false),
                    _ => (Flag::Interrupt, true),
                };
                Ir::SetFlag { flag, on }
            }
            M::Cmc => Ir::ComplementCarry,
            M::Lahf => Ir::LoadAhFromFlags,
            M::Sahf => Ir::StoreAhIntoFlags,
            M::Xlatb => Ir::Translate,
            M::Nop => Ir::Nop,
            mnemonic => match condition(mnemonic) {
                Some(cond) => Ir::Jump { cond: Some(cond), target: i.near_branch16() },
                None => self.string()?,
            },
        };
        Some(ir)
    }

    /// JMP or CALL: near direct, near indirect, far direct or far indirect.
    fn branch(&self, call: bool) -> Option<Ir> {
        let i = self.i;
        Some(match i.op0_kind() {
            OpKind::NearBranch16 if call => Ir::Call(Operand::Imm(i.near_branch16())),
            OpKind::NearBranch16 => Ir::Jump { cond: None, target: i.near_branch16() },
            OpKind::FarBranch16 => {
                // The image holds unrelocated, image-relative segments.
                let target = Address::new(i.far_branch_selector(), i.far_branch16());
                if call { Ir::FarCall(target) } else { Ir::FarJump(target) }
            }
            OpKind::Memory if i.memory_size() == MemorySize::SegPtr16 => {
                let pointer = self.mem(Width::Word);
                if call { Ir::FarCallIndirect(pointer) } else { Ir::FarJumpIndirect(pointer) }
            }
            _ if call => Ir::Call(self.op(0)?),
            _ => Ir::JumpIndirect(self.op(0)?),
        })
    }

    fn string(&self) -> Option<Ir> {
        use Mnemonic as M;
        let i = self.i;
        let (op, width) = match i.mnemonic() {
            M::Movsb => (StringOp::Movs, Width::Byte),
            M::Movsw => (StringOp::Movs, Width::Word),
            M::Stosb => (StringOp::Stos, Width::Byte),
            M::Stosw => (StringOp::Stos, Width::Word),
            M::Lodsb => (StringOp::Lods, Width::Byte),
            M::Lodsw => (StringOp::Lods, Width::Word),
            M::Scasb => (StringOp::Scas, Width::Byte),
            M::Scasw => (StringOp::Scas, Width::Word),
            M::Cmpsb => (StringOp::Cmps, Width::Byte),
            M::Cmpsw => (StringOp::Cmps, Width::Word),
            _ => return None,
        };
        let compares = matches!(op, StringOp::Scas | StringOp::Cmps);
        let repeat = if i.has_repne_prefix() {
            Repeat::WhileNotEqual
        } else if i.has_rep_prefix() {
            if compares { Repeat::WhileEqual } else { Repeat::Rep }
        } else {
            Repeat::Once
        };
        // Only the SI operand honours a segment override; ES:DI never does.
        let reads_si = matches!(op, StringOp::Movs | StringOp::Lods | StringOp::Cmps);
        let source = match i.segment_prefix() {
            prefix if reads_si && prefix != Register::None => reg16(prefix)?,
            _ => Reg16::Ds,
        };
        Some(Ir::String { op, width, repeat, source })
    }

    fn release(&self) -> u16 {
        if self.i.op_count() == 1 { self.i.immediate16() } else { 0 }
    }

    fn op(&self, n: u32) -> Option<Operand> {
        let i = self.i;
        let relocate =
            |value: u16| if self.relocated { value.wrapping_add(LOAD_SEGMENT) } else { value };
        Some(match i.op_kind(n) {
            OpKind::Register => match reg16(i.op_register(n)) {
                Some(register) => Operand::Reg(register),
                None => Operand::Reg8(reg8(i.op_register(n))?),
            },
            OpKind::Memory => Operand::Mem(self.mem(self.width(n)?)),
            OpKind::Immediate8 => Operand::Imm(relocate(u16::from(i.immediate8())) & 0xff),
            OpKind::Immediate16 => Operand::Imm(relocate(i.immediate16())),
            OpKind::Immediate8to16 => Operand::Imm(relocate(i.immediate8to16() as u16)),
            OpKind::NearBranch16 => Operand::Imm(i.near_branch16()),
            _ => return None,
        })
    }

    fn reg16(&self, n: u32) -> Option<Reg16> {
        (self.i.op_kind(n) == OpKind::Register).then(|| reg16(self.i.op_register(n))).flatten()
    }

    fn width(&self, n: u32) -> Option<Width> {
        let i = self.i;
        match i.op_kind(n) {
            OpKind::Register => {
                let register = i.op_register(n);
                if reg16(register).is_some() {
                    Some(Width::Word)
                } else {
                    reg8(register).map(|_| Width::Byte)
                }
            }
            OpKind::Memory => match i.memory_size() {
                MemorySize::UInt8 | MemorySize::Int8 => Some(Width::Byte),
                MemorySize::UInt16 | MemorySize::Int16 | MemorySize::WordOffset => {
                    Some(Width::Word)
                }
                _ => None,
            },
            OpKind::Immediate8 => Some(Width::Byte),
            OpKind::Immediate16 | OpKind::Immediate8to16 => Some(Width::Word),
            _ => None,
        }
    }

    fn mem(&self, width: Width) -> MemRef {
        let i = self.i;
        MemRef {
            segment: reg16(i.memory_segment()).unwrap_or(Reg16::Ds),
            base: reg16(i.memory_base()),
            index: reg16(i.memory_index()),
            displacement: i.memory_displacement32() as u16,
            width,
        }
    }
}

fn reg16(register: Register) -> Option<Reg16> {
    Some(match register {
        Register::AX => Reg16::Ax,
        Register::CX => Reg16::Cx,
        Register::DX => Reg16::Dx,
        Register::BX => Reg16::Bx,
        Register::SP => Reg16::Sp,
        Register::BP => Reg16::Bp,
        Register::SI => Reg16::Si,
        Register::DI => Reg16::Di,
        Register::ES => Reg16::Es,
        Register::CS => Reg16::Cs,
        Register::SS => Reg16::Ss,
        Register::DS => Reg16::Ds,
        _ => return None,
    })
}

fn reg8(register: Register) -> Option<Reg8> {
    Some(match register {
        Register::AL => Reg8::Al,
        Register::CL => Reg8::Cl,
        Register::DL => Reg8::Dl,
        Register::BL => Reg8::Bl,
        Register::AH => Reg8::Ah,
        Register::CH => Reg8::Ch,
        Register::DH => Reg8::Dh,
        Register::BH => Reg8::Bh,
        _ => return None,
    })
}

fn alu_op(mnemonic: Mnemonic) -> Option<AluOp> {
    Some(match mnemonic {
        Mnemonic::Add => AluOp::Add,
        Mnemonic::Adc => AluOp::Adc,
        Mnemonic::Sub => AluOp::Sub,
        Mnemonic::Sbb => AluOp::Sbb,
        Mnemonic::And => AluOp::And,
        Mnemonic::Or => AluOp::Or,
        Mnemonic::Xor => AluOp::Xor,
        _ => return None,
    })
}

fn shift_op(mnemonic: Mnemonic) -> Option<ShiftOp> {
    Some(match mnemonic {
        Mnemonic::Shl => ShiftOp::Shl,
        Mnemonic::Shr => ShiftOp::Shr,
        Mnemonic::Sar => ShiftOp::Sar,
        Mnemonic::Rol => ShiftOp::Rol,
        Mnemonic::Ror => ShiftOp::Ror,
        Mnemonic::Rcl => ShiftOp::Rcl,
        Mnemonic::Rcr => ShiftOp::Rcr,
        _ => return None,
    })
}

fn condition(mnemonic: Mnemonic) -> Option<Cond> {
    Some(match mnemonic {
        Mnemonic::Je => Cond::Equal,
        Mnemonic::Jne => Cond::NotEqual,
        Mnemonic::Jb => Cond::Below,
        Mnemonic::Jae => Cond::AboveOrEqual,
        Mnemonic::Jbe => Cond::BelowOrEqual,
        Mnemonic::Ja => Cond::Above,
        Mnemonic::Jl => Cond::Less,
        Mnemonic::Jge => Cond::GreaterOrEqual,
        Mnemonic::Jle => Cond::LessOrEqual,
        Mnemonic::Jg => Cond::Greater,
        Mnemonic::Js => Cond::Sign,
        Mnemonic::Jns => Cond::NotSign,
        Mnemonic::Jo => Cond::Overflow,
        Mnemonic::Jno => Cond::NotOverflow,
        Mnemonic::Jp => Cond::Parity,
        Mnemonic::Jnp => Cond::NotParity,
        _ => return None,
    })
}
