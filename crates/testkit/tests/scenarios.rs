//! Every scenario against the C++ oracle's golden trace.

use std::path::{Path, PathBuf};

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
    let failures: Vec<String> = std::thread::scope(|scope| {
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
                        Err(error) => Some(format!("{}: {error}", scenario.name)),
                        Ok(result) => first_difference(&scenario.name, &golden, &result.trace),
                    }
                })
            })
            .collect();
        runs.into_iter().filter_map(|run| run.join().expect("a scenario thread")).collect()
    });
    assert!(failures.is_empty(), "{} scenarios diverge:\n{}", failures.len(), failures.join("\n"));
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
