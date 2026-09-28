//! `tests/verify-mouse.py`: the pointer stays visible while walking, one quick
//! toolbar click opens F6, and one quick click on Quit Game ends the game.
//!
//! The verifier launched the C++ desktop itself, with a 26-second timeout on a
//! 28-second run, and read its closing log line. Here the run's [`Ending`]
//! stands in for both, in emulated time.
//!
//! [`Ending`]: crate::scenario::Ending

use super::fields::POSITION;
use super::run::Run;

/// The verifier's timeout, in milliseconds: the game must quit before it.
const TIMEOUT_MS: u64 = 26_000;
/// The BIOS text mode the game restores on quitting.
const TEXT_MODE: u8 = 3;

/// Checks the `mouse-clicks` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let ending = &run.outcome.ending;
    ensure!(
        ending.exited && ending.ms < TIMEOUT_MS,
        "the game did not exit before {TIMEOUT_MS} ms: {ending:?}"
    );
    ensure!(ending.video_mode == TEXT_MODE && ending.pit_divisor == 0, "{ending:?}");
    let states = run.reports([400, 401, 402])?;
    let [first, ..] = states;
    ensure!(first.pointer_visible && first.mouse_mode == 1, "{first:?}");
    for state in &states[1..] {
        ensure!(state.pointer_visible, "{state:?}");
        for field in POSITION {
            ensure!(field.of(state) == field.of(first), "{}: {states:?}", field.key);
        }
    }
    Ok(())
}
