//! The BIOS keyboard buffer, INT 16h and the DOS character calls.
//!
//! A buffered key is the BIOS scan/character pair in its low 16 bits, plus
//! native-only tags: bit 16 marks an auto-repeat, bits 17-23 hold an alternative
//! movement scan, and bits 24-30 name the physical key, so a release can remove
//! its queued repeats even after Num Lock changes.

use std::collections::VecDeque;

use machine::{Flag, Machine};

use crate::Dos;

/// Tag: the key is an auto-repeat.
pub const KEY_REPEAT: u32 = 1 << 16;
/// Shift of the tag holding the key's movement scan.
pub const KEY_MOVEMENT_SHIFT: u32 = 17;
/// Shift of the tag naming the physical key.
pub const KEY_SOURCE_SHIFT: u32 = 24;

/// Whether a scan code is one of the eight cursor-pad directions.
pub const fn direction_scan(scan: u16) -> bool {
    matches!(scan, 0x47 | 0x48 | 0x49 | 0x4b | 0x4d | 0x4f | 0x50 | 0x51)
}

/// The BIOS pair a reader sees: movement readers get the movement scan alone.
pub const fn bios_key(key: u32, movement: bool) -> u16 {
    let scan = (key >> KEY_MOVEMENT_SHIFT) & 0x7f;
    if movement && scan != 0 { (scan << 8) as u16 } else { key as u16 }
}

/// Whether `key` is an auto-repeat of physical key `source`.
pub const fn repeat_from(key: u32, source: u32) -> bool {
    key & KEY_REPEAT != 0 && key >> KEY_SOURCE_SHIFT == source
}

/// The keys waiting to be read.
#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    keys: VecDeque<u32>,
    /// Set while one of the game's movement readers polls: they see movement
    /// scans, so WASD walks while text fields still receive letters.
    pub movement_aliases: bool,
}

impl Keyboard {
    /// Queues a key.
    pub fn push(&mut self, key: u32) {
        self.keys.push_back(key);
    }

    /// The oldest key, if any.
    pub fn front(&self) -> Option<u32> {
        self.keys.front().copied()
    }

    /// Removes and returns the oldest key.
    pub fn pop(&mut self) -> Option<u32> {
        self.keys.pop_front()
    }

    /// The number of queued keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no key is queued.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Removes every queued key.
    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// Keeps only the keys for which `keep` holds.
    pub fn retain(&mut self, keep: impl FnMut(&u32) -> bool) {
        self.keys.retain(keep);
    }

    /// The queued keys, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.keys.iter().copied()
    }
}

impl Dos {
    /// INT 16h.
    pub(crate) fn keyboard_service(&mut self, m: &mut Machine) -> bool {
        let movement = self.keyboard.movement_aliases;
        match m.regs.ah() {
            // Peek: ZF when empty, otherwise the key stays queued.
            0x01 | 0x11 => {
                m.set_flag(Flag::Zero, self.keyboard.is_empty());
                if let Some(key) = self.keyboard.front() {
                    m.regs.ax = bios_key(key, movement);
                }
            }
            // Read: wait until a key arrives.
            0x00 | 0x10 => match self.keyboard.pop() {
                Some(key) => m.regs.ax = bios_key(key, movement),
                None => self.waiting = true,
            },
            // Shift flags: none held.
            0x02 => m.regs.set_al(0),
            _ => return false,
        }
        true
    }

    /// INT 21h AH=07h/08h: read a character without echo, waiting if needed.
    pub(crate) fn read_character(&mut self, m: &mut Machine) {
        match self.keyboard.pop() {
            Some(key) => m.regs.set_al(key as u8),
            None => self.waiting = true,
        }
    }

    /// INT 21h AH=0Bh: whether input is waiting, without consuming it. The
    /// original music loop polls this before reading Escape.
    pub(crate) fn check_input(&mut self, m: &mut Machine) {
        m.regs.set_al(if self.keyboard.is_empty() { 0 } else { 0xff });
    }
}
