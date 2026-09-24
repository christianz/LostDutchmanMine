#!/usr/bin/env python3
"""Verify SDL assay before/after and cave regression captures.

Use isolated copies of the fixtures exported by test-assay and test-cave.
The evidence root contains assay-before, assay-after and cave-after.
"""
import json
from pathlib import Path
import sys

root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('.local/update13-desktop')
def frame(scenario,n):return json.loads((root/scenario/'captures'/f'{n}.json').read_text())
old=[frame('assay-before',n) for n in range(1800,1808)]
new=[frame('assay-after',n) for n in range(1800,1808)]
assert [s['gold_bags'] for s in old]==[8]*8,'Old build did not reproduce rejected bag clicks'
assert [s['gold_bags'] for s in new]==[8,8,7,7,7,6,6,6],'Wrong bags consumed'
assert all(s['building']==4 for s in new[:7]),'Assay left the office early'
assert new[0]['cash']==new[1]['cash']==1000
assert new[2]['assay_pounds']==5 and 5<=new[2]['assay_grade']<=10
first=5*new[2]['assay_grade']*10
assert new[2]['cash']==1000+first,'First assay payout differs from its weight/grade'
assert new[2]['cash']==new[3]['cash']==new[4]['cash'],'Empty slot or Next paid twice'
assert new[5]['assay_pounds']==6 and 5<=new[5]['assay_grade']<=10
second=6*new[5]['assay_grade']*10
assert new[5]['cash']==new[2]['cash']+second,'Mule bag payout differs from its weight/grade'
assert new[5]['cash']==new[6]['cash']==new[7]['cash'],'Done/Exit repeated a payout'
assert new[7]['building']==0 and (new[7]['x'],new[7]['y'])==(80,59),'Exit lost town doorway'

cave=[frame('cave-after',n) for n in range(1700,1706)]
assert cave[0]['map_view'] and cave[5]['map_view'] and not cave[5]['cave_view']
for k in ('x','y','map_scroll_x','map_scroll_y'):
    assert cave[0][k]==cave[5][k],('Cave exit displaced player',k,cave[0],cave[5])
for s in cave[2:5]:
    assert s['cave_view'] and not s['map_view']
    assert (s['return_x'],s['return_y'])==(cave[0]['x'],cave[0]['y']),'Cave click lost return position'
print('Before: all eight bags remain after assay clicks.')
print('After: eight -> seven -> six bags; cash 1000 ->',new[2]['cash'],'->',new[5]['cash'])
print('Cave entry/exit:',(cave[0]['x'],cave[0]['y']),'->',(cave[5]['x'],cave[5]['y']))
print('PASS: player/mule bag selection, exact payouts, empty-slot repeat, Next/Done/Exit and cave return')
