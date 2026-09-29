//! Readable routines in place of translated ones, and the lockstep check that
//! both do the same.
//!
//! A translated routine yields as its original did, so a long one spans many
//! milliseconds with timer interrupts in between. Running the readable routine
//! instead finishes at once: right for play, but no longer the oracle's timing.
//! So lockstep checks each call on two copies of the machine at entry, runs the
//! translation in one and the readable routine in the other, compares what the
//! callers can observe, and lets the real machine carry on with the
//! translation, keeping every golden trace.

use engine::Stop;
use machine::{Address, Machine, Memory, Reg16};
use patches::Routine;
use translate::liveness::Uses;

use crate::{Game, decompiled, translated};

/// A translation that runs this many steps without returning has gone astray.
const STEP_LIMIT: u64 = 50_000_000;
/// Differences kept per session; the first ones explain the rest.
const KEPT_MISMATCHES: usize = 20;

/// Which implementation of the readable routines runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoutineMode {
    /// The translated originals, exactly as the oracle ran.
    #[default]
    Translated,
    /// The readable routines, for play.
    Readable,
    /// The translations, with every call also checked against the readable
    /// routine.
    Lockstep,
}

/// The routines' mode and what lockstep found.
#[derive(Clone, Debug, Default)]
pub(crate) struct Routines {
    pub(crate) mode: RoutineMode,
    /// Set while lockstep runs a translation on its copy: the routine inside
    /// must run translated, and must not reach DOS or a hook.
    pub(crate) checking: bool,
    pub(crate) checked: u64,
    pub(crate) mismatches: Vec<String>,
}

/// At a readable routine's entry: true when the readable routine ran, and
/// returned; false leaves the call to the translation.
pub(crate) fn dispatch(m: &mut Machine, g: &mut Game, routine: Routine) -> Result<bool, Stop> {
    match g.routines.mode {
        _ if g.routines.checking => Ok(false),
        RoutineMode::Translated => Ok(false),
        RoutineMode::Readable => Ok(decompiled::run(routine, m)),
        RoutineMode::Lockstep => {
            check(m, g, routine)?;
            Ok(false)
        }
    }
}

fn check(m: &Machine, g: &mut Game, routine: Routine) -> Result<(), Stop> {
    let mut readable = m.clone();
    readable.low_water = m.regs.sp;
    if !decompiled::run(routine, &mut readable) {
        return Ok(());
    }
    let mut translation = m.clone();
    translation.low_water = m.regs.sp;
    // This step was counted when it began; the copy runs it again from its start.
    translation.steps -= 1;
    let resume = resume_point(m, routine);
    g.routines.checking = true;
    let start = translation.steps;
    let ran = loop {
        if translation.steps - start > STEP_LIMIT {
            break Err(Stop::Program(format!("{routine:?} did not return")));
        }
        if let Err(stop) = translated::step(&mut translation, g) {
            break Err(stop);
        }
        let regs = &translation.regs;
        if (regs.cs, regs.ip, regs.sp) == resume {
            break Ok(());
        }
    };
    g.routines.checking = false;
    ran?;
    g.routines.checked += 1;
    let contract = translated::CONTRACTS
        .iter()
        .find(|(candidate, _)| *candidate == routine)
        .map_or(Uses::ALL, |&(_, bits)| Uses::from_bits(bits));
    if let Some(difference) = difference(contract, &translation, &readable, m.regs.sp) {
        let caller = Address::from_runtime(resume.0, resume.1);
        if g.routines.mismatches.len() < KEPT_MISMATCHES {
            g.routines.mismatches.push(format!("{routine:?} returning to {caller}: {difference}"));
        }
    }
    Ok(())
}

/// Where the caller resumes: CS, IP and SP once the return address is popped.
fn resume_point(m: &Machine, routine: Routine) -> (u16, u16, u16) {
    let (ss, sp) = (m.regs.ss, m.regs.sp);
    let ip = m.memory.read16(ss, sp);
    if decompiled::returns_far(routine) {
        (m.memory.read16(ss, sp.wrapping_add(2)), ip, sp.wrapping_add(4))
    } else {
        (m.regs.cs, ip, sp.wrapping_add(2))
    }
}

/// The first thing callers could see that differs: a register or flag in the
/// contract, the return address, or memory outside the dead stack.
fn difference(
    contract: Uses,
    translation: &Machine,
    readable: &Machine,
    entry_sp: u16,
) -> Option<String> {
    let (t, r) = (&translation.regs, &readable.regs);
    if (t.cs, t.ip) != (r.cs, r.ip) {
        return Some(format!(
            "returns to {:04x}:{:04x}, not {:04x}:{:04x}",
            r.cs, r.ip, t.cs, t.ip
        ));
    }
    for register in contract.registers() {
        let (expected, found) = (value(t, register), value(r, register));
        if expected != found {
            return Some(format!("{register:?} is {found:04x}, not {expected:04x}"));
        }
    }
    let flags = (t.flags ^ r.flags) & contract.flags_mask();
    if flags != 0 {
        return Some(format!("flags {:04x}, not {:04x} (differing {flags:04x})", r.flags, t.flags));
    }
    // Below the entry stack pointer, down to the lowest either reached, is
    // scratch space no caller reads.
    let low = translation.low_water.min(readable.low_water);
    let dead = (low < entry_sp).then(|| {
        let base = Memory::linear(t.ss, 0);
        (base + usize::from(low), base + usize::from(entry_sp))
    });
    let (expected, found) = (translation.memory.as_bytes(), readable.memory.as_bytes());
    let differs = (0..expected.len())
        .filter(|&at| dead.is_none_or(|(from, to)| !(from..to).contains(&at)))
        .find(|&at| expected[at] != found[at]);
    differs.map(|at| format!("memory at {at:05x} is {:02x}, not {:02x}", found[at], expected[at]))
}

fn value(regs: &machine::Registers, register: Reg16) -> u16 {
    match register {
        Reg16::Ax => regs.ax,
        Reg16::Cx => regs.cx,
        Reg16::Dx => regs.dx,
        Reg16::Bx => regs.bx,
        Reg16::Sp => regs.sp,
        Reg16::Bp => regs.bp,
        Reg16::Si => regs.si,
        Reg16::Di => regs.di,
        Reg16::Es => regs.es,
        Reg16::Cs => regs.cs,
        Reg16::Ss => regs.ss,
        Reg16::Ds => regs.ds,
    }
}
