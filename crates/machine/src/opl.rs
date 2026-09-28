//! The AdLib's OPL2 chip, as the game sees it through ports 388h and 389h.

use ymfm_sys::Chip;

/// Input clocks each port access costs, standing in for the ISA delay loops the
/// original driver runs between accesses.
const ACCESS_CLOCKS: u32 = 12;

/// The chip's register and timer state. Its timers advance only through port
/// accesses and [`Opl::advance`], so status reads are reproducible. Port writes
/// are also queued for the audio output, which synthesises from its own copy.
#[derive(Clone, Debug, Default)]
pub struct Opl {
    chip: Chip,
    pending: Vec<(u16, u8)>,
    data_writes: u64,
}

impl Opl {
    /// Writes the address (offset 0) or data (offset 1) port.
    pub fn write(&mut self, offset: u16, value: u8) {
        self.chip.advance(ACCESS_CLOCKS);
        self.chip.write(offset, value);
        if offset & 1 != 0 {
            self.data_writes += 1;
        }
        self.pending.push((offset, value));
    }

    /// Reads the status (offset 0) or data (offset 1) port.
    pub fn read(&mut self, offset: u16) -> u8 {
        self.chip.advance(ACCESS_CLOCKS);
        self.chip.read(offset)
    }

    /// Advances the timers by emulated input clocks (3,579,545 per second).
    pub fn advance(&mut self, clocks: u32) {
        self.chip.advance(clocks);
    }

    /// The port writes since the last call, oldest first, for audio output.
    pub fn take_writes(&mut self) -> Vec<(u16, u8)> {
        std::mem::take(&mut self.pending)
    }

    /// The number of data-port writes so far; a diagnostic.
    pub fn data_writes(&self) -> u64 {
        self.data_writes
    }

    /// The complete chip state, for savestates.
    pub fn save(&self) -> Vec<u8> {
        self.chip.save()
    }

    /// Restores a state produced by [`Opl::save`].
    pub fn restore(&mut self, state: &[u8]) {
        self.chip.restore(state);
        self.pending.clear();
    }
}
