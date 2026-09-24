#!/usr/bin/env python3
"""Check saloon-sleep desktop captures before/after the fix (requires Pillow)."""
import json
from pathlib import Path
import sys
from PIL import Image

before, after = map(Path, sys.argv[1:])
def snapshot(folder, n):
    return json.loads((folder/'captures'/f'{n}.json').read_text())
def gold(folder, n):
    with Image.open(folder/'captures'/f'{n}.bmp') as image:
        return sum(c == (255, 211, 78) for c in image.convert('RGB').getdata())

for n in (1902, 1903):
    assert snapshot(before, n)['building'] == snapshot(after, n)['building'] == 2
    assert gold(before, n) > 100, 'The old build must reproduce the hidden hover outline'
    assert gold(after, n) == 0, 'A hidden button still highlights during sleep'
for folder in (before, after):
    start, awake = snapshot(folder, 1900), snapshot(folder, 1904)
    assert start['building'] == 2 and awake['building'] == 0
    assert (awake['x'], awake['y']) == (start['return_x'], 59)
    assert gold(folder, 1904) > 100, 'Waking must restore toolbar hover'
print('PASS: sleep hides context/toolbar outlines and waking restores hover at the saved saloon doorway')
