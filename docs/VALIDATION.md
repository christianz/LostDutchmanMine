# Native port validation — 2026-09-23

Status: runnable development build. Full fidelity and complete gameplay are
not yet certified. The original DOS program was neither launched nor modified.

## Verified

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
- VGA is the supported test configuration. Other original graphics choices,
  physical joystick input and macOS remain unvalidated.
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
make -j4 build/ldm-native build/test-assets build/test-arithmetic
build/test-arithmetic
build/test-assets /nas/tmp/LDM/LDMG
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/roundtrip-saves --seconds 43 \
  --script tests/scripts/save-roundtrip.txt
```

Scripts write diagnostic BMP frames and selected recovered state fields under
`captures/`. Normal interactive play does not write those diagnostic captures.
Arithmetic and asset checks need no SDL window or DOS environment.
