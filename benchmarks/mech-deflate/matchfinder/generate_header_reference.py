#!/usr/bin/env python3
"""Execute the pinned dynamic-header builder and exact block-cost calculation."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources
from generate_path_reference import pretty


def digest(data):
    return hashlib.sha256(data).hexdigest()


def literal(name, values, width):
    return f'        {name} = [' + ', '.join(f'{v}.U{width}' for v in values) + ']\n'


def print_list(name, expression):
    return f'''        var $i_{name} = 0.U64
        while $i_{name} < {expression}.len() {{
            echo!("{name} ${{(List.get({expression}, $i_{name}) ?? 0).to_str()}}")
            $i_{name} = $i_{name} + 1
        }}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root', type=Path)
    parser.add_argument('--roc', required=True, type=Path)
    parser.add_argument('--work-dir', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = args.reference_root.resolve()
    try:
        package = pinned_package_sources(root)
    except ValueError as error:
        parser.error(str(error))
    fixtures = Path(__file__).resolve().parents[3] / 'hosts/gpu/tests/fixtures/native'
    saved = fixtures / 'deflate-tally-reference.json'
    tallies = json.loads(saved.read_text())
    if tallies['reference']['source_pin'] != PIN:
        parser.error('tally inputs differ from the pinned revision')
    runs = [(f'zero-{n}', [0] * n, n) for n in [0, 1, 2, 3, 10, 11, 138, 139, 276]]
    runs += [(f'nonzero-{n}', [7] * n, n) for n in [1, 2, 3, 4, 6, 7, 9, 10, 15]]
    mixed = [0] * 11 + [5] * 4 + [0] * 2 + [3] * 10 + [0] * 138 + [14] * 7
    runs += [('mixed', mixed, len(mixed)), ('alternating', [i % 16 for i in range(320)], 320),
             ('prefix', [1, 1, 1, 1, 1, 8, 9], 5), ('missing-reads', [], 12)]
    precode_cases = [dict(name=name, lens=lens, num_lens=n,
                          freqs=[i * 3 for i in range(19)],
                          items=[0xA0000000 + i for i in range(320)])
                     for name, lens, n in runs]
    fields = ['freqs', 'lens', 'codewords', 'items']
    scalars = ['num_items', 'num_litlen_syms', 'num_offset_syms', 'num_explicit_lens']
    header_cases = [dict(name=case['name'], **{key: case[key] for key in
                    ['litlen_lens', 'offset_lens', 'freqs_litlen', 'freqs_offset']})
                    for case in tallies['cases']]
    for name, litlens, offlens in [
            ('all-zero', [0] * 288, [0] * 32),
            ('full-lengths', [i % 16 for i in range(288)], [i % 16 for i in range(32)])]:
        header_cases.append(dict(name=name, litlen_lens=litlens, offset_lens=offlens,
                                 freqs_litlen=[(i * 17) % 101 for i in range(513)],
                                 freqs_offset=[(i * 19) % 97 for i in range(270)]))
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import BlockOut
import CompressOptimal
main! = |mode| {
'''
    for index, case in enumerate(precode_cases):
        app += f'    if mode == {index} {{\n'
        for key, width in [('lens', 8), ('freqs', 32), ('items', 32)]:
            app += literal(key, case[key], width)
        app += f'        result = BlockOut.compute_precode_items(lens, {case["num_lens"]}, freqs, items)\n'
        app += print_list('freqs', 'result.freqs') + print_list('items', 'result.items')
        app += '        echo!("num_items ${result.num_items.to_str()}")\n    } else {}\n'
    for index, case in enumerate(header_cases, len(precode_cases)):
        app += f'    if mode == {index} {{\n'
        for key in ['litlen_lens', 'offset_lens', 'freqs_litlen', 'freqs_offset']:
            app += literal(key, case[key], 8 if key.endswith('lens') else 32)
        app += '        result = BlockOut.precompute_huffman_header(litlen_lens, offset_lens)\n'
        for field in fields:
            app += print_list(field, 'result.' + field)
        for field in scalars:
            app += f'        echo!("{field} ${{result.{field}.to_str()}}")\n'
        app += '        cost = CompressOptimal.compute_true_cost(litlen_lens, offset_lens, freqs_litlen, freqs_offset)\n'
        app += '        echo!("cost ${cost.to_str()}")\n    } else {}\n'
    app += '}\n'
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / 'package/MechHeaderOracle.roc'
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / 'header-reference'
    command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(source)]
    subprocess.run(command, check=True)
    for index, case in enumerate(precode_cases + header_cases):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{index}-{case["name"]}.txt').write_text(raw)
        expected = {key: [] for key in (['freqs', 'items'] if index < len(precode_cases) else fields)}
        for line in raw.splitlines():
            key, value = line.split()
            if key in expected and isinstance(expected[key], list):
                expected[key].append(int(value))
            else:
                expected[key] = int(value)
        if len(expected['freqs']) != 19 or len(expected['items']) != 320:
            raise ValueError(f'unexpected precode buffers: {case["name"]}')
        if index >= len(precode_cases) and (len(expected['lens']) != 19 or len(expected['codewords']) != 19):
            raise ValueError(f'unexpected precode code: {case["name"]}')
        case['expected'] = expected
    table_source = package['package/DeflateTables.roc'].decode()
    tables = {key: [int(v) for v in re.findall(r'\d+', table_source.split(key + ' = [', 1)[1].split(']', 1)[0])]
              for key in ['precode_lens_permutation', 'extra_precode_bits', 'extra_length_bits', 'extra_offset_bits']}
    if [len(v) for v in tables.values()] != [19, 19, 29, 30]:
        raise ValueError('unexpected pinned header tables')
    output = {'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN,
              'package_source_sha256': {name: digest(data) for name, data in package.items()},
              'input_tally_fixture_sha256': digest(saved.read_bytes()),
              'generated_source_sha256': digest(app.encode()),
              'binary_sha256': digest(binary.read_bytes()), 'command': command},
              'tables': tables, 'precode_cases': precode_cases, 'header_cases': header_cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + '\n')
    print(json.dumps({'precode_cases': len(precode_cases), 'header_cases': len(header_cases),
                      'source_pin': PIN, 'output': str(args.out)}))


if __name__ == '__main__':
    main()
