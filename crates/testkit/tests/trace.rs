//! The C++ `trace` test: two deterministic sessions from the same start write
//! identical traces, one line per 100 emulated milliseconds. That the trace
//! matches the C++ oracle's is `crates/game/tests/boot.rs`, and that the hash
//! covers registers, memory and palette is `crates/machine/tests/hash.rs`.

use engine::Engine;
use testkit::harness::data;

/// The session's length, in emulated milliseconds.
const SESSION_MS: u64 = 3000;
/// A trace line is written every this many emulated milliseconds.
const TRACE_INTERVAL: u64 = 100;

/// The trace of a fresh game's first three seconds, saving to `saves`.
fn trace(saves: &str) -> Vec<String> {
    let saves = std::env::temp_dir().join(format!("ldm-trace-{}-{saves}", std::process::id()));
    if saves.exists() {
        std::fs::remove_dir_all(&saves).expect("a stale saves folder can be removed");
    }
    std::fs::create_dir_all(&saves).expect("a fresh saves folder");
    let (machine, game) = game::boot(data(), saves.clone(), true).expect("the game boots");
    let mut engine = Engine::new(machine, game);
    let mut lines = Vec::new();
    while engine.quanta() < SESSION_MS {
        engine.step().expect("the game runs");
        if engine.quanta() % TRACE_INTERVAL == 0 {
            lines.push(engine.trace_line());
        }
    }
    std::fs::remove_dir_all(saves).expect("the saves folder is removed");
    lines
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn deterministic_sessions_write_identical_traces() {
    let (a, b) = (trace("a"), trace("b"));
    assert!(
        a.first().is_some_and(|line| line.starts_with("ms=100 blocks=")),
        "The first trace line must describe the first 100 ms"
    );
    assert_eq!(a.len(), 30, "3,000 ms must produce one line per 100 ms");
    assert_eq!(a, b, "Two deterministic runs must produce identical traces");
}
