#!/usr/bin/env python3
"""Execute pinned all-literals block choice and fixed code table construction."""
import argparse
import hashlib
import json
from pathlib import Path
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
    cases = [dict(name=name, input=data, begin=begin, length=length) for name, data, begin, length in [
        ('empty', [], 0, 0), ('eight-literals', list(range(8)), 0, 8),
        ('eight-overlap', [97] * 8, 0, 8), ('320-overlap', [97, 98, 99, 100] * 80, 0, 320),
        ('alphabet', list(range(256)), 0, 256), ('slice', list(range(32)), 9, 13),
        ('missing-reads', [], 0, 3), ('past-end', [4, 5], 1, 5),
    ]]
    arrays = ['freqs_litlen', 'freqs_offset', 'litlen_lens', 'litlen_codewords', 'offset_lens', 'offset_codewords']
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import CompressOptimal
import CompressLazy
main! = |mode| {
'''
    for index, case in enumerate(cases):
        app += f'    if mode == {index} {{\n' + literal('input', case['input'], 8)
        app += f'        result = CompressOptimal.choose_all_literals(input, {case["begin"]}, {case["length"]})\n'
        app += ''.join(print_list(key, 'result.' + key) for key in arrays) + '    } else {}\n'
    app += f'    if mode == {len(cases)} {{\n        result = CompressLazy.build_static_codes(0)\n'
    app += ''.join(print_list(key, 'result.' + key) for key in arrays[2:]) + '    } else {}\n}\n'
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / 'package/MechLiteralChoiceOracle.roc'
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / 'literal-choice-reference'
    command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(source)]
    subprocess.run(command, check=True)
    static = dict(name='fixed-codes')
    for index, case in enumerate(cases + [static]):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{index}-{case["name"]}.txt').write_text(raw)
        expected = {key: [] for key in (arrays if index < len(cases) else arrays[2:])}
        for line in raw.splitlines():
            key, value = line.split()
            expected[key].append(int(value))
        for key, values in expected.items():
            assert len(values) == (32 if 'offset' in key else 288), (case['name'], key)
        case['expected'] = expected
    digest = lambda data: hashlib.sha256(data).hexdigest()
    output = {'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN,
              'package_source_sha256': {name: digest(data) for name, data in package.items()},
              'generated_source_sha256': digest(app.encode()), 'binary_sha256': digest(binary.read_bytes()),
              'command': command}, 'cases': cases, 'static': static}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + '\n')
    print(json.dumps({'literal_cases': len(cases), 'fixed_code_cases': 1, 'output': str(args.out)}))


if __name__ == '__main__':
    main()
