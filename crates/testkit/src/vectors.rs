//! SingleStepTests 8088 hardware vectors, run through lowering and the IR
//! interpreter.
//!
//! Each vector is one instruction captured on a real 8088: the initial and final
//! registers and the memory it touched. Only flags the hardware defines are
//! compared. Where the translated game deliberately follows the original C++
//! build instead of the silicon, the case is skipped and counted by reason.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use flate2::read::GzDecoder;
use machine::{Address, Fault, Flag, Machine, Reg8, Reg16};
use serde::Deserialize;
use translate::ir::{Ir, Operand};
use translate::{Relocations, lower};

use crate::interp::{Control, execute};

/// The vector files for the opcode forms the game uses.
pub fn stems() -> impl Iterator<Item = &'static str> {
    include_str!("../hardware-vectors.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .flat_map(str::split_whitespace)
}

#[derive(Deserialize)]
struct Vector {
    name: String,
    bytes: Vec<u8>,
    initial: Snapshot,
    #[serde(rename = "final")]
    end: Snapshot,
}

#[derive(Deserialize)]
struct Snapshot {
    regs: HashMap<String, u16>,
    ram: Vec<(u32, u8)>,
}

#[derive(Deserialize)]
struct Metadata {
    opcodes: HashMap<String, OpcodeInfo>,
}

#[derive(Deserialize)]
struct OpcodeInfo {
    #[serde(rename = "flags-mask")]
    flags_mask: Option<u16>,
    reg: Option<HashMap<String, OpcodeInfo>>,
}

/// The outcome of running vector files.
#[derive(Debug, Default)]
pub struct Summary {
    /// Vectors that matched the hardware.
    pub passed: usize,
    /// Vectors skipped, by documented reason.
    pub skipped: BTreeMap<&'static str, usize>,
    /// Instruction forms that did not lower, by mnemonic, with counts.
    pub unlowered: BTreeMap<String, usize>,
    /// Vectors compared with one documented flag left out, by reason.
    pub relaxed: BTreeMap<&'static str, usize>,
    /// Vectors that did not match, with a description.
    pub failures: Vec<String>,
}

impl Summary {
    fn skip(&mut self, reason: &'static str) {
        *self.skipped.entry(reason).or_default() += 1;
    }

    /// Failures counted by mnemonic.
    pub fn failures_by_mnemonic(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for failure in &self.failures {
            *counts.entry(failure.split([' ', ':']).next().unwrap_or_default()).or_default() += 1;
        }
        counts
    }
}

/// The defined-flag masks from `metadata.json`, by file stem.
///
/// # Errors
///
/// If the metadata cannot be read or parsed.
pub fn flag_masks(metadata: &Path) -> Result<HashMap<String, u16>, Box<dyn std::error::Error>> {
    let metadata: Metadata = serde_json::from_reader(BufReader::new(File::open(metadata)?))?;
    let mut masks = HashMap::new();
    for (opcode, info) in metadata.opcodes {
        match info.reg {
            Some(groups) => {
                for (reg, info) in groups {
                    masks.insert(format!("{opcode}.{reg}"), info.flags_mask.unwrap_or(0xffff));
                }
            }
            None => {
                masks.insert(opcode, info.flags_mask.unwrap_or(0xffff));
            }
        }
    }
    Ok(masks)
}

/// Runs up to `limit` vectors from a gzipped vector file into `summary`.
///
/// # Errors
///
/// If the file cannot be read or parsed.
pub fn run_file(
    path: &Path,
    flags_mask: u16,
    limit: usize,
    summary: &mut Summary,
) -> Result<(), Box<dyn std::error::Error>> {
    let vectors: Vec<Vector> =
        serde_json::from_reader(BufReader::new(GzDecoder::new(File::open(path)?)))?;
    // The 8086 reads FLAGS bits 3 and 5 as zero and bits 12-15 as one; neither
    // build models these reserved bits, and no instruction the game runs sets them.
    let mask = flags_mask & 0x0fd7;
    // One machine per file: memory is all zero between vectors, so after clearing
    // the addresses a vector lists, any non-zero byte is a stray write.
    let mut m = Machine::new();
    for vector in vectors.iter().take(limit) {
        run(&mut m, vector, mask, summary);
        for &(at, _) in vector.initial.ram.iter().chain(&vector.end.ram) {
            m.memory.as_bytes_mut()[at as usize] = 0;
        }
        if let Some(at) = m.memory.as_bytes().iter().position(|&byte| byte != 0) {
            summary.failures.push(format!("{}: stray write at {at:05x}", vector.name));
            m.memory.as_bytes_mut().fill(0);
        }
    }
    Ok(())
}

fn run(m: &mut Machine, vector: &Vector, flags_mask: u16, summary: &mut Summary) {
    let r = &vector.initial.regs;
    let reg = |name: &str| r.get(name).copied().unwrap_or(0);
    let regs = &mut m.regs;
    (regs.ax, regs.bx, regs.cx, regs.dx) = (reg("ax"), reg("bx"), reg("cx"), reg("dx"));
    (regs.si, regs.di, regs.bp, regs.sp) = (reg("si"), reg("di"), reg("bp"), reg("sp"));
    (regs.cs, regs.ds, regs.es, regs.ss) = (reg("cs"), reg("ds"), reg("es"), reg("ss"));
    (regs.ip, regs.flags) = (reg("ip"), reg("flags"));
    for &(at, value) in &vector.initial.ram {
        m.memory.as_bytes_mut()[at as usize] = value;
    }

    let Ok(instruction) =
        lower(Address::new(m.regs.cs, m.regs.ip), &vector.bytes, &Relocations::default())
    else {
        summary.skip("form the game never uses (not lowered)");
        let mnemonic = vector.name.split_whitespace().next().unwrap_or_default().to_owned();
        *summary.unlowered.entry(mnemonic).or_default() += 1;
        return;
    };
    if let Some(reason) = deliberate_difference(&instruction.ir, &vector.bytes) {
        summary.skip(reason);
        return;
    }
    let mut flags_mask = flags_mask;
    let count = m.regs.cx & 0xff;
    if matches!(instruction.ir, Ir::Shift { count: Operand::Reg8(Reg8::Cl), .. }) && count != 1 {
        // The 8088 computes OF for multi-bit shifts; the C++ build leaves it
        // unchanged. None of the game's 49 shifts by CL is followed by a jump on OF.
        flags_mask &= !Flag::Overflow.mask();
        *summary.relaxed.entry("OF after a multi-bit shift by CL").or_default() += 1;
    }

    let divide_trap = hardware_divide_trap(vector);
    match execute(m, &instruction, 0) {
        Err(Fault::DivideByZero | Fault::DivideOverflow) if divide_trap => {
            summary.skip("divide error: the hardware traps, the translation stops");
            return;
        }
        Err(fault) => {
            let bytes = translate::image::hex(&vector.bytes);
            let hardware =
                (vector.end.regs.get("ax"), vector.end.regs.get("dx"), vector.end.regs.get("ip"));
            summary.failures.push(format!(
                "{} [{bytes}]: unexpected {fault}; hardware ax/dx/ip {hardware:?}",
                vector.name
            ));
            return;
        }
        Ok(_) if divide_trap => {
            summary.skip("IDIV quotient of -2^(n-1): the 8086 traps, the C++ build does not");
            return;
        }
        Ok(Control::Next) => m.regs.ip = instruction.next(),
        Ok(Control::Jump(target)) => m.regs.ip = target,
        Ok(Control::Transfer) => {}
        Ok(Control::Interrupt(_)) => {
            summary.skip("INT is a DOS service");
            return;
        }
    }

    match compare(vector, m, flags_mask) {
        None => summary.passed += 1,
        Some(problem) => {
            let bytes = translate::image::hex(&vector.bytes);
            summary.failures.push(format!("{} [{bytes}]: {problem}", vector.name));
        }
    }
}

/// Instruction forms where the translation intentionally matches the original
/// C++ build rather than the silicon. The game uses none of them.
fn deliberate_difference(ir: &Ir, bytes: &[u8]) -> Option<&'static str> {
    let prefixed = |prefix: u8| {
        bytes
            .iter()
            .take_while(|&&b| matches!(b, 0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3))
            .any(|&b| b == prefix)
    };
    let overridden = bytes
        .iter()
        .take_while(|&&b| matches!(b, 0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3))
        .any(|&b| matches!(b, 0x26 | 0x2e | 0x36 | 0x3e));
    match ir {
        Ir::Translate if overridden => {
            Some("XLAT with a segment override: the C++ build always reads DS")
        }
        Ir::String { op, .. }
            if prefixed(0xf2)
                && !matches!(op, machine::StringOp::Scas | machine::StringOp::Cmps) =>
        {
            Some("REPNE on MOVS/STOS/LODS: the C++ build tests ZF")
        }
        Ir::Move { dst: Operand::Reg(Reg16::Cs), .. } | Ir::Pop(Operand::Reg(Reg16::Cs)) => {
            Some("writes to CS the 8086 does not define")
        }
        Ir::Push(Operand::Reg(Reg16::Sp)) => {
            Some("PUSH SP: the C++ build pushes the old SP; the game never pushes SP")
        }
        Ir::Multiply { .. } | Ir::Divide { .. } if prefixed(0xf2) || prefixed(0xf3) => {
            Some("REP-prefixed MUL/DIV: the 8086 microcode negates the result; the game has none")
        }
        _ => None,
    }
}

/// Whether the hardware raised the divide-error interrupt (CS:IP became IVT[0]).
fn hardware_divide_trap(vector: &Vector) -> bool {
    let byte = |at: u32| vector.initial.ram.iter().find(|&&(a, _)| a == at).map(|&(_, v)| v);
    let (Some(ip0), Some(ip1), Some(cs0), Some(cs1)) = (byte(0), byte(1), byte(2), byte(3)) else {
        return false;
    };
    let (ip, cs) = (u16::from_le_bytes([ip0, ip1]), u16::from_le_bytes([cs0, cs1]));
    // The final state lists only changed registers; IVT[0] may even point at the
    // instruction itself, leaving CS:IP unchanged.
    let last =
        |name: &str| vector.end.regs.get(name).or_else(|| vector.initial.regs.get(name)).copied();
    last("ip") == Some(ip) && last("cs") == Some(cs)
}

fn compare(vector: &Vector, m: &Machine, flags_mask: u16) -> Option<String> {
    let expected = |name: &str| {
        vector.end.regs.get(name).or_else(|| vector.initial.regs.get(name)).copied().unwrap_or(0)
    };
    let regs = &m.regs;
    let actual = [
        ("ax", regs.ax),
        ("bx", regs.bx),
        ("cx", regs.cx),
        ("dx", regs.dx),
        ("si", regs.si),
        ("di", regs.di),
        ("bp", regs.bp),
        ("sp", regs.sp),
        ("cs", regs.cs),
        ("ds", regs.ds),
        ("es", regs.es),
        ("ss", regs.ss),
        ("ip", regs.ip),
    ];
    for (name, value) in actual {
        if value != expected(name) {
            return Some(format!("{name} = {value:04x}, hardware {:04x}", expected(name)));
        }
    }
    let flags = (regs.flags & flags_mask, expected("flags") & flags_mask);
    if flags.0 != flags.1 {
        return Some(format!(
            "flags {:04x}, hardware {:04x} (mask {flags_mask:04x})",
            flags.0, flags.1
        ));
    }
    let mut image: HashMap<u32, u8> = vector.initial.ram.iter().copied().collect();
    image.extend(vector.end.ram.iter().copied());
    image.iter().find_map(|(&at, &want)| {
        let got = m.memory.as_bytes()[at as usize];
        (got != want).then(|| format!("memory {at:05x} = {got:02x}, hardware {want:02x}"))
    })
}
