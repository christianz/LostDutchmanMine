#!/usr/bin/env python3
"""Check captures from display-menu.txt and its fresh saved configuration."""
import json
from pathlib import Path
import sys

config = Path(sys.argv[1])
captures = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('captures')
settings = dict(line.split('=', 1) for line in config.read_text().splitlines()
                if '=' in line and not line.startswith('#'))
assert settings == dict(window='1', size='100', scaling='2', colour='1', crt='0',
                        brightness='90', vsync='1', startup='0'), settings
states = {n: json.loads((captures/f'{n}.json').read_text())
          for n in (303, 304, 307, 309, 310, 311)}
assert states[303]['x'] == 160 and states[304]['x'] > 160, states
for n in (307, 309, 310, 311):
    for key in ('x', 'y', 'town_page', 'building'):
        assert states[n][key] == states[304][key], (n, key, states)
assert states[310]['boundaries'] == states[311]['boundaries'], 'Game ran during F11 settings'
assert states[309]['boundaries'] > states[311]['boundaries'], 'Game failed to resume'
print('PASS: menu choices persist, F11 pauses original execution, Cancel/Apply preserve player state and resume')
