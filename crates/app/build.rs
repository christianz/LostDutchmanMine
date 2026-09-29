//! sdl2-sys links `-lSDL2`, which needs SDL2's development library. Where only
//! the runtime library is installed, as on a player's Linux system, link
//! against that directly, as the C++ build did.

use std::path::Path;

const LIBRARY_FOLDERS: [&str; 4] =
    ["/usr/lib/x86_64-linux-gnu", "/usr/lib64", "/usr/lib", "/usr/local/lib"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    let folders = LIBRARY_FOLDERS.iter().map(Path::new);
    if folders.clone().any(|folder| folder.join("libSDL2.so").exists()) {
        return;
    }
    let Some(runtime) = folders.map(|f| f.join("libSDL2-2.0.so.0")).find(|p| p.exists()) else {
        return;
    };
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let link = Path::new(&out).join("libSDL2.so");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(runtime, &link).expect("a symlink in OUT_DIR");
    println!("cargo:rustc-link-search=native={out}");
}
