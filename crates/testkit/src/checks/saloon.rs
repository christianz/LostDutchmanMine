//! `verify-saloon`: whiskey and sarsaparilla both return control to
//! the player, who walks away after each and reopens the bartender's menu on
//! returning.

use super::run::Run;

/// The BIOS mode of 320x200 VGA.
const VGA: u8 = 0x13;
/// The saloon's building number.
const SALOON: u16 = 2;

/// Checks the `saloon-drinks` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let ids = [603, 610, 611, 612, 614, 615, 616];
    let states = run.reports(ids)?;
    let [s603, s610, s611, s612, s614, s615, s616] = states;
    for (n, state) in ids.into_iter().zip(states) {
        ensure!(state.building == SALOON && state.video_mode == VGA, "{n}: {state:?}");
        ensure!(state.y == s603.y, "{n}: {state:?}");
        ensure!(state.directions == 0, "{n}: {state:?}");
    }
    ensure!(s603.mouse_visibility >= 0, "{s603:?}");
    ensure!(s610.mouse_visibility < 0, "{s610:?}");
    ensure!(s611.x > s610.x, "{states:?}");
    ensure!(s612.x == s603.x, "{states:?}");
    ensure!(s612.mouse_visibility >= 0, "{s612:?}");
    ensure!(s614.mouse_visibility < 0, "{s614:?}");
    ensure!(s615.x > s614.x, "{states:?}");
    ensure!(s616.x == s615.x, "{states:?}");
    ensure!(s616.mouse_visibility >= 0, "{s616:?}");
    Ok(())
}
