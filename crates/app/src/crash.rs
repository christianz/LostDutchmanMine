//! Crash reports: what a player can send, and what reproduces the stop.
//!
//! A report holds the error, the build, the last steps, a replay of every
//! input since launch and a savestate at the stop. The savestate holds game
//! memory, so reports stay on the player's machine unless they choose to
//! share them.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use engine::{Engine, Stop};
use game::Game;

/// Writes a report into a new folder under `crashes` and returns it.
///
/// # Errors
///
/// When the folder or its files cannot be written.
pub fn write(
    crashes: &Path,
    engine: &Engine<Game>,
    stop: &Stop,
    header: &[(String, String)],
) -> Result<PathBuf> {
    let folder = crashes.join(chrono::Local::now().format("%Y-%m-%d-%H%M%S").to_string());
    std::fs::create_dir_all(&folder)?;
    let mut report = format!(
        "{stop}\n\nbuild {}\n{}\n\nlast steps, oldest first:\n",
        game::build_id(),
        engine.trace_line()
    );
    for address in engine.recent_steps() {
        let _ = writeln!(report, "  {address}");
    }
    std::fs::write(folder.join("report.txt"), report)?;
    let mut replay = engine.replay().unwrap_or_default();
    replay.header = header.to_vec();
    std::fs::write(folder.join("replay.txt"), replay.to_text())?;
    std::fs::write(folder.join("state.ldmstate"), engine.save())?;
    Ok(folder)
}

#[cfg(test)]
mod tests {
    use engine::{Engine, InputKind, Replay};
    use game::Game;
    use machine::Machine;

    #[test]
    fn a_report_explains_and_reproduces_the_stop() {
        let root = std::env::temp_dir().join(format!("ldm-crash-{}", std::process::id()));
        let game = || Game::new(root.join("data"), root.join("saves"), true);
        let mut engine = Engine::new(Machine::new(), game());
        engine.start_recording();
        engine.input(InputKind::Key(0x1e61));
        let stop = engine.step().expect_err("no code at CS:IP");
        let header = vec![("build".to_owned(), game::build_id())];
        let folder =
            super::write(&root.join("Crashes"), &engine, &stop, &header).expect("a report");

        let report = std::fs::read_to_string(folder.join("report.txt")).expect("report.txt");
        assert!(report.starts_with(&stop.to_string()) && report.contains(&game::build_id()));
        assert!(report.contains("last steps, oldest first:\n  f000:0000"), "{report}");
        let replay =
            Replay::parse(&std::fs::read_to_string(folder.join("replay.txt")).expect("replay.txt"))
                .expect("a replay");
        assert_eq!((replay.header, replay.inputs), (header, vec![(0, InputKind::Key(0x1e61))]));
        let state = std::fs::read(folder.join("state.ldmstate")).expect("state.ldmstate");
        let mut restored = Engine::new(Machine::new(), game());
        restored.restore(&state).expect("a savestate");
        assert_eq!(restored.trace_line(), engine.trace_line());
        assert_eq!(restored.program().dos.keyboard.front(), Some(0x1e61), "the queued key");
        std::fs::remove_dir_all(root).expect("the temporary folder");
    }
}
