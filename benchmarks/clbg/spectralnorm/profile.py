#!/usr/bin/env python3
"""Compare worker scaling and macOS time resource counters for the native paths.

Diagnostic only: collector overhead is included in wall time. These samples
are kept separate from the official fresh-process corner-chart measurements.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import re
import statistics
import subprocess
import time

import run as spectral

HERE = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, default=HERE / 'results/integer-coefficient-ablation.json')
    parser.add_argument('--only')
    parser.add_argument('--workers', default='1,4,8')
    parser.add_argument('--samples', type=int, default=7)
    parser.add_argument('--output', type=Path, default=HERE / 'results/worker-scaling-counters.json')
    args = parser.parse_args()
    workers_list = [int(w) for w in args.workers.split(',')]
    if args.samples < 3 or not workers_list or any(w < 1 for w in workers_list):
        parser.error('at least three samples and positive worker counts required')
    ablation = json.loads(args.report.read_text())
    selected = set(args.only.split(',')) if args.only else None
    rows = []
    for p in ablation['programs']:
        if selected is not None and p['id'] not in selected:
            continue
        for workers in workers_list:
            r = {k: v for k, v in p.items() if k not in ('samples_seconds', 'median_seconds')}
            r['id'] = p['id'] + f'-workers{workers}'
            r['workers'] = workers
            r['samples'] = []
            rows.append(r)
    if not rows:
        parser.error('no matching programs')
    rng = random.Random(20261007)
    orders = []
    for sample in range(args.samples):
        order = rows.copy()
        rng.shuffle(order)
        orders.append([r['id'] for r in order])
        for r in order:
            env = os.environ | {'RAYON_NUM_THREADS': str(r['workers']),
                                'OPENBLAS_NUM_THREADS': '1'}
            if r['language'] == 'mech':
                env |= {'MECH_SEQUENCE_PARTITIONS': str(r['partitions']),
                        'MECH_ARRAY_GRAIN': str(r.get('grain', 16))}
            argv = ['/usr/bin/time', '-lp'] + r['command'] + ['5500']
            start = time.perf_counter_ns()
            result = subprocess.run(argv, env=env, cwd=spectral.ROOT, check=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    timeout=300)
            elapsed = (time.perf_counter_ns() - start) / 1e9
            if result.stdout != b'1.274224153\n':
                raise ValueError('output differs: ' + r['id'])
            counters = {}
            for line in result.stderr.decode().splitlines():
                match = re.fullmatch(r'(real|user|sys)\s+([\d.]+)', line.strip())
                if match:
                    counters[match[1]] = float(match[2])
                    continue
                match = re.fullmatch(r'\s*(\d+)\s+(.+)', line)
                if match:
                    counters[match[2]] = int(match[1])
            r['samples'].append({'wall_seconds': elapsed, 'counters': counters})
            print(f"{sample + 1}/{args.samples} {r['id']}: {elapsed:.6f}s, CPU {counters['user'] + counters['sys']:.2f}s", flush=True)
    for r in rows:
        r['median_seconds'] = statistics.median(s['wall_seconds'] for s in r['samples'])
        r['median_counters'] = {k: statistics.median(s['counters'][k] for s in r['samples'])
                                for k in r['samples'][0]['counters']}
        r['executable_sha256'] = spectral.sha(r['command'][0])
    output = args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({
        'created_at': datetime.now(timezone.utc).isoformat(),
        'hardware': {'platform': platform.platform(), 'machine': platform.machine(),
                     'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip()},
        'protocol': {'n': 5500, 'samples': args.samples, 'workers': workers_list,
                     'source_report': str(args.report.resolve()),
                     'collector': '/usr/bin/time -lp; hardware counters reported by macOS, not inferred from source or instruction listings',
                     'timed_region': 'Fresh process through exit including time collector overhead. No compilation or tests run during samples.',
                     'sample_order': orders, 'diagnostic_only': True},
        'compiler_snapshot': spectral.snapshot(output),
        'programs': rows,
    }, indent=2) + '\n')


if __name__ == '__main__':
    main()
