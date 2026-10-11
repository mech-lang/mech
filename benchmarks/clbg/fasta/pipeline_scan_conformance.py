#!/usr/bin/env python3
"""Check split scan state, ordered IO, guards, tails and original error order."""
import argparse
import json
import subprocess
from pathlib import Path

import conformance
import run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=run.HERE / 'results/conformance-pipeline-20261008.json')
    args = parser.parse_args()
    conformance.BUILD = run.BUILD / 'conformance-pipeline'
    conformance.BUILD.mkdir(parents=True, exist_ok=True)
    checks = []
    generic = '''Map(x<u64>) = (byte<u64>) :=
  r := number/to-f64(x) / 97.0
  byte := control/select(r >= 0.3, 65, 97) + control/select(r >= 0.7, 1, 0).
Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := number/mod(i * 17 + 29, 97)
  byte := Map(next).
n := io/argument-u64(1)
width := io/argument-u64(3)
buffer := io/argument-u64(4)
io/write("first|")
seed := io/write-scan(n, io/argument-u64(2), Step, width, "|:|", buffer)
io/write("second|")
last := io/write-scan(n + 1, seed, Step, width, "|:|", buffer)
io/write(text/format("state={}\\n", last))
'''
    for enabled in (False, True):
        conformance.ENV['MECH_CONTROL_SCAN_PIPELINE'] = str(int(enabled))
        binary = conformance.build(f'ordered-{int(enabled)}', generic)
        symbols = subprocess.check_output(['xcrun', 'llvm-objdump', '--reloc', str(binary) + '.project/mech-control.o'], text=True)
        assert ('_mech_control_scan_pipeline' in symbols) == enabled
        for buffer in (1, 7, 64, 65536):
            for width in (0, 1, 7, 60, 2**64 - 1):
                counts = {0, 1, 2 * buffer - 1, 2 * buffer, 2 * buffer + 1}
                counts.update((65537, 131073) if buffer == 65536 else (257, 1001))
                for count in sorted(counts):
                    for initial in (0, 96, 2**64 - 1):
                        state = initial
                        expected = bytearray()
                        for header, length in ((b'first|', count), (b'second|', count + 1)):
                            expected.extend(header)
                            raw = bytearray()
                            for _ in range(length):
                                state = ((state * 17 + 29) % 2**64) % 97
                                r = state / 97.0
                                raw.append((65 if r >= .3 else 97) + int(r >= .7))
                            expected.extend(raw if not width else b''.join(raw[i:i+width] + b'|:|' for i in range(0, length, width)))
                        expected.extend(f'state={state}\n'.encode())
                        actual = subprocess.check_output([str(binary), str(count), str(initial), str(width), str(buffer)])
                        assert actual == expected, (enabled, width, buffer, initial, count)
                        checks.append(['ordered-split-scan', enabled, width, buffer, initial, count])
            print('passed ordered scans', enabled, buffer, flush=True)
        for buffer in (0, 1048577):
            result = subprocess.run([str(binary), '131073', '0', '60', str(buffer)], capture_output=True)
            assert result.returncode != 0 and result.stdout == b'first|'
            checks.append(['split-buffer-guard', enabled, buffer])
    # Both the mapper and producer can fail in a block. Their split scheduling
    # must preserve the original callback's output prefix and first error status.
    for label, next_expr, byte_expr in (
        ('mapper-first', 'number/mod(i / control/select(i == 7, 0, 1) + 1, 97)', 'control/select(next == 2, 256, 65)'),
        ('producer-first', 'number/mod(i / control/select(i == 7, 0, 1) + 1, 97)', 'control/select(next == 20, 256, 65)'),
        ('later-map-error', 'number/mod(i + 1, 100003)', 'control/select(next == 70000, 256, 65)'),
    ):
        source = f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := {next_expr}
  byte := {byte_expr}.
last := io/write-scan(131073, 0, Step, 7, "|", 65536)
'''
        results = []
        for enabled in (False, True):
            conformance.ENV['MECH_CONTROL_SCAN_PIPELINE'] = str(int(enabled))
            binary = conformance.build(label + '-' + str(int(enabled)), source)
            symbols = subprocess.check_output(['xcrun', 'llvm-objdump', '--reloc', str(binary) + '.project/mech-control.o'], text=True)
            assert ('_mech_control_scan_pipeline' in symbols) == enabled, (label, enabled)
            result = subprocess.run([str(binary)], capture_output=True, timeout=30)
            assert result.returncode != 0
            results.append((result.returncode, result.stdout, result.stderr))
        assert results[0] == results[1], (label, results)
        checks.append(['original-first-error', label])
    # Reading the old state alongside next state prevents the proof from splitting.
    conformance.ENV['MECH_CONTROL_SCAN_PIPELINE'] = '1'
    source = '''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := number/mod(i + 1, 97)
  byte := number/mod(i + next, 256).
last := io/write-scan(65537, 0, Step, 60, "", 65536)
'''
    binary = conformance.build('old-state-dependent', source)
    symbols = subprocess.check_output(['xcrun', 'llvm-objdump', '--reloc', str(binary) + '.project/mech-control.o'], text=True)
    assert '_mech_control_scan_pipeline' not in symbols
    state = 0
    expected = bytearray()
    for _ in range(65537):
        next_state = (state + 1) % 97
        expected.append((state + next_state) % 256)
        state = next_state
    assert subprocess.check_output([str(binary)]) == expected
    checks.append(['old-state-dependent-fallback'])
    conformance.ENV.pop('MECH_CONTROL_SCAN_PIPELINE', None)
    for label, source, expected_pipeline in (
        ('automatic-substantial-mapping', generic, True),
        ('automatic-trivial-mapping', source.replace('number/mod(i + next, 256)', 'number/mod(next, 256)'), False),
    ):
        binary = conformance.build(label, source)
        descriptor = json.loads(Path(str(binary) + '.project/control.json').read_text())
        assert descriptor['scan_pipeline_policy'] == 'auto'
        assert bool(descriptor['pipelined_scan_functions']) == expected_pipeline, (label, descriptor)
        if expected_pipeline:
            expected = subprocess.check_output([str(conformance.BUILD / 'ordered-1'), '131073', '96', '60', '65536'])
            actual = subprocess.check_output([str(binary), '131073', '96', '60', '65536'])
        else:
            expected = bytes((i + 1) % 97 for i in range(65537))
            actual = subprocess.check_output([str(binary)])
        assert actual == expected, label
        checks.append(['automatic-pipeline-policy', label])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(dict(checks=checks, passed=len(checks), all_passed=True,
                                         compiler_executable_sha256=run.sha(conformance.MECH),
                                         compiler_snapshot=run.snapshot(args.output)), indent=2) + '\n')
    print('passed', len(checks), flush=True)


if __name__ == '__main__':
    main()
