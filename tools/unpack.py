#!/usr/bin/env python3
"""Recover the load image and relocations of LDM's EXEPACK executable.

Independent implementation of the documented EXEPACK byte format:
https://github.com/w4kfu/unEXEPACK#exepack-header
No DOS code is executed. Original input is always opened read-only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct


def unpack(data: bytes) -> tuple[bytes, dict]:
    if len(data) < 28 or data[:2] != b'MZ':
        raise ValueError('Expected an MZ executable')
    h = struct.unpack_from('<14H', data)
    start = h[4] * 16
    stub = start + h[11] * 16
    if stub + 18 > len(data):
        raise ValueError('Truncated EXEPACK header')
    ip, cs, _, size, sp, ss, paragraphs, skip, signature = struct.unpack_from('<9H', data, stub)
    if signature != 0x4252 or skip != 1:
        raise ValueError('Unsupported EXEPACK variant (expected RB, skip=1)')
    if stub + size > len(data):
        raise ValueError('Truncated EXEPACK data')
    target_length = paragraphs * 16
    output = bytearray(target_length)
    src, dst = stub, target_length
    while src > start and data[src - 1] == 0xff:
        src -= 1
    while True:
        if src - 3 < start:
            raise ValueError('Truncated compression command')
        op = data[src - 1]
        count = struct.unpack_from('<H', data, src - 3)[0]
        src -= 3
        if count > dst:
            raise ValueError('Compression command exceeds declared output')
        if op & 0xfe == 0xb0:
            if src <= start:
                raise ValueError('Missing fill byte')
            src -= 1
            output[dst-count:dst] = bytes([data[src]]) * count
        elif op & 0xfe == 0xb2:
            if src - count < start:
                raise ValueError('Literal run exceeds compressed input')
            output[dst-count:dst] = data[src-count:src]
            src -= count
        else:
            raise ValueError(f'Unknown EXEPACK command {op:#x}')
        dst -= count
        if op & 1:
            break
    remaining = src - start
    if remaining != dst:
        raise ValueError(f'Unpacked size mismatch: prefix={remaining}, space={dst}')
    output[:dst] = data[start:src]
    marker = b'Packed file is corrupt'
    marker_pos = data.find(marker, stub, stub + size)
    if marker_pos < 0:
        raise ValueError('Missing relocation marker')
    pos = marker_pos + len(marker)
    relocations = []
    for group in range(16):
        if pos + 2 > stub + size:
            raise ValueError('Truncated relocation table')
        count = struct.unpack_from('<H', data, pos)[0]
        pos += 2
        if pos + count * 2 > stub + size:
            raise ValueError('Truncated relocation group')
        for _ in range(count):
            offset = struct.unpack_from('<H', data, pos)[0] + group * 65536
            pos += 2
            if offset + 2 > len(output):
                raise ValueError('Relocation outside recovered load image')
            relocations.append(offset)
    if pos != stub + size:
        raise ValueError('Unexpected data after relocation table')
    info = {
        'source_sha256': hashlib.sha256(data).hexdigest(),
        'image_sha256': hashlib.sha256(output).hexdigest(),
        'source_bytes': len(data), 'image_bytes': len(output),
        'entry_cs': cs, 'entry_ip': ip, 'stack_ss': ss, 'stack_sp': sp,
        'relocations': relocations,
    }
    return bytes(output), info


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('exe', type=Path)
    p.add_argument('--output', type=Path, default=Path('recovered'))
    args = p.parse_args()
    image, info = unpack(args.exe.read_bytes())
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'load-image.bin').write_bytes(image)
    (args.output / 'executable.json').write_text(json.dumps(info, indent=2) + '\n')
    print(f"Recovered {len(image):,} bytes, {len(info['relocations'])} relocations; "
          f"entry {info['entry_cs']:04x}:{info['entry_ip']:04x}")


if __name__ == '__main__':
    main()
