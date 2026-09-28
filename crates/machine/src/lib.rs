//! The 8086 machine Lost Dutchman Mine runs on: registers and flags, the ALU,
//! one megabyte of memory, I/O ports, the timer, the AdLib chip and the VGA
//! colour registers. It knows nothing of files, clocks or the game.

mod address;
mod alu;
mod cond;
mod fault;
mod flags;
mod hash;
mod io;
mod memory;
mod opl;
mod registers;
mod strings;
mod vga;

pub use address::Address;
pub use alu::{AluOp, ShiftOp, Width};
pub use cond::Cond;
pub use fault::Fault;
pub use flags::{Flag, RESERVED};
pub use hash::Fnv;
pub use io::{Pit, Ports};
pub use memory::{Memory, SIZE as MEMORY_SIZE};
pub use opl::Opl;
pub use registers::{Reg8, Reg16, Registers};
pub use strings::{Repeat, StringOp};
pub use vga::Vga;

/// The segment the game's load image starts at. Image-relative segments in
/// addresses, patch sites and diagnostics are offsets from here.
pub const LOAD_SEGMENT: u16 = 0x1000;

/// The complete machine.
#[derive(Clone, Debug)]
pub struct Machine {
    /// The register file.
    pub regs: Registers,
    /// Real-mode memory.
    pub memory: Memory,
    /// Latched I/O port values.
    pub ports: Ports,
    /// The programmable interval timer.
    pub pit: Pit,
    /// The AdLib's OPL2 chip.
    pub opl: Opl,
    /// The VGA colour registers.
    pub vga: Vga,
    /// Translated runs started so far: the C++ build's "boundaries".
    pub steps: u64,
}

impl Machine {
    /// A machine after reset: zeroed memory, interrupts enabled.
    pub fn new() -> Self {
        let regs = Registers { flags: Flag::Interrupt.mask() | RESERVED, ..Registers::default() };
        Machine {
            regs,
            memory: Memory::new(),
            ports: Ports::default(),
            pit: Pit::default(),
            opl: Opl::default(),
            vga: Vga::default(),
            steps: 0,
        }
    }

    /// Whether a flag is set.
    pub const fn flag(&self, flag: Flag) -> bool {
        self.regs.flags & flag.mask() != 0
    }

    /// Sets or clears a flag.
    pub const fn set_flag(&mut self, flag: Flag, on: bool) {
        self.regs.flags =
            if on { self.regs.flags | flag.mask() } else { self.regs.flags & !flag.mask() };
    }

    /// Pushes a word onto SS:SP.
    pub fn push(&mut self, value: u16) {
        self.regs.sp = self.regs.sp.wrapping_sub(2);
        self.memory.write16(self.regs.ss, self.regs.sp, value);
    }

    /// Pops a word from SS:SP.
    pub fn pop(&mut self) -> u16 {
        let value = self.memory.read16(self.regs.ss, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(2);
        value
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine::new()
    }
}
