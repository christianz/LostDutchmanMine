//! `tests/verify-movement.py`: VGA starts without a key, a held arrow keeps
//! walking without repeat events, and release, focus loss, opposing keys and
//! stale repeats all stop it.

use game::Report;

use super::fields::position;
use super::run::{Run, ids};

/// The BIOS mode of 320x200 VGA.
const VGA: u8 = 0x13;

/// Checks the `held-movement` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let frames: [&Report; 12] = run.reports(ids(200))?;
    let [_, f201, f202, f203, f204, f205, f206, f207, f208, f209, f210, f211] = frames;
    ensure!(frames.iter().all(|f| f.video_mode == VGA), "VGA must start without a key");
    ensure!(
        f201.x < f202.x && f202.x < f203.x && f203.x < f204.x,
        "Held right must keep moving without repeat events"
    );
    ensure!(
        f202.directions == 8 && f203.directions == 8,
        "captures 202 and 203 hold directions {} and {}, expected 8",
        f202.directions,
        f203.directions
    );
    ensure!(position(f204) == position(f205), "Movement must stop on key release");
    ensure!(f206.x < f205.x, "Held left must move left");
    ensure!(position(f206) == position(f207), "Movement must stop on focus loss");
    ensure!(position(f208) == position(f209), "Opposing held directions must settle to neutral");
    ensure!(position(f210) == position(f211), "Releasing a key must discard its pending repeats");
    for (n, f) in (204..).zip(&frames[4..]) {
        ensure!(f.directions == 0, "capture {n} holds directions {}, expected none", f.directions);
    }
    Ok(())
}
