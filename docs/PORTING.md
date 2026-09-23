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
  4:3 display/mouse geometry, CRT phosphor/scanline masks and highlight glow.
  The original 320x200 framebuffer is never altered.
- `src/presentation.*`: high-DPI SDL2 rendering, monitor frame pacing and settings
  menu. Coordinates convert from window units through physical output pixels to
  the actual centred game picture; clicks in black borders are rejected.
- `src/audio.*`: synchronized YM3812 synthesis using ymfm plus PC speaker tone.
- `src/mouse.h`: desktop button transitions and their coordinates, retained until
  a complete original mouse poll reads them; current motion remains independent.
- `src/keyboard.h`: physical WASD/keypad/direction bindings and normal BIOS text.
  `keyboard_event.h` retains both meanings in the native event queue, with source
  key identity for repeat cleanup even if Num Lock/Shift changes before release.
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
- Held WASD/arrows/keypad directions feed the original joystick direction byte at
  relative segment 72ba:0001, read by 0fa7:0066. This preserves the game's own
  movement cadence without waiting for OS keyboard repeat. Keyboard events still
  reach menus. Releasing a direction removes its pending repeats; losing window
  focus clears all held controls. Opposing directions cancel per axis.
- Movement aliases are selected on the simulation thread only around the input
  calls at 0fa7:009d/00a4 and the main command reads at 0000:0882/0a90. The latter
  cover mouse-to-keyboard activation and direction forwarding through DS:5a1a.
  The context clears immediately after each read, before dispatching any dialog.
  The original text readers receive unmodified letters; Num Lock keypad digits
  receive the number-row BIOS scan codes required by the original save selector.
  Keypad movement uses physical scancodes with either Num Lock state. Tags never
  enter the original 16-bit registers or save data. The native display menu also
  accepts WASD and cardinal keypad keys; keypad Enter confirms both native and
  original dialogs.
- In the world hand-cursor loop, 0000:0887 now forwards a recognized direction
  to the original right-click return at 0000:0914. This hides the hand and restores
  keyboard mode while retaining DS:5a1a's pending direction. Without this, the
  original loop keeps consuming movement keys while remaining in mouse mode.
  Original dialogs and name-entry routines do not pass through this branch.
- The mouse helper at 0fc5:0038 separately reads buttons, Y and X through three
  INT 33h calls. Its AOT entry latches one coherent desktop sample for those reads.
  Each button transition is retained: processing down/up in one desktop batch
  previously collapsed a quick click to zero before the game observed it. This
  left DS:5d62 in keyboard mode and the hand hidden. Click coordinates also stay
  attached to the transition if the physical pointer moves before polling.
  Focus loss/F11 clears pending input; transitions older than one second expire
  so loading-screen clicks cannot replay later. A currently held button remains
  held. SDL_MOUSE_FOCUS_CLICKTHROUGH enables the initial window-activation click.
- The VGA default replaces the selector entry at 1265:0693 with AX=1333 and
  resumes its accepted-choice path at 1265:06f7. SELECT.BIN is never loaded or
  displayed. Original VGA initialization, assets and title sequence still run.
- Poker passes the far comparator 0106:0000 to qsort at 13b4:227c. This
  address-taken callback must be an explicit recovery entry point; direct-call
  traversal alone misses it. The poker test exercises the real sort and callback.
- The saloon drinking sound reaches the original `kbhit` helper at 13b4:1872.
  Its INT 21h/AH=0Bh at 13b4:188e must return AL=FFh for queued input or zero
  when empty, without consuming a key or waiting. The music loop at 10c9:0088
  then calls `getch` to detect Escape. The native status service preserves that
  behavior, and the test also exercises the original C runtime character buffer.
  See [Microsoft MS-DOS Programmer's Reference, Function 0BH](https://www.pcjs.org/documents/books/mspl13/msdos/dosref33/).
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
- CRT is optional and uses standard SDL texture blending on all targets. A
  display-resolution mask combines 200 scanlines with an RGB phosphor pattern
  and edge shading. Each output pixel integrates its part of the scanline to
  reduce aliasing in small previews. The mask is cached by output size/style;
  a small blurred highlight image adds glow when the source picture changes.
  There is no animated flicker, image warping or changed mouse hit geometry.
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
