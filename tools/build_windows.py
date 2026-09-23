#!/usr/bin/env python3
"""Cross-compile generated native code using an official local Zig toolchain."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import os, subprocess

root=Path(__file__).resolve().parents[1]
zig=root/'.local/deps/zig-x86_64-linux-0.15.1/zig'
sdl=root/'.local/deps/SDL2-2.32.0/x86_64-w64-mingw32'
out=root/'build-windows';out.mkdir(exist_ok=True)
env=dict(os.environ,ZIG_GLOBAL_CACHE_DIR=str(root/'.local/zig-cache'))
common=[str(zig),'c++','-target','x86_64-windows-gnu','-std=c++17','-O1','-Isrc','-Ibuild/generated','-Ithird_party/ymfm','-I'+str(sdl/'include/SDL2')]
sources=list((root/'build/generated').glob('*.cpp'))+[root/'src'/name for name in ('legacy.cpp','desktop.cpp','audio.cpp')]+[root/'third_party/ymfm'/name for name in ('ymfm_opl.cpp','ymfm_adpcm.cpp','ymfm_pcm.cpp')]
def compile(source):
    dest=out/(source.stem+'.o')
    if dest.exists() and dest.stat().st_mtime>max(source.stat().st_mtime,(root/'src/legacy.h').stat().st_mtime,(root/'build/generated/image_info.h').stat().st_mtime):return dest
    r=subprocess.run(common+['-c',str(source),'-o',str(dest)],cwd=root,env=env,capture_output=True,text=True)
    if r.returncode:raise RuntimeError(r.stdout+r.stderr)
    return dest
with ThreadPoolExecutor(max_workers=4) as pool:objects=list(pool.map(compile,sources))
exe=out/'LostDutchmanMine.exe'
subprocess.run(common+[str(o) for o in objects]+[str(sdl/'lib/libSDL2.dll.a'),'-o',str(exe)],cwd=root,env=env,check=True)
print('Built',exe,exe.stat().st_size,'bytes')
