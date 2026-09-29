//! Scenario checks: the Python verifiers that checked the C++ port, ported to read a
//! run's [`Outcome`](crate::scenario::Outcome) and folder instead of the C++
//! desktop's capture files.
//!
//! [`CHECKS`] lists every check with the scenarios it reads. Several verifiers
//! compared the C++ build before and after a fix; only their assertions on the
//! fixed build remain, because there is no earlier build to run. A few were
//! written for hand-made saves that the manifest's fixtures do not reproduce,
//! and the C++ oracle's own captures of these runs fail them; those are left
//! out too. What each module leaves out, and why, its own documentation says.

use std::collections::HashMap;

/// Returns the formatted failure from the enclosing check unless `holds`.
macro_rules! ensure {
    ($holds:expr, $($message:tt)+) => {
        if !$holds {
            return Err(format!($($message)+));
        }
    };
}

mod assay;
mod cave;
mod combat;
mod display_menu;
mod fields;
mod input_update;
mod keyboard;
mod menu_input;
mod mouse;
mod movement;
mod pan_inventory;
mod panning;
mod qol;
mod quit;
mod run;
mod saloon;

pub use run::Run;

/// The runs of a set of scenarios, by name.
#[derive(Debug, Default)]
pub struct Runs(HashMap<String, Run>);

impl Runs {
    /// Whether `scenario` ran.
    pub fn contains(&self, scenario: &str) -> bool {
        self.0.contains_key(scenario)
    }

    /// The run of `scenario`.
    ///
    /// # Errors
    ///
    /// A message naming the scenario when it did not run.
    pub fn get(&self, scenario: &str) -> Result<&Run, String> {
        self.0.get(scenario).ok_or_else(|| format!("no run of {scenario}"))
    }
}

impl FromIterator<(String, Run)> for Runs {
    fn from_iter<I: IntoIterator<Item = (String, Run)>>(runs: I) -> Self {
        Runs(runs.into_iter().collect())
    }
}

/// One ported verifier, applied to particular scenarios.
#[derive(Clone, Copy, Debug)]
pub struct Check {
    /// The Python verifier it ports, by name.
    pub verifier: &'static str,
    /// The scenarios it reads, all of which must have run.
    pub scenarios: &'static [&'static str],
    /// The check: `Err` holds the first failed expectation.
    pub check: fn(&Runs) -> Result<(), String>,
}

/// Every check. `verify-combat` applies to both scenarios that share its
/// capture numbers. Not ported: `verify-sleep`, whose `saloon-sleep`
/// scenario has no fixture generator and is absent from the manifest.
pub const CHECKS: &[Check] = &[
    Check {
        verifier: "verify-assay",
        scenarios: &["assay", "cave-roundtrip"],
        check: |runs| assay::check(runs.get("assay")?, runs.get("cave-roundtrip")?),
    },
    Check {
        verifier: "verify-cave",
        scenarios: &["cave-roundtrip"],
        check: |runs| cave::check(runs.get("cave-roundtrip")?),
    },
    Check {
        verifier: "verify-combat",
        scenarios: &["combat"],
        check: |runs| combat::check(runs.get("combat")?),
    },
    Check {
        verifier: "verify-combat",
        scenarios: &["combat-entry"],
        check: |runs| combat::check(runs.get("combat-entry")?),
    },
    Check {
        verifier: "verify-display-menu",
        scenarios: &["display-menu"],
        check: |runs| display_menu::check(runs.get("display-menu")?),
    },
    Check {
        verifier: "verify-input-update",
        scenarios: &[
            "combat-responsive",
            "map-pointer",
            "mining-held-qol",
            "mining-held-classic",
            "panning-classic",
        ],
        check: |runs| {
            input_update::check(
                runs.get("combat-responsive")?,
                runs.get("map-pointer")?,
                runs.get("mining-held-qol")?,
                runs.get("mining-held-classic")?,
                runs.get("panning-classic")?,
            )
        },
    },
    Check {
        verifier: "verify-keyboard",
        scenarios: &["keyboard-controls", "keyboard-save"],
        check: |runs| keyboard::check(runs.get("keyboard-controls")?, runs.get("keyboard-save")?),
    },
    Check {
        verifier: "verify-menu-input",
        scenarios: &["menu-hover", "map-diagonals", "saloon-cadence"],
        check: |runs| {
            menu_input::check(
                runs.get("menu-hover")?,
                runs.get("map-diagonals")?,
                runs.get("saloon-cadence")?,
            )
        },
    },
    Check {
        verifier: "verify-mouse",
        scenarios: &["mouse-clicks"],
        check: |runs| mouse::check(runs.get("mouse-clicks")?),
    },
    Check {
        verifier: "verify-movement",
        scenarios: &["held-movement"],
        check: |runs| movement::check(runs.get("held-movement")?),
    },
    Check {
        verifier: "verify-pan-inventory",
        scenarios: &["pan-inventory-missing", "pan-inventory-owned"],
        check: |runs| {
            pan_inventory::check(
                runs.get("pan-inventory-missing")?,
                runs.get("pan-inventory-owned")?,
            )
        },
    },
    Check {
        verifier: "verify-panning",
        scenarios: &["panning"],
        check: |runs| panning::check(runs.get("panning")?),
    },
    Check {
        verifier: "verify-qol",
        scenarios: &["qol-cadence-classic", "qol-cadence-qol"],
        check: |runs| qol::check(runs.get("qol-cadence-classic")?, runs.get("qol-cadence-qol")?),
    },
    Check {
        verifier: "verify-quit",
        scenarios: &["quit-game"],
        check: |runs| quit::check(runs.get("quit-game")?),
    },
    Check {
        verifier: "verify-saloon",
        scenarios: &["saloon-drinks"],
        check: |runs| saloon::check(runs.get("saloon-drinks")?),
    },
];
