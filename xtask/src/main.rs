//! Project automation: `cargo xtask <command>`.
//!
//! - `lint`: rustfmt, pedantic clippy, and no raw game addresses in `game`.
//! - `vectors`: download the SingleStepTests 8088 vectors into `.local/vectors`.
//! - `test`: every test, including the hardware vectors when downloaded.
//! - `verify`: the tests that need the original game, against the golden traces.
//! - `windows`: cross-build the Windows executable with Zig.
//! - `package`: a private portable bundle with the player's own game.

mod lint;
mod package;
mod vectors;
mod windows;

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

const USAGE: &str = "usage: cargo xtask <lint | vectors | test | verify | windows | package>";

fn main() -> Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("lint") => lint::lint(),
        Some("vectors") => vectors::vectors(),
        Some("test") => test(),
        Some("verify") => verify(),
        Some("windows") => windows::build().map(|_| ()),
        Some("package") => package::package(&std::env::args().skip(2).collect::<Vec<_>>()),
        _ => bail!(USAGE),
    }
}

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace")
        .to_path_buf()
}

pub(crate) fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .status()
        .with_context(|| format!("running {program}"))?;
    if !status.success() {
        bail!("{program} {} failed", args.join(" "));
    }
    Ok(())
}

fn test() -> Result<()> {
    run("cargo", &["test", "--workspace"])?;
    if root().join(".local/vectors/metadata.json").exists() {
        run(
            "cargo",
            &["test", "-p", "testkit", "--test", "hardware", "--", "--ignored", "--nocapture"],
        )?;
    } else {
        println!("hardware vectors not downloaded; run `cargo xtask vectors` to include them");
    }
    Ok(())
}

/// The executable the game crate translates: `LDM_EXE`, or the player's copy
/// in `.local/original`.
pub(crate) fn game_executable() -> Result<PathBuf> {
    let exe = std::env::var_os("LDM_EXE")
        .map_or_else(|| root().join(".local/original/LDM.EXE"), PathBuf::from);
    if !exe.is_file() {
        bail!("no LDM.EXE at {}: set LDM_EXE=/path/to/LDM.EXE", exe.display());
    }
    Ok(exe)
}

/// Every test that needs the original game: the translation, the headless
/// boot and savestates, the readable decoder on every shipped asset, and every
/// scenario against its golden trace and checks, with readable routines
/// checked in lockstep.
fn verify() -> Result<()> {
    let exe = game_executable()?;
    let status = Command::new("cargo")
        .args(["test", "-p", "translate", "-p", "game", "-p", "testkit"])
        .args(["--", "--ignored", "--nocapture"])
        .env("LDM_EXE", &exe)
        .current_dir(root())
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("verification failed");
    }
    Ok(())
}
