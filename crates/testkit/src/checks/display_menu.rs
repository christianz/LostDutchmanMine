//! `tests/verify-display-menu.py`: the settings chosen in the menu persist, F11
//! pauses the game, and Cancel or Apply keep the player where they were.

use std::collections::BTreeMap;

use super::fields::POSITION;
use super::run::Run;

/// The settings file the script's choices leave.
const SAVED: [(&str, &str); 8] = [
    ("window", "1"),
    ("size", "100"),
    ("scaling", "2"),
    ("colour", "1"),
    ("crt", "0"),
    ("brightness", "90"),
    ("startup", "0"),
    ("qol", "1"),
];

/// Checks the `display-menu` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let settings = run.settings()?;
    let saved: BTreeMap<String, String> =
        SAVED.iter().map(|&(key, value)| (key.to_owned(), value.to_owned())).collect();
    ensure!(settings == saved, "{settings:?}");
    let [s303, s304, s307, s309, s310, s311] = run.reports([303, 304, 307, 309, 310, 311])?;
    ensure!(s303.x == 160 && s304.x > 160, "{s303:?} {s304:?}");
    for (n, s) in [(307, s307), (309, s309), (310, s310), (311, s311)] {
        for field in POSITION {
            ensure!(field.of(s) == field.of(s304), "{n}, {}: {s:?} {s304:?}", field.key);
        }
    }
    ensure!(s310.steps == s311.steps, "Game ran during F11 settings");
    ensure!(s309.steps > s311.steps, "Game failed to resume");
    Ok(())
}
