#!/usr/bin/env python3
"""Record or check the golden trace of every scenario in tests/scenarios.json.

Each scenario runs deterministically (build/ldm-native --trace) in a fresh folder
under .local/scenarios/<name>: its fixture, config, captures and trace live there.
With --check, traces are compared with tests/golden/ instead of written. The
single-run verifiers listed under "checks" run afterwards in both modes.
Usage: tools/record_traces.py [--check] [--data DIR] [NAME ...]
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = ROOT / 'tests/golden'
RUNS = ROOT / '.local/scenarios'
FIXTURES = ROOT / '.local/fixtures'


def run_scenario(scenario: dict, data: Path) -> Path:
    folder = RUNS / scenario['name']
    shutil.rmtree(folder, ignore_errors=True)
    folder.mkdir(parents=True)
    saves = folder / 'Saves'
    if scenario.get('fixture'):
        shutil.copytree(FIXTURES / scenario['fixture'], saves)
    else:
        saves.mkdir()
    if 'qol' in scenario:
        (folder / 'display.ini').write_text(f"qol={scenario['qol']}\nstartup=0\n")
    command = [ROOT / 'build/ldm-native', '--data', data, '--image', ROOT / 'recovered/load-image.bin',
               '--saves', saves, '--config', folder / 'display.ini', '--seconds', scenario['seconds'],
               '--script', ROOT / 'tests/scripts' / scenario['script'], '--trace', folder / 'trace']
    if scenario.get('settings'):
        command.append('--settings')
    env = dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy', SDL_RENDER_DRIVER='software')
    with (folder / 'runtime.log').open('w') as log:
        result = subprocess.run([str(c) for c in command], cwd=folder, env=env, stdout=log, stderr=log)
    if result.returncode:
        raise RuntimeError(f"{scenario['name']} exited with {result.returncode}; see {folder / 'runtime.log'}")
    return folder / 'trace'


def compare(name: str, trace: Path) -> str | None:
    golden = GOLDEN / f'{name}.trace'
    if not golden.exists():
        return f'{name}: no golden trace'
    expected, actual = golden.read_text().splitlines(), trace.read_text().splitlines()
    for line, (want, got) in enumerate(zip(expected, actual), 1):
        if want != got:
            return f'{name}: line {line} differs\n  golden {want}\n  actual {got}'
    if len(expected) != len(actual):
        return f'{name}: {len(actual)} lines, golden has {len(expected)}'
    return None


def run_checks(checks: list, names: set) -> list:
    failures = []
    for check in checks:
        needed = set(check.get('merge', [])) | {a[1:a.index('}')] for a in check['args'] if a.startswith('{')}
        needed.discard('merged')
        if not needed <= names:
            continue
        values = {name: str(RUNS / name) for name in needed}
        if 'merge' in check:
            merged = RUNS / ('merged-' + '-'.join(check['merge']))
            shutil.rmtree(merged, ignore_errors=True)
            merged.mkdir()
            for name in check['merge']:
                shutil.copytree(RUNS / name / 'captures', merged, dirs_exist_ok=True)
            values['merged'] = str(merged)
        args = [a.format_map(values) for a in check['args']]
        result = subprocess.run([sys.executable, ROOT / check['verifier'], *args], cwd=ROOT,
                                capture_output=True, text=True)
        status = 'PASS' if result.returncode == 0 else 'FAIL'
        print(f"{status} {check['verifier']}: {(result.stdout + result.stderr).strip().splitlines()[-1:]}")
        if result.returncode:
            failures.append(check['verifier'])
    return failures


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--check', action='store_true')
    p.add_argument('--data', type=Path, default=Path(os.environ.get('LDM_DATA', ROOT / '.local/original')))
    p.add_argument('names', nargs='*')
    args = p.parse_args()
    manifest = json.loads((ROOT / 'tests/scenarios.json').read_text())
    scenarios = [s for s in manifest['scenarios'] if not args.names or s['name'] in args.names]
    subprocess.run(['make', '-j6', 'build/ldm-native'], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    if not FIXTURES.exists():
        subprocess.run([sys.executable, ROOT / 'tools/fixtures.py', '--data', args.data], check=True)
    GOLDEN.mkdir(exist_ok=True)
    failures = []
    with ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
        traces = pool.map(lambda s: (s['name'], run_scenario(s, args.data.resolve())), scenarios)
        for name, trace in traces:
            if args.check:
                problem = compare(name, trace)
                print(f'{"FAIL" if problem else "ok  "} {problem or name}')
                if problem:
                    failures.append(name)
            else:
                shutil.copy(trace, GOLDEN / f'{name}.trace')
                print(f'recorded {name}: {len(trace.read_text().splitlines())} lines')
    failures += run_checks(manifest['checks'], {s['name'] for s in scenarios})
    if failures:
        raise SystemExit(f'{len(failures)} failed: {", ".join(failures)}')


if __name__ == '__main__':
    main()
