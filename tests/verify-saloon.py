#!/usr/bin/env python3
"""Check that both saloon drink choices return control to the player."""
import json
from pathlib import Path
import sys

captures = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('captures')
states = {n: json.loads((captures / f'{n}.json').read_text())
          for n in (603, 610, 611, 612, 614, 615, 616)}
for n, state in states.items():
    assert state['building'] == 2 and state['video_mode'] == 19, (n, state)
    assert state['y'] == states[603]['y'], (n, state)
    assert state['held_directions'] == 0, (n, state)
assert states[603]['mouse_visibility'] >= 0, states[603]
assert states[610]['mouse_visibility'] < 0, states[610]
assert states[611]['x'] > states[610]['x'], states
assert states[612]['x'] == states[603]['x'], states
assert states[612]['mouse_visibility'] >= 0, states[612]
assert states[614]['mouse_visibility'] < 0, states[614]
assert states[615]['x'] > states[614]['x'], states
assert states[616]['x'] == states[615]['x'], states
assert states[616]['mouse_visibility'] >= 0, states[616]
print('PASS: whiskey and sarsaparilla return to play, WASD walks away after each, '
      'returning to the bar reopens the menu, and F1 remains responsive')
