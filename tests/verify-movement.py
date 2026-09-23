#!/usr/bin/env python3
"""Check captures from scripts/held-movement.txt (run the desktop for 23 seconds)."""
import json
from pathlib import Path
import sys

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('captures')
frames = {n: json.loads((root/f'{n}.json').read_text()) for n in range(200, 212)}
assert all(f['video_mode'] == 0x13 for f in frames.values()), 'VGA must start without a key'
assert frames[201]['x'] < frames[202]['x'] < frames[203]['x'] < frames[204]['x'], 'Held right must keep moving without repeat events'
assert frames[202]['held_directions'] == frames[203]['held_directions'] == 8
assert frames[204] == frames[205], 'Movement must stop on key release'
assert frames[206]['x'] < frames[205]['x'], 'Held left must move left'
assert frames[206] == frames[207], 'Movement must stop on focus loss'
assert frames[208] == frames[209], 'Opposing held directions must settle to neutral'
assert frames[210] == frames[211], 'Releasing a key must discard its pending repeats'
assert all(frames[n]['held_directions'] == 0 for n in range(204, 212))
print('PASS: automatic VGA, sustained movement, key release, focus loss, opposing directions, stale-repeat cleanup')
