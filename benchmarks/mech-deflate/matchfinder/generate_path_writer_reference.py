#!/usr/bin/env python3
"""Execute the pinned path writer with existing qualified representations."""
import argparse, sys, json, subprocess, hashlib
from pathlib import Path
repo = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description='Execute the pinned ' + 'path' + ' writer component')
parser.add_argument('reference_root', type=Path)
parser.add_argument('--roc', required=True, type=Path)
parser.add_argument('--work-dir', required=True, type=Path)
parser.add_argument('--out', required=True, type=Path)
args = parser.parse_args()
sys.path.insert(0, str(repo / 'benchmarks/mech-deflate/matchfinder'))
from generate_block_reference import pinned_package_sources, PIN, COMPILER_PIN
from generate_header_reference import literal, print_list
from generate_path_reference import pretty
root = args.reference_root.resolve()
package = pinned_package_sources(root)
source = package['package/BlockOut.roc'].decode()
start = source.index('\t\t\titems_top = List.len(items)')
end = source.index('\n\n\t\t\tif use_items == 0', start)
loop = source[start:end]
fixtures = repo / 'hosts/gpu/tests/fixtures/native'
paths = json.loads((fixtures / 'deflate-optimal-path-reference.json').read_text())
tables = json.loads((fixtures / 'deflate-writer-tables-reference.json').read_text())
assert paths['reference']['source_pin'] == tables['reference']['source_pin'] == PIN
fixed = next((c for c in tables['cases'] if c['name'] == 'codes-fixed'))
offset = next((c for c in tables['cases'] if c['name'] == 'offsets-fixed'))
full = next((c for c in tables['full_cases'] if c['name'] == 'fixed'))['expected']
vectors = {'litlen-words': fixed['codewords'], 'litlen-lens': fixed['lens'], 'full-words': full['codewords'], 'full-lens': full['lens'], 'offset-words': offset['codewords'], 'offset-lens': offset['lens'], 'offset-slots': paths['offset_slots'], 'offset-bases': tables['offset_slot_base'], 'offset-extra': tables['extra_offset_bits']}
cases = [dict(name=c['name'] + ('-carry' if carry else ''), items=c['nodes'], block_length=c['block_length'], out=[99, 17] if carry else [], bitbuf=5 if carry else 0, bitcount=3 if carry else 0) for c in paths['cases'] for carry in [False, True]]
app = 'app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }\nimport pf.Host exposing [echo!]\nimport DeflateTables\nwrite : List(U8), U64, U64, List(U64), U64, List(U32), List(U8), List(U32), List(U8), List(U32), List(U8), U64 -> {out: List(U8), bitbuf: U64, bitcount: U64}\nwrite = |out, bitbuf, bitcount, items, block_length, w_litlen_codewords, w_litlen_lens, full_codewords, full_lens, w_offset_codewords, w_offset_lens, use_items| {\n    var $out = out\n    var $bitbuf = bitbuf\n    var $bitcount = bitcount\n' + loop + '\n    {out: $out, bitbuf: $bitbuf, bitcount: $bitcount}\n}\nmain! = |mode| {\n'
for i, c in enumerate(cases):
    app += f'    if mode == {i} {{\n' + literal('items', c['items'], 64) + literal('out', c['out'], 8)
    names = ['litlen-words', 'litlen-lens', 'full-words', 'full-lens', 'offset-words', 'offset-lens']
    for n in names:
        app += literal(n.replace('-', '_'), vectors[n], 8 if 'lens' in n else 32)
    app += f"        result = write(out, {c['bitbuf']}, {c['bitcount']}, items, {c['block_length']}, " + ', '.join((n.replace('-', '_') for n in names)) + ', 1)\n' + print_list('out', 'result.out')
    app += '        echo!("bitbuf ${result.bitbuf.to_str()}")\n        echo!("bitcount ${result.bitcount.to_str()}")\n    } else {}\n'
app += '}\n'
work = args.work_dir.resolve()
work.mkdir(exist_ok=True)
src = root / 'package/MechPathWriterOracle.roc'
src.write_text(app)
(work / src.name).write_text(app)
binary = work / 'path-writer-reference'
command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(src)]
subprocess.run(command, check=True)
for i, c in enumerate(cases):
    raw = subprocess.check_output([str(binary), str(i)], text=True)
    (work / f'{i}.txt').write_text(raw)
    c['expected'] = {'out': [int(l.split()[1]) for l in raw.splitlines() if l.startswith('out ')]}
    for n in ['bitbuf', 'bitcount']:
        c['expected'][n] = int(next((l.split()[1] for l in raw.splitlines() if l.startswith(n + ' '))))
hash = lambda b: hashlib.sha256(b).hexdigest()
args.out.write_text(pretty({'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN, 'package_source_sha256': {n: hash(b) for n, b in package.items()}, 'loop_sha256': hash(loop.encode()), 'generated_source_sha256': hash(app.encode()), 'binary_sha256': hash(binary.read_bytes()), 'command': command}, 'tables': vectors, 'cases': cases}) + '\n')
print(json.dumps({'cases': len(cases), 'largest_block': max((c['block_length'] for c in cases))}))
