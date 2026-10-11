#!/usr/bin/env python3
"""Check integer induction with negative differences and higher-degree parents."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

import run as spectral

HERE = Path(__file__).resolve().parent


def main():
    formulas = [
        ('descending', '(100 - j) * (100 - j)', lambda i, j: (100 - j) ** 2),
        ('descending-transpose', '(100 - i) * (100 - i)', lambda i, j: (100 - i) ** 2),
        ('cubic', '(j + 1) * (j + 1) * (j + 1)', lambda i, j: (j + 1) ** 3),
        ('large', '(1000000000000000000 - j) * 9', lambda i, j: (10**18 - j) * 9),
    ]
    source = []
    expected = []
    for name, expression, formula in formulas:
        source.append(f'{name}(i<u64>, j<u64>) = (x<f64>) := x := number/to-f64({expression}).')
        for columns in (1, 3, 8, 9, 17):
            for transpose in (False, True):
                operation = 'matrix/apply-transpose' if transpose else 'matrix/apply'
                source.append(f'io/write(string/concat(text/join(number/format({operation}(sequence/fill({columns}, 1.0), 7, {name}, "multiply"), 0), ","), "\\n"))')
                expected.append([sum(float(formula(j, i) if transpose else formula(i, j))
                                     for j in range(columns)) for i in range(7)])
    configs = [(False, 1, 1), (True, 2, 2), (True, 4, 4)]
    checked = 0
    with tempfile.TemporaryDirectory(prefix='mech-induction-edges-') as temporary:
        folder = Path(temporary)
        program = folder / 'edges.mec'
        program.write_text('\n'.join(source) + '\n')
        for simd, unroll, rows in configs:
            for induction in (0, 1):
                binary = folder / 'edges'
                argv = [str(spectral.ROOT / 'benchmarks/clbg/nbody/mech-driver/target/release/mech'),
                        'build', str(program), '--backend', 'cranelift', '--no-config',
                        '--offline', '-o', str(binary)]
                if not simd:
                    argv.append('--scalar')
                env = os.environ | {
                    'CARGO_TARGET_DIR': str(spectral.ROOT / 'benchmarks/clbg/nbody/mech-driver/target'),
                    'RUSTFLAGS': '-D warnings', 'MECH_NUMERIC_INDUCTION': str(induction),
                    'MECH_NUMERIC_UNROLL': str(unroll), 'MECH_NUMERIC_ROWS': str(rows),
                    'MECH_SEQUENCE_PARTITIONS': '8', 'RAYON_NUM_THREADS': '8',
                }
                subprocess.run(argv, cwd=spectral.ROOT, env=env, check=True,
                               stdout=subprocess.DEVNULL)
                result = subprocess.run([str(binary)], env=env, check=True, capture_output=True)
                actual = [[float(value) for value in line.split(',')]
                          for line in result.stdout.decode().splitlines()]
                assert len(actual) == len(expected)
                for values, wanted in zip(actual, expected):
                    assert len(values) == len(wanted)
                    # SIMD may change the summation order for large f64 values.
                    for value, want in zip(values, wanted):
                        assert abs(value - want) <= max(1e-8, abs(want) * 3e-15), (value, want)
                checked += len(actual)
                print(f'simd={simd} unroll={unroll} rows={rows} induction={induction}: {len(actual)} passed', flush=True)
    output = HERE / 'results/induction-edge-conformance.json'
    output.write_text(json.dumps({
        'all_passed': True, 'checked_results': checked,
        'formulas': [{'name': name, 'expression': expression} for name, expression, _ in formulas],
        'policies': [{'simd': s, 'unroll': u, 'rows': r} for s, u, r in configs],
        'induction_enabled_and_disabled': True,
        'transpose_and_scalar_tails': True,
        'negative_differences_and_large_unsigned_values': True,
        'cubic_parent_with_quadratic_subgraph': True,
    }, indent=2) + '\n')


if __name__ == '__main__':
    main()
