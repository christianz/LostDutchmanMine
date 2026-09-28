# Native port validation — 2026-09-29

Status: runnable development build. Full fidelity and complete gameplay are
not yet certified. The original DOS program was neither launched nor modified.

## The Rust build

The port was rebuilt in Rust against the earlier C++ port as its oracle. The
C++ build was first made deterministic (emulated time for the timer, the OPL
chip, DOS date and time and mouse-click expiry), then recorded 29 scenarios;
the Rust build reproduces every one exactly.

- **Golden traces.** Each scenario in `tests/scenarios.json` replays desktop
  input from `tests/scripts/` through the same controller the window uses.
  Every 100 emulated milliseconds, and at the end, the FNV-1a 64 hash of the
  registers, all 1 MiB of memory and the palette equals the C++ line in
  `tests/golden/`: all 29 scenarios, line for line.
- **Captures.** Every `capture` of every scenario, all report fields and every
  pixel of the frame, was compared once with the C++ captures and matched.
- **Checks.** The scenario verifiers are ported to Rust (`crates/testkit/src/
  checks/`) and pass. Assertions that compared a build before a fix, and a few
  written for hand-made saves the generated fixtures do not reproduce, which
  the C++ build's own captures fail too, are left out; each module says which.
  `verify-sleep` is not ported: its scenario has no fixture generator.
- **Startup.** A headless boot reproduces the first 18.4 seconds of the
  held-movement trace, before its first input.
- **Savestates.** A state saved halfway through that startup and restored into
  a freshly booted game continues on the golden trace.
- **Readable routines.** The asset decoder (1265:1250) runs as readable Rust in
  play. In verification every call also runs translated on a copy of the
  machine, and both must agree on all memory outside the dead stack, the
  registers the translator's liveness analysis finds callers read (all but DS
  and ES), and every flag: all 21 shipped assets, and the 160 calls across the
  scenarios.
- **The CPU.** The IR is checked against the SingleStepTests 8088 hardware
  vectors for every opcode the game uses, and the ALU against the C++ port's
  1,315,840 arithmetic cases.
- **The desktop.** Settings files, 1.2 million menu transitions, every colour
  under every profile and brightness, every scaling filter and the CRT mask and
  glow were compared byte for byte with the C++ `display.cpp`.
- **Windows.** `cargo xtask windows` produces a PE32+ x86-64 GUI executable on
  the Universal C Runtime whose only non-system import is SDL2.dll. It has not
  been run on Windows here.

```sh
cargo xtask lint                     # rustfmt, pedantic clippy, named addresses
cargo xtask test                     # everything that needs no game data
LDM_EXE=/path/to/LDM.EXE LDM_DATA=/path/to/LDM cargo xtask verify
```

The sections below record the C++ port's validation, update by update. Its
behaviour is what the golden traces pin down, so they hold for the Rust build.

## Verified after update 16: settings menu

- The settings menu has seven rows. VSync is always requested, because
  presentation is already paced to the monitor's refresh rate; the option only
  allowed tearing. Picture size is part of Display (Fullscreen, Fullscreen 85%,
  Fullscreen 70%); windows always show the whole picture. Old `display.ini`
  files load, their `vsync` key is ignored and no longer written.
- CRT strengths are Subtle and Strong, the comfort preset is Comfort, and QoL is
  an On/Off row like Show at startup. Help text names QoL's walking and mule
  changes, Quit/Play and Cancel/Apply describe what they do, and the key hint
  names Esc instead of F11, which does nothing inside the menu.
- `test-display` checks rows, Display cycling in both directions, windowed and
  fullscreen picture sizes, old settings files, value names, presets, help text
  and that every label, value, button, help line and key hint fits its box using
  the baked font metrics. The display-menu, CRT-menu and keyboard-controls/save
  scripts pass with fresh configs; captures show the startup and F11 menus.

## Verified in update 16

- QoL aiming now composites the original crosshair at display cadence instead
  of waiting for the encounter's six-tick loop. Native tests cover Native
  American and wanted-criminal fights entered from both hand and key modes.
  They compare the sight pixel-for-pixel with the original blitter, verify
  movement without advancing simulation or ammunition, hide it under menus,
  and hit a target through the original shooting routine.
- Before/after Linux SDL captures sample twelve mouse moves after 25 ms. All
  twelve new captures show the requested point; the old build is behind in ten.
  Nine new captures show the updated sight before the simulation commits aim.
  The existing combat script also passes quick/held firing, keyboard aiming,
  Space, right-click selection, F11 pause/resume and focus-loss checks.
- QoL uses an outlined arrow with its tip at the input coordinate and ignores
  DOS cursor parking. Classic mode retains its hand, hotspot and warps. The
  original selector's setup reproduces parking at (240,140) in classic mode
  and preserves the actual mouse point with QoL. Numpad desktop runs with Num
  Lock on/off retain the pointer at (65,55) throughout four movements. Those
  particular movement scripts did not reproduce the old snap by themselves.
- Panning now animates in both modes. Original-scene tests check all three
  grades with QoL on/off, all poses, original delays, exactly one deferred
  reward, input cleanup and a preference change during playback. A classic
  desktop run plays the animation and yields one bag despite additional
  keyboard/mouse input during it. Actual pan ownership is still required.
- Holding physical Space continues the original mining loop at its original
  rate in both modes. Native checks perform five strokes, stop on release,
  then perform one tapped stroke. Desktop captures show four then eight
  strokes during one hold, and stopping on release, focus loss and settings.
  Scripted OS key repeats, including one just before release, cannot queue
  additional swings; classic text/menu key-repeat handling is preserved.
  F11 pauses the counter and resume does not retain held Space.
- Twelve native executables pass: combat, combat scene, mouse, keyboard, menu,
  menu scene, cave, assay, QoL, panning, panning scene and pan inventory.
  Eight isolated SDL runs pass `tests/verify-input-update.py` and the existing
  combat verifier. Captures use original game assets and isolated save folders.
- Windows x64 cross-compiles with an explicit GUI subsystem and
  `mainCRTStartup` entry point. PE inspection confirms subsystem 2; `-mwindows`
  alone did not change the subsystem with this Zig toolchain. Fatal interactive
  startup errors retain the existing SDL error dialog. Windows runtime has
  not been tested. No public publication or NAS delivery is implied.

## Verified in update 15

- Pan now requires item 0x0f in the player's tool inventory or an owned mule's
  tool row. The separate equipment count is insufficient: the original shop
  increments it even when a full pack rejects the pan. Both the river's label
  check and the Pan routine now use actual inventory, with QoL on and off.
- The panning test reproduces the previous phantom-pan reward and now checks
  stale positive/zero counts, first/last slots in all four rows, owned/unowned
  mules, carrier/food-table exclusions, full packs and all three gold grades.
- `test-pan-inventory` executes the original rejected full-pack purchase,
  successful purchase and last-pan discard. Mouse and P cannot pan without
  the item, including immediately after discarding it while the river's old
  button is still drawn. Re-entering the river removes the unavailable label.
- Three Linux SDL runs compare identical no-pan saves before/after and a save
  with a real pan. The old build collects two gold bags without the item; the
  fixed build neither animates nor awards gold. A real pan still completes
  both mouse- and P-triggered animations and collects two bags. Captures from
  `tests/scripts/pan-inventory.txt` pass `tests/verify-pan-inventory.py`.
- Panning, panning scene, pan inventory and the full QoL test executable pass.
  Windows x64 cross-compilation passes; Windows runtime remains untested.
  This update also includes update 14's sleep-hover and combat-entry fixes.

## Verified in update 14

- Saloon sleep suspends native context-button and toolbar hover for its entire
  noninteractive routine. The original black screen no longer acquires gold
  outlines or shortcut hints. Mouse and keyboard Sleep tests wake at 09:00,
  restore the saved town doorway and re-enable the toolbar without stale
  Sleep targets. Health/inventory modal checks still pass.
- An armed encounter starts with mouse aiming enabled even if it inherits
  the previous screen's hand mode. Queued entry clicks are discarded; a held
  entry click cannot fire or reopen hand mode over the old panel. New clicks,
  right-click menus, keyboard aiming, Space, and QoL-off behavior remain valid.
- The original scene test now runs both Native American and wanted-criminal
  encounters, each entered from keyboard mode and from hand mode with a held
  panel click. Mouse movement works without a direction-key nudge; a fresh
  click passes original hit testing and consumes exactly one bullet.
- Linux SDL before/after sleep captures reproduce the old gold outlines over
  Exit and Cash and verify none remain during sleep. The toolbar highlights
  again on waking. `tests/verify-sleep.py` checks pixels and saved doorway
  coordinates from `tests/scripts/saloon-sleep.txt`.
- A Linux SDL run loads an isolated, healthy wanted-criminal encounter and
  immediately mouse-aims left/right, fires quick/held clicks, aims by keyboard,
  fires with Space, uses right-click hand mode, and pauses/resumes with F11.
  `tests/scripts/combat-entry.txt` and `tests/verify-combat.py` verify the sight
  coordinates, exactly three bullets spent, and cleared focus/pause input.
- All eleven relevant native executables pass: combat, combat scene, QoL,
  assay, cave, keyboard, menu, mouse, menu scene, panning and panning scene.
  Windows x64 cross-compilation passes; Windows runtime remains untested.

## Verified in update 13

- Corrected an update 12 regression: the shared selector's result 9 also means
  item selection in buildings. Suppressing it globally prevented gold bags
  from reaching the assay routine. The filter now belongs only to the walking
  command poll; building selections retain their original result.
- `test-assay` reproduces the rejected bag click on update 12. With the fix,
  real mouse clicks assay eight bags across the player's pack and all three
  mule rows, with QoL on and off. Checks cover the six original bag types,
  weight/grade rules, exactly one matching cash payout per bag, non-ore items,
  repeated empty-slot clicks, Next/Done/Exit, and the saved town doorway.
- Linux SDL before/after runs load identical isolated assay-office saves.
  Update 12 leaves all bags untouched; update 13 consumes the chosen player
  and mule bags and displays their weight, grade and value. Capture metadata
  verifies cash increments equal weight times grade times 10, and that empty
  slots and Done/Exit do not repeat payouts. The fixture preserves the original
  cash encoding at DS:5b72, from which loading reconstructs the wallet.
- All eight native cave scenarios still pass. A fresh desktop run confirms
  clicking inside stays in the cave and exit restores the exact world-map
  position and scroll. `tests/verify-assay.py` checks the combined desktop
  evidence from `tests/scripts/assay.txt` and `cave-roundtrip.txt`.
- Eleven relevant native test executables pass: assay, cave, keyboard, menu,
  mouse, menu scene, QoL, panning, panning scene, combat and combat scene.
  Windows x64 cross-compilation passes; Windows runtime remains untested.

## Verified in update 12

- Clicking cave scenery with the QoL pointer no longer restarts the scene.
  Both reported symptoms came from that restart: the wilderness entrance
  replayed, and indoor coordinates replaced the saved map return position.
- `test-cave` reproduces the restart before the fix. Its eight scenarios now
  pass: quick/held clicks during entry and inside the cave; resumed-scene
  state with QoL on/off; rejected entry without a lamp or without oil. All
  exits restore exact map X/Y and scroll offsets through the original code.
- A Linux SDL desktop comparison loads identical isolated map saves, presses
  Space, clicks inside the cave, then walks left to leave. Update 11 replays
  the wilderness and returns at (60,50) instead of (135,61). Update 12 stays
  in the cave and restores (135,61), retaining scroll (13,8). Screenshots and
  frame metadata are checked by `tests/verify-cave.py`; input is recorded in
  `tests/scripts/cave-roundtrip.txt`. `test-cave GAME FRESH-DIRECTORY` exports
  the isolated QA fixture without modifying supplied saves.
- Ten relevant native executables pass: cave, keyboard, menu, mouse, menu
  scene, QoL, panning, panning scene, combat and combat scene. Real toolbar
  actions, classic scenery commands and combat input retain their routing.
  Windows x64 cross-compilation passes; Windows runtime remains untested.

## Verified in update 11

- Health and inventory no longer show hover outlines for the scene's hidden
  buttons. Mouse and keyboard status dispatch save/clear/restore the native
  hover targets; inactive toolbar hover is also suspended. Native tests open
  both dialogs through both routes and check that river hover returns on close.
- The held direction mask survives an individual key press or OS repeat. WA,
  WD, SA and SD now retain both axes in the original world-map movement code.
  Native checks cover both last-key orders and repeat, the original terrain
  increments, ordinary taps, Space/menu keys and WASD text entry.
- Linux SDL desktop runs reproduce both issues on update 10 and pass with the
  fixes. Health/inventory have no ghost gold outlines, and real river buttons
  highlight again after closing. All four diagonals move both coordinates under
  sustained key repeat; releasing one key retains the other axis, releasing
  both stops movement, and focus loss clears movement.
- Saloon speed selection now runs on every walking loop, rather than only at
  room entry. Native scene checks cover faster held walking, unchanged movement
  increments, the survival update cadence and restoration on key release.
  The loaded-saloon desktop comparison moves 45 instead of 25 pixels in the
  sampled 0.9-second hold. Over the full 2.7-second observation, survival ticks
  advance by 15 after the fix and 16 before it (the original coarse clock).
  Release and focus loss stop indoor movement.
- SOLD OUT erases the full original name/price area and stops above the shop's
  brown lower border. Rendering checks cover all three mule columns and all
  sequential ownership states, including original border pixel preservation.
- Nine relevant native test executables pass: keyboard, menu, mouse, menu scene,
  QoL, panning, panning scene, combat and combat scene. Windows x64 cross-build
  passes; Windows runtime testing remains outstanding.

## Verified in update 10

- Toolbar buttons now occupy rows 167-198, leaving the full logo and weekday
  intact. Tests retain the original warning colours, flashing and QoL-off pixels.
- With QoL on, the hand is visible during walking. A fresh click is routed to
  the original command selector while held keyboard input continues. One quick
  F6 click opens its menu; no activation click is needed. Combat retains its
  existing aiming/hand controls. Focus reset clears pending clicks.
- Only the cheapest unowned mule can be bought; the other choices say SOLD OUT.
  Tests execute the original purchase routine for every ownership combination
  and choice with QoL on/off (48 cases), checking cash and all three inventory
  rows. Existing non-sequential ownership is preserved.
- The building-entry code now honours the original loaded-scene flag before
  storing the street return position. The saved street X was being overwritten
  with the interior X. An isolated copy of the user's slot 6 reproduces X=210
  outside the saloon before the fix; after loading and clicking Exit the desktop
  restores X=240, Y=59. Down reaches Y=73 and street movement continues. The
  native check covers normal entry and loaded exit with QoL both on and off.
- A desert close-up waits for a fresh Space, Enter or Escape. The dismissal is
  consumed before map input resumes. Native and Linux desktop checks cover two
  Space presses, key repeat, staying open beyond the old timeout, and remaining
  on the map afterward. QoL off retains the original timed preview.
- Held town/saloon/mine movement halves the original walking delay. Original
  collision steps are retained; the per-loop survival update and mine hazard
  RNG run every other fast step. A Linux desktop sample moved 54 rather than
  30 pixels after 0.9 seconds of held input. The 2.4-second observation advanced
  survival ticks by 14 in both modes. Key release and focus loss stop movement.
- All 15 native test executables pass, including the new `test-qol` integration
  checks using the original translated routines and initialized graphics.
  Linux SDL desktop checks pass for the saved saloon, desert toggle, movement
  cadence, quick mouse menu/quit actions, WASD/keypad controls and the WASD42
  save-name/load round trip. Windows x64 cross-compilation
  passes; independent Windows 11 runtime testing remains outstanding.

## Verified in update 9

- Fixed the enhanced toolbar covering live health and food warnings with the
  healthy defaults from the panel atlas. Its icons now use the original live
  VGA framebuffer; the labels, hover feedback and click targets are retained.
  No health calculation, timing or original game memory is changed.
- The regression test reproduces the stuck-green icon before the fix. It boots
  the original game, runs its actual health/food routine (`0652:1d5e`) and checks
  health at 96, 64, 63, 32, 31 and 4, including both critical-flash phases and
  recovery. Food warnings also change and recover. Original pixels with QoL off,
  confined toolbar drawing and no rendering writes to game memory are covered.
- A 28-second Linux desktop check loads an isolated low-health save through F6.
  The labelled portrait is red, matching the original F2 health/food/water panel.
  This check uses the SDL software renderer, with health 31 and food/water 30
  in the fixture; it does not alter player saves.
- The original menu input regression passes. Windows x64 cross-compilation
  passes; independent Windows 11 runtime testing remains outstanding.

## Verified in update 8

- The original panning animation replaces the custom minigame. The native
  translator reconnects two bypassed loops at `033f:0303` and `033f:039e`, using
  the surviving comparisons and original sprite, page, presentation and delay
  routines. No original file or loaded image byte is patched. The signed branch
  conditions are reconstructed from the surrounding code, not verified against
  an unmodified retail executable.
- Original-code checks cover all three gold grades, one reward per action,
  no-pan/full-pack handling and original stack cleanup. A scene test runs the
  initialized sprite/timer routines and observes all three poses, the middle
  return pose, 5-9 cycles, deferred reward, restored player position, cleared
  input and a latched QoL choice. It uses a synthetic clock, not wall-clock timing.
- A 77-second Linux desktop scenario loads an isolated river save through F6,
  clicks Pan on its bevel and plays the animation. F11 pauses execution; keyboard
  and repeated Pan clicks during playback do not move the player or queue an
  extra reward. Walking works afterward. Disabling QoL restores the instant
  reward; enabling it again restores the animation. Original save/load preserves
  three bags, position, the PANGOLD name and the 6,066-byte save format.
- A real X11 mouse click on Pan's bevel starts the restored animation at
  3840x2160 with Classic CRT and an 85% picture. Captures verify animation
  before the reward and one bag afterward, with the original position restored.
  This is Linux/OpenGL on a virtual display, not a physical monitor test.
- All 14 focused native checks pass: arithmetic, assets, poker, quit, display,
  mouse, menu, menu scene, keyboard, console, panning, panning scene, combat and
  combat scene. The Windows x64 cross-build passes. Windows 11 runtime testing
  of this update remains outstanding.
- The custom panning source is removed from the build. Its artwork is retained
  as archived source but is no longer packaged or loaded. QoL, display and save
  configuration formats remain compatible with existing installations.

## Verified in previous updates

The interactive panning and creek-art checks below describe updates 5-7. Those
features are superseded by the restored animation in update 8; they are retained
here as historical validation, not claims about the current panning controls.


- Update 7 adds optional named toolbar icons, hover outlines and F1-F6 hints,
  plus larger targets for the toolbar and contextual button bevels. The original
  selector dispatches all six menus and four context actions in the input tests;
  gaps, absent choices, scene clicks, far returns and QoL-off behavior are covered.
- The new panning controls reuse the original panel atlas and eight-pixel font:
  wood, red plaque, silver buttons, hand cursor, river clock and thermometer.
  A startup/render check verifies the original 16-colour atlas conversion,
  confined toolbar drawing, pixel-exact original VGA output with QoL off,
  preserved clock/thermometer pixels and no writes to game memory from rendering.
- The 81-second panning desktop scenario passes again with the final artwork,
  toolbar hover hints and a click on Pan's previously inactive corner. It covers
  WASD, dragging, numpad, F11 pause, collecting/cancelling, QoL off/on, walking
  afterward, and the original save/load round trip with two gold bags.
- At 3840x2160, an 85% picture and Classic CRT, real X11 input hovers the enlarged
  Game-button corner, selects Pan on its bevel, drags five swings and clicks Wash.
  Correct pointer coordinates, full loosening and all five retained flakes are
  verified. The Windows x64 cross-build and eleven native regression checks pass.
  These are Linux/OpenGL tests; independent Windows 11 runtime remains open.
- Update 6 adds optional mouse aiming and one shot per left press to the original
  shooting encounters, using the existing QoL checkbox. Original-code checks
  verify pointer-centred aiming, clamping, preserved quick-click positions,
  keyboard/Space coexistence, no held-button auto-fire, focus reset, and untouched
  hand/status/victory/no-gun paths. QoL off restores original controls.
- A controlled Native American encounter runs original startup/assets and combat
  with a synthetic clock. It renders the original crosshair at two mouse
  positions; a click aimed at the opponent passes the original hit test and
  spends exactly one bullet. This is not a wall-clock timing/fidelity test.
- A 39-second native desktop scenario loads a separate combat save through F6,
  aims by mouse, fires a quick click, holds another click, then aims/fires by
  keyboard. Exactly three bullets are spent. Right-click opens the hand; a
  direction returns to aiming. F11 pauses execution and resumes mouse aiming;
  focus reset leaves no queued shot. The loaded fixture uses a bandit encounter;
  both encounter types share the same original aiming/shooting routine.
- At 3840x2160 with an 85% picture and Classic CRT, real X11 mouse motion maps
  to the original aiming coordinates and a 30 ms click spends one bullet without
  opening the hand. Linux/OpenGL rendering and the Windows x64 cross-build pass.
  Independent Windows 11 runtime remains outstanding. Mouse, keyboard, panning,
  console, display, poker and quit regression checks also pass.
- Update 5 adds optional interactive panning with new pixel artwork. The original
  Pan action still checks ownership, pack capacity and river-specific gold grade.
  Three rounds of rocking/rinsing retain or lose gold according to preparation;
  success awards exactly one original bag, while failure/cancel awards none.
  Core checks cover mouse/keyboard play, repeat/idle resistance, focus release,
  no pan, full pack, both river grades and the original behavior with QoL off.
- An 81-second native desktop scenario loads an isolated river fixture through
  F6, clicks the original Pan button, plays with WASD, mouse dragging and Num Lock
  keypad controls, and collects one bag. F11 pauses original execution and the
  activity. Walking works afterward. The QoL checkbox is unchecked/applied and
  the original instant Pan adds a second bag. After re-enabling it, Escape cancels
  a new activity without adding gold. Saving/loading `PANGOLD` through original
  slot 8 preserves both bags, position and the 6,066-byte format.
- QoL is enabled by default, persisted in display.ini, and exposed as a checkbox
  in startup/F11 settings. Invalid values fall back to the default. Cancelling
  settings keeps the active choice; display presets preserve it. New art is
  rendered at 320x200 through the existing scaling/colour/CRT pipeline. Original
  assets, VGA memory and saved-game layouts are unchanged.
- At 3840x2160 with Classic CRT and an 85% picture, actual X11 mouse events
  select the original Pan button, drag five alternating swings to fill the
  loosen meter and click Wash without losing gold. The SDL/OpenGL output and
  hit mapping pass on a virtual Linux desktop; this is not a Windows 11 runtime
  or physical-monitor test. Windows x64 cross-compilation also passes.
- Update 4 fixes the saloon drink crash: `INT 21h` at `13b4:188e`, AX=`0bff`.
  The original C `kbhit` helper reproduces the missing-service error before the
  fix. Its DOS keyboard-status call now returns immediately without consuming
  input. Original-code tests cover empty/pending input, repeated polling,
  Escape, non-character keys and the C runtime's buffered character.
- A native desktop scenario buys whiskey, walks away with D, returns with A,
  buys sarsaparilla and walks away again. Both choices complete without errors;
  the F1 cash display reads $248 after the two purchases from $250. Captured
  positions verify movement after each drink and reopening the bartender menu.
  The Linux runtime and Windows x64 rebuild pass; independent Windows 11
  validation of this fix remains outstanding.
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

## Deterministic runs and golden traces

A scenario runs one emulated millisecond per step, independent of the host:
script events apply at their emulated time; no audio device renders, so the OPL
timers advance from port accesses and emulated time only; DOS date and time are
a fixed base plus emulated time; stale mouse clicks expire after one emulated
second. Every 100 emulated milliseconds, and at the end, the trace records
`ms=<n> blocks=<n> ticks=<n> hash=<FNV-1a 64 of registers, memory and palette>`.
All 29 scenarios run in about half a minute.

`tests/scenarios.json` lists the scenarios with their fixtures, durations and
QoL settings. `qol-saloon` and `saloon-sleep` are absent because their saves
have no generator. The golden traces were recorded from the C++ build, twice,
the second time with fixtures rebuilt from scratch; they contain hashes only,
no game data. Fixtures are isolated saves built from the player's own game
into `.local/fixtures`.

Scripts are one event per line: `<ms> <type> [a] [b]`. `down`, `up` and
`repeat` take an SDL scancode and optional SDL modifier bits (4096 is Num
Lock); `key` and `ascii` type directly; `mouse`, `buttons` and `focuslost`
drive the pointer and focus; `capture` records the 320x200 frame and the
state report, `screen` the settings the presented picture follows.

`tests/scripts/poker.txt` walks to the saloon and selects Play. Its opponent is
chosen by the original game's random state and is sometimes absent; check the
captured invitation and dealt hand rather than treating a clean run as proof
that poker was exercised.
