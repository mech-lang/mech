#!/usr/bin/env python3
"""Preserve and measure compiler optimizations against matched controls."""
import argparse
import copy
import json
import os
import random
import shutil
import statistics
from datetime import datetime, timezone
from pathlib import Path

import run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--reuse-builds', action='store_true')
    parser.add_argument('--samples', type=int, default=5)
    parser.add_argument('--only', help='Comma-separated build identifiers')
    parser.add_argument('--stage', choices=('exact', 'cache', 'pipeline', 'final'), default='exact')
    parser.add_argument('--revision', default='v2', help='Unique build/report suffix for an additional experiment')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.samples < 3:
        parser.error('At least three samples are required')
    if args.output is None:
        args.output = run.HERE / 'results' / dict(exact='apple-m3-exact-compare-matched-25000000.json',
                                                cache='apple-m3-periodic-matched-25000000.json',
                                                pipeline='apple-m3-pipeline-matched-25000000.json',
                                                final='apple-m3-streaming-final-25000000.json')[args.stage]
        if args.revision != 'v2':
            args.output = args.output.with_name(args.output.stem + '-' + args.revision + args.output.suffix)
    folder = run.BUILD / ('optimize-20261008' if args.stage == 'exact' else f'optimize-{args.stage}-20261008')
    if args.revision != 'v2':
        folder = Path(str(folder) + '-' + args.revision)
    previous_path = run.HERE / 'results' / dict(exact='apple-m3-final-25000000.json',
                                             cache='apple-m3-exact-compare-matched-25000000.json',
                                             pipeline='apple-m3-periodic-matched-25000000.json',
                                             final='apple-m3-final-25000000.json')[args.stage]
    folder.mkdir(parents=True, exist_ok=True)
    prepared = folder / 'prepared.json'
    env = os.environ | {'RAYON_NUM_THREADS': '8'}
    if args.reuse_builds:
        rows = json.loads(prepared.read_text())
    else:
        previous = json.loads(previous_path.read_text())
        rows = []
        previous_ids = ('mech-fasta-compact', 'mech-fasta-optimized', 'fasta-rust-7-thin', 'fasta-gcc-2-llvm') if args.stage in ('exact', 'final') else (
            'mech-fasta-exact-compact-auto-v2', 'mech-fasta-exact-expanded-auto-v2', 'fasta-rust-7-thin-control', 'fasta-gcc-2-llvm-control')
        if args.stage == 'pipeline':
            previous_ids = ('mech-fasta-cache-compact-v2', 'mech-fasta-cache-expanded-v2', 'fasta-rust-7-thin-control', 'fasta-gcc-2-llvm-control')
        for id in previous_ids:
            row = copy.deepcopy(next(r for r in previous['programs'] if r['id'] == id))
            preserved = folder / (id + '-before')
            if not preserved.exists():
                shutil.copy2(row['command'][0], preserved)
            row['id'] += '-before' if row['language'] == 'mech' else ('' if id.endswith('-control') else '-control')
            row['command'] = [str(preserved)]
            assert run.sha(row['command'][0]) == row['executable_sha256']
            row['comparison_stage'] = 'before' if row['language'] == 'mech' else 'foreign'
            row['previous_report'] = previous_path.name
            rows.append(row)
        compiler = argparse.Namespace(mech=str(run.ROOT / 'benchmarks/clbg/nbody/mech-driver/target/release/mech'), threads=8, timeout=300)
        policies = [('compact', 'fasta.mec', True, 1, True),
                    ('compact-auto', 'fasta.mec', True, None, True),
                    ('compact-u4', 'fasta.mec', True, 4, True),
                    ('expanded', 'fasta-optimized.mec', True, 1, True),
                    ('expanded-auto', 'fasta-optimized.mec', True, None, True),
                    ('expanded-u4', 'fasta-optimized.mec', True, 4, True),
                    ('expanded-u8', 'fasta-optimized.mec', True, 8, True),
                    ('expanded-scalar', 'fasta-optimized.mec', False, 1, True),
                    ('expanded-no-exact', 'fasta-optimized.mec', True, 1, False)]
        if args.stage == 'cache':
            policies = [('compact', 'fasta.mec', True, None, True),
                        ('expanded', 'fasta-optimized.mec', True, None, True),
                        ('expanded-u1', 'fasta-optimized.mec', True, 1, True),
                        ('compact-no-cache', 'fasta.mec', True, None, True),
                        ('expanded-no-cache', 'fasta-optimized.mec', True, None, True)]
        if args.stage == 'pipeline':
            policies = [('compact', 'fasta.mec', True, None, True),
                        ('expanded', 'fasta-optimized.mec', True, None, True),
                        ('expanded-u1', 'fasta-optimized.mec', True, 1, True),
                        ('expanded-no-pipeline', 'fasta-optimized.mec', True, None, True)]
        if args.stage == 'final':
            policies = [('compact', 'fasta.mec', True, None, True),
                        ('expanded', 'fasta-optimized.mec', True, None, True),
                        ('expanded-no-pipeline', 'fasta-optimized.mec', True, None, True)]
        for policy, source, simd, unroll, exact in policies:
            p = dict(id='mech-fasta-' + args.stage + '-' + policy + '-' + args.revision, language='mech', path=source,
                     simd=simd, scan_unroll=unroll, exact_float_compare=exact,
                     upstream_starred=False, comparison_stage='after',
                     algorithm=dict(strict=True, reason='Source f64 division and comparisons preserved exactly by range-proved compiler lowering.'))
            if args.stage in ('cache', 'pipeline', 'final'):
                p['scan_cache'] = not policy.endswith('-no-cache')
                p['scan_pipeline'] = args.stage in ('pipeline', 'final') and not policy.endswith('-no-pipeline')
                if args.stage == 'final' and p['scan_pipeline']:
                    p['scan_pipeline'] = 'auto'
                p['algorithm']['reason'] += ' A compiler-proven literal-table cycle reuses bounded source-generated, preformatted bytes.'
                if p['scan_pipeline']:
                    p['algorithm']['reason'] += ' Source-proven independent byte mapping overlaps ordered state generation with bounded buffers.'
            rows.append(run.prepare(p, compiler, env))
            print('prepared', p['id'], flush=True)
        prepared.write_text(json.dumps(rows, indent=2) + '\n')
    if args.prepare_only:
        return
    if args.only:
        ids = set(args.only.split(','))
        rows = [r for r in rows if r['id'] in ids]
        if {r['id'] for r in rows} != ids:
            parser.error('Unknown build identifier')
    for row in rows:
        assert run.sha(row['command'][0]) == row['executable_sha256']
        assert run.sha(run.HERE / row['path']) == row['source']['sha256']
        for n, key in ((1000, 'correctness_1000'), (25000000, 'correctness_25000000')):
            result = run.execute(row['command'], env | row.get('runtime_environment', {}), n, 300)
            row[key] = result['validation']
        row['samples_seconds'] = []
        print('full output passed', row['id'], flush=True)
    orders = []
    rng = random.Random(20261008)
    for sample in range(args.samples):
        order = rows.copy()
        rng.shuffle(order)
        orders.append([r['id'] for r in order])
        for row in order:
            result = run.execute(row['command'], env | row.get('runtime_environment', {}), 25000000, 300)
            row['samples_seconds'].append(result['seconds'])
            print(sample + 1, row['id'], result['seconds'], flush=True)
    for row in rows:
        row.update(median_seconds=statistics.median(row['samples_seconds']),
                   min_seconds=min(row['samples_seconds']), max_seconds=max(row['samples_seconds']))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    old = json.loads((run.HERE / 'results/apple-m3-final-25000000.json').read_text())
    protocol = dict(old['protocol'], samples=args.samples, sample_order=orders,
                    environment={'RAYON_NUM_THREADS': '8'},
                    normalization='All displayed matched times are raw elapsed seconds from the same randomized batch.')
    result = dict(schema_version=1, created_at=datetime.now(timezone.utc).isoformat(), hardware=old['hardware'],
                  protocol=protocol, programs=rows, excluded=[],
                  compiler_snapshot=run.snapshot(args.output),
                  previous_report=dict(path=previous_path.name, sha256=run.sha(previous_path)))
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    for row in rows:
        print('median', row['id'], f"{row['median_seconds']:.6f}", flush=True)


if __name__ == '__main__':
    main()
