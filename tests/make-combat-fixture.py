#!/usr/bin/env python3
"""Create an isolated armed encounter save for testing the original aiming loop.

The bandit and Native American encounters share routine 040a:0002. The encounter
type flag itself is not saved; loading this fixture uses the default bandit type.
Only scene/position fields change. This fixture is never shipped to players.
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
assert all(struct.unpack_from('<H', data, offset(a))[0] > 0 for a in (0x53e0, 0x53e2)), 'Source must own a gun and ammunition'
for address, value in ((0x5b4a, 160), (0x5b4c, 64), (0x5b5e, 0),
                       (0x5b64, 0), (0x5e04, 0), (0x5e06, 0),
                       (0x5e08, 0), (0x5e0a, 0), (0x5e0c, 1)):
    struct.pack_into('<H', data, offset(address), value)
args.output.mkdir(parents=True, exist_ok=False)
(args.output / 'LDMSAVE1.SAV').write_bytes(data)
shutil.copy2(args.source.parent / 'LDMSAVE.LDM', args.output / 'LDMSAVE.LDM')
print('Prepared isolated combat fixture:', args.output)
