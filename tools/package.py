#!/usr/bin/env python3
"""Create a private portable bundle from the user's own game files."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('data', type=Path)
p.add_argument('--platform', choices=['windows', 'linux'], required=True)
p.add_argument('--output', type=Path, required=True)
args = p.parse_args()
out = args.output.resolve()
if out.exists():
    raise SystemExit('Output already exists; choose a fresh directory to protect saves.')
if args.platform == 'windows':
    executable = root/'build-windows/LostDutchmanMine.exe'
else:
    executable = root/'build/ldm-native'
if not executable.is_file():
    raise SystemExit('Build the selected native executable first.')
out.mkdir(parents=True)
(out/'Game').mkdir()
(out/'Saves').mkdir()
(out/'licenses').mkdir()
shutil.copy2(executable, out/('LostDutchmanMine.exe' if args.platform == 'windows' else 'LostDutchmanMine'))
shutil.copy2(root/'resources/ldm-icon.bmp', out/'LostDutchmanMine.bmp')
shutil.copy2(root/'resources/ldm.ico', out/'LostDutchmanMine.ico')
shutil.copy2(root/'resources/ui-font.bmp', out/'ui-font.bmp')
shutil.copy2(root/'resources/panning/creek.ppm', out/'panning-creek.ppm')
shutil.copy2(root/'recovered/load-image.bin', out/'Game/port-data.bin')
for name in ('LDMG', 'LDM.CAP', 'LDMSAVE.LDM'):
    source = args.data/name
    if source.is_dir():
        shutil.copytree(source, out/'Game'/name)
    else:
        shutil.copy2(source, out/'Game'/name)
for source in args.data.glob('LDMSAVE*.SAV'):
    shutil.copy2(source, out/'Game'/source.name)
sdl = root/'.local/deps/SDL2-2.32.0'
if args.platform == 'windows':
    shutil.copy2(sdl/'x86_64-w64-mingw32/bin/SDL2.dll', out/'SDL2.dll')
shutil.copy2(sdl/'LICENSE.txt', out/'licenses/SDL2.txt')
shutil.copy2(root/'third_party/ymfm/LICENSE', out/'licenses/ymfm.txt')
shutil.copy2(root/'resources/FONT-LICENSE.txt', out/'licenses/DejaVu-font.txt')
shutil.copy2(root/'docs/VALIDATION.md', out/'VALIDATION.md')
launch = 'Double-click LostDutchmanMine.exe.' if args.platform == 'windows' else 'Install your distribution\'s SDL2 runtime, then run ./LostDutchmanMine.'
(out/'README.txt').write_text(f'''Lost Dutchman Mine - native development build

{launch}
Keep Game, Saves and the executable together. On Windows, copy the whole folder
to your PC before running. No installation or administrator access is needed.

Choose your display settings, then Play. The 4K comfort button selects desktop
fullscreen, an 85% centred picture, soft pixel edges and gentle colours.
F11 reopens display settings during play, pausing the game and music. Settings
are saved in display.ini beside the executable; keep this file when updating.
Turn off Show at startup to skip the menu next time. F11 always remains available.
CRT monitor offers Off, Soft and Classic: steady scanlines, phosphor texture,
soft glow and gentle edge shading. Preview it in the menu; the choice is saved.

QoL improvements enables interactive panning and mouse aiming, checked by default.
In shooting encounters, move the mouse to aim and left-click to fire one shot.
Right-click opens the hand for Run/status/menu choices; a direction key returns
to aiming. WASD, arrows and numpad still aim; Space still fires. A stationary mouse
does not undo keyboard aiming. Ammunition and hit rules stay the same. Uncheck
QoL improvements to restore the original keyboard aiming/mouse selection behavior.

At a river,
choose Pan while carrying a pan. Rock left/right with A/D, arrows, numpad 4/6,
the onscreen buttons, or drag the pan. Wash when the loosen bar turns gold using
Space, Enter or the Wash button. Three careful washes preserve the gold; washing
too soon can lose it. Take Gold puts one original gold bag in your pack. Escape
abandons a pan. F11 pauses it. Uncheck QoL improvements for the original instant
Pan action; the choice persists and applies to the next pan. Keep panning-creek.ppm
beside the executable. Other movement and display improvements stay available.

VGA starts automatically. Allow the original title/credits sequence to finish.
Hold WASD, cursor keys or the numeric keypad to move; release to stop.
Numpad 8/2/4/6 move up/down/left/right; 7/9/1/3 move diagonally. Num Lock may be
on or off for movement. Combine WASD keys for diagonals. With Num Lock on, the
keypad enters numbers and selects save slots. Numpad Enter confirms. WASD letters
still type normally in save names. WASD/numpad also navigate display settings.
A movement key returns to walking when the hand cursor is active.
Use the mouse for choices,
F1-F6 for the status panel, Space for action, Alt+Enter for fullscreen.
To enter a building, align with its doorway and hold Up to walk inside.
Save and load through F6. New saves go in Saves; the supplied originals in Game
are read only. Back up Saves when moving or updating this build.
Short mouse clicks are now preserved, including the click used to focus the
window. One click restores the hand after keyboard movement.

This is a development build of a faithful port, not a fully validated release.
Windows x64 is cross-compiled; independent Windows 11 testing remains outstanding.
Linux runtime, 4K rendering, display menus, movement and saves have been tested.
See VALIDATION.md for the exact coverage and remaining work.

The original instructions are translated to C++ at build time and compiled to
native machine code. There is no DOSBox, virtual machine, DOS kernel or runtime
CPU interpreter. Original data layouts and game logic are retained. SDL2 handles
desktop services; ymfm synthesizes the original AdLib sound chip.

This private bundle includes your original game data. Original game copyright
remains with its owners. SDL2, ymfm and DejaVu font notices are in licenses.
''', encoding='utf-8')
files = {str(f.relative_to(out)): {'bytes': f.stat().st_size,
         'sha256': hashlib.sha256(f.read_bytes()).hexdigest()}
         for f in sorted(out.rglob('*')) if f.is_file()}
manifest = {'platform': args.platform,
            'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
            'files': files}
(out/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(out, len(files), 'files')
