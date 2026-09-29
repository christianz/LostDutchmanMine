//! `cargo xtask lint`: rustfmt, pedantic clippy, and no raw game addresses.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::{root, run};

pub fn lint() -> Result<()> {
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
