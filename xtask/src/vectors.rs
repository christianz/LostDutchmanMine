//! `cargo xtask vectors`: the SingleStepTests 8088 hardware vectors.

use std::path::Path;
use std::process::Command;
use std::thread;

use anyhow::{Context, Result, bail};

use crate::root;

const VECTORS_URL: &str = "https://raw.githubusercontent.com/SingleStepTests/8088/main/v2";

fn vector_stems() -> Result<Vec<String>> {
    let list = std::fs::read_to_string(root().join("crates/testkit/hardware-vectors.txt"))?;
    let stems = list.lines().filter(|line| !line.starts_with('#')).flat_map(str::split_whitespace);
    Ok(stems.map(|stem| format!("{stem}.json.gz")).chain(["metadata.json".to_owned()]).collect())
}

pub fn vectors() -> Result<()> {
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
