//! Lost Dutchman Mine: the translated original, and everything the port adds.
//!
//! The game's own code is translated from the player's LDM.EXE at build time
//! (see `build.rs`). Around it this crate names every address the port touches
//! ([`symbols`]), implements the hooks the patch table places in that code, draws
//! the quality-of-life overlay and composes frames. [`Game`] is an
//! [`engine::Program`], so the engine runs it without knowing any of this.

// Only translated code calls the hooks; a build without LDM_EXE has none.
#![cfg_attr(not(ldm_translated), allow(dead_code, unused_imports))]

mod assets;
mod boot;
mod decompiled;
mod frame;
mod hooks;
mod overlay;
mod pixels;
mod program;
mod report;
mod routines;
mod state;
pub mod symbols;
mod translated;

use std::path::PathBuf;

use dos::Dos;

pub use assets::{AssetError, decode_asset};
pub use boot::{BootError, boot};
pub use frame::Frame;
pub use report::Report;
pub use routines::RoutineMode;

/// The game's state beside the machine: DOS, and what the port adds.
#[derive(Debug)]
pub struct Game {
    /// DOS and the BIOS.
    pub dos: Dos,
    /// Whether the quality-of-life improvements are on.
    pub(crate) qol: bool,
    /// The walking pointer and its latched click.
    pub(crate) pointer: hooks::WorldPointer,
    /// Fast walking.
    pub(crate) walk: hooks::Walk,
    /// Whether a desert close-up waits for its dismissal.
    pub(crate) desert_view: bool,
    /// Panning and mining.
    pub(crate) supplies: hooks::Supplies,
    /// The encounter's mouse aiming.
    pub(crate) combat: hooks::Combat,
    /// The QoL toolbar and hover targets.
    pub(crate) overlay: overlay::Overlay,
    /// Input held across quanta.
    pub(crate) held: program::Held,
    /// Which implementation of the readable routines runs.
    pub(crate) routines: routines::Routines,
}

impl Game {
    /// A game reading the original files from `data` and saving to `saves`.
    pub fn new(data: PathBuf, saves: PathBuf, qol: bool) -> Self {
        let mut game = Game {
            dos: Dos::new(data, saves),
            qol,
            pointer: hooks::WorldPointer::default(),
            walk: hooks::Walk::default(),
            desert_view: false,
            supplies: hooks::Supplies::default(),
            combat: hooks::Combat::default(),
            overlay: overlay::Overlay::default(),
            held: program::Held::default(),
            routines: routines::Routines::default(),
        };
        game.set_qol(qol);
        game
    }

    /// Whether the quality-of-life improvements are on.
    pub fn qol(&self) -> bool {
        self.qol
    }

    /// Chooses the translated, readable or lockstep-checked routines.
    pub fn set_routine_mode(&mut self, mode: RoutineMode) {
        self.routines.mode = mode;
    }

    /// Readable routine calls checked in lockstep so far.
    pub fn routine_checks(&self) -> u64 {
        self.routines.checked
    }

    /// What lockstep found to differ, the first calls first.
    pub fn routine_mismatches(&self) -> &[String] {
        &self.routines.mismatches
    }

    /// Turns the quality-of-life improvements on or off. With them the desktop
    /// pointer persists, so the original never parks it.
    pub(crate) fn set_qol(&mut self, on: bool) {
        self.qol = on;
        self.dos.park_cursor = !on;
        self.reset_pointers();
    }

    /// Baselines both pointer filters on the pointer as it is now.
    pub(crate) fn reset_pointers(&mut self) {
        let current = self.dos.mouse.input.current();
        self.combat.reset_pointer(current);
        self.pointer.reset(current);
    }
}
