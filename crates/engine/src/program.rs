//! What the engine needs from a program, and why a run can stop.

use machine::{Address, Fault, Machine};

/// Desktop input, as the simulation receives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    /// A key with its BIOS pair and native tags (see the `dos` keyboard).
    Key(u32),
    /// A physical key was released: drop its queued auto-repeats.
    Release(u32),
    /// The held movement directions (1 up, 2 down, 4 left, 8 right).
    Directions(u8),
    /// Space is held or released.
    Space(bool),
    /// The pointer moved, in 320x200 game pixels.
    Mouse {
        /// Horizontal position.
        x: i32,
        /// Vertical position.
        y: i32,
    },
    /// The held mouse buttons (bit 0 left, bit 1 right).
    Buttons(u8),
    /// Focus was lost: release everything held.
    Clear,
    /// The QoL setting changed.
    Qol(bool),
}

/// Why execution stopped.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Stop {
    /// A CPU fault in the translated code.
    #[error("{fault} at {at}")]
    Fault {
        /// The fault.
        fault: Fault,
        /// Where.
        at: Address,
    },
    /// Execution reached code the translator did not recover.
    #[error("no translated code starts at {0}")]
    Unrecovered(Address),
    /// The game's timer handler did not return.
    #[error("the timer handler did not return")]
    TimerHandler,
    /// A service or hook failed.
    #[error("{0}")]
    Program(String),
}

/// A program the engine runs: the translated game and its DOS, or a test double.
pub trait Program {
    /// Runs translated code from CS:IP until it yields.
    ///
    /// # Errors
    ///
    /// [`Stop`] when execution cannot continue.
    fn step(&mut self, m: &mut Machine) -> Result<(), Stop>;

    /// The BIOS's own timer handler: one tick of the BIOS clock.
    fn bios_tick(&mut self, m: &mut Machine);

    /// A quantum begins at `now_ms` emulated milliseconds, with its inputs.
    fn begin_quantum(&mut self, m: &mut Machine, now_ms: u64, inputs: &[InputKind]);

    /// Timer interrupts have run; the CPU is about to.
    fn before_run(&mut self, m: &mut Machine);

    /// False once the program has exited.
    fn running(&self) -> bool;

    /// True while the program waits for input it has not received.
    fn waiting(&self) -> bool;

    /// The CPU has run for this quantum.
    fn end_quantum(&mut self, m: &mut Machine);

    /// The BIOS tick count, for traces.
    fn ticks(&self) -> u32;
}
