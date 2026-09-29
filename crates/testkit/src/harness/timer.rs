//! IRQ 0 as the C++ `State` delivered it between generated steps: the game's
//! handler runs to completion at once, including its chain to the BIOS.

use engine::Program;
use game::Game;
use machine::{Flag, Machine, RESERVED};

/// A timer handler still running after this many steps has hung.
const HANDLER_LIMIT: u64 = 1_000_000;
/// The BIOS's segment.
const BIOS_SEGMENT: u16 = 0xf000;
/// The BIOS's own INT 08h handler, where the game's handler chains.
const BIOS_TIMER: u16 = 0x0020;
/// The interrupt table's INT 08h entry.
const TIMER_VECTOR: u16 = 8 * 4;

/// Delivers one timer interrupt unless interrupts are masked. While INT 08h
/// still points into the BIOS the tick is counted directly.
///
/// # Panics
///
/// When the handler does not return, or the game stops inside it.
pub(super) fn interrupt(m: &mut Machine, game: &mut Game) {
    if !m.flag(Flag::Interrupt) {
        return;
    }
    let (ip, cs) = (m.memory.read16(0, TIMER_VECTOR), m.memory.read16(0, TIMER_VECTOR + 2));
    if cs == BIOS_SEGMENT {
        game.bios_tick(m);
        return;
    }
    let resume = (m.regs.cs, m.regs.ip, m.regs.sp);
    m.push(m.regs.flags);
    m.push(m.regs.cs);
    m.push(m.regs.ip);
    m.set_flag(Flag::Interrupt, false);
    (m.regs.cs, m.regs.ip) = (cs, ip);
    let start = m.steps;
    while game.running() && (m.regs.cs, m.regs.ip, m.regs.sp) != resume {
        assert!(m.steps - start <= HANDLER_LIMIT, "Original timer handler did not return");
        if (m.regs.cs, m.regs.ip) == (BIOS_SEGMENT, BIOS_TIMER) {
            game.bios_tick(m);
            m.regs.ip = m.pop();
            m.regs.cs = m.pop();
            m.regs.flags = m.pop() | RESERVED;
        } else if let Err(stop) = game.step(m) {
            panic!("the game's timer handler stopped: {stop}");
        }
    }
}
