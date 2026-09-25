#!/usr/bin/env python3
"""Check desktop captures from the responsive aiming, pointer and mining scripts."""
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
def shot(name, n):
    return json.loads((root/name/'captures'/f'{n}.json').read_text())

old_late = previewed = 0
for i in range(12):
    before, after = shot('combat-before', 3000+i), shot('combat-after', 3000+i)
    expected = (62+i*14, 22+(i%4)*10)
    assert after['combat'] and after['bullets'] == 20
    assert (after['sight_x'], after['sight_y']) == expected, ('Delayed aim', i)
    old_late += (before['x'], before['y']) != expected
    previewed += (after['x'], after['y']) != expected
assert old_late > 0 and previewed > 0
print(f'PASS: all 12 mouse moves displayed within 25 ms; old aim delayed in {old_late}/12 captures; {previewed}/12 new captures precede committed encounter aim')

positions = set()
for n in range(4100, 4105):
    s = shot('map-after', n)
    assert s['qol'] and s['map_view'] and (s['mouse_x'], s['mouse_y']) == (65, 55)
    positions.add((s['x'], s['y']))
assert len(positions) == 5
print('PASS: numpad movement with Num Lock on/off leaves the pointer stationary')

for name, qol in [('mining-qol', 1), ('mining-classic', 0)]:
    s = {n: shot(name, n) for n in range(4000, 4009)}
    assert all(v['qol'] == qol and v['cave_view'] for v in s.values())
    assert s[4001]['mining_strokes'] >= 3
    assert s[4002]['mining_strokes'] > s[4001]['mining_strokes']
    assert s[4001]['mining_space_held'] and s[4002]['mining_space_held']
    assert s[4004]['mining_strokes'] >= 3 and s[4004]['mining_space_held']
    for n in [4000, 4003, 4005, 4008]:
        assert not s[n]['mining_space_held'] and s[n]['mining_strokes'] == 0
    for key in ['boundaries', 'mining_strokes', 'x', 'y']:
        assert s[4006][key] == s[4007][key], ('Settings failed to pause', name, key)
    print(f'PASS: {name} continues original strokes while held; release, focus loss and settings stop the action')

s = {n: shot('panning-classic', n) for n in range(4200, 4204)}
assert all(not v['qol'] for v in s.values())
assert not s[4200]['panning_active'] and s[4200]['gold_bags'] == 0
for n in [4201, 4202]:
    assert s[n]['panning_active'] and s[n]['gold_bags'] == 0
assert not s[4203]['panning_active'] and s[4203]['gold_bags'] == 1
assert (s[4203]['x'], s[4203]['y']) == (40, 55)
print('PASS: QoL off plays the restored panning animation, blocks queued actions and awards exactly one bag')
