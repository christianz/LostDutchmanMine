//! `cargo xtask windows`: the Windows executable, cross-built on Linux.
//!
//! Zig supplies the MinGW C runtime, the C++ compiler for ymfm and the linker;
//! the official SDL2 MinGW development package supplies SDL2. Both are
//! unpacked in `.local/deps` once, by hand, as for the C++ build.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::{game_executable, root};

const TARGET: &str = "x86_64-pc-windows-gnu";
const ZIG: &str = ".local/deps/zig-x86_64-linux-0.15.1/zig";
const SDL2: &str = ".local/deps/SDL2-2.32.0/x86_64-w64-mingw32";

/// cc-rs names the target in clang's spelling; Zig takes its own `-target`.
const COMPILER_TARGET: &str = "--target=*";

/// Flags rustc passes for a MinGW gcc that Zig does not need: its compiler
/// runtime and unwinder stand in for libgcc, and it links its own MinGW C
/// runtime, threads included. SDL2's import library is named by path instead,
/// as Zig's library search does not look for MinGW's `.dll.a` names.
const DROPPED_LINKER_FLAGS: &str = "-lgcc_eh|-lgcc_s|-lgcc|-lpthread|-l:libpthread.a|-lmsvcrt|\
                                    -lmingwex|-lmingw32|-lSDL2|-Wl,--disable-auto-image-base";

/// Builds `lost-dutchman-mine.exe` for Windows and returns its path.
///
/// # Errors
///
/// When Zig, SDL2 or the game is missing, or the build fails.
pub fn build() -> Result<PathBuf> {
    let root = root();
    let zig = root.join(ZIG);
    let sdl = root.join(SDL2);
    if !zig.is_file() {
        bail!("no Zig at {}: unpack zig-x86_64-linux-0.15.1 into .local/deps", zig.display());
    }
    if !sdl.join("lib/libSDL2.dll.a").is_file() {
        bail!("no SDL2 at {}: unpack SDL2-devel-2.32.0-mingw into .local/deps", sdl.display());
    }
    let work = root.join(".local/windows");
    std::fs::create_dir_all(&work)?;
    let wrappers = Wrappers::write(&work, &zig)?;
    let icon = work.join("icon.o");
    let status = Command::new(&zig)
        .args(["rc", "/i"])
        .arg(root.join("resources"))
        .arg("/fo")
        .arg(&icon)
        .arg("--")
        .arg(root.join("resources/windows.rc"))
        .env("ZIG_GLOBAL_CACHE_DIR", work.join("zig-cache"))
        .status()
        .context("running zig rc")?;
    if !status.success() {
        bail!("zig rc failed");
    }
    let rustflags = format!(
        "-C link-arg={} -C link-arg={}",
        sdl.join("lib/libSDL2.dll.a").display(),
        icon.display()
    );
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "app", "--target", TARGET])
        .env("LDM_EXE", game_executable()?)
        .env("ZIG_GLOBAL_CACHE_DIR", work.join("zig-cache"))
        .env("CC_x86_64_pc_windows_gnu", &wrappers.cc)
        .env("CXX_x86_64_pc_windows_gnu", &wrappers.cxx)
        .env("AR_x86_64_pc_windows_gnu", &wrappers.ar)
        .env("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER", &wrappers.linker)
        .env("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS", rustflags)
        .current_dir(&root)
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("the Windows build failed");
    }
    let exe = root.join("target").join(TARGET).join("release/lost-dutchman-mine.exe");
    println!("built {}", exe.display());
    Ok(exe)
}

/// Scripts that make Zig look like the MinGW tools cargo and cc expect.
struct Wrappers {
    cc: PathBuf,
    cxx: PathBuf,
    ar: PathBuf,
    linker: PathBuf,
}

impl Wrappers {
    fn write(folder: &Path, zig: &Path) -> Result<Self> {
        let zig = zig.display();
        let script = |name: &str, body: String| -> Result<PathBuf> {
            let path = folder.join(name);
            std::fs::write(&path, format!("#!/usr/bin/env bash\n{body}\n"))?;
            make_executable(&path)?;
            Ok(path)
        };
        // Each wrapper drops the flags Zig must not see, then runs Zig.
        let filtered = |tool: &str, dropped: &str| {
            format!(
                "args=()\nfor arg in \"$@\"; do\n  case \"$arg\" in\n    {dropped}) ;;\n    \
                 *) args+=(\"$arg\") ;;\n  esac\ndone\n\
                 exec '{zig}' {tool} -target x86_64-windows-gnu \"${{args[@]}}\""
            )
        };
        Ok(Wrappers {
            cc: script("cc", filtered("cc", COMPILER_TARGET))?,
            cxx: script("c++", filtered("c++", COMPILER_TARGET))?,
            ar: script("ar", format!("exec '{zig}' ar \"$@\""))?,
            linker: script("linker", filtered("cc", DROPPED_LINKER_FLAGS))?,
        })
    }
}

fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}
