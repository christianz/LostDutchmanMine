#!/usr/bin/env python3
"""Verify desktop controls and the text/save boundary after both keyboard scripts."""
import json
from pathlib import Path
import sys

root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('captures')
config=Path(sys.argv[2]) if len(sys.argv)>2 else Path('.local/keyboard-controls/display.ini')
saves=Path(sys.argv[3]) if len(sys.argv)>3 else Path('.local/keyboard-save/Saves')
frames={n:json.loads((root/f'{n}.json').read_text()) for n in (*range(500,516),519,530,531,532)}
def position(frame):
    return tuple(frame[k] for k in ('x','y','town_page','building'))

assert frames[501]['pointer_visible'],frames[501]
assert frames[502]['held_directions']==8 and frames[502]['x']>frames[501]['x'],frames[502]
assert frames[502]['mouse_mode']==1 and frames[502]['pointer_visible'],'Keyboard movement must keep the pointer available'
assert frames[503]['x']>frames[502]['x'],'Held D must continue without OS repeat'
assert position(frames[503])==position(frames[504]),'Releasing D must stop movement'
assert frames[505]['held_directions']==5,'W+A must produce an up-left diagonal'
assert frames[506]['x']<frames[504]['x'],'W+A must move left in the town'
assert frames[507]['x']>frames[506]['x'],'Numpad 6 / Num Lock off must move right'
assert frames[508]['x']<frames[507]['x'],'Numpad 4 / Num Lock on must move left'
assert frames[509]['held_directions']==9,'Numpad 9 must produce an up-right diagonal'
assert frames[510]['x']>frames[508]['x'],'Numpad 9 must move right in the town'
for a,b in ((511,512),(513,514),(514,515),(515,519)):
    assert position(frames[a])==position(frames[b]),(a,b,frames[a],frames[b])
assert all(frames[n]['held_directions']==0 for n in (503,504,506,507,508,510,511,512,513,514,515,519))
settings=dict(line.split('=',1) for line in config.read_text().splitlines() if '=' in line)
assert settings['crt']=='1' and settings['colour']=='1',settings
assert frames[531]['x']<frames[530]['x'],'A must move away from the saved position'
assert position(frames[530])==position(frames[532]),'Keypad load must restore the saved position'
assert (saves/'LDMSAVE8.SAV').stat().st_size==6066,'Original save format changed'
assert b'WASD42' in (saves/'LDMSAVE.LDM').read_bytes(),'WASD letters/keypad digits did not survive text entry'
print('PASS: WASD/Num Lock on-off movement, diagonals, visible pointer while walking, release/focus/opposing keys, repeat cleanup across Num Lock change, native menu navigation and WASD42 save/load via keypad')
