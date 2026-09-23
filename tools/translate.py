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
        if (cs,ip)==(0x033f,0x0303):
            # The supplied executable jumps directly to the panning reward.
            # Optional native play rejoins that reward or the original cleanup.
            if i.bytes.hex()!='e90a01':raise ValueError('Unexpected panning reward layout')
            return ['s.pan_action(); return;']
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
