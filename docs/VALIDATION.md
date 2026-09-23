# Native port validation — 2026-09-23

Status: runnable development build. Full fidelity and complete gameplay are
not yet certified. The original DOS program was neither launched nor modified.

## Verified

- VGA starts automatically without loading SELECT.BIN or sending a selection
  key. A one-second capture already shows the original VGA title screen.
- Held arrows and keypad directions use the original movement cadence. The SDL
  input scenario moves x=160 → 166 → 172 → 178 without repeat events, stops at
  178 after release, moves left, then stops at 160 after focus loss. Simultaneous
  opposite directions settle to neutral, and release discards queued desktop
  repeats. Captures are checked by
  `tests/verify-movement.py`.
- Poker's original card comparator at 0106:0000 is recovered. The new test
  reproduced the reported crash before the fix and passes all 120 permutations
  of a five-card hand afterward, checking descending rank, record identity and
  stack preservation through the original qsort and far callback.
- Normal input navigation reaches the saloon, approaches the poker table and
  selects Play. A hand deals successfully and reaches the Discard prompt, beyond
  the reported crash; cash is $240 and the pot $20 after the original ante.
  This checks dealing, not every poker outcome or a complete hand.
- The prospector icon is embedded in the Windows PE as seven icon sizes plus
  an icon group. An actual SDL/X11 window exposes the 64x64 version with
  transparent corners. New artwork and the generation prompt are in resources/.
- Native Linux executable, SDL2 display and audio, original VGA title/credits,
  town rendering, cursor-key movement, mouse selection and F6 save/load menus.
- New native save, movement away from the saved position, then restoration
  through the original load menu. Captured x changes 160 → 52 → 160; x, y,
  town scroll and building fields match exactly after loading. The 6,066-byte
  saved game retains its NATIVEQA name. Writes stay in a separate save tree.
- Loading supplied DOS save slot 1 restores its Thursday game state; its health
  panel displays the stored food/water/health values.
- Walking through the Mercantile door, selecting bread, and buying it changes
  cash from $250 to $240. The original shop item grid and dialogs render.
- Transition from the town to the desert; this is a scene-transition smoke
  check, not a full exploration or completion test.
- Original AdLib detection succeeds, LDM.TIM and original MUS files are read,
  and the FM synthesizer produces nonzero audio samples. Listening on actual
  speakers and comparison with original hardware remain pending.
- All 21 packed assets decode byte-for-byte identically in the independent
  readable C++ decoder and the AOT-compiled original routine. Truncation is
  rejected for each asset.
- 1,315,840 independently computed arithmetic/flag cases pass, with additional
  carry-preservation and signed multiply/divide boundary checks.
- Windows cross-build produces a PE32+ x86-64 executable. Imports are SDL2.dll,
  KERNEL32.dll and Windows UCRT APIs. No DOSBox or DOS runtime is linked.

Runtime screenshots are captured from the native SDL framebuffer on Linux.
They are not Windows screenshots. Timed scripts use real elapsed time; the
headless recovery probe's accelerated synthetic clock is not fidelity evidence.

## Outstanding before release

- Run the packaged executable on an actual Windows 11 PC, including audio,
  mouse coordinates, fullscreen, keyboard repeat and writable save location.
- Complete an end-to-end playthrough covering further purchases, food/water, travel,
  mining, fishing, poker, combat, death/reset and the ending.
- Compare visuals, timing and sound against an independently captured original
  reference. Retaining original code reduces divergence but does not prove
  that every native platform service is exact.
- VGA is the default supported configuration; the original graphics selector
  is skipped. Physical joystick input and macOS remain unvalidated.
- Additional indirect call targets may need recovery when unexplored paths
  execute. Such a path fails explicitly; there is no interpreter fallback.
- Several unused DOS services retain minimal implementations (allocation,
  device metadata and mouse range services). Extend them if an actual game
  path requires their full behavior.
- The supplied save slots 3–5 are short or empty, unlike the 6,066-byte slots
  1–2 and native saves. They are preserved, not repaired or asserted valid.

The checked Windows test host available during development identifies itself
as Windows Server 2025, not Windows 11. No Windows 11 runtime claim is made.

## Reproduce focused checks

```sh
make -j4 build/ldm-native build/test-assets build/test-arithmetic build/test-poker
build/test-arithmetic
build/test-assets /nas/tmp/LDM/LDMG
build/test-poker
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/movement-saves --seconds 23 \
  --script tests/scripts/held-movement.txt
python3 tests/verify-movement.py
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/roundtrip-saves --seconds 43 \
  --script tests/scripts/save-roundtrip.txt
```

Scripts write diagnostic BMP frames and selected recovered state fields under
`captures/`. Normal interactive play does not write those diagnostic captures.
Arithmetic and asset checks need no SDL window or DOS environment.

`tests/scripts/poker.txt` walks to the saloon and selects Play. Its opponent is
chosen by the original game's random state and is sometimes absent; check the
captured invitation and dealt hand rather than treating a zero exit as proof
that poker was exercised.
