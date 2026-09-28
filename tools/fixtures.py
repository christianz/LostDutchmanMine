#!/usr/bin/env python3
"""Build the isolated save fixtures used by scenario runs.

Every fixture is derived from the player's own game directory ($LDM_DATA) by an
existing generator, built twice, and accepted only if both builds are byte for
byte identical. Fixtures contain game data: they stay under .local and are never
committed. Usage: tools/fixtures.py [--data DIR] [--out DIR]
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
# Scratch save folders of the native generators. Leftover files there would be
# preferred over the original game files, so they are cleared before each run.
SCRATCH = ['.local/cave-test-saves', '.local/assay-test-saves', '.local/pan-inventory-saves']


def generate(data: Path, out: Path) -> None:
    """Write every fixture into out/<name>/."""
    for scratch in SCRATCH:
        shutil.rmtree(ROOT / scratch, ignore_errors=True)
    run = lambda *cmd: subprocess.run([str(c) for c in cmd], cwd=ROOT, check=True,
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    run('python3', 'tests/make-combat-fixture.py', data / 'LDMSAVE1.SAV', out / 'combat')
    run('python3', 'tests/make-river-fixture.py', data / 'LDMSAVE1.SAV', out / 'river')
    run('build/test-assay', data, out / 'assay')
    run('build/test-cave', data, out / 'map', out / 'mining')
    run('build/test-pan-inventory', data, out / 'pan')
    for kind in ('missing', 'owned'):
        (out / 'pan' / kind).rename(out / f'pan-{kind}')
    (out / 'pan').rmdir()


def digest(folder: Path) -> str:
    h = hashlib.sha256()
    for path in sorted(p for p in folder.rglob('*') if p.is_file()):
        h.update(str(path.relative_to(folder)).encode() + b'\0' + path.read_bytes())
    return h.hexdigest()


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data', type=Path, default=Path(os.environ.get('LDM_DATA', ROOT / '.local/original')))
    p.add_argument('--out', type=Path, default=ROOT / '.local/fixtures')
    args = p.parse_args()
    subprocess.run(['make', '-j6', 'build/test-assay', 'build/test-cave', 'build/test-pan-inventory'],
                   cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    with tempfile.TemporaryDirectory(dir=ROOT / '.local') as first, tempfile.TemporaryDirectory(dir=ROOT / '.local') as second:
        generate(args.data.resolve(), Path(first))
        generate(args.data.resolve(), Path(second))
        for fixture in sorted(Path(first).iterdir()):
            a, b = digest(fixture), digest(Path(second) / fixture.name)
            if a != b:
                raise SystemExit(f'Fixture {fixture.name} is not reproducible: {a} != {b}')
            print(f'{fixture.name:12} {a}')
        shutil.rmtree(args.out, ignore_errors=True)
        shutil.copytree(first, args.out)


if __name__ == '__main__':
    main()
