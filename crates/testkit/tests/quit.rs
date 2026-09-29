//! `tests/quit.cpp`: Quit Game calls the original graphics shutdown indirectly.
//! It must take its near return, preserve the caller's registers and stack,
//! and restore the 400-line text desktop.

use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The graphics driver's shutdown routine.
const SHUTDOWN: Address = Address::new(0x1613, 0x1c7d);
/// The near return address the call ends at.
const NEAR_RETURN: u16 = 0xfffe;
/// The graphics driver's saved desktop video mode.
const DESKTOP_MODE: u16 = 0x2041;
/// Set when the desktop had an extended text font to restore.
const EXTENDED_FONT: u16 = 0x2044;
/// The adapter flags; 8 makes VGA restore the text scan lines before mode 3.
const ADAPTER: u16 = 0x380a;
/// The driver's saved text cursor shape.
const CURSOR_SHAPE: u16 = 0x36fc;
/// The BIOS data area.
const BIOS_DATA: u16 = 0x40;
/// Its video mode.
const BIOS_VIDEO_MODE: u16 = 0x49;
/// Its character height, in scan lines.
const BIOS_CHARACTER_HEIGHT: u16 = 0x85;
/// The 80x25 colour text mode.
const TEXT_MODE: u8 = 3;
/// The shutdown returns well within this many steps.
const SHUTDOWN_LIMIT: u64 = 10_000;

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn graphics_shutdown_restores_the_text_desktop_and_returns_safely() {
    for adapter in [0, 8] {
        let mut h = Harness::loaded();
        let ds = h.m.regs.ds;
        (h.m.regs.bp, h.m.regs.si, h.m.regs.di) = (0x1234, 0x5678, 0x9abc);
        h.game.dos.video.mode = game::symbols::VIDEO_MODE_VGA;
        h.m.memory.write8(ds, DESKTOP_MODE, TEXT_MODE);
        h.m.memory.write8(ds, EXTENDED_FONT, 0);
        h.m.memory.write8(ds, ADAPTER, adapter);
        h.m.memory.write16(ds, CURSOR_SHAPE, 0x1903);
        h.m.push(NEAR_RETURN);
        h.jump(SHUTDOWN);
        h.until(SHUTDOWN_LIMIT, "Graphics shutdown did not return", |h| h.m.regs.ip == NEAR_RETURN);
        let r = &h.m.regs;
        assert!(
            r.cs == SHUTDOWN.runtime_segment()
                && r.sp == STACK_TOP
                && (r.bp, r.si, r.di) == (0x1234, 0x5678, 0x9abc),
            "Graphics shutdown corrupted registers or stack"
        );
        let video = h.game.dos.video;
        assert!(
            video.mode == TEXT_MODE
                && h.m.memory.read8(BIOS_DATA, BIOS_VIDEO_MODE) == TEXT_MODE
                && video.text_scan_lines == 400
                && h.m.memory.read16(BIOS_DATA, BIOS_CHARACTER_HEIGHT) == 16,
            "Graphics shutdown did not restore the original video mode"
        );
    }
}
