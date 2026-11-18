"""Measure fixed-frame traffic and immediate pairing coverage on the bound root."""
from collections import Counter, defaultdict
from hashlib import file_digest
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
OUT = HERE / 'stack-census'
SYMBOL = '_mech_compact_closed_checked'
ENTRY = re.compile(r'^([0-9a-f]+) <([^>]+)>:$')
INST = re.compile(r'^\s+([0-9a-f]+):\s+(\w[\w.]*)\s*(.*?)\s*$')
FRAME = re.compile(r'([wx])([0-9]+), \[(sp|x29)(?:, #(0x[0-9a-f]+|-0x[0-9a-f]+|[0-9]+))?\]$')
TARGET = re.compile(r'\b(0x\w+) <')


def digest(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def root(path, output):
    cmd = ['xcrun', 'llvm-objdump', '-d', '--no-show-raw-insn', str(path)]
    raw = subprocess.check_output(cmd, text=True)
    symbol = None
    instructions = {}
    output.write_text(raw)
    for line in raw.splitlines():
        if label := ENTRY.fullmatch(line):
            if symbol == SYMBOL:
                break
            symbol = label[2]
        if symbol == SYMBOL and (inst := INST.fullmatch(line)):
            pc, op, args = inst.groups()
            instructions[int(pc, 16)] = (op, args.split(';', 1)[0].strip())
    assert instructions
    assert len(instructions) * 4 <= 51_264
    return cmd, instructions


def analyze(instructions):
    # Ignore a second instruction with an independently reachable branch
    # entry. These are encoding ceilings, without relocations/stack maps.
    branch_heads = set()
    for op, args in instructions.values():
        if op in ('b', 'cbz', 'cbnz', 'tbz', 'tbnz') or op.startswith('b.'):
            if target := TARGET.search(args):
                branch_heads.add(int(target[1], 16))
    offsets = Counter()
    counts = Counter()
    frame = {}
    for pc, (op, args) in instructions.items():
        if m := FRAME.fullmatch(args):
            if op in ('ldr', 'str', 'ldur', 'stur'):
                width = 8 if m[1] == 'x' else 4
                reg, base, offset = int(m[2]), m[3], int(m[4], 0) if m[4] else 0
                counts[op] += 1
                offsets[(base, offset)] += 1
                frame[pc] = (op[:2], width, reg, base, offset)
    pairs = defaultdict(list)
    for pc, a in sorted(frame.items()):
        if (b := frame.get(pc + 4)) is None or (pc + 4) in branch_heads:
            continue
        kind, width, reg1, base, offset1 = a
        _, _, reg2, _, offset2 = b
        if (kind, width, base) != (b[0], b[1], b[3]) or abs(offset1 - offset2) != width:
            continue
        if reg1 >= 29 or reg2 >= 29 or (kind == 'ld' and reg1 == reg2):
            continue
        if min(offset1, offset2) % width:
            continue
        pair = (pc, kind, width, base, offset1, offset2)
        pairs['adjacent'].append(pair)
        if -64*width <= min(offset1, offset2) <= 63*width:
            pairs['encodable'].append(pair)

    def disjoint(items):
        selected = []
        last = -8
        for row in items:
            if row[0] >= last + 8:
                selected.append(row)
                last = row[0]
        return selected

    return {
        'instructions': len(instructions), 'stack_scalar_ops': dict(counts),
        'stack_locations': len(offsets), 'most_used_slots': offsets.most_common(24),
        'all_adjacent_frame_pairs': len(disjoint(pairs['adjacent'])),
        'encodable_immediate_pairs': disjoint(pairs['encodable']),
        'pair_upper_bound_bytes': 4 * len(disjoint(pairs['encodable'])),
    }


def main():
    OUT.mkdir()
    bound = json.loads((HERE / 'bound/binding.json').read_bytes())
    results = {}
    for role, binding in bound['artifacts'].items():
        path = Path(binding['linked_encoder']['path'])
        assert digest(path) == binding['linked_encoder']['sha256']
        assembly = OUT / f'{role}.txt'
        command, instructions = root(path, assembly)
        results[role] = analyze(instructions) | {'object_sha256': digest(path), 'command': command,
                                                'disassembly_sha256': digest(assembly)}
    (OUT / 'summary.json').write_text(json.dumps(results, indent=2) + '\n')
    print({role: {k:v for k,v in info.items() if k in ('stack_scalar_ops','stack_locations',
                                                      'all_adjacent_frame_pairs','pair_upper_bound_bytes')}
           for role,info in results.items()}, flush=True)


if __name__ == '__main__':
    main()
