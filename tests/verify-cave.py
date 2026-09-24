#!/usr/bin/env python3
"""Verify before/after SDL captures from cave-roundtrip.txt.

Export an isolated map save with test-cave GAME-DIRECTORY FRESH-DIRECTORY.
Run the script against copies with the old and new binaries. The evidence root
contains before/captures and after/captures; no player's save is edited.
"""
import json
from pathlib import Path
import struct
import sys

root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('.local/update12-desktop')
def frame(version,n):return json.loads((root/version/'captures'/f'{n}.json').read_text())
def position(s):return s['x'],s['y']
def scroll(s):return s['map_scroll_x'],s['map_scroll_y']
def dark_scene(version,n):
    data=(root/version/'captures'/f'{n}.bmp').read_bytes()
    offset=struct.unpack_from('<I',data,10)[0]
    width,height=struct.unpack_from('<ii',data,18)
    assert width==320 and abs(height)==200 and struct.unpack_from('<H',data,28)[0]==32
    pixels=list(struct.iter_unpack('<I',data[offset:offset+320*200*4]))
    return sum((pixels[(199-y if height>0 else y)*320+x][0]&0xffffff)==0
               for y in range(10,110) for x in range(320))

old_start,old_end=[frame('before',n) for n in (1700,1705)]
new_start,new_end=[frame('after',n) for n in (1700,1705)]
assert old_start['map_view'] and old_end['map_view'] and new_start['map_view'] and new_end['map_view']
assert position(old_start)!=position(old_end),'Old build did not reproduce the displaced exit'
assert position(old_start)==position(new_start)==position(new_end),'New exit changed map coordinates'
assert scroll(new_start)==scroll(new_end),'New exit changed map scroll offsets'
for n in (1702,1703,1704):
    s=frame('after',n)
    assert s['cave_view'] and not s['map_view'] and position(s)==(60,50),('Unexpected cave state',n,s)
    assert (s['return_x'],s['return_y'])==position(new_start),('Lost return position',n,s)
# The cave flag also stays set during the broken entrance replay. Check the
# actual scene pixels: the tunnel has a broad black ceiling/floor; outdoors does not.
assert dark_scene('before',1702)>dark_scene('before',1703)+5000,'Old click did not replay the outdoor view'
assert dark_scene('after',1703)>dark_scene('before',1703)+5000,'New click left the tunnel view'
print('Before:',position(old_start),'->',position(old_end))
print('After:',position(new_start),'->',position(new_end),'scroll:',scroll(new_start),'->',scroll(new_end))
print('PASS: scenery click stays in cave and walking out restores the exact map position and scroll')
