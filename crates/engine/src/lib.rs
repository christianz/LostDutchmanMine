//! The deterministic session: emulated time in one-millisecond quanta.
//!
//! Time is emulated, never measured. Each quantum applies its inputs, fires the
//! timer interrupts that fall due, advances the OPL chip, and runs a fixed budget
//! of translated steps. Identical inputs therefore reproduce identical runs, and
//! a trace line of the state hash identifies any moment. The engine knows
//! nothing of the game: a [`Program`] supplies its code and services.

mod program;
mod timer;

pub use program::{InputKind, Program, Stop};

use machine::Machine;

/// PIT input clocks per second.
pub const PIT_HZ: u64 = 1_193_182;
/// OPL input clocks per second.
pub const OPL_HZ: u64 = 3_579_545;
/// Translated steps each quantum may run.
pub const STEPS_PER_QUANTUM: u32 = 4096;

/// What the audio output needs from one quantum.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Audio {
    /// OPL port writes (offset, value), oldest first.
    pub opl_writes: Vec<(u16, u8)>,
    /// The PC speaker's frequency, or zero when silent.
    pub speaker_hz: u32,
}

/// A running session.
pub struct Engine<P> {
    machine: Machine,
    program: P,
    quanta: u64,
    pit_clocks: u64,
    inputs: Vec<InputKind>,
    speaker_hz: u32,
}

/// The clocks falling in quantum `q`, distributing `hz` exactly over milliseconds.
const fn clocks_in(q: u64, hz: u64) -> u64 {
    hz * (q + 1) / 1000 - hz * q / 1000
}

impl<P: Program> Engine<P> {
    /// A session about to run its first quantum.
    pub fn new(machine: Machine, program: P) -> Self {
        Engine { machine, program, quanta: 0, pit_clocks: 0, inputs: Vec::new(), speaker_hz: 0 }
    }

    /// Queues an input for the next quantum.
    pub fn input(&mut self, input: InputKind) {
        self.inputs.push(input);
    }

    /// Runs one emulated millisecond.
    ///
    /// # Errors
    ///
    /// [`Stop`] when the program cannot continue.
    pub fn step(&mut self) -> Result<(), Stop> {
        let q = self.quanta;
        let inputs = std::mem::take(&mut self.inputs);
        self.program.begin_quantum(&mut self.machine, q, &inputs);

        self.pit_clocks += clocks_in(q, PIT_HZ);
        while self.pit_clocks >= self.machine.pit.timer_period() {
            self.pit_clocks -= self.machine.pit.timer_period();
            timer::interrupt(&mut self.machine, &mut self.program)?;
        }
        self.machine.opl.advance(clocks_in(q, OPL_HZ) as u32);

        self.program.before_run(&mut self.machine);
        for _ in 0..STEPS_PER_QUANTUM {
            if !self.program.running() {
                break;
            }
            self.program.step(&mut self.machine)?;
            if self.program.waiting() {
                break;
            }
        }
        self.program.end_quantum(&mut self.machine);

        self.speaker_hz = speaker_hz(&self.machine);
        self.quanta += 1;
        Ok(())
    }

    /// Emulated milliseconds run so far.
    pub fn quanta(&self) -> u64 {
        self.quanta
    }

    /// Whether the program has exited.
    pub fn finished(&self) -> bool {
        !self.program.running()
    }

    /// The trace line for the current state.
    pub fn trace_line(&self) -> String {
        let (m, ticks) = (&self.machine, self.program.ticks());
        format!("ms={} blocks={} ticks={ticks} hash={:016x}", self.quanta, m.steps, m.state_hash())
    }

    /// The audio produced since the last call.
    pub fn take_audio(&mut self) -> Audio {
        Audio { opl_writes: self.machine.opl.take_writes(), speaker_hz: self.speaker_hz }
    }

    /// The machine.
    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    /// The machine, for setup and tests.
    pub fn machine_mut(&mut self) -> &mut Machine {
        &mut self.machine
    }

    /// The program.
    pub fn program(&self) -> &P {
        &self.program
    }

    /// The program, for setup and tests.
    pub fn program_mut(&mut self) -> &mut P {
        &mut self.program
    }

    /// Both, for callers that read one while changing the other.
    pub fn parts(&mut self) -> (&mut Machine, &mut P) {
        (&mut self.machine, &mut self.program)
    }
}

/// The speaker sounds when port 61h enables both the gate and the data line.
fn speaker_hz(m: &Machine) -> u32 {
    if m.ports.get(0x61) & 3 != 3 {
        return 0;
    }
    let divisor = u32::from(m.pit.speaker_divisor);
    PIT_HZ as u32 / if divisor == 0 { 65_536 } else { divisor }
}
