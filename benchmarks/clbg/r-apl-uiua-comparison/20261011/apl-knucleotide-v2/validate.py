"""Bounded correctness probes, serialized with all benchmark processes."""
from pathlib import Path
import argparse
import collections
import fcntl
import hashlib
import importlib.util
import json
import os
import random
import subprocess

HERE = Path(__file__).resolve().parent
G = HERE.parent
C = G.parents[1]
OUT = HERE / 'results'
RUNTIME = G / '.build/toolchains/gnu-apl-local/bin/apl'
LAUNCHER = G / 'run-apl.sh'
WIDTHS = [1, 2, 3, 4, 6, 12, 18]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def vector(values):
    result = ' '.join(str(value).replace('-', '¯') for value in values) or '⍬'
    return ',' + result if len(values) == 1 else result


def run(source, variant, data=b''):
    command = [str(LAUNCHER), '--cc', '8' if variant == 'optimized' else '1', '-f', str(source)]
    process = subprocess.run(command, input=data, cwd=G, capture_output=True,
                             env=os.environ | {'LC_ALL': 'C'}, timeout=240)
    detail = dict(command=command, source=str(source), source_sha256=sha(source),
                  returncode=process.returncode, stdout=process.stdout.decode(),
                  stderr=process.stderr.decode(), stdout_bytes=len(process.stdout),
                  stdout_sha256=hashlib.sha256(process.stdout).hexdigest())
    return process, detail


def successful(process, expected):
    return (process.returncode == 0 and process.stdout == expected
            and process.stderr.decode().strip() in ('', 'WARNING: File pipe-to_client still open - closing it'))


DUMP = r'''∇ Width Dump Map;KC;CC;Positions;Order;I
 KC←⊃Map[0]
 CC←⊃Map[1]
 Positions←(CC≠0)/⍳⍴CC
 Order←Positions[⍋KC[Positions]]
 I←0
L:→(I≥⍴Order)/0
 ⊣(('%d %.0f %lld',⎕UCS 10) Width KC[Order[I]] CC[Order[I]])⎕FIO[22]OUT
 I←I+1
 →L
∇
∇ DumpAll;I
 I←0
L:→(I≥⍴Sizes)/0
 Sizes[I] Dump ⊃Maps[I]
 I←I+1
 →L
∇
'''


def map_expected(width, counts):
    return ''.join(f'{width} {key} {count}\n' for key, count in sorted(counts.items())).encode()


def fixture_base(path):
    original = path.read_text()
    assert original.endswith('Run\n⊣⎕FIO[4]OUT\n)OFF\n')
    return original.rsplit('\nRun\n', 1)[0] + '\n' + DUMP


def encoded_counts(codes):
    output = b''
    for width in WIDTHS:
        counts = collections.Counter()
        for start in range(max(0, len(codes) - width + 1)):
            key = 0
            for value in codes[start:start + width]:
                key = 4 * key + value
            counts[key] += 1
        output += map_expected(width, counts)
    return output


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    preserved = {str(path): sha(path) for path in [RUNTIME, LAUNCHER,
        G / 'sources/apl/knucleotide.compact.apl',
        G / 'apl-regex-v3/sources/regexredux.compact.apl',
        G / 'apl-regex-v3/sources/regexredux.optimized.apl',
        G / 'apl-regex-v3/manifest.json',
        G / '.build/apl-support/libregex-count-v3-apl.dylib']}
    spec = importlib.util.spec_from_file_location('apl_reference_knucleotide_v2', G / 'validate_apl.py')
    reference = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(reference)
    rng = random.Random(20261011)
    dna = ''.join(rng.choice('ACGT') for _ in range(2003)) + 'GGTATTTTAATTTATAGT'
    cases = [
        ('canonical-small', (C / 'knucleotide/inputs/knucleotide-input-1000.txt').read_bytes()),
        ('short-window', b'>THREE short\nA\n'),
        ('empty-record', b'>THREE empty\n>FOUR\nAGG\n'),
        ('missing-record', b'>ONE\nACGT\n'),
        ('comments-crlf', b'>THREE crlf\r\naCgT\r\n;ignored comment\r\nGGTATT\r\n'),
        ('ties-stop', b'>THREE ties\nACGTACGTACGT\n>FOUR\nGGGG\n'),
        ('all-widths-growing-maps', ('>THREE maps\n' + '\n'.join(dna[i:i + 37].lower()
                                                         for i in range(0, len(dna), 37)) + '\n').encode()),
        ('no-final-newline', b'>THREE last\nggtattttaatttatagtacgt'),
        ('empty-input', b''),
        ('unterminated-target-header', b'>THREE'),
        ('long-target-header', b'>THREE ' + b'H' * (2 * 65536 + 19) + b'\r\nACGTGGTATT\n'),
        ('long-comment-continuation', b'>THREE\nACGT\n;' + b'X' * (2 * 65536 + 19) + b'\nGGTATT\n'),
    ]
    for prefix in range(1, 7):
        before = b'>ONE\n' + b'A' * (65536 - 6 - prefix) + b'\n'
        assert len(before) == 65536 - prefix
        cases.append((f'target-header-split-{prefix}', before + b'>THREE header\nacgtGGTATTTTAATTTATAGT\n'))
    boundary_dna = ('ACGT' * 16385 + 'GGTATTTTAATTTATAGT').encode()
    cases.append(('selected-sequence-chunk-boundary', b'>THREE\n' + boundary_dna + b'\n'))
    rows = []
    for variant in ['compact', 'optimized']:
        source = HERE / 'sources' / f'knucleotide.{variant}.apl'
        for name, data in cases:
            process, detail = run(source, variant, data)
            expected = reference.knuc(data)
            passed = successful(process, expected)
            row = dict(case=name, variant=variant, scope='benchmark-output', passed=passed,
                reference_match=passed, input_bytes=len(data), input_sha256=hashlib.sha256(data).hexdigest(),
                expected_stdout_sha256=hashlib.sha256(expected).hexdigest(), **detail)
            rows.append(row)
            print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
            if not passed:
                (OUT / 'validation.json').write_text(json.dumps(dict(passed=False, rows=rows), indent=2) + '\n')
                raise AssertionError(detail)
        base = fixture_base(source)
        for name, updates in [
            ('incoming-and-existing-collisions', [
                ([0, 17, 34, 51, 0, -17, -34, 17], [1, 2, 3, 4, 5, 6, 7, 8]),
                ([51, 68, -17, 85, 102, 0, 17, 34], [2, 3, 4, 5, 6, 7, 8, 9])]),
            ('growth-rehash-all-keys', [
                ([17 * i for i in range(8)], [i + 1 for i in range(8)]),
                ([143 * i for i in range(-32, 33)], [1 + i % 7 for i in range(65)]),
                ([2 ** 36 - 1, 0, -143, 2 ** 35 + 7], [11, 12, 13, 14])]),
            ('large-duplicate-run', [([17] * 257 + [-17] * 193 + [0] * 101,
                                      [1 + i % 3 for i in range(551)])]),
        ]:
            lines = [base, 'MapNew 0']
            counts = collections.Counter()
            for keys, amounts in updates:
                assert len(keys) == len(amounts)
                for key, amount in zip(keys, amounts):
                    counts[key] += amount
                if variant == 'optimized':
                    lines.append(f'({vector(amounts)}) AddBatch ({vector(keys)})')
                else:
                    lines.extend(f'{amount} MapAdd {str(key).replace("-", "¯")}' for key, amount in zip(keys, amounts))
            lines.extend(['0 Dump ((⊂Keys),(⊂Counts),⊂Used)', '⊣⎕FIO[4]OUT', ')OFF'])
            fixture = OUT / f'map-{name}.{variant}.apl'
            fixture.write_text('\n'.join(lines) + '\n')
            expected = map_expected(0, counts)
            process, detail = run(fixture, variant)
            passed = successful(process, expected)
            rows.append(dict(case=name, variant=variant, scope='generic-map', passed=passed,
                reference_match=passed, assertions=len(counts), expected_stdout_sha256=hashlib.sha256(expected).hexdigest(), **detail))
            print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
        codes = [rng.randrange(4) for _ in range(257)] + [2, 2, 3, 0, 3, 3, 3, 3, 0, 0, 3, 3, 3, 0, 3, 0, 2, 3]
        for name, chunks in [('all-seven-maps-one-chunk', [len(codes)]),
                             ('all-seven-maps-tiny-boundaries', [1, 2, 5, 7, 13, 17, 18, 31, 61, 120])]:
            lines = [base, 'Init']
            offset = 0
            for size in chunks:
                part = codes[offset:offset + size]
                offset += len(part)
                lines.append(f'AddWindows ({vector(part)})')
            if offset < len(codes):
                lines.append(f'AddWindows ({vector(codes[offset:])})')
            lines.extend(['DumpAll', '⊣⎕FIO[4]OUT', ')OFF'])
            fixture = OUT / f'windows-{name}.{variant}.apl'
            fixture.write_text('\n'.join(lines) + '\n')
            expected = encoded_counts(codes)
            process, detail = run(fixture, variant)
            passed = successful(process, expected)
            rows.append(dict(case=name, variant=variant, scope='all-seven-complete-maps', passed=passed,
                reference_match=passed, expected_stdout_sha256=hashlib.sha256(expected).hexdigest(),
                encoded_input_length=len(codes), every_key_count_compared=True, **detail))
            print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
    assert all(sha(path) == digest for path, digest in preserved.items())
    report = dict(schema_version=1, passed=all(row['passed'] for row in rows),
        purpose='Small/adversarial output, explicit collision/growth and complete seven-map checks; no full timings.',
        lock='/tmp/mech-clbg-timing.lock', preserved_pins=preserved, runtime_sha256=sha(RUNTIME),
        rows=rows, checks=len(rows), benchmark_cases_per_variant=len(cases),
        generic_map_cases_per_variant=3, all_map_cases_per_variant=2,
        memory_analysis=dict(input_buffer_bytes=65536, cells_per_input_byte=1, bytes_per_cell=16,
            whole_file_not_retained=True, per_width_overlap_maximum=17,
            table_storage='Data-dependent generic growing maps; actual distinct counts are not inferred or cached.',
            conditional_capacity_risk='A table growing past 105,696,460 unique keys requires capacity 301,989,887; key/count APL-cell arrays alone would require 9 GiB. This is a worst-case condition, not measured canonical distinct-key data.'))
    (OUT / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed'], 'New bounded candidates did not pass all probes.'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--wait-for-lock', action='store_true')
    args = parser.parse_args()
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | (0 if args.wait_for_lock else fcntl.LOCK_NB))
        print('Acquired timing mutex; validating only bounded new APL candidates.', flush=True)
        main()
