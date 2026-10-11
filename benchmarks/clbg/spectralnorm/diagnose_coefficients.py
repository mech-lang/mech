#!/usr/bin/env python3
"""Diagnostic ablation of Rust #5's integer versus floating-point denominator.

Keeps the pinned upstream source unchanged. The altered copy is a diagnostic,
excluded from the published fastest/smallest source endpoints.
"""
import hashlib
import json
import os
from pathlib import Path
import random
import re
import statistics
from types import SimpleNamespace

import run as spectral

HERE = Path(__file__).resolve().parent


def main():
    source = HERE / 'upstream/spectralnorm-rust-5.rs'
    text = source.read_text()
    replacement = '''fn a(i: [usize; 2], j: [usize; 2]) -> F64x2 {
    let coefficient = |i: usize, j: usize| {
        let (i, j) = (i as f64, j as f64);
        (i + j) * (i + j + 1.0) / 2.0 + i + 1.0
    };
    F64x2::new(coefficient(i[0], j[0]), coefficient(i[1], j[1]))
}
'''
    modified, count = re.subn(r'fn a\(i: \[usize; 2\], j: \[usize; 2\]\) -> F64x2 \{.*?\n\}',
                              replacement.rstrip(), text, flags=re.S)
    if count != 1:
        raise ValueError('expected exactly one pinned coefficient definition')
    experiment = HERE / 'experiments/rust-5-f64-denominator.rs'
    experiment.parent.mkdir(exist_ok=True)
    experiment.write_text(modified)
    env = os.environ | {'RAYON_NUM_THREADS': '8', 'OPENBLAS_NUM_THREADS': '1'}
    args = SimpleNamespace(threads=8, timeout=300)
    row = spectral.prepare({'id': 'diagnostic-rust5-f64', 'language': 'rust',
                            'variant': 5, 'path': str(experiment.relative_to(HERE)),
                            'upstream_starred': False}, args, env)
    row.pop('_env')
    prepared = json.loads((HERE / '.build/prepared.json').read_text())
    ids = ('spectralnorm-rust-5', 'spectralnorm-rust-4-panicabort',
           'mech-optimized-simd1-u4-r4-p8')
    rows = [row] + [r.copy() for r in prepared['programs'] if r['id'] in ids]
    for r in rows:
        environment = env.copy()
        if r['language'] == 'mech':
            environment |= {'MECH_SEQUENCE_PARTITIONS': str(r['partitions']),
                            'MECH_ARRAY_GRAIN': str(r.get('grain', 16))}
        r['_env'] = environment
        _, out, _ = spectral.execute(r['command'], environment, 5500, 300)
        if out != b'1.274224153\n':
            raise ValueError('full output mismatch: ' + r['id'])
        r['correctness_full'] = {'output': out.decode(),
                                 'sha256': hashlib.sha256(out).hexdigest()}
        r['samples_seconds'] = []
    rng = random.Random(20261007)
    orders = []
    # All preparation and validation have completed before the timed samples.
    for sample in range(9):
        order = rows.copy()
        rng.shuffle(order)
        orders.append([r['id'] for r in order])
        for r in order:
            elapsed, out, _ = spectral.execute(r['command'], r['_env'], 5500, 300)
            if out != b'1.274224153\n':
                raise ValueError('timed output mismatch: ' + r['id'])
            r['samples_seconds'].append(elapsed)
            print(f"{sample + 1}/9 {r['id']}: {elapsed:.6f}s", flush=True)
    for r in rows:
        r.pop('_env')
        r['median_seconds'] = statistics.median(r['samples_seconds'])
    output = HERE / 'results/integer-coefficient-ablation.json'
    output.write_text(json.dumps({
        'protocol': {'n': 5500, 'samples': 9, 'sample_order': orders,
                     'runtime_environment': {'RAYON_NUM_THREADS': '8'},
                     'timed_region': 'Sequential fresh process through exit, complete prescribed work and output; builds and validation excluded.',
                     'diagnostic_only': 'The Rust floating-point copy is deliberately altered, excluded from published source endpoints.'},
        'original_source_sha256': spectral.sha(source),
        'coefficient_replacement': replacement,
        'compiler_snapshot': spectral.snapshot(output),
        'programs': rows,
    }, indent=2) + '\n')


if __name__ == '__main__':
    main()
