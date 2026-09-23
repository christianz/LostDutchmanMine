#!/usr/bin/env python3
"""Recover targets actually reached by the compiled native startup probe.

Stops on unsupported services or translation failures. Never substitutes a game
function or modifies the original executable to get past a failure.
"""
import argparse,json,re,subprocess
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('data')
p.add_argument('--rounds',type=int,default=20)
p.add_argument('--keys',default='')
args=p.parse_args()
entries=Path('tools/entry-points.json')
for n in range(args.rounds):
    subprocess.run(['.venv/bin/python','tools/translate.py'],check=True)
    with open('.local/build.log','w') as log:
        subprocess.run(['make','-j4'],stdout=log,stderr=subprocess.STDOUT,check=True)
    r=subprocess.run(['build/ldm-probe',args.data,'12000000',args.keys,'1000'],capture_output=True,text=True,timeout=30)
    print(r.stdout+r.stderr,flush=True)
    m=re.search(r'Unrecovered (?:control-flow target|code segment) at ([0-9a-f]+):([0-9a-f]+)',r.stderr)
    if not m:break
    point=[int(m[1],16),int(m[2],16)]
    points=json.loads(entries.read_text()) if entries.exists() else []
    if point in points:raise RuntimeError('Target was already recovered')
    points.append(point);points.sort()
    entries.write_text(json.dumps(points,indent=2)+'\n')
else:
    print('Discovery limit reached; inspect the last target before continuing.')
