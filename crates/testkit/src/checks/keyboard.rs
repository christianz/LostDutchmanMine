//! `verify-keyboard`: WASD and the numpad walk with Num Lock on or
//! off, diagonals combine, release and focus loss stop, keys drive the
//! settings menu, and a save named with WASD letters and keypad digits loads.

use super::fields::position;
use super::run::{Run, ids, reports};

/// The save slot the script writes, and the original format's size of it.
const SLOT: &str = "LDMSAVE8.SAV";
const SAVE_SIZE: usize = 6066;

/// Checks the `keyboard-controls` and `keyboard-save` runs, whose captures
/// the verifier read from one merged folder.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(controls: &Run, save: &Run) -> Result<(), String> {
    // The merge copied the save run's captures last, so its numbers win.
    let merged = |n| save.report(n).or_else(|_| controls.report(n));
    let [
        _,
        f501,
        f502,
        f503,
        f504,
        f505,
        f506,
        f507,
        f508,
        f509,
        f510,
        f511,
        f512,
        f513,
        f514,
        f515,
    ] = reports(ids(500), merged)?;
    let [f519, f530, f531, f532] = reports([519, 530, 531, 532], merged)?;

    ensure!(f501.pointer_visible, "{f501:?}");
    ensure!(f502.directions == 8 && f502.x > f501.x, "{f502:?}");
    ensure!(
        f502.mouse_mode == 1 && f502.pointer_visible,
        "Keyboard movement must keep the pointer available"
    );
    ensure!(f503.x > f502.x, "Held D must continue without OS repeat");
    ensure!(position(f503) == position(f504), "Releasing D must stop movement");
    ensure!(f505.directions == 5, "W+A must produce an up-left diagonal");
    ensure!(f506.x < f504.x, "W+A must move left in the town");
    ensure!(f507.x > f506.x, "Numpad 6 / Num Lock off must move right");
    ensure!(f508.x < f507.x, "Numpad 4 / Num Lock on must move left");
    ensure!(f509.directions == 9, "Numpad 9 must produce an up-right diagonal");
    ensure!(f510.x > f508.x, "Numpad 9 must move right in the town");
    let settled = [
        (511, f511, 512, f512),
        (513, f513, 514, f514),
        (514, f514, 515, f515),
        (515, f515, 519, f519),
    ];
    for (a, fa, b, fb) in settled {
        ensure!(position(fa) == position(fb), "{a}, {b}: {fa:?} {fb:?}");
    }
    let stopped = [
        (503, f503),
        (504, f504),
        (506, f506),
        (507, f507),
        (508, f508),
        (510, f510),
        (511, f511),
        (512, f512),
        (513, f513),
        (514, f514),
        (515, f515),
        (519, f519),
    ];
    for (n, f) in stopped {
        ensure!(f.directions == 0, "capture {n} holds directions {}, expected none", f.directions);
    }

    let settings = controls.settings()?;
    let on = |key: &str| settings.get(key).is_some_and(|value| value == "1");
    ensure!(on("crt") && on("colour"), "{settings:?}");
    ensure!(f531.x < f530.x, "A must move away from the saved position");
    ensure!(position(f530) == position(f532), "Keypad load must restore the saved position");
    ensure!(save.save(SLOT)?.len() == SAVE_SIZE, "Original save format changed");
    ensure!(
        save.save("LDMSAVE.LDM")?.windows(6).any(|name| name == b"WASD42"),
        "WASD letters/keypad digits did not survive text entry"
    );
    Ok(())
}
