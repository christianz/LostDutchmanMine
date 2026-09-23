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
`LostDutchmanMine.exe`. Choose display settings, then Play. **4K comfort** selects
desktop fullscreen, a centred 85% picture, soft edges, gentle colours and VSync.
**F11** reopens the menu while pausing the game and music. Preferences are saved
in `display.ini`; disable **Show at startup** to go straight into the game.

Choose Crisp pixels, Soft pixels or Pixel art smoothing; Original, Warm, Vivid or
Gentle colour; brightness; window size/fullscreen; and picture size. Fullscreen
uses the monitor's resolution with the original 4:3 proportions. Original colour
at 100% brightness preserves the game's palette, and Crisp retains hard edges.
These filters enlarge the existing artwork; they do not invent detail or new
animation frames. Monitor presentation is paced separately from the game clock.

**CRT monitor** offers Off, Soft and Classic: steady scanlines, an RGB phosphor
mask, soft highlight glow and gently darker edges. The effect appears in the
live preview and is saved with your other preferences. It is off by default;
Soft is subtle and Classic is stronger. It leaves the settings text sharp and
keeps the same mouse coordinates. CRT rendering adds GPU work at high resolutions.

VGA starts automatically, without the original graphics selector.
Hold **WASD**, cursor keys or the numeric keypad to move; release to stop.
Numpad **8/2/4/6** move up/down/left/right and **7/9/1/3** provide diagonals,
with Num Lock on or off. Combine W/A/S/D for diagonal movement. With Num Lock on,
the keypad also enters numbers and selects save slots; **numpad Enter** confirms.
WASD stays ordinary text when entering a save name. The display menu accepts
WASD and numpad 8/2/4/6 too. A movement key returns to walking when the hand
cursor is active. Use the mouse
for selections, F1-F6 for the status panel,
Space for action and Alt+Enter for fullscreen. Align with a doorway and hold Up
to walk into a building. Save/load is under F6.
One click switches from keyboard movement to the hand cursor. Short clicks are
preserved until the game polls them, including the click used to focus its window.

The executable finds `Game/` and `Saves/` alongside itself regardless of the
working directory. The supplied saved games remain in `Game/`; writes go to
`Saves/` with copy-on-write for files opened in read/write mode. Keep `Saves/`
and `display.ini` when updating. Keep `ui-font.bmp` beside the executable.
There is no installer, administrator requirement, Python runtime,
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

With CMake, a C++17 compiler and SDL2 2.26 or newer development files installed:

```sh
cmake -S . -B build-cmake -DCMAKE_BUILD_TYPE=Release
cmake --build build-cmake --parallel 4
```

The verified Linux build in this workspace uses the Makefile with local SDL2
headers and the system SDL2 shared library:

```sh
make -j4 build/ldm-native build/test-assets build/test-arithmetic build/test-poker build/test-quit build/test-display build/test-mouse build/test-keyboard
build/test-arithmetic
build/test-assets /path/to/LDM/LDMG
build/test-poker
build/test-quit
build/test-display
build/test-mouse
build/test-keyboard
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
The settings UI uses a pre-baked DejaVu font bitmap with its license in
`resources/FONT-LICENSE.txt`; no additional runtime font dependency is needed.

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
