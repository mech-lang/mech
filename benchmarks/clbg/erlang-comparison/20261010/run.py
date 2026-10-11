"""Resumable sequential Erlang measurements with full-work/output/environment guards."""
import argparse
import importlib.util
import json
import random
import statistics
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from prepare import sha

D = Path(__file__).resolve().parent
C = D.parents[1]
spec = importlib.util.spec_from_file_location('array_runner', C / 'array-comparison/all-20261009/run.py')
arrays = importlib.util.module_from_spec(spec)
spec.loader.exec_module(arrays)
compact = arrays.compact
compact.COMPILERS.add('erlc')
WORK, REQUIRED = arrays.WORK, arrays.REQUIRED
BENCHES = ['pidigits', 'spectralnorm', 'nbody', 'mandelbrot', 'regexredux', 'binarytrees', 'fannkuchredux', 'fasta', 'knucleotide', 'revcomp']
REASONS = {
    'pidigits': 'Sequential unbounded LFT spigot using Erlang arbitrary-precision integers, computing extract(3) and extract(4) on every attempt. Erlang controls all transformations and digit output; no native spigot executor.',
    'spectralnorm': 'Separate coefficient, multiply, transpose and composition functions. All forty N-squared products evaluate each coefficient during multiplication; no cached matrix. Published tuple access and parallel row partitioning are retained.',
    'nbody': 'Canonical five bodies, momentum correction, all ten symmetric pair interactions and five position updates at every one of fifty million steps, and both energies. Published floating-point arithmetic and recursive body ordering are retained.',
    'mandelbrot': 'Every one of the 256 million pixels uses the ordinary recurrence and at most fifty escape-tested iterations. The omitted last pixel is corrected; complete byte-oriented PBM output and ordinary row scheduling are retained. No interior, symmetry or periodicity shortcuts.',
    'regexredux': 'All nine complete case-insensitive non-overlapping regex searches and five ordered case-sensitive substitutions operate on the entire cleaned input. Standard Erlang re supplies general regex operations; all sequencing stays in Erlang.',
    'binarytrees': 'Each interior and depth-zero node is a separately allocated two-slot tuple. Construct each complete tree before traversal; release stretch storage, keep the long-lived root live through all temporary work, and check afterward. Optimized BEAM assembly confirms dynamic leaf/interior allocation and recursive construction/traversal. Default GC and no custom pool.',
    'fannkuchredux': 'All 479001600 permutations in canonical rotation order, full prefix flips, alternating checksum and maximum. Published factoradic tasks cover disjoint complete intervals and run on eight online schedulers.',
    'fasta': 'Canonical ALU repetition, every one of 200 million fresh LCG updates, direct binary64 division and comparisons with cumulative unscaled floating probabilities. Complete output, original buffering and RNG dependency retained; no scaled thresholds, subtraction search or period cache.',
    'knucleotide': 'Extract THREE, uppercase and build all seven complete overlapping ETS hash maps for widths 1,2,3,4,6,12,18. Every valid position does an explicit counter lookup/update, including the final window. Grow ordinary default ETS hash tables from their initial small size. Frequency ties sort by key; no selective counting, indexed histogram or period shortcut.',
    'revcomp': 'Read all three complete FASTA records, preserve headers, uppercase/reverse/complement every sequence, and emit the entire sixty-column output. Published byte/binary processing and ordered worker output retained.'
}


def exclusion(row):
    if row['build_status'] != 'passed':
        return 'Build failed; compiler diagnostics retained.'
    if not row.get('local_adaptation') and row['benchmark'] in ['binarytrees', 'fasta', 'knucleotide', 'mandelbrot', 'regexredux']:
        return {'binarytrees': 'Depth-zero tuples can be shared compiler literals; use the separate dynamic-leaf correction.',
            'fasta': 'Subtraction search or scaled-integer probabilities differ from the common cumulative binary64 workload.',
            'knucleotide': 'Equal-frequency keys have unspecified order in the original ETS enumeration; corrected ordering is pinned separately.',
            'mandelbrot': 'Original omits the final pixel of each row; two versions also UTF-8 encode PBM bytes.',
            'regexredux': 'Original skips ALU and sequence boundaries and performs case-insensitive final substitutions.'}[row['benchmark']]
    return None


def execute(row, full, timeout):
    if row['benchmark'] == 'nbody':
        return compact.execute(row, full, timeout, watch_compilers=True, expected_power=REQUIRED)
    return arrays.execute(row, full, timeout)


def save(path, value):
    tmp = path.with_suffix('.json.tmp')
    tmp.write_text(json.dumps(value, indent=2) + '\n')
    tmp.replace(path)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--phase', choices=['small', 'measure'], default='measure')
    ap.add_argument('--benchmarks', default=','.join(BENCHES))
    ap.add_argument('--samples', type=int, default=3)
    ap.add_argument('--timeout', type=int, default=14400)
    args = ap.parse_args()
    assert args.samples >= 3
    selected = args.benchmarks.split(',')
    assert set(selected) <= set(BENCHES)
    built = json.loads((D / 'results/builds.json').read_text())

    def pins():
        for runtime in built['runtimes'].values():
            assert sha(runtime['path']) == runtime['sha256']
        for row in built['programs']:
            assert sha(row['source']) == row['source_sha256']
            if row['build_status'] == 'passed':
                assert sha(row['binary']) == row['binary_sha256']

    pins()
    path = D / 'results/small-checks.json'
    small = json.loads(path.read_text()) if path.exists() else dict(status='running', programs={}, power=REQUIRED)
    if args.phase == 'small':
        for row in built['programs']:
            if row['benchmark'] not in selected or row['build_status'] != 'passed':
                continue
            previous = small['programs'].get(row['key'])
            if previous and previous['source_sha256'] == row['source_sha256']:
                continue
            compact.wait_for_environment(REQUIRED)
            print('SMALL', row['id'], flush=True)
            try:
                result = execute(row, False, args.timeout)
                assert not result['competing_compilers'] and not result['power_changes']
                small['programs'][row['key']] = dict(status='passed', source_sha256=row['source_sha256'], **result)
                print('PASS', row['id'], f"{result['seconds']:.6f}s", flush=True)
            except Exception as error:
                small['programs'][row['key']] = dict(status='failed', source_sha256=row['source_sha256'], error=str(error))
                print('FAIL', row['id'], str(error)[:220], flush=True)
            save(path, small)
        small['status'] = 'passed' if all(small['programs'].get(row['key'], {}).get('status') == 'passed'
            for row in built['programs'] if exclusion(row) is None) else 'partial'
        save(path, small)
        print('SMALL REPORT', small['status'], flush=True)
        return

    audit = json.loads((D / 'source-audit.json').read_text())
    assert audit['status'] == 'reviewed' and audit['builds_sha256'] == sha(D / 'results/builds.json')
    assert small['status'] == 'passed'
    rows = {}
    for row in built['programs']:
        if exclusion(row):
            continue
        metric = row['loc_audit']
        record = dict(row, loc=metric['normalized_loc'], physical_loc=metric['physical_loc'], loc_expansions=metric['expansions'],
            small_validation=small['programs'][row['key']]['validation'], samples_seconds=[], sample_validation=[], eligible_for_chart=True,
            algorithm=dict(strict=True, status='required-work-reviewed', reason=REASONS[row['benchmark']], native_benchmark_kernel=False))
        rows[row['key']] = record
    for row in audit['controls']:
        rows[row['key']] = dict(row, samples_seconds=[], sample_validation=[], eligible_for_chart=True)
    path = D / 'results/timings.json'
    report = json.loads(path.read_text()) if path.exists() else dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
        sample_count=args.samples, programs=rows, completed_benchmarks=[], orders={}, discarded=[],
        excluded=[dict(id=r['id'], benchmark=r['benchmark'], language=r['language'], source_url=r['source_url'], reason=exclusion(r)) for r in built['programs'] if exclusion(r)],
        machine=subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
        os=subprocess.check_output(['sw_vers'], text=True).strip(), power=compact.power_profile(), workloads=WORK,
        runtimes=built['runtimes'], runtime_details=built['runtime_details'], runtime_policy=built['runtime_policy'], timing_boundary=built['timing_boundary'],
        source_pins={f: sha(D / f) for f in ['fetch.py', 'prepare.py', 'build.py', 'run.py', 'audit.py', 'manifest.json', 'corrections.json', 'results/builds.json', 'source-audit.json', 'results/small-checks.json']},
        protocol='Three shuffled sequential fresh-process full-workload samples for every admitted published Erlang candidate or separately labelled work correction, plus an unchanged Mech AOT control. Startup, BEAM module loading/load-time JIT, input, computation, full output to a regular file and shutdown are timed; erlc compilation, validation and source/LOC audit are excluded. First full sample also establishes full-output validation. Native Apple Silicon, AC power, Low Power Mode off, eight online schedulers, default GC/heap settings. Compiler and power changes invalidate samples. No published Linux timing reused, input-specific shortcuts, native benchmark executors, or compressed-source metric.')
    assert report['sample_count'] == args.samples

    def all_pins():
        pins()
        for file, digest in report['source_pins'].items():
            assert sha(D / file) == digest, file
        for row in audit['controls']:
            assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256']
        for file, digest in audit['evidence'].items():
            assert sha(D / file) == digest

    save(path, report)
    all_pins()
    for bench in selected:
        candidates = [row for row in report['programs'].values() if row['benchmark'] == bench]
        rng = random.Random(20261010 + sum(map(ord, bench)))
        for run in range(args.samples):
            order = candidates.copy()
            rng.shuffle(order)
            report['orders'].setdefault(bench, [])
            if len(report['orders'][bench]) <= run:
                report['orders'][bench].append([row['key'] for row in order])
            else:
                assert report['orders'][bench][run] == [row['key'] for row in order]
            for row in order:
                if len(row['samples_seconds']) > run:
                    continue
                while True:
                    compact.wait_for_environment(REQUIRED)
                    all_pins()
                    print('RUN', run + 1, row['id'], flush=True)
                    result = execute(row, True, args.timeout)
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'], run=run + 1, result=result))
                        save(path, report)
                        continue
                    row['samples_seconds'].append(result['seconds'])
                    row['sample_validation'].append(result['validation'])
                    row.setdefault('full_validation', result['validation'])
                    row['median_seconds'] = statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds']) > 1:
                        row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                    print('SAMPLE', run + 1, row['id'], f"{result['seconds']:.6f}s", flush=True)
                    save(path, report)
                    break
        if bench not in report['completed_benchmarks']:
            report['completed_benchmarks'].append(bench)
        save(path, report)
    report['status'] = 'complete' if set(report['completed_benchmarks']) == set(BENCHES) else 'partial'
    report['finished_at'] = datetime.now(timezone.utc).isoformat()
    all_pins()
    save(path, report)
    print('REPORT', path, report['status'], flush=True)


if __name__ == '__main__':
    main()
