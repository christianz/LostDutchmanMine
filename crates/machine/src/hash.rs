//! The state hash that identifies a moment of execution in traces.

use crate::Machine;

const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64.
#[derive(Clone, Copy, Debug)]
pub struct Fnv(u64);

impl Fnv {
    /// An empty hash.
    pub const fn new() -> Self {
        Fnv(OFFSET_BASIS)
    }

    /// Mixes in bytes.
    pub fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(PRIME);
        }
    }

    /// The hash so far.
    pub const fn finish(self) -> u64 {
        self.0
    }
}

impl Default for Fnv {
    fn default() -> Self {
        Fnv::new()
    }
}

impl Machine {
    /// FNV-1a 64 over the registers in trace order, the 1 MiB memory and the DAC
    /// palette, all little-endian. The C++ oracle computes exactly the same.
    pub fn state_hash(&self) -> u64 {
        let mut hash = Fnv::new();
        for register in self.regs.in_trace_order() {
            hash.write(&register.to_le_bytes());
        }
        hash.write(self.memory.as_bytes());
        for colour in self.vga.palette {
            hash.write(&colour.to_le_bytes());
        }
        hash.finish()
    }
}
