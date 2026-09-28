//! Project automation: `cargo xtask <command>`.
//!
//! - `lint`: rustfmt, pedantic clippy, and no raw game addresses in `game`.
//! - `vectors`: download the SingleStepTests 8088 vectors into `.local/vectors`.
//! - `test`: every test, including the hardware vectors when downloaded.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use anyhow::{Context, Result, bail};

const USAGE: &str = "usage: cargo xtask <lint | vectors | test>";
const VECTORS_URL: &str = "https://raw.githubusercontent.com/SingleStepTests/8088/main/v2";

fn main() -> Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("lint") => lint(),
        Some("vectors") => vectors(),
        Some("test") => test(),
        _ => bail!(USAGE),
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace")
        .to_path_buf()
}

fn run(program: &str, args: &[&str]) -> Result<()> {
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

fn lint() -> Result<()> {
    run("cargo", &["fmt", "--all", "--check"])?;
    run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    raw_addresses()
}

/// Game code names its addresses in `symbols.rs`; a long hex literal anywhere
/// else in the game crate is an unnamed address.
fn raw_addresses() -> Result<()> {
    let source = root().join("crates/game/src");
    if !source.exists() {
        return Ok(());
    }
    let mut problems = Vec::new();
    for path in rust_files(&source)? {
        if path.file_name().is_some_and(|name| name == "symbols.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            if has_long_hex(code) {
                problems.push(format!("{}:{}: {}", path.display(), number + 1, line.trim()));
            }
        }
    }
    if problems.is_empty() {
        return Ok(());
    }
    bail!("raw addresses outside symbols.rs:\n{}", problems.join("\n"))
}

fn has_long_hex(code: &str) -> bool {
    code.match_indices("0x").any(|(at, _)| {
        code[at + 2..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
            .filter(char::is_ascii_hexdigit)
            .count()
            >= 3
    })
}

fn rust_files(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

fn vector_stems() -> Result<Vec<String>> {
    let list = std::fs::read_to_string(root().join("crates/testkit/hardware-vectors.txt"))?;
    let stems = list.lines().filter(|line| !line.starts_with('#')).flat_map(str::split_whitespace);
    Ok(stems.map(|stem| format!("{stem}.json.gz")).chain(["metadata.json".to_owned()]).collect())
}

fn vectors() -> Result<()> {
    let folder = root().join(".local/vectors");
    std::fs::create_dir_all(&folder)?;
    let missing: Vec<String> =
        vector_stems()?.into_iter().filter(|name| !folder.join(name).exists()).collect();
    println!("downloading {} vector files into {}", missing.len(), folder.display());
    let failures: Vec<String> = thread::scope(|scope| {
        let workers: Vec<_> = missing
            .chunks(missing.len().div_ceil(8).max(1))
            .map(|chunk| {
                let folder = &folder;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter(|name| {
                            download(&format!("{VECTORS_URL}/{name}"), &folder.join(name)).is_err()
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers.into_iter().flat_map(|worker| worker.join().expect("download worker")).collect()
    });
    if !failures.is_empty() {
        bail!("could not download: {}", failures.join(", "));
    }
    Ok(())
}

fn download(url: &str, destination: &Path) -> Result<()> {
    let partial = destination.with_extension("partial");
    let status = Command::new("curl")
        .args(["-sSfL", "-o"])
        .arg(&partial)
        .arg(url)
        .status()
        .context("running curl")?;
    if !status.success() {
        bail!("curl {url} failed");
    }
    std::fs::rename(partial, destination)?;
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
