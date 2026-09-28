//! `tests/verify-qol.py`: with QoL, held walking goes faster while the survival
//! clock keeps its pace, release and focus loss stop it, and the pointer shows.
//!
//! Left out, because no run the manifest makes can meet them:
//!
//! - the loaded saloon's Exit and walking (captures 1100 to 1103): its
//!   `qol-saloon` scenario needs a copy of a player's own save that no fixture
//!   generator builds, so the manifest does not have it;
//! - the desert close-up (captures 1200 to 1204): the verifier's save stood
//!   the player on the map at (40, 55), but `qol-desert` loads the `map`
//!   fixture at (135, 61) beside a cave, so Space enters the cave. The C++
//!   oracle's captures of that run fail both expectations alike.

use game::Report;

use super::fields::position;
use super::run::{Run, ids};

/// Checks the `qol-cadence-classic` and `qol-cadence-qol` runs.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(classic: &Run, qol: &Run) -> Result<(), String> {
    let mut distance = [0; 2];
    let mut clocks = [0; 2];
    for (i, (run, with_qol)) in [(classic, false), (qol, true)].into_iter().enumerate() {
        let frames: [&Report; 6] = run.reports(ids(1300))?;
        distance[i] = i32::from(frames[1].x) - i32::from(frames[0].x);
        clocks[i] = i32::from(frames[2].survival_ticks) - i32::from(frames[0].survival_ticks);
        ensure!(position(frames[2]) == position(frames[3]), "Release did not stop walking");
        ensure!(position(frames[4]) == position(frames[5]), "Focus loss did not stop walking");
        ensure!(frames.iter().all(|s| s.pointer_visible == with_qol), "{frames:?}");
    }
    // More than one and a half times as far, in whole pixels.
    ensure!(distance[1] * 2 > distance[0] * 3, "{distance:?}");
    ensure!((clocks[1] - clocks[0]).abs() <= 1, "{clocks:?}");
    Ok(())
}
