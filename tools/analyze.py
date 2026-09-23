#!/usr/bin/env python3
"""Conservative recursive 8086 disassembly for the native-port investigation."""
from collections import Counter, deque
import json
from pathlib import Path
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_16, CS_GRP_JUMP, CS_GRP_CALL, CS_GRP_RET, CS_GRP_IRET
from capstone.x86 import X86_OP_IMM


def recover(image, meta, extra_seeds=()):
    md = Cs(CS_ARCH_X86, CS_MODE_16)
    md.detail = True
    seeds = {(meta['entry_cs'], meta['entry_ip']), (0, 0)}
    seeds.update(tuple(x) for x in extra_seeds)
    # EXEPACK relocations let us identify immediate far-call/jump operands without
    # treating every byte in data tables as an instruction.
    for p in meta['relocations']:
        if p >= 3 and image[p-3] in (0x9a, 0xea):
            off, seg = struct.unpack_from('<HH', image, p-2)
            if seg * 16 + off < 0x20000:
                seeds.add((seg, off))
    queue = deque(sorted(seeds))
    result = {}
    indirect = []
    invalid = []
    while queue:
        seg, off = queue.popleft()
        while (seg, off) not in result:
            linear = seg * 16 + off
            if not 0 <= linear < len(image) or off > 65535:
                invalid.append((seg, off))
                break
            ins = next(md.disasm(image[linear:linear+15], off, 1), None)
            if ins is None:
                invalid.append((seg, off))
                break
            result[(seg, off)] = ins
            nxt = (off + ins.size) & 65535
            jump = ins.group(CS_GRP_JUMP)
            call = ins.group(CS_GRP_CALL)
            if jump or call:
                if ins.mnemonic in ('lcall', 'ljmp') and len(ins.operands) == 2:
                    queue.append((ins.operands[0].imm, ins.operands[1].imm))
                elif ins.operands and ins.operands[0].type == X86_OP_IMM:
                    queue.append((seg, ins.operands[0].imm & 65535))
                else:
                    indirect.append((seg, off, ins.mnemonic, ins.op_str))
                if ins.mnemonic in ('jmp', 'ljmp'):
                    break
            if ins.group(CS_GRP_RET) or ins.group(CS_GRP_IRET) or ins.mnemonic == 'hlt':
                break
            off = nxt
    return result, seeds, indirect, invalid


if __name__ == '__main__':
    root = Path('recovered')
    data = (root/'load-image.bin').read_bytes()
    meta = json.loads((root/'executable.json').read_text())
    entry_points=Path('tools/entry-points.json')
    extra=json.loads(entry_points.read_text()) if entry_points.exists() else ()
    ins, seeds, indirect, invalid = recover(data, meta, extra)
    counts = Counter(i.mnemonic for i in ins.values())
    (root/'disassembly.txt').write_text('\n'.join(
        f'{cs:04x}:{ip:04x}  {i.bytes.hex():20} {i.mnemonic:9} {i.op_str}'
        for (cs, ip), i in sorted(ins.items())) + '\n')
    (root/'analysis.json').write_text(json.dumps({
        'instructions': len(ins), 'seeds': sorted(seeds),
        'mnemonics': counts, 'indirect_control_flow': indirect,
        'invalid_targets': invalid,
    }, indent=2) + '\n')
    print('Instructions:',len(ins),'seed targets:',len(seeds))
    print('Mnemonics:',dict(counts.most_common()))
    print('Indirect branches:',len(indirect),'invalid targets:',invalid[:15])
