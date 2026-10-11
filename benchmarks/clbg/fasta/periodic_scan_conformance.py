#!/usr/bin/env python3
"""Verify proven cyclic scans, preformatted replay, tails and fallback guards."""
import argparse
import json
import math
import subprocess
from pathlib import Path

import conformance
import run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=run.HERE / 'results/conformance-periodic-20261008.json')
    args = parser.parse_args()
    conformance.BUILD = run.BUILD / 'conformance-periodic'
    conformance.BUILD.mkdir(parents=True, exist_ok=True)
    checks = []
    for period in (1, 5, 60, 287):
        table = ''.join(chr(65 + i % 26) for i in range(period))
        source = f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  table := bytes/from-text("{table}")
  byte := sequence/get(table, i + 1)
  next := number/mod(i + 1, sequence/length(table)).
last := io/write-scan(io/argument-u64(1), io/argument-u64(2), Step, io/argument-u64(3), "|:|", io/argument-u64(4))
io/write(text/format("state={{}}\\n", last))
'''
        for enabled in (False, True):
            conformance.ENV['MECH_CONTROL_SCAN_CACHE'] = str(int(enabled))
            binary = conformance.build(f'period-{period}-{int(enabled)}', source)
            symbols = subprocess.check_output(['xcrun', 'llvm-objdump', '--reloc', str(binary) + '.project/mech-control.o'], text=True)
            assert ('_mech_control_scan_periodic' in symbols) == enabled, (period, enabled, symbols)
            for width, buffer in ((0, 7), (0, 65536), (1, 65536), (7, 7), (7, 65536), (60, 65), (60, 65536), (2**64 - 1, 7)):
                alignment = math.lcm(period, width) if width else period
                counts = {0, 1, 2, period - 1, period, period + 1, 59, 60, 61, 65535, 65536, 65537}
                if alignment <= 65536:
                    counts.update((alignment - 1, alignment, alignment + 1, alignment * 2 + 1))
                for start in sorted({0, period // 2, period - 1}):
                    for count in sorted(counts):
                        raw = bytes(ord(table[(start + i) % period]) for i in range(count))
                        expected = raw if not width else b''.join(raw[i:i+width] + b'|:|' for i in range(0, count, width))
                        expected += f'state={(start + count) % period}\n'.encode()
                        actual = subprocess.check_output([str(binary), str(count), str(start), str(width), str(buffer)])
                        assert actual == expected, (period, enabled, width, buffer, start, count)
                        checks.append(['cycle', period, enabled, width, buffer, start, count])
            for start in (period, 2**64 - 1):
                result = subprocess.run([str(binary), '1', str(start), '60', '65536'], capture_output=True)
                assert result.returncode != 0 and result.stdout == b'', (period, enabled, start)
                # Empty scans must not evaluate the callback or reject its initial state.
                result = subprocess.check_output([str(binary), '0', str(start), '60', '65536'])
                assert result == f'state={start}\n'.encode()
                checks.append(['cycle-initial-guard', period, enabled, start])
            for width, buffer in ((60, 0), (60, 1048577)):
                result = subprocess.run([str(binary), '100', '0', str(width), str(buffer)], capture_output=True)
                assert result.returncode != 0 and result.stdout == b''
                checks.append(['cycle-buffer-guard', period, enabled, width, buffer])
            print('passed cycle', period, enabled, flush=True)
    conformance.ENV.pop('MECH_CONTROL_SCAN_CACHE', None)
    # Similar-looking scans without an exact same-table cycle cannot be cached.
    for label, next_expr, byte_expr in (
        ('wrong-period', 'number/mod(i + 1, 4)', 'sequence/get(table, i + 1)'),
        ('different-index', 'number/mod(i + 1, 5)', 'sequence/get(table, 1)'),
        ('step-two', 'number/mod(i + 2, 5)', 'sequence/get(table, i + 1)'),
    ):
        source = f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  table := bytes/from-text("ABCDE")
  next := {next_expr}
  byte := {byte_expr}.
last := io/write-scan(65537, 0, Step, 60, "", 65536)
'''
        binary = conformance.build(label, source)
        symbols = subprocess.check_output(['xcrun', 'llvm-objdump', '--reloc', str(binary) + '.project/mech-control.o'], text=True)
        assert '_mech_control_scan_periodic' not in symbols, label
        state = 0
        raw = bytearray()
        for _ in range(65537):
            raw.append(65 if label == 'different-index' else 65 + state)
            state = (state + (2 if label == 'step-two' else 1)) % (4 if label == 'wrong-period' else 5)
        assert subprocess.check_output([str(binary)]) == raw, label
        checks.append(['unproved-cycle-fallback', label])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(dict(checks=checks, passed=len(checks), all_passed=True,
                                         compiler_executable_sha256=run.sha(conformance.MECH),
                                         compiler_snapshot=run.snapshot(args.output)), indent=2) + '\n')
    print('passed', len(checks), flush=True)


if __name__ == '__main__':
    main()
