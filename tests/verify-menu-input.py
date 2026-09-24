#!/usr/bin/env python3
"""Check before/after SDL evidence from menu-hover.txt and map-diagonals.txt.

The evidence root contains hover-before/after and diagonals-before/after, each
with captures. Use isolated copies of the river slot 1 and map slot 1 fixtures;
never run against a player's save directory.
"""
import json
from pathlib import Path
import struct
import sys

root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('.local/update11-desktop')
def frame(scenario,version,n):
    return json.loads((root/f'{scenario}-{version}/captures/{n}.json').read_text())
def gold(version,n):
    data=(root/f'hover-{version}/captures/{n}.bmp').read_bytes()
    offset=struct.unpack_from('<I',data,10)[0]
    assert struct.unpack_from('<H',data,28)[0]==32
    return sum(colour==0xffffd34e for colour, in struct.iter_unpack('<I',data[offset:]))
def position(state):return state['x'],state['y']

for n in (1402,1403,1406,1408):
    assert gold('before',n)>0,('Missing before reproduction',n)
    assert gold('after',n)==0,('Hidden button still highlights',n)
for n in (1400,1404,1407,1409):
    assert gold('after',n)>0,('River hover was not restored',n)

for i,(label,mask,dx,dy) in enumerate((('WA',5,-1,-1),('WD',9,1,-1),('SA',6,-1,1),('SD',10,1,1))):
    n=1500+i*4
    old=[frame('diagonals','before',n+j) for j in range(2)]
    new=[frame('diagonals','after',n+j) for j in range(4)]
    assert old[0]['y']==old[1]['y'],('Missing repeat reproduction',label,old)
    assert all(s['map_view'] for s in new),('Left map unexpectedly',label,new)
    assert new[0]['held_directions']==new[1]['held_directions']==mask
    assert (new[1]['x']-new[0]['x'])*dx>0 and (new[1]['y']-new[0]['y'])*dy>0,(label,new)
    assert new[2]['held_directions']==(mask&12) and new[3]['held_directions']==0
    assert position(new[2])==position(new[3]),('Release did not stop movement',label,new)
    print(label,'under repeat:',position(old[0]),'->',position(old[1]),'before;',position(new[0]),'->',position(new[1]),'after')
a,b=[frame('diagonals','after',n) for n in (1520,1521)]
assert a['held_directions']==b['held_directions']==0 and position(a)==position(b),'Focus loss retained movement'
distance={};clocks={}
for version in ('before','after','classic'):
    states=[frame('saloon',version,n) for n in range(1600,1606)]
    assert all(s['building']==2 for s in states),'Cadence check left the saloon'
    distance[version]=states[0]['x']-states[1]['x']
    clocks[version]=states[3]['survival_ticks']-states[0]['survival_ticks']
    assert position(states[2])==position(states[3]),'Saloon release retained movement'
    assert position(states[4])==position(states[5]),'Saloon focus loss retained movement'
# QoL off starts in the modal hand. Use update 10 with QoL on for the matching
# walking-mode comparison; the native test separately compares exact loop steps.
assert distance['after']>distance['before']*1.5 and distance['after']>distance['classic'],distance
assert abs(clocks['after']-clocks['classic'])<=1 and abs(clocks['after']-clocks['before'])<=1,clocks
print('Saloon pixels:',distance,'survival ticks:',clocks)
print('PASS: hidden hover removed, scene hover restored, four sustained diagonals, saloon speed, release and focus reset')
