//! The headless boot against the C++ oracle's golden trace.
//!
//! The held-movement scenario's first input arrives at 18.5 s, so until then
//! its golden trace is the game starting up on its own: the title, the
//! introduction and the town, with QoL on and no graphics selector.

use std::path::{Path, PathBuf};

use engine::Engine;

const GOLDEN: &str = include_str!("../../../tests/golden/held-movement.trace");
const FIRST_INPUT_MS: u64 = 18_500;

fn data() -> PathBuf {
    std::env::var_os("LDM_DATA").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/original"),
        PathBuf::from,
    )
}

fn milliseconds(line: &str) -> u64 {
    let field = line.split_whitespace().next().expect("a trace line");
    field.trim_start_matches("ms=").parse().expect("ms=<number>")
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn startup_matches_the_golden_trace_until_the_first_input() {
    let saves = std::env::temp_dir().join(format!("ldm-boot-{}", std::process::id()));
    std::fs::create_dir_all(&saves).expect("a saves folder");
    let (machine, game) = game::boot(data(), saves.clone(), true).expect("the game boots");
    let mut engine = Engine::new(machine, game);
    let startup: Vec<&str> =
        GOLDEN.lines().take_while(|line| milliseconds(line) < FIRST_INPUT_MS).collect();
    assert_eq!(startup.len(), 184, "one line per 100 ms before the first input");
    for expected in startup {
        while engine.quanta() < milliseconds(expected) {
            engine.step().expect("the game runs");
        }
        assert_eq!(engine.trace_line(), expected, "the first diverging trace line");
    }
    std::fs::remove_dir_all(saves).expect("the saves folder is removed");
}
