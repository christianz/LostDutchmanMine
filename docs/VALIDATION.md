# Native port validation — 2026-09-23

Status: runnable development build. Full fidelity and complete gameplay are
not yet certified. The original DOS program was neither launched nor modified.

## Verified

- Update 4 adds WASD and completes keypad input. The original movement/text
  helper test checks all four WASD keys with Shift/Caps Lock, all eight keypad
  directions with Num Lock on/off, digit/slot scan codes and keypad Enter.
  The desktop scenario switches from the hand back to walking with D, moves
  continuously without OS repeat, combines W+A, uses keypad 6/4/9, and stops
  correctly on release, opposing directions and focus loss. Queued keypad
  repeats are discarded even if Num Lock changes before release.
- WASD/keypad navigation also changes CRT/colour settings and confirms with
  keypad Enter. A save/load scenario selects slot 8 using the keypad, enters
  `WASD42` using WASD letters and keypad digits, confirms with keypad Enter,
  walks away with A and reloads with keypad 8. The original 6,066-byte save and
  its name survive; player x changes 160 -> 70 -> 160 and all position fields
  match after loading. Existing arrow/focus/repeat and quick mouse/quit checks
  also pass. These are native Linux tests; the Windows x64 cross-build passes,
  with independent Windows 11 keyboard testing still outstanding.
- Update 3 reproduces missed mouse activation: after keyboard movement, a press
  and release in the same event batch leave mouse mode 1 and visibility -1, even
  after further quick clicks. The fix retains both transitions and a coherent
  click position across the original helper's three BIOS calls. The same script
  now restores the hand (mode 0, visibility 0), opens F6 and exits through Quit
  Game using one quick click each, with no movement of the player.
- At 3840x2160 with Classic CRT and an 85% picture, an actual X11 focus switch
  followed by one 5 ms mouse click restores the hand. Two further 5 ms clicks on
  F6 and Quit exit normally at 22.73s. This checks Linux input/rendering;
  independent Windows 11 focus/DPI behaviour remains unvalidated.
- The original mouse-helper test checks quick press/release, FIFO order, coherent
  coordinates despite interleaved movement, held buttons, focus clearing, game
  pointer positioning and expiry of clicks made while the game was not polling.
- CRT Off/Soft/Classic render the same captured scene at 4K through SDL/OpenGL.
  Checks cover disabled identity, both strengths, scanlines, RGB phosphors,
  edge shading, local highlight glow, source preservation, persistence and
  malformed settings. CRT preview, Cancel and Apply also run through the UI.
  These are spatial effects; no flicker, frame interpolation or curvature is used.
- Update 2 reproduces the Quit Game crash at 1613:1c7d using F6 and the original
  Quit button. It recovers graphics cleanup and the subsequent C file cleanup
  callback at 13b4:0752, and implements the observed BIOS text scan-line restore.
  The final scenario exits normally in 21.66 seconds before its 30-second limit,
  restores mode 3 and the default timer vector, and reaches DOS exit 13b4:02d7.
- The new display menu previews scaling/colour choices, persists settings,
  optionally skips itself at startup, and reopens with F11. The scripted test
  selects Warm/90% brightness, disables startup, moves, cancels a draft filter,
  then applies Pixel art. Player fields remain unchanged across settings; two
  snapshots show identical original execution counts while F11 is open, and
  execution resumes afterward. Movement and the save/load round trip were also
  rerun after the simulation/presentation thread split and passed.
- Full 3840x2160 rendering and actual mouse input pass in an SDL/OpenGL X11
  virtual desktop. At 85%, the 4:3 picture is 2448x1836 at (696,162). An actual
  F6 key and mouse click at desktop (2264,1117) select the original Quit button
  and exit normally. Xvfb has no window manager, so the harness explicitly gives
  the SDL window its requested desktop geometry and focus. This is not a
  physical monitor, high-DPI Windows scaling, or Windows 11 runtime test.
- Display unit checks cover configuration replacement and malformed values,
  4K centring/aspect/mouse geometry, rejection of border clicks, pixel-for-pixel
  Original/Crisp output, source preservation, Scale2x diagonal behaviour and
  colour grading. Settings pause play and audio; no new animation frames are
  generated and no claim is made about resolving motion sickness.
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
make -j4 build/ldm-native build/test-assets build/test-arithmetic build/test-poker build/test-quit build/test-display build/test-mouse build/test-keyboard
build/test-arithmetic
build/test-assets /nas/tmp/LDM/LDMG
build/test-poker
build/test-quit
build/test-display
build/test-mouse
build/test-keyboard
python3 tests/verify-quit.py
python3 tests/verify-mouse.py
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/movement-saves --seconds 23 \
  --script tests/scripts/held-movement.txt
python3 tests/verify-movement.py
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/keyboard-controls/Saves --config .local/keyboard-controls/display.ini \
  --seconds 33 --script tests/scripts/keyboard-controls.txt
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/keyboard-save/Saves --seconds 43 \
  --script tests/scripts/keyboard-save.txt
python3 tests/verify-keyboard.py
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/display-test/Saves --config .local/display-test/display.ini \
  --settings --seconds 25 --script tests/scripts/display-menu.txt
python3 tests/verify-display-menu.py .local/display-test/display.ini
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy build/ldm-native \
  --data /nas/tmp/LDM --image recovered/load-image.bin \
  --saves .local/roundtrip-saves --seconds 43 \
  --script tests/scripts/save-roundtrip.txt
```

Scripts write diagnostic BMP frames and selected recovered state fields under
`captures/`. Normal interactive play does not write those diagnostic captures.
Arithmetic and asset checks need no SDL window or DOS environment.
Use a fresh config path for the display-menu script. `screen` script events and
`--screenshot` capture presented output; `capture` retains the original 320x200
framebuffer and selected state fields. Timed/scripted runs skip startup settings
unless explicitly launched with `--settings`.
The keyboard-controls script also needs a fresh config. Its optional final
column on `down`/`up`/`repeat` supplies SDL modifier bits (4096 is Num Lock).
`tests/scripts/crt-menu.txt` previews both CRT strengths, saves Classic and
checks Cancel/Apply from F11. Run it with a fresh config, `--settings --seconds 4`.

`tests/scripts/poker.txt` walks to the saloon and selects Play. Its opponent is
chosen by the original game's random state and is sometimes absent; check the
captured invitation and dealt hand rather than treating a zero exit as proof
that poker was exercised.
