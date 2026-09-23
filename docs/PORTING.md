# Recovery notes

The source of truth is the user's DOS LDM.EXE, SHA-256
`de0726a1cb0a475cd05f19ffb56e6c84f014fdb34f986d374d5e09797b925e07`.
The EXEPACK image is 484,448 bytes with 2,413 relocation sites. The initial
entry is 13b4:00ca and the C main function is 0000:0000 (relative segments).
The native state loads data at paragraph 1000; DS is normally 82bd at runtime.

## Architecture

- `tools/unpack.py`: bounds-checked EXEPACK recovery; opens the original read only.
- `tools/analyze.py`: conservative control-flow recovery using Capstone in 16-bit
  mode. Far references and observed indirect targets seed reachable code.
- `tools/translate.py`: emits compiled C++ operations and branches. Calls and
  backward branches yield to a native dispatcher. No instruction decoding or
  execution from the data image occurs at runtime. Unknown targets throw.
- `tools/entry-points.json`: additional code addresses found from actual indirect
  control transfers and graphics callback tables. Do not seed arbitrary bytes.
- `src/legacy.*`: retained original data/register representation, exact-width
  arithmetic helpers, native files, keyboard/mouse, palettes and timer boundary.
- `src/desktop.cpp`: SDL2 lifecycle, settings/menu input and diagnostic scripts.
- `src/session.*`: dedicated original-game thread, real PIT clock, queued inputs
  and synchronized framebuffer snapshots. Pausing the settings menu freezes the
  game clock; GPU/VSync waits cannot pace the game simulation.
- `src/display.*`: persisted display preferences, colour grading, Scale2x and
  4:3 display/mouse geometry. The original 320x200 framebuffer is never altered.
- `src/presentation.*`: high-DPI SDL2 rendering, monitor frame pacing and settings
  menu. Coordinates convert from window units through physical output pixels to
  the actual centred game picture; clicks in black borders are rejected.
- `src/audio.*`: synchronized YM3812 synthesis using ymfm plus PC speaker tone.
- `src/assets.*`: readable packed-asset decoder, differentially checked against
  the translated original routine at 1265:1250. The runtime still uses the
  translated original decoder.

## Details that matter

- Capstone prints opcodes 98/99 as CWDE/CDQ in this build; their actual 16-bit
  semantics are CBW/CWD. The translator handles that explicitly.
- VGA detection requires BIOS 10h/1Bh's real function/state and static capability
  table, not only mode 13h. Without it the Microsoft library chose EGA/CGA
  callbacks and drew menu text without its backgrounds. The IBM April 1987
  reference, pp. 2-40 through 2-43, defines this table.
- Keyboard reads require BIOS scan codes as well as ASCII. The original save
  selector compares scans 02 through 09 for slots 1 through 8. Its key reader
  intentionally drains queued typeahead; scripted text needs reasonable pacing.
- Held arrows/keypad directions feed the original joystick direction byte at
  relative segment 72ba:0001, read by 0fa7:0066. This preserves the game's own
  movement cadence without waiting for OS keyboard repeat. Keyboard events still
  reach menus. Releasing a direction removes its pending repeats; losing window
  focus clears all held controls. Opposing directions cancel per axis.
- The VGA default replaces the selector entry at 1265:0693 with AX=1333 and
  resumes its accepted-choice path at 1265:06f7. SELECT.BIN is never loaded or
  displayed. Original VGA initialization, assets and title sequence still run.
- Poker passes the far comparator 0106:0000 to qsort at 13b4:227c. This
  address-taken callback must be an explicit recovery entry point; direct-call
  traversal alone misses it. The poker test exercises the real sort and callback.
- Quit Game calls the graphics shutdown function at 1613:1c7d indirectly. It
  selects 400-line text mode with BIOS 10h/12h/BL=30h, then restores mode 3 before
  returning through the original exit path. The C runtime also calls its file
  cleanup callback at 13b4:0752 indirectly. Both recovery seeds and the native
  text-mode service are required. The host monitor stays in its desktop mode.
- Display choices are stored atomically in `display.ini` beside the executable.
  Apply commits the draft; Cancel restores previous settings. F11 releases held
  controls and pauses original execution and audio, with no timer catch-up when
  resuming. `--settings`, `--no-settings` and `--config` support launch control.
- Soft scaling uses a 2x nearest enlargement followed by GPU linear sampling.
  Pixel art uses the published Scale2x neighbourhood rule followed by linear
  sampling. Colour grading is applied only to the presented pixels. There is
  no temporal interpolation, AI reconstruction or new sprite animation.
- Original timer handler: 1398:00ce, relocated to 2398:00ce. Timer 0 is driven
  by the real 1,193,182 Hz PIT rate and the divisor requested by the original
  game/music routines. Chaining to the default BIOS timer updates its tick count.
- The packed image format starts with 019Dh, with MSB-first variable-width
  codes beginning at nine bits. Code 256 increases width. Some original asset
  outputs exceed the nominal header size slightly; the original routine does
  so too. In particular SPRT_VGA produces 64,273 bytes. Header size is a minimum,
  while the 65,535-byte segment bound is enforced.
- Recovered player fields in DS: x=5b4a, y=5b4c, town scroll=5b5a, building=5b5e.
  The town walk changes x by six pixels per accepted key, not four. Walking
  through a door requires repeated Up events until y reaches 55.
- Writes resolve to a separate save tree. Existing originals opened for
  read/write are copied there first. Reads prefer the save tree and then the
  original data directory. Paths are case-insensitive on Linux too.
- The data image has a build-time fingerprint which must match the executable.
  Mixing a different DOS revision with this build produces an explicit error.

## Continue recovery

Run native scenarios and retain the first precise diagnostic. For an unrecovered
target, disassemble that address and confirm it is actual code before adding it
to the entry-point list, regenerating and rebuilding. `tools/discover.py` automates
this for startup using a synthetic clock; it is not a real-time gameplay test.
Never substitute an interpreter or silently skip an unimplemented service.

Generated C++ and recovered images are private build products excluded from Git.
They retain original proprietary game material. The repository carries recovery
tools, platform adapters and the separately licensed ymfm subset only.
