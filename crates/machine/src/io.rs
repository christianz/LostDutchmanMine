//! I/O ports and the programmable interval timer.

use crate::{Machine, Width};

/// The PIT's channel 0 (the system timer) and channel 2 (the PC speaker), as
/// programmed through ports 40h, 42h and 43h in low-byte-then-high-byte mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pit {
    /// Channel 0 reload value; zero means 65536.
    pub timer_divisor: u16,
    /// Channel 2 reload value; zero means 65536.
    pub speaker_divisor: u16,
    timer_low: u8,
    timer_writes: u32,
    speaker_low: u8,
    speaker_writes: u32,
}

impl Pit {
    /// PIT input clocks per timer interrupt.
    pub const fn timer_period(&self) -> u64 {
        if self.timer_divisor == 0 { 65536 } else { self.timer_divisor as u64 }
    }

    fn write(&mut self, port: u16, value: u8) {
        match port {
            0x43 if value & 0xc0 == 0x00 => self.timer_writes = 0,
            0x43 if value & 0xc0 == 0x80 => self.speaker_writes = 0,
            0x40 => {
                if self.timer_writes.is_multiple_of(2) {
                    self.timer_low = value;
                } else {
                    self.timer_divisor = u16::from_le_bytes([self.timer_low, value]);
                }
                self.timer_writes = self.timer_writes.wrapping_add(1);
            }
            0x42 => {
                if self.speaker_writes.is_multiple_of(2) {
                    self.speaker_low = value;
                } else {
                    self.speaker_divisor = u16::from_le_bytes([self.speaker_low, value]);
                }
                self.speaker_writes = self.speaker_writes.wrapping_add(1);
            }
            _ => {}
        }
    }
}

/// The last value written to each port, as the game reads them back.
#[derive(Clone)]
pub struct Ports {
    values: Box<[u8; 65536]>,
}

impl Ports {
    fn new() -> Self {
        let values = vec![0; 65536].into_boxed_slice().try_into().expect("one byte per port");
        Ports { values }
    }

    /// The latched byte of a port.
    pub fn get(&self, port: u16) -> u8 {
        self.values[usize::from(port)]
    }
}

impl Default for Ports {
    fn default() -> Self {
        Ports::new()
    }
}

impl std::fmt::Debug for Ports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Ports")
    }
}

/// The AdLib ports.
const OPL_ADDRESS: u16 = 0x388;
const OPL_DATA: u16 = 0x389;
/// The VGA input status register; the game polls it for retrace.
const VGA_STATUS: u16 = 0x3da;

impl Machine {
    /// OUT: programs the PIT and OPL, and latches the value for later reads.
    pub fn port_out(&mut self, port: u16, value: u16, width: Width) {
        if port == OPL_ADDRESS || port == OPL_DATA {
            self.opl.write(port - OPL_ADDRESS, value as u8);
        }
        self.pit.write(port, value as u8);
        let [low, high] = value.to_le_bytes();
        self.ports.values[usize::from(port)] = low;
        if width == Width::Word {
            self.ports.values[usize::from(port.wrapping_add(1))] = high;
        }
    }

    /// IN: reads the OPL status, toggles the VGA retrace bits, or returns the
    /// latched value.
    pub fn port_in(&mut self, port: u16, width: Width) -> u16 {
        if port == OPL_ADDRESS || port == OPL_DATA {
            return u16::from(self.opl.read(port - OPL_ADDRESS));
        }
        if port == VGA_STATUS {
            self.ports.values[usize::from(port)] ^= 0x09;
        }
        let low = self.ports.values[usize::from(port)];
        let high = if width == Width::Word {
            self.ports.values[usize::from(port.wrapping_add(1))]
        } else {
            0
        };
        u16::from_le_bytes([low, high])
    }
}
