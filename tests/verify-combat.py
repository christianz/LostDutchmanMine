#!/usr/bin/env python3
"""Check original encounter input using snapshots from scripts/combat.txt."""
import json
from pathlib import Path
import sys

folder = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('captures')
s = {n: json.loads((folder/f'{n}.json').read_text()) for n in range(910, 923)}
def point(n):
    return s[n]['x'], s[n]['y']
assert all(f['combat'] and not f['panning_active'] for f in s.values())
assert point(910) == (160, 64)
assert point(911) == (62, 37) and point(912) == (237, 27)
bullets = s[910]['bullets']
assert s[913]['bullets'] == bullets-1, 'Quick click must fire exactly once'
assert s[914]['bullets'] == bullets-2, 'Held click must not repeatedly fire'
assert s[915]['x'] > s[914]['x'], 'Keyboard must aim with stationary mouse'
assert s[916]['bullets'] == bullets-3, 'Space must still fire'
assert s[917]['mouse_mode'] == 0 and s[917]['mouse_visibility'] >= 0
assert s[918]['mouse_mode'] == 1 and point(918) == (87, 42)
for key in ('boundaries', 'x', 'y', 'bullets'):
    assert s[919][key] == s[920][key], ('F11 must pause', key)
assert point(921) == point(922) == (137, 52)
assert s[922]['bullets'] == bullets-3
print('PASS: original combat mouse aim, quick/held click ammunition, keyboard aim/Space, right-click hand, F11 pause/resume and focus reset')
