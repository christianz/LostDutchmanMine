//! The readable asset decoder in lockstep with the translated original, on
//! every packed asset the game ships, called as the C++ `test-assets` did.

use std::path::{Path, PathBuf};

use engine::Program;
use game::{RoutineMode, decode_asset};
use machine::LOAD_SEGMENT;

/// Where the call puts the stream, the output and the stack.
const STREAM: (u16, u16) = (12_288, 256);
const OUTPUT: u16 = 20_480;
const STACK: (u16, u16) = (36_864, 65_534);
/// The return address that ends the call.
const DONE: u16 = u16::MAX;
/// The decoder's entry: 1265:1250.
const DECODER: (u16, u16) = (0x1265, 0x1250);

fn data() -> PathBuf {
    std::env::var_os("LDM_DATA").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/original"),
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_shipped_asset_decodes_identically_in_both_implementations() {
    let saves = std::env::temp_dir().join(format!("ldm-decoder-{}", std::process::id()));
    std::fs::create_dir_all(&saves).expect("a saves folder");
    let (booted, mut game) = game::boot(data(), saves.clone(), true).expect("the game boots");
    game.set_routine_mode(RoutineMode::Lockstep);
    let mut assets = 0;
    for entry in std::fs::read_dir(data().join("LDMG")).expect("the LDMG folder") {
        let path = entry.expect("an entry").path();
        let packed_asset = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zzz") || e.eq_ignore_ascii_case("bin"));
        if !packed_asset {
            continue;
        }
        let packed = std::fs::read(&path).expect("a readable asset");
        let expected = decode_asset(&packed).expect("the native decoder reads every shipped asset");
        let mut m = booted.clone();
        (m.regs.cs, m.regs.ip) = (DECODER.0 + LOAD_SEGMENT, DECODER.1);
        (m.regs.ds, m.regs.si, m.regs.es, m.regs.di) = (STREAM.0, STREAM.1, OUTPUT, 0);
        (m.regs.ss, m.regs.sp) = STACK;
        m.push(DONE);
        for (i, &byte) in (0..).zip(&packed) {
            m.memory.write8(STREAM.0, STREAM.1 + i, byte);
        }
        let checks = game.routine_checks();
        while m.regs.ip != DONE {
            game.step(&mut m).expect("the translated decoder runs");
        }
        assert_eq!(game.routine_checks(), checks + 1, "{}: checked once", path.display());
        let output: Vec<u8> = (0..m.regs.ax).map(|i| m.memory.read8(OUTPUT, i)).collect();
        assert_eq!(
            output,
            expected,
            "{}: the translation matches the native decoder",
            path.display()
        );
        assets += 1;
    }
    assert_eq!(assets, 21, "every packed asset the game ships");
    assert_eq!(game.routine_mismatches(), [] as [String; 0]);
    std::fs::remove_dir_all(saves).expect("the saves folder is removed");
}
