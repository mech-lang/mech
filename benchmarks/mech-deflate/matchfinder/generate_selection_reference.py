#!/usr/bin/env python3
"""Execute the unchanged pinned optimization and final selection spans with qualified block caches."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources
from generate_header_reference import literal, print_list
from generate_path_reference import pretty


def digest(data):
    return hashlib.sha256(data).hexdigest()


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
    saved = fixtures / 'deflate-block-reference.json'
    blocks = json.loads(saved.read_text())
    if blocks['reference']['source_pin'] != PIN:
        parser.error('block inputs differ from the pinned revision')
    original = package['package/CompressOptimal.roc'].decode()
    start = original.index('\t\t\tvar $freqs_litlen = List.repeat(0.U32, DeflateTables.num_litlen_syms)')
    end = original.index('\n\t\t\tvar $use_items = 1.U64', start)
    # Execute this exact source span, including the original placeholder
    # allocations, saved-model assignments, stop condition and ten-pass policy.
    loop = original[start:end]
    selection_end = original.index('\n\t\t\tflushed = BlockOut.flush_block(', end)
    selection = original[end:selection_end]
    static_start = original.index('\t\t\tvar $static_cost = ')
    static_end = original.index('\n\t\t\t# Set the model', static_start)
    static_path = original[static_start:static_end]
    cases = []
    for block in blocks['cases']:
        frequencies = [0] * 259
        for at, value in block['frequencies']:
            frequencies[at] = value
        cases.append(dict(name=block['name'], input=block['input'], frequencies=frequencies,
                          begin=block['scalars']['block-begin'],
                          length=block['scalars']['block-end'] - block['scalars']['block-begin'],
                          cache=block['cache'], cache_end=block['scalars']['cache-end'], passes=10))
    arrays = ['literal', 'length', 'offset_slot', 'saved_literal', 'saved_length', 'saved_offset_slot',
              'nodes', 'freqs_litlen', 'freqs_offset', 'litlen_lens', 'litlen_codewords', 'offset_lens', 'offset_codewords']
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import CompressOptimal
import CompressLazy
import DeflateTables
main! = |mode| {
'''
    for index, case in enumerate(cases):
        app += f'    if mode == {index} {{\n'
        app += literal('input', case['input'], 8) + literal('frequencies', case['frequencies'], 32)
        app += '        var $cache = List.repeat(0.U32, 1500513)\n'
        for at, value in case['cache']:
            app += f'''        $cache = match List.set($cache, {at}, {value}.U32) {{
            Ok(next) => next
            Err(_) => crash "oracle cache index"
        }}
'''
        app += f'''        block_length = {case['length']}.U64 + (mode - {index}.U64)
        var $cache_end = {case['cache_end']}.U64
        var $nodes = List.with_capacity(CompressOptimal.optimum_nodes_size)
        var $cost_literal = List.repeat(0.U32, 256)
        var $cost_length = List.repeat(0.U32, 259)
        var $cost_offset_slot = List.repeat(0.U32, 32)
        var $saved_literal = List.repeat(0.U32, 256)
        var $saved_length = List.repeat(0.U32, 259)
        var $saved_offset_slot = List.repeat(0.U32, 32)
        defaults = CompressOptimal.choose_default_litlen_costs(input, {case['begin']}, block_length, frequencies, 300)
        initial = CompressOptimal.set_default_costs(defaults.lit_cost, defaults.len_sym_cost, $cost_literal, $cost_length, $cost_offset_slot)
        $cost_literal = initial.literal
        $cost_length = initial.length
        $cost_offset_slot = initial.offset_slot
        params = {{ max_optim_passes: {case['passes']}.U64, min_improvement_to_continue: 1.U64, min_bits_to_use_nonfinal_path: 1.U64, max_len_to_optimize_static_block: 10000.U64 }}
'''
        app += f"""        var $in_block_begin = {case['begin']}.U64
        var $seqs = List.repeat({{ litrunlen_and_length: 0.U32, offset: 0.U16, offset_slot: 0.U16 }}, 1)
        static = CompressLazy.build_static_codes(0)
        var $s_litlen_lens = static.litlen_lens
        var $s_offset_lens = static.offset_lens
        lits = CompressOptimal.choose_all_literals(input, $in_block_begin, block_length)
        only_lits_cost = CompressOptimal.compute_true_cost(lits.litlen_lens, lits.offset_lens, lits.freqs_litlen, lits.freqs_offset)
"""
        app += static_path + '\n' + loop + '\n' + selection + '\n'
        app += '        echo!("only_lits_cost ${only_lits_cost.to_str()}")\n'
        app += '        echo!("static_cost ${$static_cost.to_str()}")\n'
        app += '        echo!("use_items ${$use_items.to_str()}")\n'
        app += '        echo!("used_only_literals ${$used_only_literals.to_str()}")\n'
        app += '        seq = List.get($seqs, 0) ?? {litrunlen_and_length: 0.U32, offset: 0.U16, offset_slot: 0.U16}\n'
        app += '        echo!("seq_length ${seq.litrunlen_and_length.to_str()}")\n'
        expressions = ['$cost_literal', '$cost_length', '$cost_offset_slot', '$saved_literal', '$saved_length', '$saved_offset_slot',
                       '$nodes', '$freqs_litlen', '$freqs_offset', '$litlen_lens', '$litlen_codewords', '$offset_lens', '$offset_codewords']
        for name, expression in zip(arrays, expressions):
            app += print_list(name, expression)
        for name in ['best_true_cost', 'true_cost', 'passes_remaining']:
            app += '        echo!("' + name + ' ${$' + name + '.to_str()}")\n'
        app += '    } else {}\n'
    app += '}\n'
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / 'package/MechSelectionOracle.roc'
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / 'selection-reference'
    command = [str(args.roc), 'build', '--opt=speed', f'--output={binary}', str(source)]
    subprocess.run(command, check=True)
    for index, case in enumerate(cases):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{case["name"]}.txt').write_text(raw)
        expected = {name: [] for name in arrays}
        for line in raw.splitlines():
            name, value = line.split()
            if name in arrays:
                expected[name].append(int(value))
            else:
                expected[name] = int(value)
        frequency_sizes = [288, 32] if expected['used_only_literals'] == 1 else [513, 270]
        sizes = [256, 259, 32, 256, 259, 32, case['length'] + 258, *frequency_sizes, 288, 288, 32, 32]
        if [len(expected[name]) for name in arrays] != sizes:
            raise ValueError(f'unexpected optimization buffers for {case["name"]}')
        case['expected'] = expected
    output = {'reference': {'source_pin': PIN, 'compiler_pin': COMPILER_PIN,
              'package_source_sha256': {name: digest(data) for name, data in package.items()},
              'input_block_fixture_sha256': digest(saved.read_bytes()),
              'unchanged_loop_sha256': digest(loop.encode()), 'unchanged_selection_sha256': digest(selection.encode()), 'unchanged_selection_byte_range': [end, selection_end], 'unchanged_static_path_sha256': digest(static_path.encode()), 'unchanged_loop_byte_range': [start, end],
              'generated_source_sha256': digest(app.encode()),
              'binary_sha256': digest(binary.read_bytes()), 'command': command}, 'cases': cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + '\n')
    print(json.dumps([(case['name'], case['expected']['best_true_cost'], case['expected']['passes_remaining']) for case in cases]))


if __name__ == '__main__':
    main()
