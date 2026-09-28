//! `tests/verify-combat.py`: in an encounter the mouse aims, a quick click
//! fires once and a held one no more, the keyboard aims and Space fires, a
//! right click opens the hand, and F11 pauses the fight.

use game::Report;

use super::fields::{BOUNDARIES, BULLETS, X, Y, point};
use super::run::{Run, ids};

/// Checks a run of `combat.txt` or `combat-entry.txt`, which number their
/// captures alike.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let s: [&Report; 13] = run.reports(ids(910))?;
    let [s910, s911, s912, s913, s914, s915, s916, s917, s918, s919, s920, s921, s922] = s;
    ensure!(
        s.iter().all(|f| f.combat && !f.panning),
        "every capture must be in combat and not panning"
    );
    ensure!(point(s910) == (160, 64), "capture 910 at {:?}, expected (160, 64)", point(s910));
    ensure!(
        point(s911) == (62, 37) && point(s912) == (237, 27),
        "captures 911 and 912 at {:?} and {:?}, expected (62, 37) and (237, 27)",
        point(s911),
        point(s912)
    );
    let bullets = i32::from(s910.bullets);
    let spent = |f: &Report| bullets - i32::from(f.bullets);
    ensure!(spent(s913) == 1, "Quick click must fire exactly once");
    ensure!(spent(s914) == 2, "Held click must not repeatedly fire");
    ensure!(s915.x > s914.x, "Keyboard must aim with stationary mouse");
    ensure!(spent(s916) == 3, "Space must still fire");
    ensure!(
        s917.mouse_mode == 0 && s917.mouse_visibility >= 0,
        "capture 917: mouse mode {} and visibility {}, expected the shown hand",
        s917.mouse_mode,
        s917.mouse_visibility
    );
    ensure!(
        s918.mouse_mode == 1 && point(s918) == (87, 42),
        "capture 918: mouse mode {} at {:?}, expected aiming at (87, 42)",
        s918.mouse_mode,
        point(s918)
    );
    for field in [BOUNDARIES, X, Y, BULLETS] {
        ensure!(field.of(s919) == field.of(s920), "F11 must pause: {}", field.key);
    }
    ensure!(
        point(s921) == (137, 52) && point(s922) == (137, 52),
        "captures 921 and 922 at {:?} and {:?}, expected (137, 52)",
        point(s921),
        point(s922)
    );
    ensure!(spent(s922) == 3, "capture 922: {} bullets spent, expected 3", spent(s922));
    Ok(())
}
