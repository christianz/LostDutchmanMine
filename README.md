# Lost Dutchman Mine — native port

Development build of a faithful native port of the supplied 1989 DOS game.
The native Linux build runs the original title, town, movement, menus, saved
games and AdLib music. A Windows x64 executable is cross-compiled from the same
C++17 source. Independent Windows 11 testing and a complete playthrough remain open.
macOS is a prospective SDL2 target and has not been built or tested.

The original executable is EXEPACK-compressed Microsoft C code. Its game logic
is recovered with the original assets and data layouts retained. The port uses
ahead-of-time translation into C++: instructions
are translated at build time, not decoded or interpreted by a CPU emulator at
runtime. File access, windowing, input, timing and audio use native platform
services. The runtime retains the original register and segmented data model;
this is an AOT recompilation, not a hand-rewritten engine. The original decoder
has also been recovered into readable C++ and checked against the translated
routine for every supplied packed asset. FM synthesis uses BSD-licensed ymfm.

Original game files are read from a user-supplied directory. They are not included
in this source repository. The working copy at `/nas/tmp/LDM` must remain intact,
including its existing saved games. Build outputs and recovered proprietary data
are ignored by Git. Do not publish recovered game code or assets automatically.

## Play

Copy the complete Windows bundle to a writable folder and double-click
`LostDutchmanMine.exe`. VGA starts automatically, without a graphics selector.
Hold cursor keys or the numeric keypad to move and release to stop. Use the mouse
for selections, F1-F6 for the status panel,
Space for action and Alt+Enter for fullscreen. Align with a doorway and hold Up
to walk into a building. Save/load is under F6.

The executable finds `Game/` and `Saves/` alongside itself regardless of the
working directory. The supplied saved games remain in `Game/`; writes go to
`Saves/` with copy-on-write for files opened in read/write mode. Keep `Saves/`
when updating. There is no installer, administrator requirement, Python runtime,
DOS executable, DOSBox, CPU interpreter or VM in the playable bundle.

See [validation and remaining work](docs/VALIDATION.md). Unknown control flow
stops with a diagnostic identifying the original address.

## Build

Use the supplied DOS executable as build input. Its SHA-256 is
`de0726a1cb0a475cd05f19ffb56e6c84f014fdb34f986d374d5e09797b925e07`.

```sh
uv venv .venv
uv pip install --python .venv/bin/python -r requirements-dev.txt
.venv/bin/python tools/unpack.py /path/to/LDM/LDM.EXE
.venv/bin/python tools/analyze.py
.venv/bin/python tools/translate.py
```

`recovered/` contains the unpacked load image, relocation metadata and conservative
disassembly. Control flow through indirect calls needs further recovery; the
disassembler's instruction count is not a coverage claim. Run all commands from
the repository root.

With CMake, a C++17 compiler and the SDL2 development package installed:

```sh
cmake -S . -B build-cmake -DCMAKE_BUILD_TYPE=Release
cmake --build build-cmake --parallel 4
```

The verified Linux build in this workspace uses the Makefile with local SDL2
headers and the system SDL2 shared library:

```sh
make -j4 build/ldm-native build/test-assets build/test-arithmetic build/test-poker
build/test-arithmetic
build/test-assets /path/to/LDM/LDMG
build/test-poker
build/ldm-native --data /path/to/LDM --image recovered/load-image.bin --saves .local/saves
```

`SDL_INCLUDE` and `SDL_LIBS` can be overridden for another SDL2 installation.
`tools/build_windows.py` uses official Zig 0.15.1 and the official SDL2 2.32.0
MinGW development archive in `.local/deps/`. These tools are build-only.

```sh
.venv/bin/python tools/build_windows.py
.venv/bin/python tools/package.py /path/to/LDM --platform windows --output dist/Windows-x64
.venv/bin/python tools/package.py /path/to/LDM --platform linux --output dist/Linux-x64
```

Packaging requires a fresh output directory so an existing player's saves are
never removed. Generated game code, original assets and private bundles are
excluded from Git. The vendored ymfm subset retains its upstream BSD license
and a pinned commit in `third_party/ymfm/UPSTREAM.json`.

## Acceptance before calling this a faithful port

- Native Windows executable runs without DOSBox, an x86 interpreter or a VM.
- Original visual assets, palette, game rules, timing, controls and sound match.
- New game, town, travel, supplies, mining, fishing, poker, combat and ending work.
- Save/load round trips work; original saves are kept intact and compatibility is
  checked explicitly.
- Runtime checks on Windows 11 and Linux; macOS support is claimed only when tested.
- Missing functions or unsupported behavior produce a diagnostic rather than a
  fabricated approximation.

## References

- [EXEPACK format](https://github.com/w4kfu/unEXEPACK#exepack-header)
- [Microsoft: 16-bit application support](https://learn.microsoft.com/en-us/windows/compatibility/ntvdm-and-16-bit-app-support)
- [IBM PS/2 BIOS reference, April 1987, pp. 2-40 to 2-43](https://bitsavers.trailing-edge.com/pdf/ibm/pc/ps2/PS2_and_PC_BIOS_Interface_Technical_Reference_Apr87.pdf)
- [SDL2](https://github.com/libsdl-org/SDL/tree/SDL2)
- [ymfm](https://github.com/aaronsgiles/ymfm)
