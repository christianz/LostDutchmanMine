//! Routing interrupts to services, and the DOS calls that belong to no family.

use machine::{Flag, Machine};

use crate::{Dos, DosError};

impl Dos {
    /// Runs the service for `INT number`. Afterwards [`Dos::waiting`] and
    /// [`Dos::running`] tell the caller whether to stop the current run.
    ///
    /// # Errors
    ///
    /// [`DosError`] for services the game was never seen to call, and for host
    /// file errors.
    pub fn interrupt(&mut self, m: &mut Machine, number: u8) -> Result<(), DosError> {
        self.waiting = false;
        let handled = match number {
            0x20 => {
                self.running = false;
                true
            }
            0x10 => self.video_service(m),
            0x16 => self.keyboard_service(m),
            0x1a => self.timer_service(m),
            0x21 => self.dos_service(m)?,
            0x33 => self.mouse_service(m),
            _ => false,
        };
        if handled { Ok(()) } else { Err(DosError::Unsupported { number, ax: m.regs.ax }) }
    }

    fn dos_service(&mut self, m: &mut Machine) -> Result<bool, DosError> {
        let (ah, al) = (m.regs.ah(), m.regs.al());
        let vector = u16::from(al) * 4;
        match ah {
            0x00 | 0x4c => self.running = false,
            0x07 | 0x08 => self.read_character(m),
            0x09 => {
                let text = read_string(m, m.regs.ds, m.regs.dx, b'$')?;
                self.console.extend(text);
            }
            0x0b => self.check_input(m),
            0x0e => m.regs.set_al(3),
            0x19 => m.regs.set_al(2),
            // Set DTA, free and resize memory: nothing to track.
            0x1a | 0x49 | 0x4a => success(m),
            0x25 => {
                m.memory.write16(0, vector, m.regs.dx);
                m.memory.write16(0, vector + 2, m.regs.ds);
            }
            0x2a => self.clock.date(m),
            0x2c => self.clock.time(m),
            0x30 => (m.regs.ax, m.regs.bx, m.regs.cx) = (5, 0, 0),
            0x33 => m.regs.dx = 0,
            0x35 => {
                (m.regs.bx, m.regs.es) =
                    (m.memory.read16(0, vector), m.memory.read16(0, vector + 2))
            }
            0x36 => (m.regs.ax, m.regs.bx, m.regs.cx, m.regs.dx) = (1, 0x4000, 512, 0x8000),
            0x3c..=0x40 | 0x42 | 0x43 => return self.file_service(m),
            0x44 if al == 0 => {
                m.regs.dx = if m.regs.bx < 5 { 0x80d3 } else { 0 };
                success(m);
            }
            0x47 => {
                m.memory.write8(m.regs.ds, m.regs.si, 0);
                m.regs.ax = 0x100;
                success(m);
            }
            0x48 => self.allocate(m),
            0x57 if al == 0 => {
                (m.regs.cx, m.regs.dx) = (0, 0x12e1);
                success(m);
            }
            0x57 if al == 1 => success(m),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// INT 21h AH=48h: memory is handed out upwards and never reclaimed; the
    /// game allocates once at startup.
    fn allocate(&mut self, m: &mut Machine) {
        const TOP: u16 = 0x9fff;
        if u32::from(self.next_paragraph) + u32::from(m.regs.bx) > u32::from(TOP) {
            m.regs.bx = TOP - self.next_paragraph;
            failure(m, 8);
        } else {
            m.regs.ax = self.next_paragraph;
            self.next_paragraph = self.next_paragraph.wrapping_add(m.regs.bx).wrapping_add(1);
            success(m);
        }
    }
}

/// Reads a string ending in `end` from `segment:offset`.
pub(crate) fn read_string(
    m: &Machine,
    segment: u16,
    offset: u16,
    end: u8,
) -> Result<Vec<u8>, DosError> {
    let mut text = Vec::new();
    for i in 0..=u16::MAX {
        let byte = m.memory.read8(segment, offset.wrapping_add(i));
        if byte == end {
            return Ok(text);
        }
        text.push(byte);
    }
    Err(DosError::Unterminated { segment, offset })
}

/// A DOS call that succeeded: carry clear.
pub(crate) fn success(m: &mut Machine) {
    m.set_flag(Flag::Carry, false);
}

/// A DOS call that failed with `error`: carry set, the code in AX.
pub(crate) fn failure(m: &mut Machine, error: u16) {
    m.set_flag(Flag::Carry, true);
    m.regs.ax = error;
}
