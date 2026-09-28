//! The game in a savestate: DOS, and everything the port keeps beside the
//! translated game's own memory.

use dos::MouseSample;
use machine::state::{Persist, Reader, StateError, Writer};

use crate::Game;

impl Persist for Game {
    fn save(&self, w: &mut Writer) {
        w.section(b"GAME");
        self.dos.save(w);
        w.bool(self.qol);
        self.pointer.save(w);
        self.walk.save(w);
        w.bool(self.desert_view);
        w.bool(self.supplies.panning);
        w.bool(self.supplies.mining_space_held);
        self.combat.save(w);
        self.overlay.save(w);
        self.held.save(w);
    }

    fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        r.section(b"GAME")?;
        self.dos.restore(r)?;
        self.qol = r.bool()?;
        self.pointer.restore(r)?;
        self.walk.restore(r)?;
        self.desert_view = r.bool()?;
        self.supplies.panning = r.bool()?;
        self.supplies.mining_space_held = r.bool()?;
        self.combat.restore(r)?;
        self.overlay.restore(r)?;
        self.held.restore(r)
    }
}

/// A pointer sample, as the hooks keep them.
pub(crate) fn save_sample(w: &mut Writer, sample: MouseSample) {
    w.i32(sample.x);
    w.i32(sample.y);
    w.u8(sample.buttons);
}

/// See [`save_sample`].
pub(crate) fn restore_sample(r: &mut Reader) -> Result<MouseSample, StateError> {
    Ok(MouseSample { x: r.i32()?, y: r.i32()?, buttons: r.u8()? })
}
