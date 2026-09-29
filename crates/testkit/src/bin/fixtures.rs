//! Builds the scenario fixtures from the player's own game into
//! `.local/fixtures`: every fixture twice, kept only when both builds agree
//! byte for byte.
//!
//! The game is read from `LDM_DATA`, or `.local/original`. Usage:
//! `LDM_EXE=/path/to/LDM.EXE cargo run -p testkit --bin fixtures -- [--out DIR]`,
//! or `cargo xtask fixtures [--data DIR]`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sha2::{Digest, Sha256};
use testkit::fixtures::{self, FixtureError};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = testkit::harness::workspace();
    let data = testkit::harness::data();
    let mut out = root.join(".local/fixtures");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or_else(|| format!("{arg} needs a folder"))?;
        match arg.as_str() {
            "--out" => out = PathBuf::from(value),
            _ => return Err(format!("unknown argument {arg}; usage: fixtures [--out DIR]")),
        }
    }
    let scratch = root.join(".local/fixture-builds");
    let builds = [scratch.join("first"), scratch.join("second")];
    for build in &builds {
        fresh(build)?;
        fixtures::build_all(&data, build).map_err(|error: FixtureError| error.to_string())?;
    }
    let mut names: Vec<PathBuf> = std::fs::read_dir(&builds[0])
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()).map_err(|error| error.to_string()))
        .collect::<Result<_, _>>()?;
    names.sort();
    for first in &names {
        let name = first.file_name().expect("a named fixture");
        let (a, b) = (digest(first)?, digest(&builds[1].join(name))?);
        if a != b {
            return Err(format!(
                "fixture {} is not reproducible: {a} != {b}",
                name.to_string_lossy()
            ));
        }
        println!("{:12} {a}", name.to_string_lossy());
    }
    fresh(&out)?;
    std::fs::remove_dir(&out).map_err(|error| error.to_string())?;
    std::fs::rename(&builds[0], &out).map_err(|error| error.to_string())?;
    std::fs::remove_dir_all(&scratch).map_err(|error| error.to_string())
}

/// An empty folder at `path`.
fn fresh(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    std::fs::create_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// SHA-256 over every file's relative path and contents, in path order.
fn digest(folder: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect(folder, folder, &mut files)?;
    files.sort();
    let mut hash = Sha256::new();
    for (name, path) in files {
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update(std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?);
    }
    Ok(hash.finalize().iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
}

fn collect(root: &Path, folder: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    for entry in std::fs::read_dir(folder).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let name = path
                .strip_prefix(root)
                .expect("inside the folder")
                .to_string_lossy()
                .replace('\\', "/");
            files.push((name, path));
        }
    }
    Ok(())
}
