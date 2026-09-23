#!/usr/bin/env python3
"""Prepare an isolated river save from a supplied DOS save, without editing it.

The eight blocks are read/written by original routines 0e5a:0140 and 0e5a:0380.
Only scene/position fields change; the supplied slot 1 already owns a pan.
This is a QA fixture, not a player save migration or a shipped saved game.
"""
import argparse
from pathlib import Path
import shutil
import struct

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('source', type=Path)
p.add_argument('output', type=Path)
args = p.parse_args()
blocks = [(0x5b4a, 0x36), (0x5e04, 0x20), (0x5314, 0x24),
          (0x53d4, 0x32), (0x5d5a, 6), (0x500e, 0x58),
          (0x5bd4, 0x58), (0x389c, 0x1650)]
data = bytearray(args.source.read_bytes())
assert len(data) == sum(size for _, size in blocks) == 6066
def offset(address):
    at = 0
    for start, size in blocks:
        if start <= address < start + size:
            return at + address - start
        at += size
    raise ValueError(hex(address))
assert struct.unpack_from('<H', data, offset(0x53dc))[0] > 0, 'Fixture source must already own a pan'
for address, value in ((0x5b4a, 40), (0x5b4c, 55), (0x5b5e, 0),
                       (0x5e04, 0), (0x5e06, 0), (0x5e08, 1),
                       (0x5e0a, 0), (0x5e0c, 0), (0x532c, 0)):
    struct.pack_into('<H', data, offset(address), value)
args.output.mkdir(parents=True, exist_ok=False)
(args.output / 'LDMSAVE1.SAV').write_bytes(data)
shutil.copy2(args.source.parent / 'LDMSAVE.LDM', args.output / 'LDMSAVE.LDM')
print('Prepared isolated river fixture:', args.output)
