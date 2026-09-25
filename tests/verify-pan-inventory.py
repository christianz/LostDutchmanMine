#!/usr/bin/env python3
"""Verify desktop panning rejects a phantom counter but accepts a carried pan."""
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
def captures(name):
    return {n: json.loads((root/name/'captures'/f'{n}.json').read_text()) for n in range(2000,2005)}

missing = captures('missing-after')
assert all(s['qol'] and not s['panning_active'] and s['gold_bags'] == 0 for s in missing.values())
assert all((s['x'], s['y']) == (40, 55) for s in missing.values())
for name in ('missing-before', 'owned-after'):
    s = captures(name)
    assert s[2000]['gold_bags'] == 0 and not s[2000]['panning_active']
    assert s[2001]['panning_active'] and s[2001]['gold_bags'] == 0
    assert not s[2002]['panning_active'] and s[2002]['gold_bags'] == 1
    assert s[2003]['panning_active'] and s[2003]['gold_bags'] == 1
    assert not s[2004]['panning_active'] and s[2004]['gold_bags'] == 2
print('PASS: old build pans without a pan; fixed mouse/P actions reject it; a real pan still animates and collects two bags')
