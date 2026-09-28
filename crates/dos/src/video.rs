//! The video BIOS: INT 10h as the game uses it.
//!
//! Function numbers and tables follow the IBM PS/2 and PC BIOS Interface
//! Technical Reference, April 1987.

use machine::Machine;

use crate::Dos;

/// The display mode and the text scan lines the next text mode uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    /// The current BIOS video mode.
    pub mode: u8,
    /// 200, 350 or 400: selected before a text mode set.
    pub text_scan_lines: u16,
}

impl Default for Video {
    fn default() -> Self {
        Video { mode: 3, text_scan_lines: 400 }
    }
}

/// Converts a 6-bit DAC component to 8 bits, as the original build did.
fn to_eight_bits(value: u8) -> u32 {
    u32::from(value) * 255 / 63
}

/// Converts an 8-bit colour component back to 6 bits.
fn to_six_bits(value: u32) -> u8 {
    ((value & 0xff) * 63 / 255) as u8
}

fn colour(red: u8, green: u8, blue: u8) -> u32 {
    0xff00_0000 | (to_eight_bits(red) << 16) | (to_eight_bits(green) << 8) | to_eight_bits(blue)
}

impl Dos {
    /// INT 10h.
    pub(crate) fn video_service(&mut self, m: &mut Machine) -> bool {
        let (ah, al) = (m.regs.ah(), m.regs.al());
        match ah {
            0x00 => self.set_mode(m, al),
            // Cursor shape and page selection have no visible effect here.
            0x01 | 0x02 | 0x05 | 0xef => {}
            0x03 => (m.regs.cx, m.regs.dx) = (0x0607, 0),
            0x0b => m.ports.set(0x3d9, m.regs.bl()),
            0x0e => self.console.push(al),
            0x0f => {
                m.regs.ax = (m.memory.read16(0x40, 0x4a) << 8) | u16::from(self.video.mode);
                m.regs.bx &= 0x00ff;
            }
            0x10 => return palette_service(m, al),
            0x11 if al == 0x30 => {
                // The 8x8 font pointer in the BIOS ROM.
                (m.regs.es, m.regs.bp, m.regs.cx, m.regs.dx) = (0xf000, 0xfa6e, 8, 24);
            }
            0x12 => return self.alternate_select(m, al),
            0x1a => {
                m.regs.set_al(0x1a);
                m.regs.bx = 8; // VGA with an analogue colour display.
            }
            0x1b => self.functionality_state(m),
            _ => return false,
        }
        true
    }

    fn set_mode(&mut self, m: &mut Machine, al: u8) {
        self.video.mode = al & 0x7f;
        m.memory.write8(0x40, 0x49, self.video.mode);
        m.memory.write16(0x40, 0x4a, if al == 3 { 80 } else { 40 });
        if self.video.mode == 3 {
            m.memory.write8(0x40, 0x84, 24);
            m.memory.write16(0x40, 0x85, self.video.text_scan_lines / 25);
        }
    }

    /// AH=12h: alternate select.
    fn alternate_select(&mut self, m: &mut Machine, al: u8) -> bool {
        match m.regs.bl() {
            0x10 => (m.regs.bx, m.regs.cx) = (3, 0),
            // Select the text scan lines for the next mode set. Quit restores the
            // original 400-line text desktop; the native window simply closes.
            0x30 if al <= 2 => {
                self.video.text_scan_lines = [200, 350, 400][usize::from(al)];
                m.regs.set_al(0x12);
            }
            // Disable grey-scale summing: the palette stays in colour.
            0x33 if al == 1 => m.regs.set_al(0x12),
            _ => return false,
        }
        true
    }

    /// AH=1Bh: the 64-byte functionality and state table at ES:DI.
    fn functionality_state(&self, m: &mut Machine) {
        if m.regs.bx != 0 {
            m.regs.set_al(0);
            return;
        }
        let (es, di) = (m.regs.es, m.regs.di);
        let at = |offset: u16| di.wrapping_add(offset);
        for offset in 0..64 {
            m.memory.write8(es, at(offset), 0);
        }
        let graphics = self.video.mode == 0x13;
        m.memory.write16(es, at(0), 0x200); // Static functionality table at F000:0200.
        m.memory.write16(es, at(2), 0xf000);
        m.memory.write8(es, at(4), self.video.mode);
        m.memory.write16(es, at(5), m.memory.read16(0x40, 0x4a));
        m.memory.write16(es, at(7), if graphics { 64_000 } else { 0x4000 });
        m.memory.write16(es, at(0x1b), 0x0607);
        m.memory.write16(es, at(0x1e), 0x3d4);
        m.memory.write8(es, at(0x22), 24);
        m.memory.write16(es, at(0x23), 8);
        m.memory.write8(es, at(0x25), 8);
        m.memory.write16(es, at(0x27), if graphics { 256 } else { 16 });
        m.memory.write8(es, at(0x29), 1);
        m.memory.write8(es, at(0x2d), 1);
        m.memory.write8(es, at(0x31), 3);
        m.regs.set_al(0x1b);
    }
}

/// AH=10h: the attribute controller and the DAC palette.
fn palette_service(m: &mut Machine, al: u8) -> bool {
    let (bx, cx, dx) = (m.regs.bx, m.regs.cx, m.regs.dx);
    match al {
        0x00 => {
            if let Some(slot) = m.vga.attributes.get_mut(usize::from(m.regs.bl())) {
                *slot = m.regs.bh();
            }
        }
        0x01 => {}
        0x02 => {
            for (i, slot) in m.vga.attributes.iter_mut().enumerate() {
                *slot = m.memory.read8(m.regs.es, dx.wrapping_add(i as u16));
            }
        }
        0x10 => {
            m.vga.palette[usize::from(bx & 0xff)] = colour(m.regs.dh(), m.regs.ch(), m.regs.cl())
        }
        0x12 => {
            for i in 0..cx {
                let index = usize::from(bx) + usize::from(i);
                if index >= 256 {
                    break;
                }
                let at = dx.wrapping_add(i.wrapping_mul(3));
                let read = |offset: u16| m.memory.read8(m.regs.es, at.wrapping_add(offset));
                m.vga.palette[index] = colour(read(0), read(1), read(2));
            }
        }
        0x15 => {
            let entry = m.vga.palette[usize::from(bx & 0xff)];
            m.regs.dx = u16::from(to_six_bits(entry >> 16)) << 8;
            m.regs.cx = (u16::from(to_six_bits(entry >> 8)) << 8) | u16::from(to_six_bits(entry));
        }
        _ => return false,
    }
    true
}
