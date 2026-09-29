//! `tests/assets.cpp`: the game's own asset decoder, run where the game runs
//! it, and the port's native decoder agree byte for byte on all 21 packed
//! assets the game ships, and the native decoder rejects each one cut short.
//! The decoder's rules on hand-made streams are `crates/game/tests/assets.rs`.
//!
//! Not ported: the C++ test also wrote each decoded asset to
//! `recovered/assets/`, a by-product for inspection rather than a check.

use std::path::PathBuf;

use engine::Program;
use game::{Game, decode_asset};
use machine::{Address, Flag, Machine, Memory};
use testkit::harness::data;

/// The original decoder: packed bytes at DS:SI to ES:DI, returning the end of
/// the output in AX and setting CF for a foreign stream.
const DECODER: Address = Address::new(0x1265, 0x1250);
/// Where the test places the packed input.
const PACKED: (u16, u16) = (0x3000, 0x0100);
/// Where the decoder writes its output.
const UNPACKED: (u16, u16) = (0x5000, 0x0000);
/// The decoder's stack.
const STACK: (u16, u16) = (0x9000, 0xfffe);
/// The near return address the decoder ends at.
const NEAR_RETURN: u16 = 0xffff;
/// The largest asset decodes well within this many steps.
const DECODE_LIMIT: u64 = 2_000_000;
/// The packed assets in the game's `LDMG` folder.
const ASSETS: usize = 21;

/// Every packed asset the game ships: its `.BIN` and `.ZZZ` files.
fn packed_assets() -> Vec<PathBuf> {
    let folder = data().join("LDMG");
    let entries = std::fs::read_dir(&folder).expect("the game's LDMG folder");
    let mut assets: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "BIN" || extension == "ZZZ")
        })
        .collect();
    assets.sort();
    assets
}

/// Runs the original decoder on `packed` in a machine of its own, and returns
/// the output it reports, or `None` when it rejects the stream.
fn decode_originally(packed: &[u8]) -> Option<Vec<u8>> {
    let mut game = Game::new(PathBuf::new(), PathBuf::new(), true);
    let mut m = Machine::new();
    let r = &mut m.regs;
    (r.ds, r.si, r.es, r.di, r.ss, r.sp) =
        (PACKED.0, PACKED.1, UNPACKED.0, UNPACKED.1, STACK.0, STACK.1);
    (r.cs, r.ip) = (DECODER.runtime_segment(), DECODER.offset);
    m.push(NEAR_RETURN);
    let input = Memory::linear(PACKED.0, PACKED.1);
    m.memory.as_bytes_mut()[input..input + packed.len()].copy_from_slice(packed);
    while m.regs.ip != NEAR_RETURN && m.steps < DECODE_LIMIT {
        game.step(&mut m).expect("the original decoder runs");
    }
    if m.regs.ip != NEAR_RETURN || m.flag(Flag::Carry) {
        return None;
    }
    let output = Memory::linear(UNPACKED.0, UNPACKED.1);
    Some(m.memory.as_bytes()[output..output + usize::from(m.regs.ax)].to_vec())
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_original_and_native_decoders_agree_on_every_packed_asset() {
    let assets = packed_assets();
    for path in &assets {
        let name = path.file_name().expect("a file name").to_string_lossy();
        let mut packed = std::fs::read(path).expect("a readable asset");
        let native = decode_asset(&packed).expect("every shipped asset decodes");
        assert_eq!(
            decode_originally(&packed).as_ref(),
            Some(&native),
            "Original decoder differs: {name}"
        );
        packed.pop();
        assert!(decode_asset(&packed).is_err(), "Truncated stream accepted: {name}");
    }
    assert_eq!(assets.len(), ASSETS, "Expected all 21 supplied packed assets");
}
