//! Translates the player's LDM.EXE into Rust in `OUT_DIR`.
//!
//! `LDM_EXE=/path/to/LDM.EXE cargo build` checks the executable's SHA-256,
//! unpacks it, recovers its code, places every patch and writes one module per
//! code segment. A wrong executable, an instruction the translator cannot lower
//! or a patch on unexpected bytes stops the build with the address and what was
//! found. Without `LDM_EXE` the crate still builds, so everything that needs no
//! game data can be checked and tested, but it has no game to run.

use std::path::Path;

use translate::emit::{File, emit};
use translate::recover::{parse_entry_points, recover};
use translate::translation::Translation;

fn main() {
    println!("cargo:rerun-if-env-changed=LDM_EXE");
    println!("cargo:rerun-if-changed=entry_points.toml");
    println!("cargo::rustc-check-cfg=cfg(ldm_translated)");
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let files = if let Ok(exe) = std::env::var("LDM_EXE") {
        translated(&exe)
    } else {
        println!(
            "cargo:warning=LDM_EXE is not set: building without the translated game; \
             set LDM_EXE=/path/to/LDM.EXE to run it"
        );
        untranslated()
    };
    for file in files {
        write_changed(&Path::new(&out).join(&file.name), &file.contents);
    }
}

fn translated(exe: &str) -> Vec<File> {
    println!("cargo:rerun-if-changed={exe}");
    println!("cargo::rustc-cfg=ldm_translated");
    let bytes = std::fs::read(exe).unwrap_or_else(|error| fail(&format!("{exe}: {error}")));
    let image = translate::image::load(&bytes).unwrap_or_else(|error| fail(&error.to_string()));
    let seeds = parse_entry_points(include_str!("entry_points.toml"))
        .unwrap_or_else(|error| fail(&format!("entry_points.toml: {error}")));
    // Addresses that do not decode are data a transfer happens to precede;
    // reaching one at run time stops with `Stop::Unrecovered`.
    let recovered = recover(&image, &seeds);
    let translation = Translation::build(&image, &recovered, patches::PATCHES)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let sha = translate::image::sha256_hex(&bytes);
    let mut files = emit(&translation, &sha);
    files.push(File {
        name: "source.rs".to_owned(),
        contents: format!("/// The SHA-256 of the executable this build translated.\npub(crate) const SOURCE: Option<&str> = Some(\"{sha}\");\n"),
    });
    files
}

fn untranslated() -> Vec<File> {
    let mut files = emit(&Translation::default(), "none");
    files.push(File {
        name: "source.rs".to_owned(),
        contents: "/// This build translated no executable.\npub(crate) const SOURCE: Option<&str> = None;\n"
            .to_owned(),
    });
    files
}

fn fail(message: &str) -> ! {
    panic!("cannot translate LDM.EXE: {message}");
}

/// Rewrites a file only when its contents change, so unchanged segments are
/// not recompiled.
fn write_changed(path: &Path, contents: &str) {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return;
    }
    std::fs::write(path, contents)
        .unwrap_or_else(|error| fail(&format!("{}: {error}", path.display())));
}
