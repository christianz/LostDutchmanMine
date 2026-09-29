//! The DOS loader, and the machine state DOS and the BIOS leave for a program.

use machine::{Address, LOAD_SEGMENT, Machine, Memory};

use crate::DosError;

/// The highest image size that fits below the video memory at A000:0000.
const MAX_IMAGE: usize = 0x9_0000;
/// The program segment prefix sits just below the load segment.
const PSP: u16 = LOAD_SEGMENT - 0x10;
/// The environment block DOS passes: PATH, COMSPEC and the program's name.
const ENVIRONMENT: &[u8] = b"PATH=C:\\\0COMSPEC=C:\\COMMAND.COM\0\0\x01\0C:\\LDM.EXE\0\0";
const ENVIRONMENT_SEGMENT: u16 = 0x0f00;

/// An unpacked program image and how to start it.
#[derive(Clone, Copy, Debug)]
pub struct Image<'a> {
    /// The image, unrelocated.
    pub bytes: &'a [u8],
    /// Image offsets of the segment words to relocate.
    pub relocations: &'a [u32],
    /// The entry point, image-relative.
    pub entry: Address,
    /// The initial stack, image-relative segment and pointer.
    pub stack: Address,
}

/// Loads `image` at [`LOAD_SEGMENT`] and prepares the environment the game
/// expects: its PSP and environment block, an interrupt table pointing into the
/// BIOS, the BIOS data area, the video functionality table, and the default
/// EGA-compatible palette.
///
/// # Errors
///
/// [`DosError::ImageTooLarge`] when the image would overlap video memory.
pub fn boot(m: &mut Machine, image: &Image) -> Result<(), DosError> {
    if image.bytes.len() > MAX_IMAGE {
        return Err(DosError::ImageTooLarge(image.bytes.len()));
    }
    let base = Memory::linear(LOAD_SEGMENT, 0);
    let memory = m.memory.as_bytes_mut();
    memory[base..base + image.bytes.len()].copy_from_slice(image.bytes);
    for &offset in image.relocations {
        let at = base + offset as usize;
        let word = u16::from_le_bytes([memory[at], memory[at + 1]]).wrapping_add(LOAD_SEGMENT);
        memory[at..at + 2].copy_from_slice(&word.to_le_bytes());
    }
    let environment = Memory::linear(ENVIRONMENT_SEGMENT, 0);
    memory[environment..environment + ENVIRONMENT.len()].copy_from_slice(ENVIRONMENT);

    let r = &mut m.regs;
    (r.cs, r.ip) = (image.entry.runtime_segment(), image.entry.offset);
    (r.ss, r.sp) = (image.stack.runtime_segment(), image.stack.offset);
    (r.ds, r.es) = (PSP, PSP);

    m.memory.write16(PSP, 0x00, 0x20cd); // INT 20h: return to DOS.
    m.memory.write16(PSP, 0x02, 0x9fff); // Top of memory.
    m.memory.write16(PSP, 0x2c, ENVIRONMENT_SEGMENT);
    m.memory.write8(PSP, 0x80, 0); // Empty command tail...
    m.memory.write8(PSP, 0x81, 0x0d); // ...ending in a carriage return.

    for vector in 0..256_u16 {
        m.memory.write16(0, vector * 4, vector * 4);
        m.memory.write16(0, vector * 4 + 2, 0xf000);
    }
    bios_data_area(m);
    default_palette(m);
    Ok(())
}

/// The BIOS data area at 0040:0000 and the static video functionality table.
fn bios_data_area(m: &mut Machine) {
    m.memory.write16(0x40, 0x10, 0x21); // Equipment: floppy, colour 80x25.
    m.memory.write16(0x40, 0x13, 640); // Kilobytes of conventional memory.
    m.memory.write8(0x40, 0x49, 3); // Video mode.
    m.memory.write16(0x40, 0x4a, 80); // Text columns.
    m.memory.write8(0x40, 0x84, 24); // Text rows minus one.
    m.memory.write8(0x40, 0x87, 0x60); // EGA/VGA information.
    m.memory.write8(0x40, 0x88, 0x09);
    // Static functionality: standard VGA modes, 200/350/400-line text.
    m.memory.write8(0xf000, 0x200, 0xff);
    m.memory.write8(0xf000, 0x201, 0xe0);
    m.memory.write8(0xf000, 0x202, 0x0f);
    m.memory.write8(0xf000, 0x207, 7);
}

/// The 64 EGA colours in the first DAC entries, and the attribute controller
/// mapping the 16 text colours onto them (brown is entry 20).
fn default_palette(m: &mut Machine) {
    for c in 0..64_u32 {
        let level = |high: u32, low: u32| {
            (if c & high != 0 { 170 } else { 0 }) + (if c & low != 0 { 85 } else { 0 })
        };
        let (red, green, blue) = (level(4, 32), level(2, 16), level(1, 8));
        m.vga.palette[c as usize] = 0xff00_0000 | (red << 16) | (green << 8) | blue;
    }
    for (c, slot) in m.vga.attributes.iter_mut().enumerate() {
        *slot = match c {
            6 => 20,
            0..8 => c as u8,
            _ => c as u8 + 48,
        };
    }
}
