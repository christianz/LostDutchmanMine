//! `tests/verify-menu-input.py`: dialogs hide the river's hover outlines and
//! closing them restores the outlines, diagonals keep both axes while held,
//! and release and focus loss stop walking.
//!
//! Left out, because no run the manifest makes can meet them; the C++
//! oracle's captures of the same runs fail them alike:
//!
//! - every reproduction on the build before the fixes, which cannot be run;
//! - the SA and SD diagonals: on the deterministic run a random encounter
//!   takes the player off the map at capture 1509;
//! - that the cadence captures are in the saloon: `saloon-cadence` loads save
//!   slot 6 but has no fixture to provide it, so the player stays in the
//!   street, where the release and focus-loss expectations still hold;
//! - the saloon's comparisons of walking distance and survival ticks, which
//!   weigh the fixed build against that earlier build and against a QoL-off
//!   run of `saloon-cadence.txt`, a scenario the manifest does not have.

use game::{Frame, Report};

use super::fields::point;
use super::run::{Run, ids};

/// The hover outline's gold, as a capture pixel.
const GOLD: u32 = 0xffff_d34e;
/// The held-direction bits for left and right, the axis still held once the
/// script releases W or S.
const SIDEWAYS: u8 = 0b1100;

/// Checks the `menu-hover`, `map-diagonals` and `saloon-cadence` runs.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(hover: &Run, diagonals: &Run, saloon: &Run) -> Result<(), String> {
    hover_outlines(hover)?;
    held_diagonals(diagonals)?;
    saloon_stops(saloon)
}

/// Pixels of the outline's gold.
fn gold(frame: &Frame) -> usize {
    frame.pixels().iter().filter(|&&pixel| pixel == GOLD).count()
}

/// Health and inventory open over the river, then closed.
fn hover_outlines(run: &Run) -> Result<(), String> {
    for n in [1402, 1403, 1406, 1408] {
        ensure!(gold(&run.capture(n)?.frame) == 0, "Hidden button still highlights: {n}");
    }
    for n in [1400, 1404, 1407, 1409] {
        ensure!(gold(&run.capture(n)?.frame) > 0, "River hover was not restored: {n}");
    }
    Ok(())
}

/// Each diagonal's pair of keys, held, then one released, then both.
fn held_diagonals(run: &Run) -> Result<(), String> {
    let diagonals = [("WA", 5, -1, -1), ("WD", 9, 1, -1)];
    for ((label, mask, dx, dy), n) in diagonals.into_iter().zip((1500..).step_by(4)) {
        let new: [&Report; 4] = run.reports(ids(n))?;
        ensure!(new.iter().all(|s| s.map_view), "Left map unexpectedly: {label}: {new:?}");
        ensure!(
            new[0].directions == mask && new[1].directions == mask,
            "{label}: directions {} and {}, expected {mask}",
            new[0].directions,
            new[1].directions
        );
        let moved = |axis: fn(&Report) -> u16| i32::from(axis(new[1])) - i32::from(axis(new[0]));
        ensure!(moved(|s| s.x) * dx > 0 && moved(|s| s.y) * dy > 0, "{label}: {new:?}");
        let sideways = mask & SIDEWAYS;
        ensure!(
            new[2].directions == sideways && new[3].directions == 0,
            "{label}: directions {} and {} after release, expected {sideways} and 0",
            new[2].directions,
            new[3].directions
        );
        ensure!(point(new[2]) == point(new[3]), "Release did not stop movement: {label}: {new:?}");
    }
    let [a, b] = run.reports([1520, 1521])?;
    ensure!(
        a.directions == 0 && b.directions == 0 && point(a) == point(b),
        "Focus loss retained movement"
    );
    Ok(())
}

/// Walking, released, then walking again and losing focus.
fn saloon_stops(run: &Run) -> Result<(), String> {
    let states: [&Report; 6] = run.reports(ids(1600))?;
    ensure!(point(states[2]) == point(states[3]), "Saloon release retained movement");
    ensure!(point(states[4]) == point(states[5]), "Saloon focus loss retained movement");
    Ok(())
}
