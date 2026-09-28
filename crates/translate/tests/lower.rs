//! Lowering decoded 8086 instructions into IR, rule by rule.

use machine::{Address, AluOp, Cond, Reg8, Reg16, Repeat, ShiftOp, StringOp, Width};
use translate::ir::{Ir, MemRef, Operand};
use translate::{Relocations, lower};

const AT: Address = Address::new(0x0505, 0x0010);

fn ir(bytes: &[u8]) -> Ir {
    lower(AT, bytes, &Relocations::default()).expect("supported instruction").ir
}

fn word_at(segment: Reg16, base: Option<Reg16>, displacement: u16) -> Operand {
    Operand::Mem(MemRef { segment, base, index: None, displacement, width: Width::Word })
}

#[test]
fn immediate_moves_keep_their_value() {
    let expected =
        Ir::Move { dst: Operand::Reg(Reg16::Ax), src: Operand::Imm(0x1234), width: Width::Word };
    assert_eq!(ir(&[0xb8, 0x34, 0x12]), expected);
}

#[test]
fn bp_addressing_defaults_to_the_stack_segment() {
    let expected = Ir::Move {
        dst: Operand::Reg(Reg16::Ax),
        src: word_at(Reg16::Ss, Some(Reg16::Bp), 4),
        width: Width::Word,
    };
    assert_eq!(ir(&[0x8b, 0x46, 0x04]), expected);
}

#[test]
fn segment_overrides_select_the_segment() {
    let src = Operand::Mem(MemRef {
        segment: Reg16::Es,
        base: Some(Reg16::Bx),
        index: None,
        displacement: 0,
        width: Width::Byte,
    });
    assert_eq!(
        ir(&[0x26, 0x8a, 0x07]),
        Ir::Move { dst: Operand::Reg8(Reg8::Al), src, width: Width::Byte }
    );
}

#[test]
fn negative_displacements_wrap() {
    let expected = Ir::Alu {
        op: AluOp::Add,
        dst: Operand::Reg(Reg16::Ax),
        src: word_at(Reg16::Ds, Some(Reg16::Bx), 0xfffe),
        width: Width::Word,
    };
    assert_eq!(ir(&[0x03, 0x47, 0xfe]), expected);
}

#[test]
fn relocated_immediates_move_by_the_load_segment() {
    let relocations = Relocations::from_offsets([0x5050 + 0x10 + 1]);
    let lowered = lower(AT, &[0xb8, 0x00, 0x00], &relocations).expect("mov ax, seg");
    assert_eq!(
        lowered.ir,
        Ir::Move { dst: Operand::Reg(Reg16::Ax), src: Operand::Imm(0x1000), width: Width::Word }
    );
}

#[test]
fn far_calls_name_their_image_relative_target() {
    let relocations = Relocations::from_offsets([0x5050 + 0x10 + 3]);
    let lowered = lower(AT, &[0x9a, 0x0c, 0x00, 0x05, 0x05], &relocations).expect("lcall");
    assert_eq!(lowered.ir, Ir::FarCall(Address::new(0x0505, 0x000c)));
    assert_eq!(lowered.next(), 0x0015);
}

#[test]
fn sign_extended_immediates_fill_the_word() {
    let expected = Ir::Alu {
        op: AluOp::Add,
        dst: Operand::Reg(Reg16::Ax),
        src: Operand::Imm(0xffff),
        width: Width::Word,
    };
    assert_eq!(ir(&[0x83, 0xc0, 0xff]), expected);
}

#[test]
fn byte_compares_use_byte_operands() {
    let a = Operand::Mem(MemRef {
        segment: Reg16::Ds,
        base: None,
        index: None,
        displacement: 0x5d5a,
        width: Width::Byte,
    });
    assert_eq!(
        ir(&[0x80, 0x3e, 0x5a, 0x5d, 0x00]),
        Ir::Compare { op: AluOp::Sub, a, b: Operand::Imm(0), width: Width::Byte }
    );
}

#[test]
fn repeat_prefixes_follow_the_instruction() {
    let string = |op, width, repeat, source| Ir::String { op, width, repeat, source };
    assert_eq!(ir(&[0xf3, 0xa4]), string(StringOp::Movs, Width::Byte, Repeat::Rep, Reg16::Ds));
    assert_eq!(
        ir(&[0xf3, 0xa6]),
        string(StringOp::Cmps, Width::Byte, Repeat::WhileEqual, Reg16::Ds)
    );
    assert_eq!(
        ir(&[0xf2, 0xae]),
        string(StringOp::Scas, Width::Byte, Repeat::WhileNotEqual, Reg16::Ds)
    );
    assert_eq!(ir(&[0x26, 0xac]), string(StringOp::Lods, Width::Byte, Repeat::Once, Reg16::Es));
    assert_eq!(ir(&[0xab]), string(StringOp::Stos, Width::Word, Repeat::Once, Reg16::Ds));
}

#[test]
fn shift_counts_are_one_or_cl() {
    assert_eq!(
        ir(&[0xd1, 0xe0]),
        Ir::Shift {
            op: ShiftOp::Shl,
            dst: Operand::Reg(Reg16::Ax),
            count: Operand::Imm(1),
            width: Width::Word
        }
    );
    assert_eq!(
        ir(&[0xd3, 0xe8]),
        Ir::Shift {
            op: ShiftOp::Shr,
            dst: Operand::Reg(Reg16::Ax),
            count: Operand::Reg8(Reg8::Cl),
            width: Width::Word
        }
    );
}

#[test]
fn jumps_carry_their_absolute_target() {
    assert_eq!(ir(&[0x75, 0xfe]), Ir::Jump { cond: Some(Cond::NotEqual), target: 0x0010 });
    assert_eq!(ir(&[0xe3, 0x02]), Ir::Jump { cond: Some(Cond::CxZero), target: 0x0014 });
    assert_eq!(ir(&[0xe8, 0x00, 0x01]), Ir::Call(Operand::Imm(0x0113)));
}

#[test]
fn interrupts_and_returns() {
    assert_eq!(ir(&[0xcd, 0x21]), Ir::Interrupt(0x21));
    assert_eq!(ir(&[0xca, 0x04, 0x00]), Ir::ReturnFar { release: 4 });
    assert_eq!(ir(&[0xc3]), Ir::Return { release: 0 });
}

#[test]
fn instructions_the_8086_lacks_are_rejected() {
    assert!(
        lower(AT, &[0xc8, 0x04, 0x00, 0x00], &Relocations::default()).is_err(),
        "ENTER is 186+"
    );
    assert!(lower(AT, &[0x0f, 0x0b], &Relocations::default()).is_err(), "UD2");
}
