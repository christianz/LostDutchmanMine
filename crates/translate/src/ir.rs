//! The intermediate representation: what each 8086 instruction means, before
//! it is written out as Rust.

use machine::{Address, AluOp, Cond, Flag, Reg8, Reg16, Repeat, ShiftOp, StringOp, Width};

/// A memory operand: `segment:[base + index + displacement]`, wrapping at 64K.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemRef {
    /// The segment register, after defaults (SS for BP) and overrides.
    pub segment: Reg16,
    /// The base register, if any.
    pub base: Option<Reg16>,
    /// The index register, if any.
    pub index: Option<Reg16>,
    /// The displacement.
    pub displacement: u16,
    /// The width accessed.
    pub width: Width,
}

impl MemRef {
    /// The same reference, `delta` bytes further on (the segment half of a far pointer).
    #[must_use]
    pub const fn offset_by(self, delta: u16) -> Self {
        MemRef { displacement: self.displacement.wrapping_add(delta), ..self }
    }
}

/// An instruction operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    /// A 16-bit register.
    Reg(Reg16),
    /// An 8-bit register.
    Reg8(Reg8),
    /// An immediate, already relocated and masked to its width.
    Imm(u16),
    /// Memory.
    Mem(MemRef),
}

/// Which LOOP instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoopKind {
    /// LOOP: while CX is not zero.
    Always,
    /// LOOPE: while CX is not zero and ZF is set.
    WhileEqual,
    /// LOOPNE: while CX is not zero and ZF is clear.
    WhileNotEqual,
}

/// The meaning of one instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ir {
    /// MOV.
    Move {
        /// Destination.
        dst: Operand,
        /// Source.
        src: Operand,
        /// Width.
        width: Width,
    },
    /// ADD, ADC, SUB, SBB, AND, OR, XOR: `dst = dst op src`.
    Alu {
        /// Operation.
        op: AluOp,
        /// Destination and first operand.
        dst: Operand,
        /// Second operand.
        src: Operand,
        /// Width.
        width: Width,
    },
    /// CMP (`Sub`) or TEST (`And`): flags only.
    Compare {
        /// Operation.
        op: AluOp,
        /// First operand.
        a: Operand,
        /// Second operand.
        b: Operand,
        /// Width.
        width: Width,
    },
    /// INC.
    Increment {
        /// Operand.
        dst: Operand,
        /// Width.
        width: Width,
    },
    /// DEC.
    Decrement {
        /// Operand.
        dst: Operand,
        /// Width.
        width: Width,
    },
    /// NEG.
    Negate {
        /// Operand.
        dst: Operand,
        /// Width.
        width: Width,
    },
    /// NOT; flags unchanged.
    Not {
        /// Operand.
        dst: Operand,
        /// Width.
        width: Width,
    },
    /// Shifts and rotates.
    Shift {
        /// Operation.
        op: ShiftOp,
        /// Operand.
        dst: Operand,
        /// Count: 1 or CL.
        count: Operand,
        /// Width.
        width: Width,
    },
    /// PUSH.
    Push(Operand),
    /// POP.
    Pop(Operand),
    /// PUSHF.
    PushFlags,
    /// POPF.
    PopFlags,
    /// XCHG.
    Exchange {
        /// First operand.
        a: Operand,
        /// Second operand.
        b: Operand,
        /// Width.
        width: Width,
    },
    /// LEA: the offset of a memory reference.
    LoadAddress {
        /// Destination register.
        dst: Reg16,
        /// The reference whose offset is taken.
        address: MemRef,
    },
    /// LDS or LES: a far pointer into a register and DS or ES.
    LoadFarPointer {
        /// Register receiving the offset.
        dst: Reg16,
        /// DS or ES, receiving the segment.
        segment: Reg16,
        /// Where the far pointer is stored.
        address: MemRef,
    },
    /// CBW: sign-extend AL into AX.
    ConvertByteToWord,
    /// CWD: sign-extend AX into DX:AX.
    ConvertWordToDouble,
    /// MUL or IMUL with one operand.
    Multiply {
        /// Multiplier.
        src: Operand,
        /// Width.
        width: Width,
        /// IMUL.
        signed: bool,
    },
    /// DIV or IDIV.
    Divide {
        /// Divisor.
        src: Operand,
        /// Width.
        width: Width,
        /// IDIV.
        signed: bool,
    },
    /// JMP (no condition), Jcc and JCXZ to an offset in this segment.
    Jump {
        /// The condition, if conditional.
        cond: Option<Cond>,
        /// Target offset.
        target: u16,
    },
    /// JMP through a register or memory word.
    JumpIndirect(Operand),
    /// LOOP, LOOPE, LOOPNE.
    Loop {
        /// Which loop.
        kind: LoopKind,
        /// Target offset.
        target: u16,
    },
    /// Near CALL; direct targets are immediates.
    Call(Operand),
    /// Far JMP to an image address.
    FarJump(Address),
    /// Far JMP through a far pointer in memory.
    FarJumpIndirect(MemRef),
    /// Far CALL to an image address.
    FarCall(Address),
    /// Far CALL through a far pointer in memory.
    FarCallIndirect(MemRef),
    /// RET, releasing bytes of arguments.
    Return {
        /// Bytes released.
        release: u16,
    },
    /// RETF, releasing bytes of arguments.
    ReturnFar {
        /// Bytes released.
        release: u16,
    },
    /// IRET.
    ReturnFromInterrupt,
    /// INT: a DOS or BIOS service.
    Interrupt(u8),
    /// IN.
    In {
        /// AL or AX.
        dst: Operand,
        /// An immediate port or DX.
        port: Operand,
        /// Width.
        width: Width,
    },
    /// OUT.
    Out {
        /// An immediate port or DX.
        port: Operand,
        /// AL or AX.
        src: Operand,
        /// Width.
        width: Width,
    },
    /// CLC, STC, CLD, STD, CLI, STI.
    SetFlag {
        /// The flag.
        flag: Flag,
        /// Set or clear.
        on: bool,
    },
    /// CMC.
    ComplementCarry,
    /// LAHF.
    LoadAhFromFlags,
    /// SAHF.
    StoreAhIntoFlags,
    /// XLATB: `AL = DS:[BX + AL]`.
    Translate,
    /// A string instruction.
    String {
        /// Operation.
        op: StringOp,
        /// Width.
        width: Width,
        /// Repeat prefix.
        repeat: Repeat,
        /// Segment of the source (DS unless overridden).
        source: Reg16,
    },
    /// NOP.
    Nop,
}
