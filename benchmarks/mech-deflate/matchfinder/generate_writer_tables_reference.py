#!/usr/bin/env python3
"""Execute the pinned block writer's packed-code table builders."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources
from generate_header_reference import literal, print_list
from generate_path_reference import pretty


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root', type=Path)
    parser.add_argument('--roc', required=True, type=Path)
    parser.add_argument('--work-dir', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = args.reference_root.resolve()
    package = pinned_package_sources(root)
    tables = (root / 'package/DeflateTables.roc').read_text()
    def table(name):
        body = re.search(r'\b' + name + r' = \[(.*?)\]', tables, re.S).group(1)
        return [int(n) for n in re.findall(r'\d+', body)]
    fixed = json.loads((Path(__file__).resolve().parents[3] / 'hosts/gpu/tests/fixtures/native/deflate-literal-choice-reference.json').read_text())
    assert fixed['reference']['source_pin'] == PIN
    fixed = fixed['static']['expected']
    cases = []
    for kind, prefix in [('codes', 'litlen'), ('offsets', 'offset')]:
        for name, words, lens in [
            ('empty', [], []), ('fixed', fixed[prefix + '_codewords'], fixed[prefix + '_lens']),
            ('missing-lengths', [0, 1, 0xffffffff], [15]),
        ]:
            cases.append(dict(name=kind + '-' + name, kind=kind, codewords=words, lens=lens))
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import BlockOut
main! = |mode| {
'''
    for i, case in enumerate(cases):
        app += f'    if mode == {i} {{\n' + literal('words', case['codewords'], 32) + literal('lens', case['lens'], 8)
        app += f'        packed = BlockOut.pack_{case["kind"]}(words, lens)\n'
        app += print_list('packed', 'packed') + '    } else {}\n'
    full_cases = [dict(name='fixed', lens=fixed['litlen_lens'], codewords=fixed['litlen_codewords']), dict(name='zeros', lens=[0]*288, codewords=[0]*288)]
    for i, case in enumerate(full_cases, len(cases)):
        app += f'    if mode == {i} {{\n' + literal('words', case['codewords'], 32) + literal('lens', case['lens'], 8)
        app += '        result = BlockOut.compute_full_len_codewords(lens, words)\n'
        app += print_list('codewords', 'result.codewords') + print_list('lens', 'result.lens') + '    } else {}\n'
    app += '}\n'
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / 'package/MechWriterTablesOracle.roc'
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / 'writer-tables-reference'
    command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(source)]
    subprocess.run(command, check=True)
    for i, case in enumerate(cases):
        raw = subprocess.check_output([str(binary), str(i)], text=True)
        (args.work_dir / f'{i}-{case["name"]}.txt').write_text(raw)
        case['packed'] = [int(line.split()[1]) for line in raw.splitlines()]
        assert len(case['packed']) == len(case['codewords'])
    for i, case in enumerate(full_cases, len(cases)):
        raw = subprocess.check_output([str(binary), str(i)], text=True)
        (args.work_dir / f'{i}-full-{case["name"]}.txt').write_text(raw)
        case['expected'] = {key: [int(line.split()[1]) for line in raw.splitlines() if line.split()[0] == key] for key in ['codewords', 'lens']}
        assert all(len(values) == 259 for values in case['expected'].values())
    digest = lambda data: hashlib.sha256(data).hexdigest()
    output = {'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN,
              'package_source_sha256': {name: digest(data) for name, data in package.items()},
              'generated_source_sha256': digest(app.encode()), 'binary_sha256': digest(binary.read_bytes()),
              'command': command}, 'extra_offset_bits': table('extra_offset_bits'),
              'offset_slot_base': table('offset_slot_base'), 'cases': cases, 'full_cases': full_cases, 'length_slot_tab': table('length_slot_tab'), 'length_slot_base': table('length_slot_base'), 'extra_length_bits': table('extra_length_bits')}
    args.out.write_text(pretty(output) + '\n')
    print(json.dumps({'packed_cases': len(cases), 'full_length_cases': len(full_cases), 'output': str(args.out)}))


if __name__ == '__main__':
    main()
