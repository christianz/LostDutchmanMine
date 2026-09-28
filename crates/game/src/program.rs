//! The game as the engine runs it: translated steps, and desktop input applied
//! at the start of each quantum exactly as the C++ oracle's session did.

use dos::{KEY_REPEAT, bios_key, repeat_from};
use engine::{InputKind, Program, Stop};
use machine::Machine;
use machine::state::{Reader, StateError, Writer};

use crate::symbols::set_joystick;
use crate::{Game, translated};

/// The Space scan: with QoL its auto-repeats are dropped, since holding Space
/// is a held state of its own.
const SPACE_SCAN: u16 = 0x39;

/// Input held across quanta.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Held {
    /// The movement directions the desktop holds (1 up, 2 down, 4 left, 8 right).
    pub(crate) directions: u8,
    /// Whether panning was under way when this quantum began: input made while
    /// panning never reaches the game, and changing either way drops it.
    was_panning: bool,
}

impl Held {
    pub(crate) fn save(self, w: &mut Writer) {
        w.u8(self.directions);
        w.bool(self.was_panning);
    }

    pub(crate) fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        (self.directions, self.was_panning) = (r.u8()?, r.bool()?);
        Ok(())
    }
}

impl Game {
    fn apply(&mut self, input: InputKind) {
        let panning = self.held.was_panning;
        match input {
            InputKind::Key(key) => {
                let space_repeat = key & KEY_REPEAT != 0 && bios_key(key, false) >> 8 == SPACE_SCAN;
                let dropped = panning || (self.qol && space_repeat);
                if !dropped {
                    self.dos.keyboard.push(key);
                }
            }
            InputKind::Release(source) => {
                self.dos.keyboard.retain(|&key| !repeat_from(key, source));
            }
            InputKind::Directions(directions) => self.held.directions = directions,
            InputKind::Space(held) => self.supplies.mining_space_held = !panning && held,
            InputKind::Mouse { x, y } => self.dos.mouse.input.move_to(x, y),
            InputKind::Buttons(buttons) => {
                if !panning {
                    self.dos.mouse.input.buttons(buttons);
                }
            }
            InputKind::Clear => {
                self.held.directions = 0;
                self.supplies.mining_space_held = false;
                self.dos.keyboard.clear();
                self.dos.mouse.input.clear();
                self.reset_pointers();
            }
            InputKind::Qol(on) => self.set_qol(on),
        }
    }
}

impl Program for Game {
    fn step(&mut self, m: &mut Machine) -> Result<(), Stop> {
        translated::step(m, self)
    }

    fn bios_tick(&mut self, m: &mut Machine) {
        self.dos.clock.tick(m);
    }

    fn begin_quantum(&mut self, _m: &mut Machine, now_ms: u64, inputs: &[InputKind]) {
        self.dos.clock.emulated_ms = now_ms;
        self.dos.mouse.input.set_time(now_ms);
        self.held.was_panning = self.supplies.panning;
        for &input in inputs {
            self.apply(input);
        }
    }

    fn before_run(&mut self, m: &mut Machine) {
        let directions = if self.held.was_panning { 0 } else { self.held.directions };
        set_joystick(m, directions);
    }

    fn running(&self) -> bool {
        self.dos.running
    }

    fn waiting(&self) -> bool {
        self.dos.waiting
    }

    fn end_quantum(&mut self, m: &mut Machine) {
        if self.held.was_panning != self.supplies.panning {
            self.held.directions = 0;
            self.supplies.mining_space_held = false;
            set_joystick(m, 0);
            self.dos.keyboard.clear();
            self.dos.mouse.input.clear();
        }
    }

    fn ticks(&self) -> u32 {
        self.dos.clock.ticks
    }
}
