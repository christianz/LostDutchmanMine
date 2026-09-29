//! Replays and the record of recent steps: what a crash report is made of.

use engine::{Engine, InputKind, Program, Replay, Stop};
use machine::{Address, Machine};

/// A program that walks IP forward one step at a time, and reads its inputs.
#[derive(Default)]
struct Walker {
    seen: u32,
}

impl Program for Walker {
    fn step(&mut self, m: &mut Machine) -> Result<(), Stop> {
        m.steps += 1;
        m.regs.ip = m.regs.ip.wrapping_add(1);
        m.regs.ax = m.regs.ax.wrapping_add(self.seen as u16);
        Ok(())
    }
    fn bios_tick(&mut self, _m: &mut Machine) {}
    fn begin_quantum(&mut self, _m: &mut Machine, _now_ms: u64, inputs: &[InputKind]) {
        self.seen = self.seen.wrapping_add(inputs.len() as u32 * 7);
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
        0
    }
}

fn engine() -> Engine<Walker> {
    let mut m = Machine::new();
    m.memory.write16(0, 0x22, 0xf000);
    (m.regs.cs, m.regs.ip) = (0x1000, 0);
    Engine::new(m, Walker::default())
}

#[test]
fn the_last_64_steps_are_remembered_oldest_first() {
    let mut e = engine();
    e.step().expect("quantum");
    let recent = e.recent_steps();
    assert_eq!(recent.len(), 64);
    assert_eq!(recent.first(), Some(&Address::new(0, 4096 - 64)));
    assert_eq!(recent.last(), Some(&Address::new(0, 4095)));
}

#[test]
fn a_recorded_session_replays_to_the_same_state() {
    let inputs = [
        (0, InputKind::Key(0x3920 | 1 << 16)),
        (0, InputKind::Mouse { x: -3, y: 250 }),
        (3, InputKind::Directions(9)),
        (3, InputKind::Space(true)),
        (5, InputKind::Release(26)),
        (7, InputKind::Buttons(3)),
        (7, InputKind::Clear),
        (9, InputKind::Qol(false)),
    ];
    let mut original = engine();
    original.start_recording();
    for (ms, input) in inputs {
        while original.quanta() < ms {
            original.step().expect("quantum");
        }
        original.input(input);
    }
    for _ in 0..3 {
        original.step().expect("quantum");
    }
    let mut replay = original.replay().expect("recording");
    replay.header.push(("build".to_owned(), "test".to_owned()));
    let text = replay.to_text();
    let parsed = Replay::parse(&text).expect("a replay");
    assert_eq!(parsed, replay);

    let mut again = engine();
    let mut pending = parsed.inputs.iter().peekable();
    while again.quanta() < original.quanta() {
        while let Some(&&(_, input)) = pending.peek().filter(|(ms, _)| *ms == again.quanta()) {
            again.input(input);
            pending.next();
        }
        again.step().expect("quantum");
    }
    assert_eq!(again.trace_line(), original.trace_line());
}

#[test]
fn a_damaged_replay_names_its_line() {
    let error = Replay::parse("# build x\n12 key 5\n13 jump 4\n").expect_err("unknown input");
    assert_eq!(error.line, 3);
}
