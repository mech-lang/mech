#!/usr/bin/env python3
"""Execute the pinned sequence writer with existing qualified representations."""
import argparse, sys, json, subprocess, hashlib
from pathlib import Path
repo = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description='Execute the pinned ' + 'sequence' + ' writer component')
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
tables = json.loads((repo / 'hosts/gpu/tests/fixtures/native/deflate-writer-tables-reference.json').read_text())
assert tables['reference']['source_pin'] == PIN
packed = next((c['packed'] for c in tables['cases'] if c['name'] == 'codes-fixed'))
full = next((c['expected'] for c in tables['full_cases'] if c['name'] == 'fixed'))
packed += [w + (n << 32) for w, n in zip(full['codewords'], full['lens'])] + [0] * (512 - 259)
packed += next((c['packed'] for c in tables['cases'] if c['name'] == 'offsets-fixed'))
assert len(packed) == 832
seq = lambda literals, length=0, offset=0: dict(litrunlen_and_length=literals + (length << 23), offset=offset, offset_slot=0)
cases = [dict(name=f'literals-{n}', input=[(i * 29 + 17) % 256 for i in range(n)], seqs=[seq(n)], out=[], bitbuf=0, bitcount=0) for n in range(10)]
cases += [dict(name=f'overlap-{n}', input=[65] * (n + 1), seqs=[seq(1, n, 1), seq(0)], out=[], bitbuf=0, bitcount=0) for n in [3, 258]]
cases += [dict(name=f'carry-{n}', input=[65] * n, seqs=[seq(n)], out=[99, 17], bitbuf=5, bitcount=3) for n in [1, 7]]
app = 'app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }\nimport pf.Host exposing [echo!]\nimport BlockOut\nmain! = |mode| {\n'
for i, c in enumerate(cases):
    app += f'    if mode == {i} {{\n' + literal('packed', packed, 64) + literal('input', c['input'], 8) + literal('out', c['out'], 8)
    app += '        seqs : List(BlockOut.Sequence)\n        seqs = [' + ', '.join(('{ ' + ', '.join((f'{k}: {v}' for k, v in s.items())) + ' }' for s in c['seqs'])) + ']\n'
    app += f"        result = BlockOut.write_sequences(out, {c['bitbuf']}, {c['bitcount']}, input, 0, seqs, packed)\n" + print_list('out', 'result.out')
    app += '        echo!("bitbuf ${result.bitbuf.to_str()}")\n        echo!("bitcount ${result.bitcount.to_str()}")\n    } else {}\n'
app += '}\n'
work = args.work_dir.resolve()
work.mkdir(exist_ok=True)
source = root / 'package/MechSequenceWriterOracle.roc'
source.write_text(app)
(work / source.name).write_text(app)
binary = work / 'sequence-writer-reference'
command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(source)]
subprocess.run(command, check=True)
for i, c in enumerate(cases):
    text = subprocess.check_output([str(binary), str(i)], text=True)
    (work / f'{i}.txt').write_text(text)
    c['expected'] = {'out': [int(line.split()[1]) for line in text.splitlines() if line.startswith('out ')]}
    for name in ['bitbuf', 'bitcount']:
        c['expected'][name] = int(next((line.split()[1] for line in text.splitlines() if line.startswith(name + ' '))))
hash = lambda data: hashlib.sha256(data).hexdigest()
result = {'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN, 'package_source_sha256': {n: hash(d) for n, d in package.items()}, 'generated_source_sha256': hash(app.encode()), 'binary_sha256': hash(binary.read_bytes()), 'command': command}, 'packed': packed, 'cases': cases}
args.out.write_text(pretty(result) + '\n')
print(json.dumps({'cases': len(cases), 'largest_input': max((len(c['input']) for c in cases)), 'output': str(args.out)}))
