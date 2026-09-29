//! `verify-assay`: the assay office sells the chosen player and mule
//! bags for exactly weight times grade times 10 dollars, and a cave visit
//! returns the player to the exact map position.
//!
//! Left out: that the build before the fix rejected every bag click, since
//! there is no such build to run.

use game::Report;

use super::fields::{MAP_SCROLL_X, MAP_SCROLL_Y, X, Y, point};
use super::run::{Run, ids};

/// Checks the `assay` and `cave-roundtrip` runs.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(assay: &Run, cave: &Run) -> Result<(), String> {
    let new: [&Report; 8] = assay.reports(ids(1800))?;
    ensure!(new.map(|s| s.gold_bags) == [8, 8, 7, 7, 7, 6, 6, 6], "Wrong bags consumed");
    ensure!(new[..7].iter().all(|s| s.building == 4), "Assay left the office early");
    ensure!(
        new[0].cash == 1000 && new[1].cash == 1000,
        "cash before the first sale: {} and {}, expected 1000",
        new[0].cash,
        new[1].cash
    );
    ensure!(
        new[2].assay_pounds == 5 && (5..=10).contains(&new[2].assay_grade),
        "first bag: {} pounds of grade {}, expected 5 pounds of grade 5 to 10",
        new[2].assay_pounds,
        new[2].assay_grade
    );
    let first = 5 * u32::from(new[2].assay_grade) * 10;
    ensure!(new[2].cash == 1000 + first, "First assay payout differs from its weight/grade");
    ensure!(
        new[2].cash == new[3].cash && new[3].cash == new[4].cash,
        "Empty slot or Next paid twice"
    );
    ensure!(
        new[5].assay_pounds == 6 && (5..=10).contains(&new[5].assay_grade),
        "mule bag: {} pounds of grade {}, expected 6 pounds of grade 5 to 10",
        new[5].assay_pounds,
        new[5].assay_grade
    );
    let second = 6 * u32::from(new[5].assay_grade) * 10;
    ensure!(new[5].cash == new[2].cash + second, "Mule bag payout differs from its weight/grade");
    ensure!(
        new[5].cash == new[6].cash && new[6].cash == new[7].cash,
        "Done/Exit repeated a payout"
    );
    ensure!(new[7].building == 0 && point(new[7]) == (80, 59), "Exit lost town doorway");
    cave_return(cave)
}

/// The cave half: in and out of the cave through the map.
fn cave_return(run: &Run) -> Result<(), String> {
    let cave: [&Report; 6] = run.reports(ids(1700))?;
    ensure!(
        cave[0].map_view && cave[5].map_view && !cave[5].cave_view,
        "captures 1700 and 1705 must show the map, not the cave"
    );
    for field in [X, Y, MAP_SCROLL_X, MAP_SCROLL_Y] {
        ensure!(
            field.of(cave[0]) == field.of(cave[5]),
            "Cave exit displaced player: {}: {:?} {:?}",
            field.key,
            cave[0],
            cave[5]
        );
    }
    for s in &cave[2..5] {
        ensure!(s.cave_view && !s.map_view, "captures 1702 to 1704 must show the cave: {s:?}");
        ensure!((s.return_x, s.return_y) == point(cave[0]), "Cave click lost return position");
    }
    Ok(())
}
