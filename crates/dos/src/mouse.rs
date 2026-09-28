//! The mouse driver: desktop events in, INT 33h out.

use std::collections::VecDeque;

use machine::Machine;

use crate::Dos;

/// Clicks older than this are dropped: made long before an interactive screen
/// was ready (title, loading), they must never replay into a shop.
const CLICK_LIFETIME_MS: u64 = 1000;

/// A pointer position in 320x200 game pixels, and the buttons held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseSample {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
    /// Bit 0 left, bit 1 right.
    pub buttons: u8,
}

impl Default for MouseSample {
    fn default() -> Self {
        MouseSample { x: 160, y: 100, buttons: 0 }
    }
}

/// Desktop events are edges, but the game polls button levels. Each edge is kept
/// until a complete original mouse read has observed it, so a short press and
/// release are never lost between polls, and a click keeps its own position.
#[derive(Clone, Debug, Default)]
pub struct MouseInput {
    current: MouseSample,
    sample: MouseSample,
    pending: VecDeque<(MouseSample, u64)>,
    now_ms: u64,
}

impl MouseInput {
    /// Emulated time, set before each simulation quantum.
    pub fn set_time(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// The pointer moved.
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.current.x = x.clamp(0, 319);
        self.current.y = y.clamp(0, 199);
    }

    /// The held buttons changed.
    pub fn buttons(&mut self, mask: u8) {
        let mask = mask & 3;
        if mask != self.current.buttons {
            self.current.buttons = mask;
            self.pending.push_back((self.current, self.now_ms));
        }
    }

    /// Latches the next sample for one original read: the oldest unexpired edge,
    /// or the current state.
    pub fn poll(&mut self) {
        while self.pending.front().is_some_and(|&(_, at)| self.now_ms > at + CLICK_LIFETIME_MS) {
            self.pending.pop_front();
        }
        self.sample = self.pending.pop_front().map_or(self.current, |(sample, _)| sample);
    }

    /// Focus was lost: release the buttons and forget queued edges.
    pub fn clear(&mut self) {
        self.current.buttons = 0;
        self.discard_pending();
    }

    /// Forgets queued edges; the sample becomes the current state.
    pub fn discard_pending(&mut self) {
        self.pending.clear();
        self.sample = self.current;
    }

    /// The program positioned the pointer.
    pub fn warp(&mut self, x: i32, y: i32) {
        self.move_to(x, y);
        (self.sample.x, self.sample.y) = (self.current.x, self.current.y);
    }

    /// The latest state.
    pub fn current(&self) -> MouseSample {
        self.current
    }

    /// The sample latched by the last [`MouseInput::poll`].
    pub fn sample(&self) -> MouseSample {
        self.sample
    }
}

/// The INT 33h driver's state.
#[derive(Clone, Debug)]
pub struct Mouse {
    /// Desktop input.
    pub input: MouseInput,
    /// The driver's cursor counter: shown when non-negative.
    pub visibility: i32,
    /// The program's cursor hotspot.
    pub hotspot: (i32, i32),
    /// The program's cursor: 16 screen-mask rows, then 16 cursor-mask rows.
    pub shape: [u16; 32],
    /// Whether the program defined its own cursor.
    pub custom_cursor: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Mouse {
            input: MouseInput::default(),
            visibility: -1,
            hotspot: (0, 0),
            shape: [0; 32],
            custom_cursor: false,
        }
    }
}

impl Dos {
    /// INT 33h.
    pub(crate) fn mouse_service(&mut self, m: &mut Machine) -> bool {
        let mouse = &mut self.mouse;
        match m.regs.ax {
            0x00 => {
                (m.regs.ax, m.regs.bx) = (0xffff, 2);
                mouse.visibility = -1;
                mouse.input.clear();
            }
            0x01 => mouse.visibility += 1,
            0x02 => mouse.visibility -= 1,
            0x03 => {
                let sample = mouse.input.sample();
                m.regs.bx = u16::from(sample.buttons);
                m.regs.cx = (sample.x * 2) as u16;
                m.regs.dx = sample.y as u16;
            }
            // The DOS selectors park the hidden hand beside the clock. With a
            // persistent desktop pointer (QoL) only physical motion moves it.
            0x04 if self.park_cursor => {
                mouse.input.warp(i32::from(m.regs.cx / 2), i32::from(m.regs.dx))
            }
            0x04 | 0x07 | 0x08 | 0x0a | 0x0f => {}
            0x09 => {
                mouse.hotspot = (i32::from(m.regs.bx as i16), i32::from(m.regs.cx as i16));
                for (row, slot) in mouse.shape.iter_mut().enumerate() {
                    *slot = m.memory.read16(m.regs.es, m.regs.dx.wrapping_add(row as u16 * 2));
                }
                mouse.custom_cursor = true;
            }
            0x0b => (m.regs.cx, m.regs.dx) = (0, 0),
            _ => return false,
        }
        true
    }
}
