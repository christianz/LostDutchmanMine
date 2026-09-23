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
- `src/desktop.cpp`: SDL2 presentation, physical input, real clock and diagnostic
  script runner. 320x200 VGA pixels are displayed with a 4:3 aspect ratio.
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
