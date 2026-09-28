//! `tests/verify-cave.py`: clicking cave scenery keeps the player in the cave,
//! and walking out restores the exact map position and scroll.
//!
//! Left out, because each compares with the build before the fix, which there
//! is no way to run: that it returned the player elsewhere, that its start
//! matched the fixed build's, and both pixel comparisons. Those counted black
//! pixels in the shooting rows to tell the tunnel from the replayed outdoor
//! entrance; with no outdoor picture from the old build there is nothing to
//! hold the fixed build's tunnel against.

use game::Report;

use super::fields::point;
use super::run::Run;

/// Where the cave puts the player, whatever the map position.
const INSIDE: (u16, u16) = (60, 50);

/// Checks the `cave-roundtrip` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let [new_start, new_end]: [&Report; 2] = run.reports([1700, 1705])?;
    let scroll = |s: &Report| (s.map_scroll_x, s.map_scroll_y);
    ensure!(
        new_start.map_view && new_end.map_view,
        "captures 1700 and 1705 must show the map: {new_start:?} {new_end:?}"
    );
    ensure!(point(new_start) == point(new_end), "New exit changed map coordinates");
    ensure!(scroll(new_start) == scroll(new_end), "New exit changed map scroll offsets");
    for n in [1702, 1703, 1704] {
        let s = run.report(n)?;
        ensure!(
            s.cave_view && !s.map_view && point(s) == INSIDE,
            "Unexpected cave state: {n}: {s:?}"
        );
        ensure!((s.return_x, s.return_y) == point(new_start), "Lost return position: {n}: {s:?}");
    }
    Ok(())
}
