//! Every scenario against the C++ oracle's golden trace, and every check ported
//! from its verifiers against the runs it reads.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use testkit::checks::{CHECKS, Run, Runs};
use testkit::scenario::{Manifest, run};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the workspace")
}

fn data(root: &Path) -> PathBuf {
    std::env::var_os("LDM_DATA").map_or_else(|| root.join(".local/original"), PathBuf::from)
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_scenario_reproduces_its_golden_trace() {
    let root = root();
    let manifest = Manifest::load(&root).expect("tests/scenarios.json");
    let data = data(&root);
    let (mut failures, runs): (Vec<String>, Runs) = std::thread::scope(|scope| {
        let runs: Vec<_> = manifest
            .scenarios
            .iter()
            .map(|scenario| {
                let (root, data) = (&root, &data);
                scope.spawn(move || {
                    let folder = root.join(".local/rust-scenarios").join(&scenario.name);
                    let golden = std::fs::read_to_string(
                        root.join("tests/golden").join(format!("{}.trace", scenario.name)),
                    )
                    .expect("a golden trace");
                    match run(scenario, root, data, &folder) {
                        Err(error) => (Some(format!("{}: {error}", scenario.name)), None),
                        Ok(outcome) => (
                            first_difference(&scenario.name, &golden, &outcome.trace),
                            Some((scenario.name.clone(), Run { outcome, folder })),
                        ),
                    }
                })
            })
            .collect();
        let results = runs.into_iter().map(|run| run.join().expect("a scenario thread"));
        let (failures, runs): (Vec<_>, Vec<_>) = results.unzip();
        (failures.into_iter().flatten().collect(), runs.into_iter().flatten().collect())
    });
    let scenarios: HashSet<&str> = manifest.scenarios.iter().map(|s| s.name.as_str()).collect();
    failures.extend(check_failures(&scenarios, &runs));
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// Every check's failure. A check naming a scenario the manifest lacks fails;
/// one whose scenario failed to run is skipped, that failure being reported.
fn check_failures(scenarios: &HashSet<&str>, runs: &Runs) -> Vec<String> {
    let mut failures = Vec::new();
    for check in CHECKS {
        let label = format!("{} on {}", check.verifier, check.scenarios.join(", "));
        if let Some(absent) = check.scenarios.iter().find(|name| !scenarios.contains(**name)) {
            failures.push(format!("{label}: no scenario {absent} in tests/scenarios.json"));
        } else if check.scenarios.iter().all(|name| runs.contains(name))
            && let Err(message) = (check.check)(runs)
        {
            failures.push(format!("{label}: {message}"));
        }
    }
    failures
}

fn first_difference(name: &str, golden: &str, trace: &[String]) -> Option<String> {
    let expected: Vec<&str> = golden.lines().collect();
    let differs = expected.iter().zip(trace).position(|(expected, actual)| expected != actual);
    match differs {
        Some(i) => {
            Some(format!("{name}: line {}: expected {}, got {}", i + 1, expected[i], trace[i]))
        }
        None if expected.len() != trace.len() => {
            Some(format!("{name}: {} lines, expected {}", trace.len(), expected.len()))
        }
        None => None,
    }
}
