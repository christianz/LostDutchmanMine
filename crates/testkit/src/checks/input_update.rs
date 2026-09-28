//! `tests/verify-input-update.py`: the encounter's sight follows the mouse at
//! once, numpad walking leaves the pointer alone, held Space keeps mining until
//! released, and panning with QoL off plays the animation for one bag.
//!
//! Left out: that the build before the fix drew the sight late, since there is
//! no such build to run; and that every aiming capture holds 20 bullets. The
//! verifier's hand-made save carried 20, but the `combat` fixture the manifest
//! builds carries 227, and the C++ oracle's captures of this run hold 227 too.

use game::Report;

use super::fields::{BOUNDARIES, MINING_STROKES, X, Y, point};
use super::run::{Run, ids};

/// Checks the `combat-responsive`, `map-pointer`, both `mining-held` and the
/// `panning-classic` runs.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(
    combat: &Run,
    map: &Run,
    mining_qol: &Run,
    mining_classic: &Run,
    panning: &Run,
) -> Result<(), String> {
    responsive_aim(combat)?;
    stationary_pointer(map)?;
    held_mining(mining_qol, "mining-held-qol", true)?;
    held_mining(mining_classic, "mining-held-classic", false)?;
    classic_panning(panning)
}

/// Twelve mouse moves, each captured 25 ms later.
fn responsive_aim(run: &Run) -> Result<(), String> {
    let mut previewed = 0;
    for (i, after) in (0..).zip(run.reports::<12>(ids(3000))?) {
        let expected = (62 + i * 14, 22 + (i % 4) * 10);
        ensure!(after.combat, "capture {} is not in combat", 3000 + i);
        ensure!((after.sight_x, after.sight_y) == expected, "Delayed aim: {i}");
        previewed += u32::from((i32::from(after.x), i32::from(after.y)) != expected);
    }
    ensure!(previewed > 0, "no capture shows the sight ahead of the encounter's committed aim");
    Ok(())
}

/// Numpad walking on the map with Num Lock on and off.
fn stationary_pointer(run: &Run) -> Result<(), String> {
    let mut positions = std::collections::BTreeSet::new();
    for s in run.reports::<5>(ids(4100))? {
        ensure!(
            s.qol && s.map_view && (s.mouse_x, s.mouse_y) == (65, 55),
            "pointer must stay at (65, 55) on the map with QoL: {s:?}"
        );
        positions.insert(point(s));
    }
    ensure!(positions.len() == 5, "five captures, only {} positions", positions.len());
    Ok(())
}

/// Space held in a cave, released, lost to focus and paused by the settings.
fn held_mining(run: &Run, name: &str, qol: bool) -> Result<(), String> {
    let s: [&Report; 9] = run.reports(ids(4000))?;
    ensure!(
        s.iter().all(|v| v.qol == qol && v.cave_view),
        "{name}: every capture must be in the cave with QoL {qol}"
    );
    ensure!(s[1].mining_strokes >= 3, "{name}: capture 4001 has {} strokes", s[1].mining_strokes);
    ensure!(
        s[2].mining_strokes > s[1].mining_strokes,
        "{name}: strokes {} then {}, expected more while held",
        s[1].mining_strokes,
        s[2].mining_strokes
    );
    ensure!(
        s[1].mining_space_held && s[2].mining_space_held,
        "{name}: Space must be held at captures 4001 and 4002"
    );
    ensure!(
        s[4].mining_strokes >= 3 && s[4].mining_space_held,
        "{name}: capture 4004 has {} strokes, Space held {}",
        s[4].mining_strokes,
        s[4].mining_space_held
    );
    for n in [0, 3, 5, 8] {
        ensure!(
            !s[n].mining_space_held && s[n].mining_strokes == 0,
            "{name}: capture {} must have stopped mining: {:?}",
            4000 + n,
            s[n]
        );
    }
    for field in [BOUNDARIES, MINING_STROKES, X, Y] {
        ensure!(
            field.of(s[6]) == field.of(s[7]),
            "Settings failed to pause: {name}, {}",
            field.key
        );
    }
    Ok(())
}

/// A Pan click with QoL off, with more input queued during the animation.
fn classic_panning(run: &Run) -> Result<(), String> {
    let s: [&Report; 4] = run.reports(ids(4200))?;
    ensure!(s.iter().all(|v| !v.qol), "every capture must have QoL off");
    ensure!(
        !s[0].panning && s[0].gold_bags == 0,
        "capture 4200 must be before panning, with no gold: {:?}",
        s[0]
    );
    for (n, v) in [(4201, s[1]), (4202, s[2])] {
        ensure!(v.panning && v.gold_bags == 0, "capture {n} must be panning, with no gold: {v:?}");
    }
    ensure!(
        !s[3].panning && s[3].gold_bags == 1,
        "capture 4203 must be after panning, with one bag: {:?}",
        s[3]
    );
    ensure!(point(s[3]) == (40, 55), "capture 4203 at {:?}, expected (40, 55)", point(s[3]));
    Ok(())
}
