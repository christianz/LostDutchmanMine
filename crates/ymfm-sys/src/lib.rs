//! A safe wrapper around one vendored ymfm YM3812 (OPL2), the AdLib's chip.
//!
//! This is the workspace's only `unsafe` code: each call hands a pointer that
//! this type owns exclusively to the C shim in `shim.cpp`.

use std::ffi::c_void;
use std::ptr::NonNull;

unsafe extern "C" {
    fn ldm_opl_new() -> *mut c_void;
    fn ldm_opl_free(chip: *mut c_void);
    fn ldm_opl_write(chip: *mut c_void, offset: u32, value: u8);
    fn ldm_opl_read(chip: *mut c_void, offset: u32) -> u8;
    fn ldm_opl_advance(chip: *mut c_void, clocks: u32);
    fn ldm_opl_generate(chip: *mut c_void) -> i32;
    fn ldm_opl_save(chip: *mut c_void, out: *mut u8, size: usize) -> usize;
    fn ldm_opl_restore(chip: *mut c_void, data: *const u8, size: usize);
}

/// One YM3812. Timers advance only when [`Chip::advance`] is called, so a
/// caller that advances by emulated time gets reproducible status reads.
pub struct Chip {
    raw: NonNull<c_void>,
}

// SAFETY: the chip is plain heap memory with no thread affinity, and `Chip`
// owns it exclusively; `&mut self` serialises every access.
unsafe impl Send for Chip {}

impl Chip {
    /// A reset chip.
    ///
    /// # Panics
    ///
    /// If the C++ allocation returns null, which `operator new` never does.
    pub fn new() -> Self {
        // SAFETY: allocates a fresh chip; it is never null.
        let raw = unsafe { ldm_opl_new() };
        Chip { raw: NonNull::new(raw).expect("ymfm chip allocation") }
    }

    /// Writes the address (offset 0) or data (offset 1) port.
    pub fn write(&mut self, offset: u16, value: u8) {
        // SAFETY: `raw` is a live chip owned by `self`.
        unsafe { ldm_opl_write(self.raw.as_ptr(), u32::from(offset), value) }
    }

    /// Reads the status (offset 0) or data (offset 1) port.
    pub fn read(&mut self, offset: u16) -> u8 {
        // SAFETY: `raw` is a live chip owned by `self`.
        unsafe { ldm_opl_read(self.raw.as_ptr(), u32::from(offset)) }
    }

    /// Advances the timers by input clocks (3,579,545 per second).
    pub fn advance(&mut self, clocks: u32) {
        // SAFETY: `raw` is a live chip owned by `self`.
        unsafe { ldm_opl_advance(self.raw.as_ptr(), clocks) }
    }

    /// Generates one output sample at the chip's native rate (input clock / 72).
    pub fn generate(&mut self) -> i32 {
        // SAFETY: `raw` is a live chip owned by `self`.
        unsafe { ldm_opl_generate(self.raw.as_ptr()) }
    }

    /// The complete chip state, including timers.
    pub fn save(&self) -> Vec<u8> {
        // SAFETY: a zero-capacity call only reports the size; the second call
        // writes at most `buffer.len()` bytes into `buffer`.
        unsafe {
            let size = ldm_opl_save(self.raw.as_ptr(), std::ptr::null_mut(), 0);
            let mut buffer = vec![0; size];
            ldm_opl_save(self.raw.as_ptr(), buffer.as_mut_ptr(), buffer.len());
            buffer
        }
    }

    /// Restores a state produced by [`Chip::save`].
    pub fn restore(&mut self, state: &[u8]) {
        // SAFETY: `state` is a valid slice for the duration of the call.
        unsafe { ldm_opl_restore(self.raw.as_ptr(), state.as_ptr(), state.len()) }
    }
}

impl Default for Chip {
    fn default() -> Self {
        Chip::new()
    }
}

impl Clone for Chip {
    fn clone(&self) -> Self {
        let mut copy = Chip::new();
        copy.restore(&self.save());
        copy
    }
}

impl Drop for Chip {
    fn drop(&mut self) {
        // SAFETY: `raw` came from `ldm_opl_new` and is freed exactly once.
        unsafe { ldm_opl_free(self.raw.as_ptr()) }
    }
}

impl std::fmt::Debug for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Chip(YM3812)")
    }
}
