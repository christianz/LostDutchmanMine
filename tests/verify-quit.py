#!/usr/bin/env python3
"""Exercise the original Quit Game button, distinguishing exit from a timeout."""
import os
from pathlib import Path
import subprocess
import time

start = time.monotonic()
run = subprocess.run([
    'build/ldm-native', '--data', os.environ.get('LDM_DATA', '/nas/tmp/LDM'),
    '--image', 'recovered/load-image.bin', '--saves', '.local/quit-test-saves',
    '--seconds', '30', '--script', 'tests/scripts/quit-game.txt'],
    env=dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy'),
    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=27)
elapsed = time.monotonic()-start
Path('.local/quit-test.log').write_text(run.stdout)
assert run.returncode == 0, run.stdout
assert 'video mode 3\n' in run.stdout and 'mode 3, PIT=' in run.stdout, run.stdout
assert 21 <= elapsed < 27, elapsed
print(f'PASS: F6 > Quit Game exited normally in {elapsed:.2f}s, before the 30s test limit')
