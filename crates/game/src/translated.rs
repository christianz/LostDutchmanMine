//! The translated game, and what its generated code relies on.
//!
//! `build.rs` writes one module per original code segment into `OUT_DIR`. Each
//! block is a small function over the [`Machine`] and the [`Game`], and hands
//! control back with a [`Next`]. Calls, returns, backward jumps and waiting
//! interrupts yield, exactly where the C++ oracle's generated code returned, so
//! a translated step here is a step there and traces line up.

use engine::Stop;
use machine::{Address, Fault, LOAD_SEGMENT, Machine, RESERVED};
use patches::{After, Hook};

use crate::{Game, hooks};

/// The names generated code uses.
pub(crate) mod prelude {
    pub(crate) use engine::Stop;
    pub(crate) use machine::{
        Address, AluOp, Cond, Flag, LOAD_SEGMENT, Machine, RESERVED, Repeat, ShiftOp, StringOp,
        Width,
    };
    pub(crate) use patches::Hook;

    #[allow(
        unused_imports,
        reason = "the emitter writes every transfer; LDM.EXE has no far jumps"
    )]
    pub(crate) use super::{
        AtSite, Next, Replaced, before, call, far_call, far_call_indirect, far_jump,
        far_jump_indirect, interrupt, iret, replace, ret, retf, yield_at,
    };
    pub(crate) use crate::Game;
}

#[allow(
    unused,
    clippy::all,
    clippy::pedantic,
    reason = "generated code: formatted and annotated, not linted"
)]
mod generated {
    use super::prelude;
    use super::prelude::*;
    include!(concat!(env!("OUT_DIR"), "/translated.rs"));
}

pub(crate) use generated::step;

include!(concat!(env!("OUT_DIR"), "/source.rs"));

/// What a block does next.
pub(crate) enum Next {
    /// Continue with the block at this offset, in this run.
    Goto(u16),
    /// End this translated step; CS:IP say where the next one starts.
    Yield,
}

/// What follows a hook that replaces its instruction.
pub(crate) enum Replaced {
    /// Run the original instruction after all.
    Original,
    /// Carry on after the instruction.
    Done,
    /// Leave the block.
    Transfer(Next),
}

/// Runs a `Before` hook: `None` continues with the instruction.
pub(crate) fn before(
    m: &mut Machine,
    g: &mut Game,
    hook: Hook,
    next: u16,
) -> Result<Option<Next>, Stop> {
    Ok(match hooks::run(hook, m, g, next)? {
        After::Continue => None,
        After::Original => return Err(misplaced(hook, m)),
        After::Goto(offset) => Some(Next::Goto(offset)),
        After::Yield => Some(Next::Yield),
        After::YieldAt(offset) => Some(yielding_at(m, offset)),
    })
}

/// Runs a `Replace` hook in place of its instruction.
pub(crate) fn replace(
    m: &mut Machine,
    g: &mut Game,
    hook: Hook,
    next: u16,
) -> Result<Replaced, Stop> {
    Ok(match hooks::run(hook, m, g, next)? {
        After::Continue => Replaced::Done,
        After::Original => Replaced::Original,
        After::Goto(offset) => Replaced::Transfer(Next::Goto(offset)),
        After::Yield => Replaced::Transfer(Next::Yield),
        After::YieldAt(offset) => Replaced::Transfer(yielding_at(m, offset)),
    })
}

/// A hook yields; IP already addresses its site, unless it names another.
fn yielding_at(m: &mut Machine, offset: u16) -> Next {
    m.regs.ip = offset;
    Next::Yield
}

fn misplaced(hook: Hook, m: &Machine) -> Stop {
    let at = Address::from_runtime(m.regs.cs, m.regs.ip);
    Stop::Program(format!("hook {hook:?} at {at} asked for its instruction, but runs before it"))
}

/// `INT number`: true when the run must stop, because the program exited or
/// waits for input. IP still addresses the `INT`, so a wait retries it.
pub(crate) fn interrupt(m: &mut Machine, g: &mut Game, number: u8) -> Result<bool, Stop> {
    g.dos.interrupt(m, number).map_err(|error| {
        Stop::Program(format!("{error} at {}", Address::from_runtime(m.regs.cs, m.regs.ip)))
    })?;
    Ok(!g.dos.running || g.dos.waiting)
}

/// Division faults carry the address of their instruction.
pub(crate) trait AtSite {
    /// Attaches the image-relative address `segment:offset`.
    fn at(self, segment: u16, offset: u16) -> Result<(), Stop>;
}

impl AtSite for Result<(), Fault> {
    fn at(self, segment: u16, offset: u16) -> Result<(), Stop> {
        self.map_err(|fault| Stop::Fault { fault, at: Address::new(segment, offset) })
    }
}

/// Continues at `offset` in the next step.
#[allow(clippy::unnecessary_wraps, reason = "generated blocks return it as their result")]
pub(crate) fn yield_at(m: &mut Machine, offset: u16) -> Result<Next, Stop> {
    m.regs.ip = offset;
    Ok(Next::Yield)
}

/// `CALL`: the target was read before the return address is pushed.
pub(crate) fn call(m: &mut Machine, target: u16, next: u16) -> Result<Next, Stop> {
    m.push(next);
    yield_at(m, target)
}

/// `CALL FAR` to an image-relative address.
pub(crate) fn far_call(m: &mut Machine, target: Address, next: u16) -> Result<Next, Stop> {
    m.push(m.regs.cs);
    m.push(next);
    far_jump(m, target)
}

/// `JMP FAR` to an image-relative address.
pub(crate) fn far_jump(m: &mut Machine, target: Address) -> Result<Next, Stop> {
    m.regs.cs = target.segment.wrapping_add(LOAD_SEGMENT);
    yield_at(m, target.offset)
}

/// `JMP FAR` through a pointer in memory, which holds a runtime address.
pub(crate) fn far_jump_indirect(m: &mut Machine, segment: u16, offset: u16) -> Result<Next, Stop> {
    let (ip, cs) = far_pointer(m, segment, offset);
    m.regs.cs = cs;
    yield_at(m, ip)
}

/// `CALL FAR` through a pointer in memory, read before anything is pushed.
pub(crate) fn far_call_indirect(
    m: &mut Machine,
    segment: u16,
    offset: u16,
    next: u16,
) -> Result<Next, Stop> {
    let (ip, cs) = far_pointer(m, segment, offset);
    m.push(m.regs.cs);
    m.push(next);
    m.regs.cs = cs;
    yield_at(m, ip)
}

fn far_pointer(m: &Machine, segment: u16, offset: u16) -> (u16, u16) {
    (m.memory.read16(segment, offset), m.memory.read16(segment, offset.wrapping_add(2)))
}

/// `RET`, releasing `release` bytes of arguments.
pub(crate) fn ret(m: &mut Machine, release: u16) -> Result<Next, Stop> {
    let ip = m.pop();
    m.regs.sp = m.regs.sp.wrapping_add(release);
    yield_at(m, ip)
}

/// `RETF`, releasing `release` bytes of arguments.
pub(crate) fn retf(m: &mut Machine, release: u16) -> Result<Next, Stop> {
    let ip = m.pop();
    m.regs.cs = m.pop();
    m.regs.sp = m.regs.sp.wrapping_add(release);
    yield_at(m, ip)
}

/// `IRET`.
pub(crate) fn iret(m: &mut Machine) -> Result<Next, Stop> {
    let ip = m.pop();
    m.regs.cs = m.pop();
    m.regs.flags = m.pop() | RESERVED;
    yield_at(m, ip)
}
