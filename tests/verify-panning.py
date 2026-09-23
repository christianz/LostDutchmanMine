#!/usr/bin/env python3
"""Verify panning.txt snapshots and the original-format save/load round trip."""
import json
from pathlib import Path
import struct
import sys

captures = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('captures')
saves = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('.local/panning-test/Saves')
s = {n: json.loads((captures/f'{n}.json').read_text()) for n in range(700, 716)}
assert s[700]['panning_phase'] == 0 and s[700]['gold_bags'] == 0
assert s[701]['panning_phase'] == 1 and s[701]['gold_bags'] == 0
assert s[702]['panning_loosened'] == 100 and s[702]['panning_round'] == 0
assert s[703]['panning_phase'] == 2
assert s[704]['panning_loosened'] == 100 and s[704]['panning_round'] == 1
assert s[705]['panning_phase'] == 3 and s[705]['panning_gold'] == 5 and s[705]['gold_bags'] == 0
assert s[706]['panning_phase'] == 0 and s[706]['gold_bags'] == 1
assert s[707]['x'] > s[706]['x'] and s[707]['gold_bags'] == 1
assert s[715]['qol'] == 0 and s[715]['gold_bags'] == 1
assert s[708]['qol'] == 0 and s[708]['panning_phase'] == 0 and s[708]['gold_bags'] == 2
assert s[709]['qol'] == 1 and s[709]['panning_phase'] == 1 and s[709]['gold_bags'] == 2
assert s[710]['panning_phase'] == 0 and s[710]['gold_bags'] == 2
for key in ('x', 'y', 'town_page', 'building', 'gold_bags'):
    assert s[711][key] == s[712][key], (key, s[711], s[712])
for key in ('boundaries', 'panning_phase', 'panning_round', 'panning_loosened', 'panning_gold', 'gold_bags'):
    assert s[713][key] == s[714][key], (key, s[713], s[714])
data = (saves/'LDMSAVE8.SAV').read_bytes()
assert len(data) == 6066
# Gold count is in the fourth original save block, DS:53d4 through 5405.
assert struct.unpack_from('<H', data, 0x36+0x20+0x24+0x53ea-0x53d4)[0] == 2
assert b'PANGOLD' in (saves/'LDMSAVE.LDM').read_bytes()
print('PASS: original river Pan, WASD/mouse/numpad washes, delayed single reward, '
      'F11 pause, walking afterward, QoL off/on, cancel without reward and '
      'two gold bags preserved through original save/load')
