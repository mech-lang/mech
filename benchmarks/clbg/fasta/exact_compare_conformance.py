#!/usr/bin/env python3
"""Check rounded-division predicates against independent IEEE evaluation."""
import argparse
import json
import math
import random
import subprocess
from pathlib import Path

import conformance
import run


def literal(value):
    if math.isnan(value):
        return '(0.0 / 0.0)'
    if math.isinf(value):
        return '(1.0 / 0.0)' if value > 0 else '(0.0 - 1.0 / 0.0)'
    if value < 0:
        return '(0.0 - ' + literal(-value) + ')'
    # Canonical literals use decimal notation, including subnormal values.
    if value == 0:
        return '0.0'
    text = format(value, '.340f').rstrip('0')
    return text + '0' if text.endswith('.') else text


def divide(x, d):
    if d == 0:
        return math.nan if x == 0 else math.inf
    return float(x) / d


def bits(r, p):
    return int(r < p) + 2 * int(r <= p) + 4 * int(r > p) + 8 * int(r >= p) + 16 * int(r == p) + 32 * int(r != p)


def boundary(maximum, d, p, strict):
    lo, hi = 0, maximum + 1
    while lo < hi:
        mid = lo + (hi - lo) // 2
        r = divide(mid, d)
        if r > p if strict else r >= p:
            hi = mid
        else:
            lo = mid + 1
    return lo


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=run.HERE / 'results/conformance-exact-20261008.json')
    args = parser.parse_args()
    conformance.BUILD = run.BUILD / 'conformance-exact'
    conformance.BUILD.mkdir(parents=True, exist_ok=True)
    checks = []
    cases = [(139968.0, p) for p in (0.0, 0.27, 0.39, 0.51, 0.78, 0.98, 1.0)]
    cases += [(3.0, 1.0 / 3.0), (7.0, 0.5), (1.0, float(2**53)),
              (math.ulp(0.0), 1.0), (1e300, 1e-300), (1e-300, 1e300),
              (7.0, -1.0), (7.0, math.inf), (7.0, math.nan),
              (-7.0, 0.5), (0.0, 0.5)]
    operators = ['<', '<=', '>', '>=', '==', '!=']
    rng = random.Random(20261008)
    for maximum in (139967, 2**53, 2**64 - 1):
        source = 'Step(x<u64>, i<u64>) = (same<u64>, next<u64>, byte<u64>) :=\n  same := x\n  next := i + 1\n'
        for j, (d, p) in enumerate(cases):
            source += f'  r{j} := number/to-f64(x) / {literal(d)}\n'
            terms = [f'control/select(r{j} {op} {literal(p)}, {1 << k}, 0)' for k, op in enumerate(operators)]
            source += f'  b{j} := ' + ' + '.join(terms) + '\n'
        expression = 'b' + str(len(cases) - 1)
        for j in reversed(range(len(cases) - 1)):
            expression = f'control/select(i == {j}, b{j}, {expression})'
        source += f'  byte := {expression}.\ns := io/write-scan({len(cases)}, (number/bounded(io/argument-u64(1), 0, {maximum}), 0), Step, 7, "|", 7)\n'
        values = {0, 1, maximum, maximum - 1}
        for d, p in cases:
            if d > 0 and math.isfinite(d) and not math.isnan(p):
                for strict in (False, True):
                    cut = boundary(maximum, d, p, strict)
                    values.update(v for v in (cut - 1, cut, cut + 1) if 0 <= v <= maximum)
        values.update(rng.randrange(maximum + 1) for _ in range(128))
        for enabled, scalar in ((False, False), (True, False), (True, True)):
            conformance.ENV['MECH_CONTROL_EXACT_FLOAT_COMPARE'] = str(int(enabled))
            binary = conformance.build(f'predicates-{maximum}-{int(enabled)}-{int(scalar)}', source, scalar)
            for x in sorted(values):
                raw = bytes(bits(divide(x, d), p) for d, p in cases)
                expected = b'|'.join(raw[i:i+7] for i in range(0, len(raw), 7)) + b'|'
                actual = subprocess.check_output([str(binary), str(x)])
                assert actual == expected, (maximum, enabled, scalar, x, actual, expected)
                checks.append(['rounded-predicates', maximum, enabled, scalar, x])
            print('passed rounded predicates', maximum, enabled, scalar, len(values), flush=True)

    # Literal indexed thresholds must retain exact division boundaries and
    # bounds errors when a source loop carries immutable arrays and scalars.
    search = (run.HERE / 'fasta.mec').read_text().split('Repeat(')[0]
    for table in ((0.1, 0.27, 0.39, 0.51, 0.78, 0.98, 1.0), (0.0, 0.5, 1.0)):
        source = search + 'Step(x<u64>) = (next<u64>, byte<u64>) :=\n  next := number/mod(x + 1, 139968)\n'
        source += f'  r := number/to-f64(x) / 139968.0\n  p := sequence/f64-values(({", ".join(literal(p) for p in table)}))\n'
        source += f'  byte := Choose(r, p, bytes/from-text("{"abcdefg"[:len(table)]}")).\ns := io/write-scan(io/argument-u64(1), 0, Step, 0, "", 65536)\n'
        expected = bytearray()
        for x in range(139968):
            lo, hi = 0, len(table)
            while lo + 1 < hi:
                mid = (lo + hi) // 2
                if x / 139968.0 < table[mid - 1]:
                    hi = mid
                else:
                    lo = mid
            expected.append(ord('a') + hi - 1)
        for enabled in (False, True):
            conformance.ENV['MECH_CONTROL_EXACT_FLOAT_COMPARE'] = str(int(enabled))
            binary = conformance.build(f'indexed-{len(table)}-{int(enabled)}', source)
            actual = subprocess.check_output([str(binary), '139968'])
            assert actual == expected, ('indexed', len(table), enabled)
            checks.append(['indexed-thresholds-full-domain', len(table), enabled, len(expected)])
            print('passed indexed thresholds', len(table), enabled, flush=True)

    thresholds = [0.02 * j for j in range(1, 15)]
    sizes = (2, 3, 4, 5, 14)
    source = 'Step(x<u64>, i<u64>) = (same<u64>, next<u64>, byte<u64>) :=\n  same := x\n  next := i + 1\n  r := number/to-f64(x) / 139968.0\n  f := control/select(x == 0, 0.0 / 0.0, r)\n'
    for j, size in enumerate(sizes):
        terms = [f'control/select(r >= {literal(p)}, 1, 0)' for p in thresholds[:size]]
        # Include floating predicates, mixed integer predicates, and a wrapping
        # offset alongside the four-lane unsigned count reduction.
        terms += ['control/select(f != 0.5, 1, 0)', 'control/select(f < 0.5, 1, 0)',
                  'control/select(r < 0.5, 1, 0)', 'control/select(r > 0.5, 1, 0)']
        source += f'  b{j} := number/mod(18446744073709551615 + ' + ' + '.join(terms) + ', 128)\n'
    expression = 'b4'
    for j in reversed(range(4)):
        expression = f'control/select(i == {j}, b{j}, {expression})'
    source += f'  byte := {expression}.\ns := io/write-scan(5, (number/bounded(io/argument-u64(1), 0, 139967), 0), Step, 0, "", 7)\n'
    values = {0, 1, 69983, 69984, 139967}
    values.update(rng.randrange(139968) for _ in range(128))
    for enabled, scalar in ((False, False), (True, False), (True, True)):
        conformance.ENV['MECH_CONTROL_EXACT_FLOAT_COMPARE'] = str(int(enabled))
        binary = conformance.build(f'packed-{int(enabled)}-{int(scalar)}', source, scalar)
        for x in sorted(values):
            r = x / 139968.0
            f = math.nan if x == 0 else r
            extra = int(f != .5) + int(f < .5) + int(r < .5) + int(r > .5)
            expected = bytes(((2**64 - 1 + sum(r >= p for p in thresholds[:n]) + extra) % 2**64) % 128 for n in sizes)
            actual = subprocess.check_output([str(binary), str(x)])
            assert actual == expected, ('packed', enabled, scalar, x, actual, expected)
            checks.append(['packed-mixed-counts', enabled, scalar, x])
        print('passed packed mixed counts', enabled, scalar, flush=True)
    conformance.ENV.pop('MECH_CONTROL_EXACT_FLOAT_COMPARE', None)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(dict(checks=checks, passed=len(checks), all_passed=True,
                                         compiler_executable_sha256=run.sha(conformance.MECH),
                                         compiler_snapshot=run.snapshot(args.output)), indent=2) + '\n')
    print('passed', len(checks), flush=True)


if __name__ == '__main__':
    main()
