#!/usr/bin/env python3
"""Execute the pinned block writer's cost and selection statements verbatim."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from generate_block_reference import pinned_package_sources, PIN, COMPILER_PIN
from generate_header_reference import literal
from generate_path_reference import pretty


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root', type=Path)
    parser.add_argument('--roc', type=Path, required=True)
    parser.add_argument('--work-dir', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = args.reference_root.resolve()
    package = pinned_package_sources(root)
    original = package['package/BlockOut.roc'].decode()
    start = original.index('\t\tprecode = BlockOut.precompute_huffman_header', original.index('flush_block ='))
    end = original.index('\n\n\t\tvar $out = out_0', start)
    calculation = original[start:end]
    fixtures = Path(__file__).resolve().parents[3] / 'hosts/gpu/tests/fixtures/native'
    headers = json.loads((fixtures / 'deflate-header-reference.json').read_text())
    tables = json.loads((fixtures / 'deflate-writer-tables-reference.json').read_text())
    assert headers['reference']['source_pin'] == tables['reference']['source_pin'] == PIN
    fixed = next(c['lens'] for c in tables['cases'] if c['name'] == 'codes-fixed')
    cases = []
    for c in headers['header_cases']:
        for length, bits in [(1, 0), (65535, 5), (65536, 7)]:
            cases.append(dict(name=f"{c['name']}-{length}-{bits}", block_length=length,
                bitcount=bits, **{k:c[k] for k in ['litlen_lens','offset_lens','freqs_litlen','freqs_offset']},
                precode={k:v for k,v in c['expected'].items() if k != 'cost'}))
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import BlockOut
import DeflateTables
cost = |freqs_litlen, freqs_offset, litlen_lens, offset_lens, s_litlen_lens, block_length_0, bitcount_0| {
''' + calculation + '''
    kind = if best_cost == uncompressed_cost { 0.U64 } else if best_cost == $static_cost { 1 } else { 2 }
    { dynamic: $dynamic_cost, fixed: $static_cost, stored: uncompressed_cost, kind }
}
main! = |mode| {
'''
    for i,c in enumerate(cases):
        app += f'    if mode == {i} {{\n'
        for key in ['freqs_litlen','freqs_offset','litlen_lens','offset_lens']:
            app += literal(key,c[key],32 if key.startswith('freqs') else 8)
        app += literal('s_litlen_lens',fixed,8)
        app += f"        result = cost(freqs_litlen, freqs_offset, litlen_lens, offset_lens, s_litlen_lens, {c['block_length']}.U64, {c['bitcount']}.U64)\n"
        for key in ['dynamic','fixed','stored','kind']:
            app += f'        echo!("{key} ${{result.{key}.to_str()}}")\n'
        app += '    } else {}\n'
    app += '}\n'
    work = args.work_dir.resolve(); work.mkdir(parents=True,exist_ok=True)
    source = root / 'package/MechBlockChoiceOracle.roc'; source.write_text(app)
    (work/source.name).write_text(app)
    binary = work/'block-choice-reference'
    command = [str(args.roc),'build','--opt=speed',f'--output={binary}',str(source)]
    subprocess.run(command,check=True)
    for i,c in enumerate(cases):
        raw = subprocess.check_output([str(binary),str(i)],text=True)
        (work/f'{i}.txt').write_text(raw)
        c['expected']={k:int(v) for k,v in (line.split() for line in raw.splitlines())}
    digest=lambda b:hashlib.sha256(b).hexdigest()
    result=dict(reference=dict(source_pin=PIN,compiler_pin=COMPILER_PIN,
        package_source_sha256={k:digest(v) for k,v in package.items()},
        calculation_sha256=digest(calculation.encode()),generated_source_sha256=digest(app.encode()),
        binary_sha256=digest(binary.read_bytes()),command=command),
        tables={**headers['tables'],'fixed_litlen_lens':fixed},cases=cases)
    args.out.write_text(pretty(result)+'\n')
    print(json.dumps({'cases':len(cases),'kinds':sorted(set(c['expected']['kind'] for c in cases))}))

if __name__ == '__main__':
    main()
