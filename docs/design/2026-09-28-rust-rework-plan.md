# Rust rework — implementation plan

> **For agentic workers:** execute with superpowers:executing-plans; independent
> tasks may be dispatched with superpowers:subagent-driven-development. Steps use
> checkbox (`- [ ]`) syntax for tracking. Each milestone gets its own detailed plan
> in `docs/design/plans/` immediately before it starts; this file fixes the
> milestones, shared interfaces and gates.

**Goal:** rebuild Lost Dutchman Mine's host in Rust around a deterministic core,
proven identical to the current C++ build, then replace translated routines with
readable Rust proven equivalent in lockstep.

**Architecture:** see `2026-09-28-rust-rework-design.md`. A Cargo workspace of
single-purpose crates; the C++ build becomes a deterministic oracle whose traces
(per-100 ms state hashes) the Rust build must reproduce exactly.

**Tech stack:** Rust 1.94 (edition 2024), iced-x86, sdl2 0.38, sha2, thiserror,
anyhow, cc + vendored ymfm, insta, serde_json. C++17/Python for the oracle only.

---

## Gates

A milestone is complete only when its gate command passes on a clean checkout
with `LDM_EXE` and `LDM_DATA` set (the game directory, e.g. `.local/ldm-data`).

| Milestone | Gate |
| --- | --- |
| M0 | `python3 tools/record_traces.py --check` reproduces every committed golden trace twice |
| M1 | `cargo test -p machine -p testkit` including the hardware vectors |
| M2 | `cargo xtask verify --boot` matches the golden trace of the startup scenario |
| M3 | `cargo xtask verify` matches every golden trace and passes every ported assertion and native test |
| M4 | `cargo xtask verify` plus the desktop checks; Windows build links; C++ removed |
| M5 | `cargo xtask verify --lockstep` with the asset decoder replaced |

## Shared interfaces

These names are fixed across milestones; detailed plans must not rename them.

```rust
// machine
pub struct Address { pub segment: u16, pub offset: u16 }       // program-relative
pub struct Registers { pub ax: u16, pub bx: u16, pub cx: u16, pub dx: u16,
    pub si: u16, pub di: u16, pub bp: u16, pub sp: u16,
    pub cs: u16, pub ds: u16, pub es: u16, pub ss: u16, pub ip: u16, pub flags: u16 }
pub struct Machine { pub regs: Registers, pub memory: Memory, pub ports: Ports,
    pub pit: Pit, pub opl: Opl, pub palette: Palette, /* … */ }
pub enum Cond { Equal, NotEqual, Below, AboveOrEqual, BelowOrEqual, Above,
    Less, GreaterOrEqual, LessOrEqual, Greater, Sign, NotSign,
    Overflow, NotOverflow, Parity, NotParity, CxZero }

// translated code
pub enum Flow { Yield, Wait, Exit, Unrecovered(Address) }      // returned per block run

// patches
pub struct Patch { pub name: &'static str, pub site: Address,
    pub expect: &'static [u8], pub action: Action }
pub enum Action { Before(Hook), Replace(Hook), SkipTo(Condition, u16), Routine(Routine) }

// engine
pub trait Program { fn step(&mut self, m: &mut Machine, g: &mut GameState) -> Flow; }
pub struct Ticks(pub u64);                                     // PIT input clocks
pub struct InputEvent { pub at_ms: u64, pub kind: InputKind }
pub enum InputKind { Key(u32), Release(u16), Directions(u8), Space(bool),
    Mouse { x: i32, y: i32 }, Buttons(u8), Clear, Qol(bool) }
```

Trace line (C++ and Rust, byte-identical):

```
ms=<emulated ms> blocks=<boundaries> ticks=<BIOS ticks> hash=<016x FNV-1a 64>
```

Hash input, in order: `ax bx cx dx si di bp sp cs ds es ss ip flags` as
little-endian u16; the 1 MiB memory; the 256 palette entries as little-endian u32.

Quantum (C++ deterministic mode and Rust, identical order):

1. Apply the input events stamped for this millisecond, in script order.
2. `pit_clocks += 1193182*(q+1)/1000 - 1193182*q/1000`; while
   `pit_clocks >= period` (divisor, 0 means 65536): subtract, fire the timer.
3. Advance the OPL chip clock by `3579545*(q+1)/1000 - 3579545*q/1000`.
4. Set the joystick movement byte (0 while panning).
5. Run up to 4,096 blocks; stop early when waiting or not running.
6. If panning started or stopped, clear movement, Space, keys and mouse.
7. Update the speaker frequency.
8. Every 100th quantum, and at the end, append a trace line.

Emulated time also drives DOS date/time (fixed base 2026-01-01T12:00:00Z unless a
live session sets the wall clock at start) and mouse-click expiry (1,000 ms).

Scenario manifest `tests/scenarios.json` (read by Python and Rust):

```json
{ "name": "held-movement", "script": "held-movement.txt", "seconds": 23,
  "settings": false, "config": {"qol": 1, "startup": 0},
  "fixture": null, "verify": ["tests/verify-movement.py"] }
```

### What changed on the way

The interfaces above were the plan; the code differs where the work showed a
simpler or more faithful shape.

- **Program and flow.** `Program::step(&mut self, m) -> Result<(), Stop>` runs
  one translated step; blocks hand back `Next::{Goto, Yield}` inside `game`, so
  `Flow` and a `GameState` parameter were not needed. The engine queues
  `InputKind`s per quantum rather than timestamped `InputEvent`s: the desktop
  and the scenario runner already deliver input at its millisecond.
- **Patch actions.** `SkipTo` was not needed: a hook returning
  `After::Goto(offset)` covers every conditional skip. `Routine` marks a
  routine's entry instead of replacing its instructions, so the translation
  stays available for lockstep.
- **Lockstep.** A translated routine yields as its original did and may span
  many quanta with timer interrupts between them, so running the readable
  routine in the real machine would change the timing. Lockstep instead runs
  both implementations on copies taken at entry, compares them under the
  contract, and leaves the real machine on the translation.
- **The window.** The SDL2 shell is its own crate, `app`, so `desktop` stays
  free of windowing and `testkit` can drive its controller without SDL.
- **Missing LDM_EXE.** The game crate builds without it, with no game to run,
  so lints and every test that needs no game data run anywhere.

---

## M0 — deterministic C++ oracle and golden traces

Files:

- Modify `src/legacy.h`, `src/legacy.cpp`: emulated clock, DOS date/time from it.
- Modify `src/mouse.h`: click expiry measured by `set_time`, no `steady_clock`.
- Modify `src/audio.h`, `src/audio.cpp`: `advance_clock` for emulated time.
- Modify `src/session.h`, `src/session.cpp`: one `quantum()` used by both modes;
  synchronous deterministic mode; trace writer; `state_hash`.
- Modify `src/desktop.cpp`: `--trace FILE` implies deterministic mode.
- Create `tests/clock.cpp`, `tests/trace.cpp`; modify `tests/mouse.cpp`, `Makefile`.
- Create `tests/scenarios.json`, `tools/fixtures.py`, `tools/record_traces.py`,
  `tests/golden/*.trace`.

### Task 0.1: Emulated clock for DOS date and time

- [x] Write `tests/clock.cpp`: with `State` defaults, `int 21h AH=2Ah` returns
  2026-01-01 (Thursday, AL=4) and `AH=2Ch` returns 12:00:00.00; after
  `emulated_ms=3723450` it returns 13:02:03.45; `clock_base` of 1 Mar 2027
  00:00:00 returns that date. Add a `build/test-clock` Makefile target.
- [x] Run `make build/test-clock && build/test-clock`; expect a compile failure
  (`emulated_ms` missing).
- [x] Add `int64_t clock_base=1767268800; uint64_t emulated_ms=0;` to `State`;
  compute date/time in `interrupt` from `clock_base+emulated_ms/1000` with a UTC
  civil-from-days conversion (no `localtime`), hundredths `emulated_ms%1000/10`.
- [x] Run the test; expect PASS. Run `build/test-console`; expect PASS.
- [x] Commit: "Derive DOS date and time from emulated time".

### Task 0.2: Mouse click expiry in emulated time

- [x] Change `tests/mouse.cpp` lines 41–44 to use `set_time(0)` for the press,
  then `set_time(2000)` before polling; expect the stale click to expire and the
  held button to survive, exactly as before.
- [x] Run; expect a compile failure (`set_time` missing).
- [x] Replace `Clock::time_point` with `uint64_t` milliseconds in `MouseInput`;
  add `set_time(uint64_t)`; `buttons(mask)` stamps with the current time;
  `poll()` expires entries older than 1,000 ms.
- [x] Run `build/test-mouse`, `build/test-combat`, `build/test-qol`; expect PASS.
- [x] Commit: "Measure mouse click expiry in emulated time".

### Task 0.3: One quantum for both session modes

- [x] Write `tests/trace.cpp`: two independent `State`s loaded from the recovered
  image, each driven by `Session` in deterministic mode for 3,000 quanta, produce
  identical trace lines; changing one memory byte changes `state_hash`.
- [x] Run; expect a compile failure.
- [x] Add `Audio::advance_clock(unsigned)`. Extract the loop body of
  `Session::run` into `Session::quantum(uint64_t pit_clocks,uint64_t opl_clocks)`
  following the shared quantum order; threaded mode converts wall time to clocks
  and calls it; add `Session(State&,Mode)` with `Mode::Deterministic` (no thread,
  `step()` runs quantum `q`, `snapshot()` reads state directly) and
  `trace(std::ostream&)`; add `uint64_t state_hash(const State&)`.
- [x] Run `build/test-trace`; expect PASS. Run the full native test list from
  `docs/VALIDATION.md`; expect PASS.
- [x] Commit: "Run the simulation in reproducible quanta".

### Task 0.4: Deterministic desktop runs

- [x] Add `--trace FILE`: no audio device, virtual clock advancing 1 ms per loop,
  script events applied at their virtual time, session stepped once per
  unpaused millisecond, frames rendered only for `screen`/`--screenshot`.
- [x] Run `display-menu.txt` twice with `--trace`; expect identical files and
  `tests/verify-display-menu.py` PASS.
- [x] Commit: "Add deterministic trace runs to the desktop".

### Task 0.5: Scenario manifest, fixtures and golden traces

- [x] Write `tests/scenarios.json` for every script except `saloon-sleep`
  (fixture has no generator), with durations from the scripts, fixtures named by
  generator: `combat` (make-combat-fixture.py), `river` (make-river-fixture.py),
  `assay` (test-assay), `map` and `mining` (test-cave), `pan-owned` and
  `pan-missing` (test-pan-inventory).
- [x] Write `tools/fixtures.py`: build each fixture into `.local/fixtures/<name>`
  from `$LDM_DATA`, twice, and fail unless both builds are byte-identical.
- [x] Write `tools/record_traces.py`: for each scenario create a fresh temp dir,
  copy the fixture, write the config, run `build/ldm-native --trace`, run its
  verifiers against the temp dir, write `tests/golden/<name>.trace`; `--check`
  compares instead of writing. Run scenarios in parallel.
- [x] Record; then `--check` twice; expect identical results and every verifier
  PASS. Fix scripts whose timing assumed wall-clock pacing, noting each change.
- [x] Document deterministic runs and golden traces in `docs/VALIDATION.md`.
- [x] Commit: "Record golden traces for every scenario".

## M1 — machine, IR and hardware vectors

- [x] 1.1 Workspace skeleton: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`,
  `clippy.toml`, `xtask` with `lint` (fmt, clippy pedantic, raw-address check).
- [x] 1.2 `machine`: `Registers`, flags, `Memory` (masked 20-bit), `alu`, `shift`,
  `multiply`, `divide`, stack, string operations, ports, PIT state; unit tests
  ported from `tests/arithmetic.cpp` (1,315,840 cases).
- [x] 1.3 `ymfm-sys` + `machine::Opl` (register/timer/status state, clock advance).
- [x] 1.4 `translate::ir` and `translate::lower` from iced-x86 for every mnemonic
  in `translate.py`; `testkit::interp` executes IR on `Machine`.
- [x] 1.5 SingleStepTests 8088 vectors (downloaded by `xtask vectors` into
  `.local/vectors`) for every opcode LDM uses; compare defined flags only; record
  deliberate differences with reasons.
- [x] 1.6 FNV state hash identical to the C++ `state_hash` (fixture test).

## M2 — translator emits Rust

- [x] 2.1 `translate::image`: SHA-256 check and EXEPACK unpack (port `unpack.py`),
  tested on synthetic packed data.
- [x] 2.2 `translate::recover` (port `analyze.py`) and `game/entry_points.toml`.
- [x] 2.3 `patches` crate: every site from `translate.py`, as data.
- [x] 2.4 `translate::emit`: blocks as functions, segment dispatchers, annotations;
  `insta` snapshots on synthetic programs.
- [x] 2.5 `game/build.rs` and a headless boot: the startup scenario matches its
  golden trace.

## M3 — DOS, game, engine

- [x] 3.1 `dos`: every service in `legacy.cpp::interrupt`, file paths and
  copy-on-write, BIOS data area and load layout.
- [x] 3.2 `game`: `symbols.rs`, hooks from `legacy.cpp`, `GameUi` overlay,
  frame composition from `session.cpp::read_frame`.
- [x] 3.3 `engine`: quantum loop, inputs, frames, audio events, traces, savestates.
- [x] 3.4 `testkit` scenario runner and controller-level input mapping (from
  `desktop.cpp` and `keyboard.h`); every golden trace matches.
- [x] 3.5 Port the scenario verifiers and the native tests to Rust tests.

## M4 — desktop parity and C++ removal

- [x] 4.1 `desktop`: settings file, menu model (from `display.cpp`), presentation
  (scaling, colour, CRT), SDL window and renderer, audio output from events.
- [x] 4.2 Windows cross-build (x86_64-pc-windows-gnu, Zig as linker, SDL2 MinGW).
- [x] 4.3 Packaging (`xtask package`), README and VALIDATION updates.
- [x] 4.4 Delete `src/`, the C++ tests, `Makefile`, `CMakeLists.txt` and the
  Python translator; golden traces remain the reference.

## M5 — lockstep and the first decompiled routine

- [x] 5.1 Stack low-water tracking and per-routine live-out register analysis.
- [x] 5.2 Lockstep mode in testkit; `Action::Routine`.
- [x] 5.3 Decompile the asset decoder routine into `game::decompiled::assets`;
  lockstep passes on every scenario; release build uses it.
