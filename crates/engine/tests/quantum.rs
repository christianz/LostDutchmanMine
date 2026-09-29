//! The engine's quantum with a synthetic program: time, timers, inputs, traces.

use engine::{Engine, InputKind, Program, Stop};
use machine::Machine;

/// A program whose every step counts, and whose timer handler lives at 2000:0000.
#[derive(Default)]
struct Counter {
    steps: u32,
    ticks: u32,
    handler_runs: u32,
    inputs: Vec<(u64, InputKind)>,
    now_ms: u64,
    wait_after: Option<u32>,
    before_runs: u32,
    after_runs: u32,
}

const HANDLER: (u16, u16) = (0x2000, 0x0000);

impl Program for Counter {
    fn step(&mut self, m: &mut Machine) -> Result<(), Stop> {
        m.steps += 1;
        if (m.regs.cs, m.regs.ip) == HANDLER {
            // The game's handler does its work, then chains to the BIOS.
            self.handler_runs += 1;
            (m.regs.cs, m.regs.ip) = (0xf000, 0x0020);
        } else {
            self.steps += 1;
        }
        Ok(())
    }
    fn bios_tick(&mut self, _m: &mut Machine) {
        self.ticks += 1;
    }
    fn begin_quantum(&mut self, _m: &mut Machine, now_ms: u64, inputs: &[InputKind]) {
        self.now_ms = now_ms;
        self.inputs.extend(inputs.iter().map(|&input| (now_ms, input)));
    }
    fn before_run(&mut self, _m: &mut Machine) {
        self.before_runs += 1;
    }
    fn running(&self) -> bool {
        true
    }
    fn waiting(&self) -> bool {
        self.wait_after.is_some_and(|limit| self.steps >= limit)
    }
    fn end_quantum(&mut self, _m: &mut Machine) {
        self.after_runs += 1;
    }
    fn ticks(&self) -> u32 {
        self.ticks
    }
}

fn engine() -> Engine<Counter> {
    let mut m = Machine::new();
    (m.regs.cs, m.regs.ip) = (0x1000, 0x0100);
    // INT 08h points into the BIOS, as DOS leaves it.
    m.memory.write16(0, 0x20, 0x0020);
    m.memory.write16(0, 0x22, 0xf000);
    Engine::new(m, Counter::default())
}

#[test]
fn one_emulated_second_fires_the_bios_timer_eighteen_times() {
    let mut engine = engine();
    for _ in 0..1000 {
        engine.step().expect("quantum");
    }
    assert_eq!(engine.quanta(), 1000);
    assert_eq!(engine.program().ticks, 1_193_182 / 65_536, "the default divisor is 65536");
    assert_eq!(engine.program().steps, 4096 * 1000, "a full budget each quantum");
}

#[test]
fn a_programmed_divisor_sets_the_timer_rate() {
    let mut engine = engine();
    engine.machine_mut().port_out(0x43, 0x36, machine::Width::Byte);
    engine.machine_mut().port_out(0x40, 0x00, machine::Width::Byte);
    engine.machine_mut().port_out(0x40, 0x20, machine::Width::Byte); // 0x2000 = 8192
    for _ in 0..1000 {
        engine.step().expect("quantum");
    }
    assert_eq!(engine.program().ticks, 1_193_182 / 8192);
}

#[test]
fn the_games_timer_handler_runs_and_execution_resumes() {
    let mut engine = engine();
    let m = engine.machine_mut();
    m.memory.write16(0, 0x20, HANDLER.1);
    m.memory.write16(0, 0x22, HANDLER.0);
    for _ in 0..1000 {
        engine.step().expect("quantum");
    }
    let (runs, ticks) = (engine.program().handler_runs, engine.program().ticks);
    assert_eq!(
        (runs, ticks),
        (18, 18),
        "each interrupt runs the handler, which chains to the BIOS"
    );
    let r = &engine.machine().regs;
    assert_eq!((r.cs, r.ip), (0x1000, 0x0100), "the interrupted code resumes");
}

#[test]
fn inputs_arrive_in_the_quantum_they_are_stamped_for() {
    let mut engine = engine();
    engine.step().expect("quantum");
    engine.input(InputKind::Space(true));
    engine.input(InputKind::Mouse { x: 10, y: 20 });
    engine.step().expect("quantum");
    engine.step().expect("quantum");
    let inputs = &engine.program().inputs;
    assert_eq!(inputs, &[(1, InputKind::Space(true)), (1, InputKind::Mouse { x: 10, y: 20 })]);
}

#[test]
fn a_waiting_program_ends_its_quantum_early() {
    let mut engine = engine();
    engine.program_mut().wait_after = Some(10);
    engine.step().expect("quantum");
    assert_eq!(engine.program().steps, 10);
    assert_eq!((engine.program().before_runs, engine.program().after_runs), (1, 1));
}

#[test]
fn trace_lines_name_time_steps_ticks_and_the_state_hash() {
    let mut engine = engine();
    engine.step().expect("quantum");
    let hash = engine.machine().state_hash();
    assert_eq!(engine.trace_line(), format!("ms=1 blocks=4096 ticks=0 hash={hash:016x}"));
}
