#!/usr/bin/env python3
"""Quick clicks must restore the hand, open F6 and quit without retries."""
import json
import os
from pathlib import Path
import subprocess
import time

start = time.monotonic()
run = subprocess.run([
    'build/ldm-native', '--data', os.environ.get('LDM_DATA', '/nas/tmp/LDM'),
    '--image', 'recovered/load-image.bin', '--saves', '.local/mouse-test-saves',
    '--seconds', '28', '--script', 'tests/scripts/mouse-clicks.txt'],
    env=dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy'),
    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=26)
elapsed = time.monotonic()-start
Path('.local/mouse-test.log').write_text(run.stdout)
assert run.returncode == 0, run.stdout
assert 'mode 3, PIT=0' in run.stdout, run.stdout
states = [json.loads(Path(f'captures/{n}.json').read_text()) for n in (400, 401, 402)]
assert states[0]['mouse_visibility'] < 0 and states[0]['mouse_mode'] == 1, states[0]
for state in states[1:]:
    assert state['mouse_visibility'] >= 0 and state['mouse_mode'] == 0, state
    for key in ('x', 'y', 'town_page', 'building'):
        assert state[key] == states[0][key], (key, states)
print(f'PASS: single quick clicks restore the hand, open F6 and quit; player unchanged, exit at {elapsed:.2f}s')
