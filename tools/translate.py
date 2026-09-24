#!/usr/bin/env python3
"""Ahead-of-time translation of recovered LDM control flow into native C++.

No instruction bytes are decoded at runtime. Unresolved control flow fails with
its original address so recovery can be expanded without silently approximating.
"""
import json
from pathlib import Path
import struct
from collections import defaultdict
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG
from analyze import recover

REG8 = {'al':('ax',0),'ah':('ax',8),'bl':('bx',0),'bh':('bx',8),
        'cl':('cx',0),'ch':('cx',8),'dl':('dx',0),'dh':('dx',8)}
CONDS = {
    'je':'s.flag(ZF)','jne':'!s.flag(ZF)','jb':'s.flag(CF)','jae':'!s.flag(CF)',
    'jbe':'(s.flag(CF)||s.flag(ZF))','ja':'(!s.flag(CF)&&!s.flag(ZF))',
    'jl':'(s.flag(SF)!=s.flag(OF))','jge':'(s.flag(SF)==s.flag(OF))',
    'jle':'(s.flag(ZF)||(s.flag(SF)!=s.flag(OF)))',
    'jg':'(!s.flag(ZF)&&(s.flag(SF)==s.flag(OF)))',
    'js':'s.flag(SF)','jns':'!s.flag(SF)','jo':'s.flag(OF)','jno':'!s.flag(OF)',
    'jp':'s.flag(PF)','jnp':'!s.flag(PF)','jcxz':'s.cx==0',
}
MOVEMENT_READ_SITES = {
    (0x0fa7,0x009d):('9a3a00a70f',True),
    (0x0fa7,0x00a4):('9a3a00a70f',True),
    (0x0fa7,0x00a2):('eb0c',False),
    (0x0fa7,0x00a9):('b90800',False),
    # The main command poll can consume a direction between movement ticks,
    # or while the hand is active. It forwards directions through DS:5a1a.
    (0x0000,0x0882):('9a4000680b',True),
    (0x0000,0x0887):('8946f4',False),
    (0x0000,0x0a90):('9a4000680b',True),
    (0x0000,0x0a95):('8946fa',False),
}


class Emitter:
    def __init__(self, image, meta, instructions):
        self.image, self.meta, self.instructions = image, meta, instructions
        self.relocations = set(meta['relocations'])
        self.cs = self.ip = 0
        self.ins = None

    def reg(self, r):
        name = self.ins.reg_name(r)
        if name in REG8:
            parent, shift = REG8[name]
            return f'uint8_t(s.{parent} >> {shift})'
        if name not in ('ax','bx','cx','dx','si','di','bp','sp','cs','ds','es','ss','ip'):
            raise ValueError(f'Unsupported register {name}')
        return 's.'+name

    def address(self, o, delta=0):
        m = o.mem
        base = self.ins.reg_name(m.base) if m.base else ''
        seg = self.ins.reg_name(m.segment) if m.segment else ('ss' if base=='bp' else 'ds')
        terms = [self.reg(r) for r in (m.base,m.index) if r]
        terms.append(str(m.disp+delta))
        return 's.'+seg, 'uint16_t('+ '+'.join(terms)+')'

    def read(self, o):
        if o.type == X86_OP_REG: return self.reg(o.reg)
        if o.type == X86_OP_IMM:
            value = o.imm
            pos = self.cs*16+self.ip
            if self.ins.mnemonic not in ('lcall','ljmp') and any(p in self.relocations for p in range(pos,pos+self.ins.size)):
                value = (value+0x1000)&65535
            return str(value & ((1<<(o.size*8))-1))
        if o.type == X86_OP_MEM:
            seg,off = self.address(o)
            if o.size not in (1,2): raise ValueError(f'Unsupported memory size {o.size}')
            return f's.u{o.size*8}({seg},{off})'
        raise ValueError('Unsupported operand')

    def write(self, o, value):
        if o.type == X86_OP_REG:
            name=self.ins.reg_name(o.reg)
            if name in REG8:
                parent,shift=REG8[name]
                return f's.{parent}=uint16_t((s.{parent}&{0xff00 if shift==0 else 0xff})|((uint16_t({value})&255)<<{shift}));'
            return f'{self.reg(o.reg)}=uint16_t({value});'
        seg,off=self.address(o)
        return f's.w{o.size*8}({seg},{off},uint{o.size*8}_t({value}));'

    def goto(self, target, yield_back=True):
        if target <= self.ip and yield_back:
            return f's.ip={target}; return;'
        if (self.cs,target) in self.instructions: return f'goto L{target:04x};'
        return f's.ip={target}; return;'

    def translate(self, cs,ip,i):
        self.cs,self.ip,self.ins=cs,ip,i
        if cs==0x05d6 and ip in (0x0627,0x064f,0x06cf):
            # A desert close-up used to be a timed preview. Space pressed while
            # it was displayed stayed queued and opened another random preview.
            if i.bytes.hex()!='9a0c000505':raise ValueError('Unexpected desert preview delay')
            return ['if(s.qol_improvements || s.desert_view_active) {',
                    'if(!s.finish_desert_view())return;',self.goto(ip+5,False),'}',
                    f's.push(s.cs); s.push({ip+5}); s.cs=LoadSegment+0x0505; s.ip=0x000c; return;']
        if (cs,ip)==(0x08c0,0x0088):
            # DS:5b60 is already saved. On resume AX contains the *interior* X;
            # use the same loaded-scene flag as the saloon's position setup.
            if i.bytes.hex()!='a3605b':raise ValueError('Unexpected building return position')
            return ['if(!s.u16(s.ds,0x5b86))s.w16(s.ds,0x5b60,s.ax);',self.goto(ip+3,False)]
        if (cs,ip)==(0x033f,0x0303):
            # Restore the outer loop's signed comparison at 0300. The supplied
            # EXE bypasses its intact animation with an unconditional jump.
            # QoL is latched at entry; the original counter/RNG/timing stay intact.
            if i.bytes.hex()!='e90a01':raise ValueError('Unexpected panning reward layout')
            return ['s.ip=s.panning_active && s.flag(SF)!=s.flag(OF)?0x0308:0x0410; return;']
        if (cs,ip)==(0x033f,0x039e):
            # The second bypass skips all three frames. Reconnect the surviving
            # cmp [bp-4],312 to its original body at 03a0 -> 0316.
            if i.bytes.hex()!='eb03':raise ValueError('Unexpected panning frame loop layout')
            return [f'if(s.flag(SF)==s.flag(OF)) {{ {self.goto(0x03a3,False)} }}',self.goto(0x03a0,False)]
        if (cs,ip)==(0x1265,0x0693):
            # Choose VGA before the original SELECT.BIN load/draw/input block.
            # Rejoin the original accepted-'3' path, retaining its initialization.
            if i.bytes.hex()!='ba22fa': raise ValueError('Unexpected video selector layout')
            return ['s.ax=0x1333;',self.goto(0x06f7,False)]
        m=i.mnemonic; ops=i.operands; nxt=(ip+i.size)&65535
        a=lambda n=0:self.read(ops[n])
        w=lambda v,n=0:self.write(ops[n],v)
        bits=ops[0].size*8 if ops else 16
        code=[]; terminal=False
        if m=='mov': code=[w(a(1))]
        elif m in ('add','adc','sub','sbb','and','or','xor','cmp','test'):
            op={'cmp':'Sub','test':'And'}.get(m,m.capitalize())
            expr=f's.alu(Op::{op},{a()},{a(1)},{bits})'
            code=[expr+';' if m in ('cmp','test') else w(expr)]
        elif m in ('inc','dec'):
            code=[w(f's.alu(Op::{m.capitalize()},{a()},1,{bits})')]
        elif m=='neg': code=[w(f's.alu(Op::Neg,0,{a()},{bits})')]
        elif m=='not': code=[w('~'+a())]
        elif m in ('shl','shr','sar','rol','ror','rcl','rcr'):
            code=[w(f's.shift(Shift::{m.capitalize()},{a()},{a(1)},{bits})')]
        elif m=='push': code=[f's.push({a()});']
        elif m=='pop': code=['auto value=s.pop();',w('value')]
        elif m=='pushf': code=['s.push(s.flags|2);']
        elif m=='popf': code=['s.flags=s.pop()|2;']
        elif m=='xchg': code=[f'auto value={a()};',w(a(1)),w('value',1)]
        elif m=='lea': code=[w(self.address(ops[1])[1])]
        elif m in ('lds','les'):
            seg,off=self.address(ops[1]);_,off2=self.address(ops[1],2)
            code=[f'auto offset=s.u16({seg},{off});',f'auto segment=s.u16({seg},{off2});',w('offset'),f's.{m[1:]}=segment;']
        elif m in ('cwde','cbw'): code=['s.ax=uint16_t(int16_t(int8_t(s.ax)));']
        elif m in ('cdq','cwd'): code=['s.dx=(s.ax&0x8000)?0xffff:0;']
        elif m in ('mul','imul'):
            if len(ops)!=1: raise ValueError('Multi-operand multiply not yet recovered')
            code=[f's.multiply({a()},{bits},{str(m=="imul").lower()});']
        elif m in ('div','idiv'): code=[f's.divide({a()},{bits},{str(m=="idiv").lower()});']
        elif m in CONDS:
            code=[f'if({CONDS[m]}) {{ {self.goto(ops[0].imm&65535)} }}']
        elif m.startswith('loop'):
            cond='s.cx!=0'
            if m in ('loope','loopz'):cond+='&&s.flag(ZF)'
            if m in ('loopne','loopnz'):cond+='&&!s.flag(ZF)'
            code=['--s.cx;',f'if({cond}) {{ {self.goto(ops[0].imm&65535)} }}']
        elif m=='jmp':
            terminal=True
            code=[self.goto(ops[0].imm&65535)] if ops[0].type==X86_OP_IMM else [f's.ip={a()}; return;']
        elif m=='call':
            terminal=True
            code=[f'auto target=uint16_t({a()});',f's.push({nxt});','s.ip=target; return;']
        elif m in ('lcall','ljmp'):
            terminal=True
            if len(ops)==2:
                code=[f'uint16_t target_cs={ops[0].imm+0x1000},target_ip={ops[1].imm};']
            else:
                seg,off=self.address(ops[0]);_,off2=self.address(ops[0],2)
                code=[f'auto target_ip=s.u16({seg},{off});',f'auto target_cs=s.u16({seg},{off2});']
            if m=='lcall':code += ['s.push(s.cs);',f's.push({nxt});']
            code+=['s.cs=target_cs; s.ip=target_ip; return;']
        elif m in ('ret','retf','iret'):
            terminal=True; code=['s.ip=s.pop();']
            if m in ('retf','iret'):code+=['s.cs=s.pop();']
            if m=='iret':code+=['s.flags=s.pop()|2;']
            if ops:code += [f's.sp+={ops[0].imm};']
            code+=['return;']
        elif m=='int':code=[f's.interrupt({ops[0].imm});', 'if(!s.running || s.waiting) return;']
        elif m=='in':code=[w(f's.in({a(1)},{bits})')]
        elif m=='out':code=[f's.out({a()},{a(1)},{ops[1].size*8});']
        elif m in ('clc','stc','cmc','cld','std','cli','sti'):
            f={'clc':'CF,false','stc':'CF,true','cmc':'CF,!s.flag(CF)','cld':'DF,false','std':'DF,true','cli':'IF,false','sti':'IF,true'}[m]
            code=[f's.set_flag({f});']
        elif m=='lahf':code=['s.ax=(s.ax&255)|(((s.flags&0xd5)|2)<<8);']
        elif m=='sahf':code=['s.flags=(s.flags&~0xd5)|((s.ax>>8)&0xd5)|2;']
        elif m=='xlatb':code=['s.ax=(s.ax&0xff00)|s.u8(s.ds,uint16_t(s.bx+uint8_t(s.ax)));']
        elif any(m.endswith(x) for x in ('movsb','movsw','stosb','stosw','lodsb','lodsw','scasb','scasw','cmpsb','cmpsw')):
            op=m.split()[-1]; size=2 if op[-1]=='w' else 1
            rep=2 if m.startswith('repe ') else (3 if m.startswith('repne ') else (1 if m.startswith('rep ') else 0))
            source='s.ds'
            for o in ops:
                if o.type==X86_OP_MEM and o.mem.base and i.reg_name(o.mem.base)=='si' and o.mem.segment:
                    source=self.reg(o.mem.segment)
            code=[f's.string_op("{op[:4]}",{size},{rep},{source});']
        elif m=='nop':pass
        else:raise ValueError(f'Unsupported instruction {cs:04x}:{ip:04x}: {m} {i.op_str}')
        if (cs,ip)==(0x0fc5,0x0038):
            # This original helper reads buttons, Y and X through three BIOS
            # calls. Latch one desktop event for the whole helper so a short
            # click survives batching and all three reads agree on its point.
            if i.bytes.hex()!='55':raise ValueError('Unexpected mouse polling layout')
            code.insert(0,'s.mouse.poll();')
        panning_sites = {
            (0x033f,0x02cc):('55','s.begin_panning();'),
            (0x033f,0x0432):('5f','s.finish_panning();'),
        }
        if (cs,ip) in panning_sites:
            expected,hook=panning_sites[cs,ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected panning entry/return layout')
            code.insert(0,hook)
        combat_sites = {
            (0x040a,0x0002):('55','s.begin_combat();'),
            (0x040a,0x0288):('9a0600a70f','s.begin_combat_input();'),
            (0x0fa7,0x0011):('258000','s.filter_combat_mouse();'),
            (0x040a,0x028d):('e97800','s.finish_combat_input();'),
            (0x040a,0x0768):('5f','s.combat_active=false; s.combat_input_read=false;'),
        }
        if (cs,ip) in combat_sites:
            expected,hook=combat_sites[cs,ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected combat input layout')
            code.insert(0,hook)
        if (cs,ip)==(0x0fa7,0x0011):
            code.insert(0,'s.filter_world_mouse();')
        if (cs,ip)==(0x0fa7,0x00dc):
            # SI already contains the combined held directions. A single key's
            # scan (especially auto-repeat) must not replace that with one axis.
            # Keep taps without held input, Space and menu keys on their paths.
            if i.bytes.hex()!='23ff':raise ValueError('Unexpected held direction dispatch')
            code.insert(0,'if(direction_scan(s.di) && (s.si&15))s.di=0;')
        pointer_sites = {
            (0x0000,0x07f5):('833e625d00',f'if(s.qol_improvements && s.world_click_pending) {{ {self.goto(0x07fc,False)} }}'),
            # Only the walking command poll treats selector result 9 as a
            # scene restart. Suppress it here to preserve cave return X/Y.
            # Buildings call the shared selector directly and need result 9
            # to select items, notably gold bags in the assay office.
            (0x0000,0x0801):('eb29','if(s.qol_improvements && !s.combat_active && s.ax==9)s.ax=0;'),
            (0x0000,0x084b):('8d46fa',f'if(s.dispatch_world_click()) {{ {self.goto(0x08cc,False)} }}'),
            (0x0000,0x08bf):('23c0',f'if(s.qol_improvements && !s.combat_active && !s.ax) {{ {self.goto(0x0914,False)} }}'),
            (0x0fa7,0x0032):('c706045a0100','s.reset_world_pointer();'),
        }
        if (cs,ip) in pointer_sites:
            expected,hook=pointer_sites[cs,ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected world pointer layout')
            code.insert(0,hook)
        # Twice as many original walking steps while a direction is held.
        # Advance the per-loop survival clock and mine hazard RNG every other
        # fast step; all other scenes, delays and the PIT/music remain original.
        # Saloon walking loops back to 0acb; 0ac6 runs only on initial entry.
        walk_starts={(0x0000,0x029d):'9a0800d605',(0x08c0,0x0acb):'9ae6000505',
                     (0x0bb4,0x069b):'9ae6000505'}
        walk_updates={(0x0000,0x02bd),(0x08c0,0x0acb),(0x0bb4,0x069b)}
        walk_waits={(0x0000,0x02b0),(0x08c0,0x0b28),(0x0bb4,0x0741)}
        if (cs,ip) in walk_updates:
            if i.bytes.hex()!='9ae6000505':raise ValueError('Unexpected walking clock call')
            code.insert(0,f'if(s.walk_extra_tick) {{ {self.goto(nxt,False)} }}')
        if (cs,ip) in walk_starts:
            if i.bytes.hex()!=walk_starts[cs,ip]:raise ValueError('Unexpected walking loop')
            code.insert(0,'s.begin_walk_tick();')
        if (cs,ip) in walk_waits:
            if i.bytes.hex()!='9a8c000505':raise ValueError('Unexpected walking delay')
            code.insert(0,'if(s.walk_fast)s.w16(s.ss,s.sp,(s.u16(s.ss,s.sp)+1)/2);')
        if (cs,ip)==(0x0bb4,0x076d):
            if i.bytes.hex()!='b83000':raise ValueError('Unexpected mine hazard poll')
            code.insert(0,f'if(s.walk_extra_tick) {{ {self.goto(0x07a6,False)} }}')
        mule_checks={0x2235:(0,'833e5a5d00'),0x2273:(1,'833e5c5d00'),0x22b1:(2,'833e5e5d00')}
        if cs==0x08c0 and ip in mule_checks:
            index,expected=mule_checks[ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected mule purchase guard')
            code=[f's.alu(Op::Sub,s.mule_available({index})?0:1,0,16);']
        menu_sites = {
            (0x0505,0x096b):('cb','s.game_ui.prepare(s);'),
            (0x0505,0x032e):('55','s.game_ui.context(s);'),
            (0x0505,0x0538):('9a65006512','s.game_ui.context_buttons=0;'),
            (0x0000,0x082e):('55','s.game_ui.choosing=true;'),
            (0x0000,0x093d):('cb','s.game_ui.choosing=false;'),
            (0x0000,0x093e):('55','s.game_ui.map_click(s); s.game_ui.begin_menu(); s.game_ui.choosing=false; s.game_ui.mule_shop_visible=false;'),
            (0x0000,0x0a83):('cb','s.game_ui.end_menu(); s.game_ui.choosing=true;'),
            (0x0000,0x0ae4):('55','s.game_ui.begin_menu(); s.game_ui.mule_shop_visible=false;'),
            (0x0000,0x0b6b):('cb','s.game_ui.end_menu();'),
            # Saloon sleep runs after its command dispatcher has returned.
            # Suspend the panel for the entire noninteractive sleep routine.
            (0x08c0,0x29b6):('b8a409','s.game_ui.begin_menu();'),
            (0x08c0,0x2a8b):('cb','s.game_ui.end_menu();'),
            (0x08c0,0x1de2):('55','s.game_ui.mule_shop=true;'),
            (0x08c0,0x21b5):('cb','s.game_ui.mule_shop=false; s.game_ui.mule_shop_visible=false;'),
            (0x08c0,0x0189):('b80600','s.game_ui.mule_shop_visible=s.game_ui.mule_shop;'),
            (0x08c0,0x2065):('83c404','s.game_ui.mule_shop_visible=false;'),
        }
        if (cs,ip) in menu_sites:
            expected,hook=menu_sites[cs,ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected original panel layout')
            code.insert(0,hook)
        if (cs,ip) in MOVEMENT_READ_SITES:
            # Only movement/command polls use native direction aliases. Clear
            # the context before dispatching commands that open text fields.
            # The simulation thread decides, independently of desktop snapshots.
            expected,active=MOVEMENT_READ_SITES[cs,ip]
            if i.bytes.hex()!=expected:raise ValueError('Unexpected movement key reader layout')
            code.insert(0,f's.movement_key_read={str(active).lower()};')
        if (cs,ip)==(0x0000,0x0887):
            # In the world's hand-cursor loop a direction is already forwarded
            # to DS:5a1a, but the original waits for a right click to return to
            # walking. Reuse that exact hide-cursor/keyboard-mode return path.
            # Dialogs/text fields use their own readers and never pass here.
            directions=' || '.join(f's.ax==0x{scan:02x}' for scan in (0x47,0x48,0x49,0x4b,0x4d,0x4f,0x50,0x51))
            code.insert(1,f'if({directions}) {{ {self.goto(0x0914,False)} }}')
        if not terminal:code += [self.goto(nxt,False)]
        return code


def main():
    root=Path('recovered'); image=(root/'load-image.bin').read_bytes()
    meta=json.loads((root/'executable.json').read_text())
    # Optional evidence-based additional entry points (indirect calls/tables).
    extra=Path('tools/entry-points.json')
    instructions,_,_,_=recover(image,meta,json.loads(extra.read_text()) if extra.exists() else ())
    out=Path('build/generated');out.mkdir(parents=True,exist_ok=True)
    fingerprint=14695981039346656037
    for byte in image:fingerprint=((fingerprint^byte)*1099511628211)&0xffffffffffffffff
    grouped=defaultdict(dict)
    for (cs,ip),i in instructions.items():grouped[cs][ip]=i
    e=Emitter(image,meta,instructions)
    for cs,items in sorted(grouped.items()):
        lines=['// Generated at build time from the user-supplied game. Do not publish.',
               '#include "legacy.h"','namespace ldm {',f'void segment_{cs:04x}(State& s) {{','switch(s.ip) {']
        for ip in sorted(items):lines.append(f'case {ip}: goto L{ip:04x};')
        lines+=['default: s.fail("Unrecovered control-flow target"); return;','}']
        for ip,i in sorted(items.items()):
            lines += [f'L{ip:04x}: {{ // {cs:04x}:{ip:04x} {i.mnemonic} {i.op_str}',f's.ip={ip};']
            lines += e.translate(cs,ip,i)
            lines += ['}']
        lines += ['}', '}']
        write_changed(out/f'segment_{cs:04x}.cpp','\n'.join(lines)+'\n')
    lines=['#include "legacy.h"','namespace ldm {']
    for cs in sorted(grouped):lines += [f'void segment_{cs:04x}(State&);']
    lines += ['void native_step(State& s) {','++s.boundaries;','switch(uint16_t(s.cs-LoadSegment)) {']
    for cs in sorted(grouped):lines += [f'case {cs}: segment_{cs:04x}(s); break;']
    lines += ['default: s.fail("Unrecovered code segment");','}', '}', '}']
    write_changed(out/'dispatch.cpp','\n'.join(lines)+'\n')
    write_changed(out/'image_info.h','#pragma once\n#include <vector>\n#include <cstdint>\n'+
        'inline const std::vector<uint32_t> image_relocations = {'+','.join(map(str,meta['relocations']))+'};\n'+
        f"constexpr uint16_t entry_cs={meta['entry_cs']},entry_ip={meta['entry_ip']},stack_ss={meta['stack_ss']},stack_sp={meta['stack_sp']};\n"+
        f'constexpr unsigned image_bytes={len(image)};\nconstexpr uint64_t image_fingerprint={fingerprint}ULL;\n')
    print(f'Translated {len(instructions)} instructions into {len(grouped)} native C++ units')


def write_changed(path,content):
    if not path.exists() or path.read_text()!=content:path.write_text(content)


if __name__=='__main__':main()
