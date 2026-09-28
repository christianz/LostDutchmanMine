//! Savestates of a whole session: restoring, then continuing, is the same run.

use engine::{Engine, InputKind, Program, Stop};
use machine::Machine;
use machine::state::{Persist, Reader, StateError, Writer};

/// A program that folds every input and step into its state.
#[derive(Default)]
struct Accumulator {
    ticks: u32,
    total: u64,
}

impl Program for Accumulator {
    fn step(&mut self, m: &mut Machine) -> Result<(), Stop> {
        m.steps += 1;
        self.total = self.total.wrapping_mul(31).wrapping_add(u64::from(m.regs.ax));
        m.regs.ax = m.regs.ax.wrapping_add(7);
        Ok(())
    }
    fn bios_tick(&mut self, m: &mut Machine) {
        self.ticks += 1;
        m.memory.write16(0x40, 0x6c, self.ticks as u16);
    }
    fn begin_quantum(&mut self, m: &mut Machine, now_ms: u64, inputs: &[InputKind]) {
        for input in inputs {
            if let InputKind::Key(key) = input {
                m.regs.bx = m.regs.bx.wrapping_add(*key as u16);
            }
        }
        self.total ^= now_ms;
    }
    fn before_run(&mut self, _m: &mut Machine) {}
    fn running(&self) -> bool {
        true
    }
    fn waiting(&self) -> bool {
        false
    }
    fn end_quantum(&mut self, _m: &mut Machine) {}
    fn ticks(&self) -> u32 {
        self.ticks
    }
}

impl Persist for Accumulator {
    fn save(&self, w: &mut Writer) {
        w.u32(self.ticks);
        w.u64(self.total);
    }
    fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        self.ticks = r.u32()?;
        self.total = r.u64()?;
        Ok(())
    }
}

fn engine() -> Engine<Accumulator> {
    let mut m = Machine::new();
    m.memory.write16(0, 0x22, 0xf000);
    Engine::new(m, Accumulator::default())
}

#[test]
fn a_restored_session_continues_exactly() {
    let mut original = engine();
    for q in 0..700 {
        if q % 97 == 0 {
            original.input(InputKind::Key(q as u32));
        }
        original.step().expect("quantum");
    }
    original.input(InputKind::Key(5));
    original.input(InputKind::Mouse { x: 3, y: 4 });
    let state = original.save();
    let mut copy = engine();
    copy.restore(&state).expect("a valid state");
    assert_eq!(copy.trace_line(), original.trace_line());
    for _ in 0..700 {
        original.step().expect("quantum");
        copy.step().expect("quantum");
        assert_eq!(copy.trace_line(), original.trace_line());
    }
    assert_eq!(copy.program().total, original.program().total);
}

#[test]
fn a_damaged_state_is_refused_before_the_machine_changes() {
    let mut original = engine();
    original.step().expect("quantum");
    let state = original.save();
    let mut copy = engine();
    let before = copy.trace_line();
    assert_eq!(copy.restore(&state[..state.len() / 2]), Err(StateError::Truncated));
    assert_eq!(copy.trace_line(), before, "the machine and clocks are untouched");
}
