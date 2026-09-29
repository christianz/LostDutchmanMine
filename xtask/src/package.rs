//! `cargo xtask package --platform <windows|linux> --output DIR [--data DIR]`:
//! a private, portable bundle of the executable and the player's own game.
//!
//! The bundle holds the player's original files, so it is never published.
//! A manifest records every file's size and SHA-256 and the source commit.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::{game_executable, root, windows};

const USAGE: &str =
    "usage: cargo xtask package --platform <windows|linux> --output DIR [--data DIR]";
/// The player's files the game reads; saves they make go to `Saves`.
const GAME_FILES: [&str; 4] = ["LDM.EXE", "LDMG", "LDM.CAP", "LDMSAVE.LDM"];

/// Builds and writes the bundle.
///
/// # Errors
///
/// For bad arguments, an existing output folder, or a failed build or copy.
pub fn package(args: &[String]) -> Result<()> {
    let (windows, output, data) = parse(args)?;
    let root = root();
    if output.exists() {
        bail!("{} exists; choose a fresh folder, so no saves are overwritten", output.display());
    }
    let executable = if windows { windows::build()? } else { linux()? };
    for folder in ["Game", "Saves", "licenses"] {
        std::fs::create_dir_all(output.join(folder))?;
    }
    let name = if windows { "LostDutchmanMine.exe" } else { "LostDutchmanMine" };
    copy(&executable, &output.join(name))?;
    for file in GAME_FILES {
        copy(&data.join(file), &output.join("Game").join(file))?;
    }
    for entry in std::fs::read_dir(&data)? {
        let path = entry?.path();
        let file = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let is_save =
            path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("sav"));
        if is_save && file.to_ascii_uppercase().starts_with("LDMSAVE") {
            copy(&path, &output.join("Game").join(file))?;
        }
    }
    let sdl = root.join(".local/deps/SDL2-2.32.0");
    if windows {
        copy(&sdl.join("x86_64-w64-mingw32/bin/SDL2.dll"), &output.join("SDL2.dll"))?;
    }
    copy(&sdl.join("LICENSE.txt"), &output.join("licenses/SDL2.txt"))?;
    copy(&root.join("third_party/ymfm/LICENSE"), &output.join("licenses/ymfm.txt"))?;
    copy(&root.join("resources/FONT-LICENSE.txt"), &output.join("licenses/DejaVu-font.txt"))?;
    copy(&root.join("docs/VALIDATION.md"), &output.join("VALIDATION.md"))?;
    let launch = if windows {
        "Double-click LostDutchmanMine.exe."
    } else {
        "Install your distribution's SDL2 runtime, then run ./LostDutchmanMine."
    };
    let readme = std::fs::read_to_string(root.join("resources/bundle-readme.txt"))?;
    std::fs::write(output.join("README.txt"), readme.replace("{launch}", launch))?;
    let files = manifest(&output, if windows { "windows" } else { "linux" })?;
    println!("{} ({files} files)", output.display());
    Ok(())
}

fn parse(args: &[String]) -> Result<(bool, PathBuf, PathBuf)> {
    let (mut platform, mut output) = (None, None);
    let mut data = root().join(".local/original");
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let value = args.next().with_context(|| format!("{arg} needs a value\n{USAGE}"))?;
        match arg.as_str() {
            "--platform" => platform = Some(value.clone()),
            "--output" => output = Some(PathBuf::from(value)),
            "--data" => data = PathBuf::from(value),
            _ => bail!("unknown argument {arg}\n{USAGE}"),
        }
    }
    let windows = match platform.as_deref() {
        Some("windows") => true,
        Some("linux") => false,
        _ => bail!(USAGE),
    };
    Ok((windows, output.context(USAGE)?, data))
}

fn linux() -> Result<PathBuf> {
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "app"])
        .env("LDM_EXE", game_executable()?)
        .current_dir(root())
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("the Linux build failed");
    }
    Ok(root().join("target/release/lost-dutchman-mine"))
}

/// Copies a file, or a folder and everything in it.
fn copy(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let path = entry?.path();
            copy(&path, &to.join(path.file_name().context("a named entry")?))?;
        }
        return Ok(());
    }
    std::fs::copy(from, to).with_context(|| format!("copying {}", from.display()))?;
    Ok(())
}

/// Writes `manifest.json` and returns how many files it lists.
fn manifest(output: &Path, platform: &str) -> Result<usize> {
    let mut files = Vec::new();
    collect(output, output, &mut files)?;
    files.sort();
    let commit = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(root()).output()?;
    let commit = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
    let mut json = format!(
        "{{\n  \"platform\": \"{platform}\",\n  \"source_commit\": \"{commit}\",\n  \"files\": {{\n"
    );
    for (i, (name, bytes, sha)) in files.iter().enumerate() {
        let comma = if i + 1 < files.len() { "," } else { "" };
        let _ =
            writeln!(json, "    \"{name}\": {{\"bytes\": {bytes}, \"sha256\": \"{sha}\"}}{comma}");
    }
    json.push_str("  }\n}\n");
    std::fs::write(output.join("manifest.json"), json)?;
    Ok(files.len())
}

fn collect(root: &Path, folder: &Path, files: &mut Vec<(String, u64, String)>) -> Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let bytes = std::fs::read(&path)?;
            let name = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            let sha = Sha256::digest(&bytes).iter().fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            });
            files.push((name, bytes.len() as u64, sha));
        }
    }
    Ok(())
}
