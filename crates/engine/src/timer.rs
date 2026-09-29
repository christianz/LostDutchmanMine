//! The timer interrupt: IRQ 0 delivered to whatever INT 08h points at.

use machine::{Flag, Machine, RESERVED};

use crate::recent::Recent;
use crate::{Program, Stop};

/// The BIOS segment; its own INT 08h handler lives at F000:0020.
const BIOS: u16 = 0xf000;
const BIOS_TIMER_HANDLER: u16 = 0x0020;
/// A handler still running after this many steps has hung.
const HANDLER_STEP_LIMIT: u64 = 1_000_000;

/// Delivers one timer interrupt if interrupts are enabled. While INT 08h still
/// points into the BIOS the tick is counted directly; otherwise the game's
/// handler runs to completion here, including its chain back to the BIOS.
///
/// # Errors
///
/// [`Stop::TimerHandler`] if the handler never returns, or the program's stop.
pub fn interrupt<P: Program>(
    m: &mut Machine,
    program: &mut P,
    recent: &mut Recent,
) -> Result<(), Stop> {
    if !m.flag(Flag::Interrupt) {
        return Ok(());
    }
    let (handler_ip, handler_cs) = (m.memory.read16(0, 0x20), m.memory.read16(0, 0x22));
    if handler_cs == BIOS {
        program.bios_tick(m);
        return Ok(());
    }
    let resume = (m.regs.cs, m.regs.ip, m.regs.sp);
    m.push(m.regs.flags);
    m.push(m.regs.cs);
    m.push(m.regs.ip);
    m.set_flag(Flag::Interrupt, false);
    (m.regs.cs, m.regs.ip) = (handler_cs, handler_ip);
    let start = m.steps;
    while program.running() && (m.regs.cs, m.regs.ip, m.regs.sp) != resume {
        if m.steps - start > HANDLER_STEP_LIMIT {
            return Err(Stop::TimerHandler);
        }
        if (m.regs.cs, m.regs.ip) == (BIOS, BIOS_TIMER_HANDLER) {
            program.bios_tick(m);
            m.regs.ip = m.pop();
            m.regs.cs = m.pop();
            m.regs.flags = m.pop() | RESERVED;
        } else {
            recent.note(m);
            program.step(m)?;
        }
    }
    Ok(())
}
