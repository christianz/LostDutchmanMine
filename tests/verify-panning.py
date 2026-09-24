#!/usr/bin/env python3
"""Verify the restored river animation and original-format save/load round trip."""
import json
from pathlib import Path
import struct
import sys

captures = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('captures')
saves = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('.local/panning-test/Saves')
s = {n: json.loads((captures/f'{n}.json').read_text())
     for n in (700,701,702,703,706,707,708,709,710,711,712,713,714,715)}
assert not s[700]['panning_active'] and s[700]['gold_bags'] == 0
assert s[701]['panning_active'] and s[701]['gold_bags'] == 0
assert s[702]['panning_active'] and s[702]['gold_bags'] == 0
assert not s[706]['panning_active'] and s[706]['gold_bags'] == 1
assert s[706]['x'] == s[700]['x'] and s[706]['y'] == s[700]['y']
assert s[707]['x'] > s[706]['x'] and s[707]['gold_bags'] == 1
assert s[715]['qol'] == 0 and s[715]['gold_bags'] == 1
assert s[708]['qol'] == 0 and not s[708]['panning_active'] and s[708]['gold_bags'] == 2
assert s[709]['qol'] == 1 and s[709]['panning_active'] and s[709]['gold_bags'] == 2
assert s[703]['panning_active'] and s[703]['gold_bags'] == 2
assert not s[710]['panning_active'] and s[710]['gold_bags'] == 3
for key in ('x', 'y', 'town_page', 'building', 'gold_bags'):
    assert s[711][key] == s[712][key], (key, s[711], s[712])
for key in ('boundaries', 'panning_active', 'x', 'y', 'gold_bags'):
    assert s[713][key] == s[714][key], (key, s[713], s[714])
data = (saves/'LDMSAVE8.SAV').read_bytes()
assert len(data) == 6066
# Gold count is in the fourth original save block, DS:53d4 through 5405.
assert struct.unpack_from('<H', data, 0x36+0x20+0x24+0x53ea-0x53d4)[0] == 3
assert b'PANGOLD' in (saves/'LDMSAVE.LDM').read_bytes()
print('PASS: original river animation, deferred single reward, drained input, '
      'F11 pause, walking afterward, QoL off/on, and three gold bags preserved '
      'through the original save/load format')
