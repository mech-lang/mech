#!/usr/bin/env python3
"""Fast AOT lexical-capture conformance; no performance measurements."""
import argparse
import json
import os
import pathlib
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
DEFAULT_CLI = ROOT / 'benchmarks/clbg/nbody/mech-driver/target/release/mech'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=pathlib.Path, default=os.environ.get('MECH_CLI', DEFAULT_CLI))
    parser.add_argument('--report', type=pathlib.Path, default=HERE / 'verification-capture.json')
    options = parser.parse_args()
    environment = os.environ | {
        'CARGO_TARGET_DIR': str(ROOT / 'benchmarks/clbg/nbody/mech-driver/target'),
        'RUSTFLAGS': '-D warnings',
        'CPATH': '/opt/homebrew/opt/gmp/include:/opt/homebrew/opt/mpfr/include',
        'LIBRARY_PATH': '/opt/homebrew/opt/gmp/lib:/opt/homebrew/opt/mpfr/lib',
        'MECH_NUMERIC_UNROLL': '4',
        'MECH_NUMERIC_ROWS': '4',
    }

    def build(source, binary, diagnostic=None):
        result = subprocess.run([
            str(options.cli), 'build', str(source), '--backend', 'cranelift',
            '--no-config', '--offline', '-o', str(binary),
        ], cwd=ROOT, env=environment, capture_output=True, text=True, timeout=120)
        if diagnostic is None:
            if result.returncode:
                raise AssertionError(result.stdout + result.stderr)
        else:
            if result.returncode == 0:
                raise AssertionError(f'invalid capture program accepted: {source.name}')
            if diagnostic not in result.stdout + result.stderr:
                raise AssertionError(f'{source.name}: expected {diagnostic!r}: ' + result.stdout + result.stderr)
        return result

    with tempfile.TemporaryDirectory(prefix='mech-application-captures-') as temporary:
        temporary = pathlib.Path(temporary)
        binary = temporary / 'captures'
        build(HERE / 'capture-generic.mec', binary)
        expected = (
            b'7.000\n16.000\n31.000\n4.000\n11.000\n41.000\n23.000\n15.000\n18.000\n6.000\n21.000\n91.000\n2.50\n'
            b'ROOT:>one\nTG\nCA\nROOT:>two\nAC\nGT\n'
        )
        result = subprocess.run([str(binary), '5', '2'], env=environment, capture_output=True, timeout=30)
        if result.returncode or result.stdout != expected:
            raise AssertionError((result.returncode, result.stdout, expected, result.stderr))
        invalid = {
            'not-yet-defined': (
                'Add(x<f64>) => <f64> := x + offset\n'
                'result := sequence/iterate(1.0, 2, Add)\n'
                'offset := 5.0\n'
                'io/write(number/format(result, 3))\n',
                'not defined before use',
            ),
            'undefined-global': (
                'Add(x<f64>) => <f64> := x + missing\n'
                'result := sequence/iterate(1.0, 2, Add)\n'
                'io/write(number/format(result, 3))\n',
                'undefined variable missing',
            ),
            'caller-local-is-not-a-global': (
                'Inner(x<f64>) => <f64> := x + offset\n'
                'Outer(x<f64>) = (out<f64>) := offset := 5.0; out := Inner(x).\n'
                'result := sequence/iterate(1.0, 2, Outer)\n'
                'io/write(number/format(result, 3))\n',
                'undefined variable offset',
            ),
            'coefficient-capture-not-implemented': (
                'offset := 1.0\n'
                'Entry(i<u64>, j<u64>) => <f64> := number/to-f64(i + j + 1) + offset\n'
                'result := matrix/apply(sequence/fill(2, 1.0), 2, Entry, "multiply")\n'
                'io/write(number/format(stats/dot(result, result), 3))\n',
                'matrix coefficient captures require scalar capture lowering',
            ),
            'duplicate-parameter': (
                'Add(x<f64>, x<f64>) => <f64> := x + x\n'
                'result := Add(1.0, 2.0)\n'
                'io/write(number/format(result, 3))\n',
                'duplicate function parameter',
            ),
        }
        for name, (source, diagnostic) in invalid.items():
            path = temporary / f'{name}.mec'
            path.write_text(source)
            build(path, temporary / name, diagnostic)

    report = {
        'cli': str(options.cli),
        'direct_capture': True,
        'nested_higher_order_capture': True,
        'parameter_and_local_shadowing': True,
        'global_in_local_initializer': True,
        'immutable_vector_capture': True,
        'four_scalar_captures': True,
        'nine_scalar_captures_multiple_environment_appends': True,
        'captured_vector_aliases_iteration_seed_without_mutation': True,
        'five_records_map_captures': True,
        'byte_translation_table_capture': True,
        'integer_and_text_capture': True,
        'records_map_capture_snapshot': True,
        'function_before_global_before_application': True,
        'rejected_cases': list(invalid),
        'all_passed': True,
    }
    options.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
