//! `verify-pan-inventory`: without a pan in the inventory neither the
//! mouse nor P pans, and with one both animate and collect a bag each.
//!
//! Left out: that the build before the fix panned without a pan, since there
//! is no such build to run.

use game::Report;

use super::fields::point;
use super::run::{Run, ids};

/// Checks the `pan-inventory-missing` and `pan-inventory-owned` runs.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(missing: &Run, owned: &Run) -> Result<(), String> {
    let s: [&Report; 5] = missing.reports(ids(2000))?;
    ensure!(
        s.iter().all(|s| s.qol && !s.panning && s.gold_bags == 0),
        "without a pan every capture must have QoL on, no panning and no gold"
    );
    ensure!(
        s.iter().all(|s| point(s) == (40, 55)),
        "without a pan the player must stay at (40, 55)"
    );
    let s: [&Report; 5] = owned.reports(ids(2000))?;
    let expected = [(false, 0), (true, 0), (false, 1), (true, 1), (false, 2)];
    for ((n, s), (panning, bags)) in (2000..).zip(s).zip(expected) {
        ensure!(
            s.panning == panning && s.gold_bags == bags,
            "capture {n}: panning {} with {} bags, expected panning {panning} with {bags}",
            s.panning,
            s.gold_bags
        );
    }
    Ok(())
}
