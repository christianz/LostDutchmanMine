//! Which registers and flags a routine's callers read after it returns.

use machine::{Address, Flag, Reg16};
use translate::image::LoadImage;
use translate::liveness::{Uses, routine_live_out};
use translate::recover::recover;
use translate::translation::Translation;

fn translation(code: &[u8]) -> Translation {
    let mut bytes = vec![0xcc; 0x40];
    bytes[..code.len()].copy_from_slice(code);
    let image = LoadImage {
        bytes,
        relocations: Vec::new(),
        entry: Address::new(0, 0),
        stack_segment: 0,
        stack_pointer: 0x100,
    };
    Translation::build(&image, &recover(&image, &[]), &[]).expect("translation")
}

const ROUTINE: Address = Address::new(0, 0x20);

fn with_routine(caller: &[u8]) -> Vec<u8> {
    let mut code = vec![0xcc; 0x21];
    code[..caller.len()].copy_from_slice(caller);
    code[0x20] = 0xc3; // 0020 ret: the routine
    code
}

#[test]
fn registers_written_on_every_path_before_a_read_are_dead() {
    let code = with_routine(&[
        0xe8, 0x1d, 0x00, // 0000 call 0020
        0x5b, // 0003 pop bx
        0x8b, 0xc8, // 0004 mov cx, ax
        0x72, 0x04, // 0006 jc 000c
        0x31, 0xd2, // 0008 xor dx, dx
        0xeb, 0x03, // 000a jmp 000f
        0xba, 0x01, 0x00, // 000c mov dx, 1
        0x31, 0xc0, // 000f xor ax, ax
        0xc3, // 0011 ret
    ]);
    let live = routine_live_out(&translation(&code), ROUTINE);
    for dead in [Reg16::Bx, Reg16::Cx, Reg16::Dx] {
        assert!(!live.register(dead), "{dead:?} is overwritten before any read");
    }
    for read in [Reg16::Ax, Reg16::Sp, Reg16::Ss, Reg16::Si, Reg16::Ds] {
        assert!(live.register(read), "{read:?} is read, or live at the final return");
    }
    assert!(live.flag(Flag::Carry), "JC reads the carry");
    for dead in [Flag::Zero, Flag::Sign, Flag::Parity, Flag::Adjust, Flag::Overflow] {
        assert!(!live.flag(dead), "{dead:?} is overwritten by XOR on both paths");
    }
    assert!(live.flag(Flag::Direction), "untouched flags are live at the final return");
}

#[test]
fn clearing_a_register_with_itself_does_not_read_it() {
    let code = with_routine(&[
        0xe8, 0x1d, 0x00, // 0000 call 0020
        0x31, 0xc0, // 0003 xor ax, ax
        0xc3, // 0005 ret
    ]);
    assert!(!routine_live_out(&translation(&code), ROUTINE).register(Reg16::Ax));
}

#[test]
fn a_routine_with_no_known_callers_keeps_everything() {
    let code = with_routine(&[0xc3]);
    assert_eq!(routine_live_out(&translation(&code), ROUTINE), Uses::ALL);
}

#[test]
fn a_call_after_the_return_keeps_everything_it_might_read() {
    let code = with_routine(&[
        0xe8, 0x1d, 0x00, // 0000 call 0020
        0xe8, 0x1a, 0x00, // 0003 call 0020 again
        0xc3, // 0006 ret
    ]);
    assert_eq!(routine_live_out(&translation(&code), ROUTINE), Uses::ALL);
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_caller_of_the_asset_decoder_restores_its_own_data_segments() {
    let exe = std::fs::read(std::env::var("LDM_EXE").expect("LDM_EXE")).expect("readable LDM.EXE");
    let image = translate::image::load(&exe).expect("the supported release");
    let seeds =
        translate::recover::parse_entry_points(include_str!("../../game/entry_points.toml"))
            .expect("entry points");
    let translation = Translation::build(&image, &recover(&image, &seeds), patches::PATCHES)
        .expect("the translation");
    let live = routine_live_out(&translation, Address::new(0x1265, 0x1250));
    assert!(!live.register(Reg16::Ds) && !live.register(Reg16::Es), "callers pop DS and ES");
    let kept =
        [Reg16::Ax, Reg16::Cx, Reg16::Dx, Reg16::Bx, Reg16::Sp, Reg16::Bp, Reg16::Si, Reg16::Di];
    assert!(
        kept.iter().all(|&register| live.register(register)),
        "{:?}",
        live.registers().collect::<Vec<_>>()
    );
    assert!(live.flag(Flag::Carry), "CF says whether the stream decoded");
}
