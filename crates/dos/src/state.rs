//! DOS in a savestate: the program's status, queued input, the mouse, the
//! video mode, the clock and the files it has open.

use machine::state::{Persist, Reader, StateError, Writer};

use crate::Dos;

impl Persist for Dos {
    fn save(&self, w: &mut Writer) {
        w.section(b"DOS ");
        w.bool(self.running);
        w.bool(self.waiting);
        w.bool(self.park_cursor);
        w.u16(self.next_paragraph);
        w.bytes(&self.console);
        self.keyboard.save(w);
        self.mouse.save(w);
        w.u8(self.video.mode);
        w.u16(self.video.text_scan_lines);
        w.i64(self.clock.base);
        w.u64(self.clock.emulated_ms);
        w.u32(self.clock.ticks);
        self.files.save(w);
    }

    fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        r.section(b"DOS ")?;
        self.running = r.bool()?;
        self.waiting = r.bool()?;
        self.park_cursor = r.bool()?;
        self.next_paragraph = r.u16()?;
        self.console = r.bytes()?.to_vec();
        self.keyboard.restore(r)?;
        self.mouse.restore(r)?;
        self.video.mode = r.u8()?;
        self.video.text_scan_lines = r.u16()?;
        self.clock.base = r.i64()?;
        self.clock.emulated_ms = r.u64()?;
        self.clock.ticks = r.u32()?;
        self.files.restore(r)
    }
}
