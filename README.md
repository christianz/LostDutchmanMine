# Lost Dutchman Mine — native port

Development build of a faithful native port of the 1989 DOS game. The native
Linux build runs the original title, town, movement, menus, saved games and
AdLib music. A Windows x64 executable is cross-compiled from the same Rust
source. Independent Windows 11 testing and a complete playthrough remain open.
macOS is a prospective SDL2 target and has not been built or tested.

The original executable is EXEPACK-compressed Microsoft C code. At build time
the port unpacks it, recovers its code and translates every instruction into
Rust, which is compiled to native machine code: nothing is decoded or
interpreted at run time, and there is no DOSBox, CPU emulator or VM. The game
runs on its original register and segmented data model, with the original
assets and data layouts. File access, windowing, input, timing and audio use
native platform services. Original routines are being replaced one at a time
with readable Rust proven equivalent to their translation; the first is the
asset decoder. FM synthesis uses BSD-licensed ymfm.

Everything is deterministic: the game advances in emulated milliseconds, never
wall-clock time, so a run from the same input is the same run. The build was
checked against the earlier C++ port line for line: the state of the whole
machine every 100 ms of 29 recorded scenarios is identical (see
[Verify](#verify)).

Original game files are not included in this repository; you need your own copy
of Lost Dutchman Mine. They are read from a directory you supply, which is never
written to: its saved games stay intact and new saves go to a separate `Saves/`
directory. Build outputs, generated game code and recovered proprietary data are
ignored by Git. Do not commit or publish recovered game code or assets.

## Play

Build a playable bundle with `cargo xtask package` (see [Build](#build)), copy
the complete Windows bundle to a writable folder and double-click
`LostDutchmanMine.exe`. Choose display settings, then Play. **Comfort** selects
desktop fullscreen, a centred 85% picture, soft edges and gentle colours.
**F11** reopens the menu while pausing the game and music. Preferences are saved
in `display.ini`; set **Show at startup** to No to go straight into the game.

**Display** offers three window sizes and Fullscreen, Fullscreen 85% or
Fullscreen 70%; the smaller fullscreen pictures leave a border on large monitors.
Fullscreen uses the monitor's resolution with the original 4:3 proportions, and
windows always fill with the whole picture. Choose Crisp pixels, Soft pixels or
Pixel art smoothing; Original, Warm, Vivid or Gentle colour; and brightness.
Original colour at 100% brightness preserves the game's palette, and Crisp
retains hard edges. These filters enlarge the existing artwork; they do not
invent detail or new animation frames. Presentation always uses VSync where
available and is paced to the monitor separately from the game clock.

**CRT monitor** offers Off, Subtle and Strong: steady scanlines, an RGB phosphor
mask, soft highlight glow and gently darker edges. The effect appears in the
live preview and is saved with your other preferences. It is off by default.
It leaves the settings text sharp and keeps the same mouse coordinates. CRT
rendering adds GPU work at high resolutions.

**QoL improvements** enables clearer in-game menus, an arrow pointer and smooth
mouse aiming in combat, and is on by default. The six original
toolbar icons gain names, larger click targets and a gold hover outline, fitted
below the logo and weekday. Hovering shows the relevant F1-F6 shortcut in the empty message
strip. River and other contextual buttons accept clicks on their bevels too.
Opening a menu suspends hover on the buttons underneath; closing it restores
the current scene's hover targets.
Health and food icons follow the original live warnings, including critical-health
flashing. Turning QoL off restores the original panel artwork and click targets.
At a river, choose Pan while carrying a pan. The prospector plays his original
three-pose panning animation in the river scene, then receives one original gold
bag with the river's original grade. This restored action applies with QoL on or
off. F11 pauses it, and changing display preferences cannot interrupt it.
With QoL enabled, the arrow pointer remains visible
while walking and one click selects a toolbar or context action. Keyboard and
mouse work together without switching modes, and keyboard movement leaves the
pointer where you put it. Town, saloon and mine walking
respond faster while a direction is held; the survival clock and mine hazard
checks retain their original pace. Space opens a desert close-up and a second
Space returns to the map (Enter or Escape also closes it). Hold Space while using
the pick to keep mining at the original stroke rate; release it to stop. This
works with QoL on or off, and opening settings or losing focus releases the key.

Only the cheapest unowned mule can be bought with QoL enabled. Other mules show
**SOLD OUT**; buying the available mule unlocks the next one. Existing mule
ownership and inventory rows are preserved. Loading a save inside the saloon
now preserves the street position used by Exit, with QoL either on or off.

The supplied executable skips two intact animation loops. The native translator
reconnects their surviving comparisons and bodies, preserving the original
sprites, page blits, timer waits, random cycle count and inventory reward. The
signed branch conditions are reconstructed from the surrounding code; they have
not been compared with an unmodified retail executable. The custom panning
minigame is retired, and its creek artwork is no longer packaged or required.

Mouse aiming is ready when an armed fight starts. Move over the scene to aim
the original crosshair and **left-click** to fire one shot. Its display follows
mouse motion between encounter ticks. **Right-click** opens the pointer for
Run/status/menu selections; a direction key returns to aiming.
WASD, arrows and numpad still aim, and **Space** still fires. A stationary mouse
does not override keyboard aiming. Turn QoL improvements off for the original
keyboard aiming and mouse selection behavior. Ammunition and hit rules are unchanged.

The Windows executable opens only the game window. Startup failures still show
an error dialog.

VGA starts automatically, without the original graphics selector.
Hold **WASD**, cursor keys or the numeric keypad to move; release to stop.
Numpad **8/2/4/6** move up/down/left/right and **7/9/1/3** provide diagonals,
with Num Lock on or off. Combine W/A/S/D for diagonal movement. Held combinations
keep both axes on the world map even while a key repeats.
With Num Lock on,
the keypad also enters numbers and selects save slots; **numpad Enter** confirms.
WASD stays ordinary text when entering a save name. The display menu accepts
WASD and numpad 8/2/4/6 too. A movement key returns to walking when the hand
cursor is active. Use the mouse
for selections, F1-F6 for the status panel,
Space for action and Alt+Enter for fullscreen. Align with a doorway and hold Up
to walk into a building. Save/load is under F6.
With QoL off, one click switches from keyboard movement to the hand cursor. Short clicks are
preserved until the game polls them, including the click used to focus its window.

The executable finds `Game/` and `Saves/` alongside itself regardless of the
working directory. The supplied saved games remain in `Game/`; writes go to
`Saves/` with copy-on-write for files opened in read/write mode. Keep `Saves/`
and `display.ini` when updating. There is no installer, administrator
requirement, Python runtime, DOSBox, CPU interpreter or VM in the playable
bundle; the original `LDM.EXE` in `Game/` is read for its data, never run.

See [validation and remaining work](docs/VALIDATION.md). Unknown control flow
stops with a diagnostic identifying the original address.

## Build

You need Rust 1.94 or newer and your own `LDM.EXE`, whose SHA-256 must be
`de0726a1cb0a475cd05f19ffb56e6c84f014fdb34f986d374d5e09797b925e07`. The
game crate's build script translates it, so every build names it:

```sh
LDM_EXE=/path/to/LDM/LDM.EXE cargo build --release -p app
target/release/lost-dutchman-mine --data /path/to/LDM --saves .local/saves
```

Without `LDM_EXE` everything still builds and every test that needs no game
data runs, but the executable has no game to start. On Linux the window needs
the SDL2 library (the runtime package is enough).

The workspace, one concern per crate:

| Crate | Owns |
| --- | --- |
| `machine` | 8086 registers and flags, the ALU, memory, ports, the timer, the AdLib chip, savestates |
| `dos` | The DOS and BIOS services the game calls: files, keyboard, mouse, clock, video |
| `translate` | Build time only: EXEPACK, control-flow recovery, the IR, the patch table's placement, Rust emission, routine contracts |
| `patches` | Every change to the original, as data: site, expected bytes, hook or routine |
| `game` | The translated game, named addresses (`symbols.rs`), the hooks, the QoL overlay, frames, readable routines |
| `engine` | The deterministic session: 1 ms quanta, timer interrupts, input, traces |
| `desktop` | Settings, the settings menu, input mapping, audio synthesis and picture processing, without a window |
| `app` | The SDL2 window around it all |
| `testkit` | Scenario runs, golden traces, ported verifiers, hardware vectors, the IR interpreter |

`cargo xtask lint` runs rustfmt, pedantic clippy and a check that game code names
every address it touches. `cargo xtask test` runs every test that needs no game
data, including the SingleStepTests 8088 hardware vectors once downloaded with
`cargo xtask vectors`.

The Windows executable is cross-built on Linux with official Zig 0.15.1 and the
official SDL2 2.32.0 MinGW development archive, both unpacked into
`.local/deps/`:

```sh
LDM_EXE=/path/to/LDM/LDM.EXE cargo xtask windows
LDM_EXE=/path/to/LDM/LDM.EXE cargo xtask package --platform windows --output dist/Windows-x64 --data /path/to/LDM
LDM_EXE=/path/to/LDM/LDM.EXE cargo xtask package --platform linux --output dist/Linux-x64 --data /path/to/LDM
```

Packaging requires a fresh output folder, so an existing player's saves are
never removed, and writes a manifest of every file's SHA-256. Bundles contain
the player's own game files and are never published. The vendored ymfm subset
retains its upstream BSD license and a pinned commit in
`third_party/ymfm/UPSTREAM.json`. The settings menu's pre-baked DejaVu font
bitmap is built into the executable; its license is in
`resources/FONT-LICENSE.txt`.

## Verify

```sh
LDM_EXE=/path/to/LDM/LDM.EXE LDM_DATA=/path/to/LDM cargo xtask verify
```

runs every test that needs the original game:

- **Golden traces.** 29 scenarios in `tests/scenarios.json` replay desktop
  input from `tests/scripts/` through the same controller the window uses.
  Every 100 emulated milliseconds the FNV-1a hash of all registers, all 1 MiB
  of memory and the palette must equal the line recorded from the C++ port in
  `tests/golden/`. The hashes contain no game content.
- **Checks.** The scenario verifiers, ported to Rust, read each run's captures,
  saves and settings file.
- **Lockstep.** Every call of a readable routine is also run translated, on a
  copy of the machine, and the two must agree on everything the callers can
  observe: the registers and flags the translator's liveness analysis finds
  they read, and all memory outside the dead stack.
- **Savestates.** A state restored halfway through the startup continues on
  the golden trace.

Fixtures for the scenarios are built from your own saves into `.local/fixtures`.

## Acceptance before calling this a faithful port

- Native Windows executable runs without DOSBox, an x86 interpreter or a VM.
- Original visual assets, palette, game rules, timing, controls and sound match.
- New game, town, travel, supplies, mining, fishing, poker, combat and ending work.
- Save/load round trips work; original saves are kept intact and compatibility is
  checked explicitly.
- Runtime checks on Windows 11 and Linux; macOS support is claimed only when tested.
- Missing functions or unsupported behavior produce a diagnostic rather than a
  fabricated approximation.

## License

The port's own source code, tools, tests and new artwork are released under the
[MIT License](LICENSE). Bundled third-party components keep their own licenses:
ymfm is BSD-3-Clause (`third_party/ymfm/LICENSE`) and the DejaVu-derived settings
font is covered by `resources/FONT-LICENSE.txt`. SDL2 is linked, not vendored, and
is distributed under the zlib license.

Lost Dutchman Mine itself, including its executable, artwork, music and data, is
not part of this repository and is not covered by this license.

## References

- [EXEPACK format](https://github.com/w4kfu/unEXEPACK#exepack-header)
- [Microsoft: 16-bit application support](https://learn.microsoft.com/en-us/windows/compatibility/ntvdm-and-16-bit-app-support)
- [IBM PS/2 BIOS reference, April 1987, pp. 2-40 to 2-43](https://bitsavers.trailing-edge.com/pdf/ibm/pc/ps2/PS2_and_PC_BIOS_Interface_Technical_Reference_Apr87.pdf)
- [SDL2](https://github.com/libsdl-org/SDL/tree/SDL2)
- [ymfm](https://github.com/aaronsgiles/ymfm)
