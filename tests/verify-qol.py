#!/usr/bin/env python3
"""Verify QoL desktop evidence (isolated saloon/map saves and two cadence runs).

The saloon script expects a COPY of the supplied WeirdBug slot 6 in its save
directory. The desert script uses a copy of slot 1 with scene=map, X=40, Y=55,
map offsets=0/0, and other scene flags cleared (same save blocks as the river
fixture). Never edit the source saves. Run cadence once with qol=0 and once
with qol=1 into cadence-classic/captures and cadence-qol/captures respectively.
"""
import json
from pathlib import Path
import sys

captures=Path(sys.argv[1]) if len(sys.argv)>1 else Path('captures')
cadence=Path(sys.argv[2]) if len(sys.argv)>2 else Path('.local/qol-desktop')
def read(root,n):return json.loads((root/f'{n}.json').read_text())
def position(s):return tuple(s[k] for k in ('x','y','town_page','building'))

saloon=[read(captures,n) for n in range(1100,1104)]
assert saloon[0]['building']==2 and saloon[0]['x']==210,saloon[0]
assert saloon[1]['building']==0 and (saloon[1]['x'],saloon[1]['y'])==(240,59),saloon[1]
assert saloon[2]['y']==73 and position(saloon[3])!=position(saloon[2]),saloon
desert=[read(captures,n) for n in range(1200,1205)]
assert [s['desert_view'] for s in desert]==[0,1,1,0,0],desert
assert all(s['map_view'] and position(s)==position(desert[0]) for s in desert),desert
distance=[];clocks=[]
for label in ('classic','qol'):
    frames=[read(cadence/f'cadence-{label}'/'captures',n) for n in range(1300,1306)]
    distance.append(frames[1]['x']-frames[0]['x'])
    clocks.append(frames[2]['survival_ticks']-frames[0]['survival_ticks'])
    assert position(frames[2])==position(frames[3]),'Release did not stop walking'
    assert position(frames[4])==position(frames[5]),'Focus loss did not stop walking'
    assert all(s['pointer_visible']==(label=='qol') for s in frames),frames
assert distance[1]>distance[0]*1.5,distance
assert abs(clocks[1]-clocks[0])<=1,clocks
print(f'PASS: original slot 6 load/Exit/walking, Space toggle/repeat, visible cursor; '
      f'held movement {distance[0]} -> {distance[1]} pixels with survival clock {clocks}')
