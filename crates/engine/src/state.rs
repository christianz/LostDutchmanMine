//! A session in a savestate: the engine's clocks and pending input, then the
//! machine, then the program.

use machine::Machine;
use machine::state::{Persist, Reader, StateError, Writer};

use crate::{Engine, InputKind, Program};

impl<P: Program + Persist> Engine<P> {
    /// The whole session as a savestate.
    pub fn save(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.section(b"ENGN");
        w.u64(self.quanta);
        w.u64(self.pit_clocks);
        w.u32(self.speaker_hz);
        w.u32(self.inputs.len() as u32);
        for &input in &self.inputs {
            save_input(&mut w, input);
        }
        self.machine.save(&mut w);
        self.program.save(&mut w);
        w.finish()
    }

    /// Restores a state made by [`Engine::save`]; continuing then runs exactly
    /// as the saved session would have.
    ///
    /// # Errors
    ///
    /// [`StateError`] for a damaged or foreign state. The engine's clocks and
    /// the machine change only once the whole state has been read, but the
    /// program restores in place, so after an error discard the session.
    pub fn restore(&mut self, state: &[u8]) -> Result<(), StateError> {
        let mut r = Reader::new(state)?;
        r.section(b"ENGN")?;
        let (quanta, pit_clocks, speaker_hz) = (r.u64()?, r.u64()?, r.u32()?);
        let mut inputs = Vec::new();
        for _ in 0..r.u32()? {
            inputs.push(restore_input(&mut r)?);
        }
        let mut machine = Machine::new();
        machine.restore(&mut r)?;
        self.program.restore(&mut r)?;
        r.finish()?;
        (self.quanta, self.pit_clocks, self.speaker_hz) = (quanta, pit_clocks, speaker_hz);
        self.inputs = inputs;
        self.machine = machine;
        Ok(())
    }
}

fn save_input(w: &mut Writer, input: InputKind) {
    match input {
        InputKind::Key(key) => {
            w.u8(0);
            w.u32(key);
        }
        InputKind::Release(source) => {
            w.u8(1);
            w.u32(source);
        }
        InputKind::Directions(directions) => {
            w.u8(2);
            w.u8(directions);
        }
        InputKind::Space(held) => {
            w.u8(3);
            w.bool(held);
        }
        InputKind::Mouse { x, y } => {
            w.u8(4);
            w.i32(x);
            w.i32(y);
        }
        InputKind::Buttons(buttons) => {
            w.u8(5);
            w.u8(buttons);
        }
        InputKind::Clear => w.u8(6),
        InputKind::Qol(on) => {
            w.u8(7);
            w.bool(on);
        }
    }
}

fn restore_input(r: &mut Reader) -> Result<InputKind, StateError> {
    Ok(match r.u8()? {
        0 => InputKind::Key(r.u32()?),
        1 => InputKind::Release(r.u32()?),
        2 => InputKind::Directions(r.u8()?),
        3 => InputKind::Space(r.bool()?),
        4 => InputKind::Mouse { x: r.i32()?, y: r.i32()? },
        5 => InputKind::Buttons(r.u8()?),
        6 => InputKind::Clear,
        7 => InputKind::Qol(r.bool()?),
        other => return Err(StateError::Invalid(format!("input kind {other}"))),
    })
}
