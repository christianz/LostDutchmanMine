//! `tests/verify-quit.py`: F6 and the original Quit Game button end the game
//! normally, restoring text mode, well before the run's time limit.
//!
//! The verifier launched the C++ desktop itself for a 30-second run under a
//! 27-second timeout, timed it by the wall clock, and searched its log for the
//! game's switch to text mode and the closing line's final mode. Here the
//! run's [`Ending`] stands in for all three, in emulated time.
//!
//! [`Ending`]: crate::scenario::Ending

use super::run::Run;

/// When the game may exit: not before the script's Quit click has been
/// played out, and before the verifier's timeout.
const EXIT_MS: std::ops::Range<u64> = 21_000..27_000;
/// The BIOS text mode the game restores on quitting.
const TEXT_MODE: u8 = 3;

/// Checks the `quit-game` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let ending = &run.outcome.ending;
    ensure!(ending.exited, "the game did not exit by itself: {ending:?}");
    ensure!(
        ending.video_modes.contains(&TEXT_MODE) && ending.video_mode == TEXT_MODE,
        "{ending:?}"
    );
    ensure!(EXIT_MS.contains(&ending.ms), "{}", ending.ms);
    Ok(())
}
