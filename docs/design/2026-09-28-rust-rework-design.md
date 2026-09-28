# Rust rework — design

Status: approved 2026-09-28. Implementation follows `2026-09-28-rust-rework-plan.md`.

## Goal

Extremely readable code that stays provably faithful to the original game.

- **Phase 1:** rebuild the host (runtime, translator, desktop) in Rust with clean
  layers and a deterministic core. The translated game remains generated code.
- **Phase 2:** replace translated routines, one at a time, with readable Rust,
  each proven equivalent to the translated original.

Decisions: Rust; everything in the public repository; migrate in parallel with the
current C++ build acting as the oracle until the Rust build matches it exactly.

## Architecture

A Cargo workspace. Each crate owns one concern; dependencies point one way.

| Crate | Owns | Must not know about |
| --- | --- | --- |
| `machine` | 8086 registers and flags, ALU and shifts, 1 MB memory, I/O ports, PIT, OPL chip state | files, clocks, the game |
| `dos` | DOS and BIOS services, file handles behind a `Host` trait, keyboard buffer, mouse driver, video modes | the game |
| `patches` | The patch table as data: site, expected bytes, action, hook id | how hooks are implemented |
| `translate` | Build-time: EXE check and EXEPACK unpack, control-flow recovery, IR, patch application, Rust emission | runtime |
| `game` | Symbols (named addresses), hook implementations, QoL overlay, rules, translated code from `OUT_DIR`, later decompiled routines | the desktop |
| `engine` | Deterministic session: `advance`, input events, frames, audio events, replays, savestates, traces | wall clock, SDL |
| `ymfm-sys` | The only `unsafe`: a thin binding to the vendored ymfm OPL emulator | everything else |
| `desktop` | SDL2 window, settings menu, presentation, audio output, input mapping, settings file | machine internals |
| `testkit` | Scenario runner, golden traces, IR interpreter, hardware vectors, lockstep | — |

`machine ← dos ← game ← engine ← desktop`; `patches` is shared by `translate` and
`game`; `translate` runs from `game/build.rs`. The engine is generic over a
`Program` trait so everything except `game` builds and tests without game data.

## Determinism

Time is emulated, never measured. The engine counts PIT input clocks
(1,193,182 Hz) and advances in 1 ms quanta. Each quantum applies the input events
stamped for it, fires due timer interrupts, then runs a fixed budget of 4,096
translated blocks or until DOS reports that the game waits for input.

The current C++ build has four sources of nondeterminism, all removed in
milestone M0 before golden traces are recorded:

1. The simulation thread converts wall-clock time into timer interrupts.
2. OPL timers advance from the audio callback, and the game reads OPL status.
   Deterministic mode advances the chip clock from port accesses and emulated
   time only and opens no audio device.
3. DOS date/time services read the system clock. Deterministic mode uses a fixed
   clock base plus emulated time; replays record the base.
4. Pending mouse clicks expire after one second of `steady_clock` time. They
   expire after one second of emulated time instead.

Inputs are `InputEvent { at, kind }`. Outputs are a `Frame` and timestamped
`AudioEvents`; audio never feeds back into the simulation. Pausing is not advancing.

Traces hash registers, flags and the full 1 MB memory (FNV-1a 64) every 100 ms of
emulated time and at the end of a scenario. Hashes contain no game content and
are committed as golden files. On mismatch, testkit bisects to the first diverging
millisecond and prints changed registers and memory labelled from `symbols`.

Savestates hold machine, DOS (open files by path and position) and patch state in
a versioned binary format. Restore-then-continue must reproduce the hashes of an
uninterrupted run.

## Translator

`LDM_EXE=/path/to/LDM.EXE cargo build` runs `game/build.rs`:

1. `Image::load` checks SHA-256 `de0726a1…25e07` and unpacks EXEPACK.
2. `recover` walks control flow from the entry point, relocated far references and
   `game/entry_points.toml` (each entry with its evidence).
3. `lower` turns each decoded instruction (iced-x86) into one IR node.
4. `apply` inserts patches, checking expected bytes and instruction starts.
5. `emit` writes basic blocks as small Rust functions and one dispatcher per
   segment, each line annotated with its original address and instruction.

Blocks start at every resume point: jump targets, return addresses, interrupts
that may wait, and patch sites. Forward jumps continue inside the segment;
backward jumps, calls, returns and waits hand control back, exactly like the C++
emitter, so block budgets line up with the oracle.

Patch actions are `Before(hook)`, `Replace(hook)`, `SkipTo(condition, target)`
and `Routine(routine)`. Hooks and routines are enums mapped to functions by an
exhaustive `match` in `game`.

## Phase 2

A decompiled routine is a patch whose action replaces a whole routine.

Its contract: all memory except the dead stack (below the entry stack pointer,
down to the lowest stack pointer either implementation reached), plus every
register that any caller reads after the call before overwriting it. The
translator computes those registers per routine.

In lockstep mode testkit runs both implementations at every call: the translated
routine in a clone, the readable one in the real machine. It compares the
contracts, then continues from the translated state, so whole-run golden hashes
stay identical while every call is verified. Release builds run only the readable
routine; replays record the build ID.

Readable routines use typed views over `symbols`; raw addresses are rejected by
`cargo xtask lint` outside `symbols.rs` and the patch table.

## Errors

- Build time: missing `LDM_EXE`, wrong SHA-256, unsupported instruction or a
  mismatched patch are compile errors with the address and what was found.
- Runtime: unrecovered code, unsupported services and CPU faults become typed
  `Fault`s with the address and the last 64 block addresses. The desktop shows a
  dialog and writes a crash bundle (savestate, replay, trace).
- `machine`, `dos` and `engine` return typed errors (`thiserror`). A panic is a
  bug. `anyhow` is used only by `desktop` and `xtask`.

## Testing

Without game data (CI): SingleStepTests 8088 hardware vectors through `lower` and
the IR interpreter; translator snapshot tests (`insta`); menu model, text fit,
scaling and CRT; savestate round trips with a synthetic `Program`.

With `LDM.EXE` (`cargo xtask verify`): every scenario against golden hashes,
scenario assertions ported from `tests/verify-*.py`, the native routine tests
ported from `tests/*.cpp`, lockstep for replaced routines, savestate checks.

## Definition of beautiful

- rustfmt; `clippy::pedantic` as errors with a short justified allow list.
- `#![forbid(unsafe_code)]` everywhere except `ymfm-sys`.
- `#![deny(missing_docs)]`; every module opens with what it owns.
- No raw game addresses outside `symbols.rs` and the patch table.
- Files under about 300 lines; names explain, comments give reasons.
- Generated code is formatted, annotated and exempt from lints at module level only.
- Dependencies: `iced-x86`, `sdl2`, `sha2`, `thiserror`, `anyhow` (desktop, xtask),
  `cc` (ymfm), `insta` and `serde_json` (tests).

## Public repository guardrails

No game bytes are committed. Golden hashes carry no game content. Replays are
input logs and safe to share; savestates and crash bundles contain game memory and
stay local.

## Milestones

- **M0** deterministic C++ oracle and golden traces for every scenario.
- **M1** `machine`, IR, interpreter and hardware vectors.
- **M2** `translate` emits Rust; a headless run matches golden hashes.
- **M3** `dos`, `game`, `engine` complete; all scenarios and native tests pass; savestates.
- **M4** desktop parity (menu, presentation, audio, input, Windows); the C++ build is deleted.
- **M5** lockstep, then the first decompiled routine: the asset decoder.
