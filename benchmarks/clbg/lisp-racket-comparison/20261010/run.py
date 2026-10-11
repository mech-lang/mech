"""Resumable sequential Common Lisp and Racket full-workload measurements."""
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
compact.COMPILERS.add('raco')
WORK, REQUIRED = arrays.WORK, arrays.REQUIRED
BENCHES = ['pidigits', 'spectralnorm', 'nbody', 'mandelbrot', 'regexredux', 'binarytrees', 'fannkuchredux', 'fasta', 'knucleotide', 'revcomp']
REASONS = {
 'pidigits': 'Sequential unbounded LFT spigot, with extract(3) and extract(4) on every attempted step. Language code controls every transformation and digit output. GMP, where used, supplies general arbitrary-precision arithmetic only; FFI bindings and application macros count toward LOC.',
 'spectralnorm': 'Forty complete N-squared matrix-vector products, including transpose and on-demand coefficient evaluation in every inner product. No cached matrix; all rows and final reductions are covered.',
 'nbody': 'Canonical five bodies, momentum correction, all ten symmetric pair interactions and five position updates at every one of fifty million steps, and both energies. Binary64 source arithmetic and published ordinary arrays/structs retained.',
 'mandelbrot': 'All 256 million pixels use ordinary escape-tested recurrence with a fifty-iteration limit. Complete byte-oriented PBM output. Published row scheduling, general arithmetic macros and packing retained; no symmetry, interior or period shortcuts.',
 'regexredux': 'All nine complete case-insensitive non-overlapping regex searches and five ordered case-sensitive substitutions operate on the entire cleaned input. General regex libraries supply matching; all workload sequencing remains in the measured language.',
 'binarytrees': 'A fresh separately allocated two-slot node at every leaf and interior. Construct then traverse every complete tree, retain the long-lived root across temporary work and check afterward. Allocation/identity evidence is pinned separately. Default GC and no custom pool.',
 'fannkuchredux': 'Every one of the 479001600 canonical rotation-order permutations, full prefix flipping, alternating checksum and maximum. Published scalar or disjoint parallel enumeration and general generated Lisp functions/macros retained.',
 'fasta': 'Canonical ALU repetition, every fresh LCG update, direct binary64 division, cumulative unscaled binary64 probabilities and complete output. Preserve ordinary binary/linear search and buffering; no scaled thresholds, period cache or subtraction search.',
 'knucleotide': 'THREE input, case-insensitive DNA, and all seven complete overlapping growable hash maps for widths 1,2,3,4,6,12,18. Explicit lookup/update for every valid window; offset partitions cover each position once. Small initial hash maps, lexical tie ordering; no indexed histograms or selective counting.',
 'revcomp': 'All complete FASTA records, original headers, uppercase reverse/complement of every sequence and complete sixty-column output. Published byte/string buffering retained.'
}


def exclusion(row):
    if row['build_status'] != 'passed':
        return 'Build failed; diagnostics retained.'
    corrections = json.loads((D / 'corrections.json').read_text())['programs']
    replacement = next((r for r in corrections if r['upstream_id'] == row['id']), None)
    if replacement:
        return 'Superseded by separately labelled common-workload/portable correction: ' + ' '.join(replacement['adaptations'])
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

    def pins(verify_binaries=True):
        for runtime in built['runtimes'].values():
            assert sha(runtime['path']) == runtime['sha256']
        for row in built['programs']:
            assert sha(row['source']) == row['source_sha256']
            if verify_binaries and row['build_status'] == 'passed':
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
        source_pins={f: sha(D / f) for f in ['fetch.py', 'prepare.py', 'build.py', 'loc.py', 'run.py', 'audit.py', 'manifest.json', 'corrections.json', 'results/builds.json', 'source-audit.json', 'results/small-checks.json']},
        protocol='Three shuffled sequential fresh-process full-workload samples for every admitted published Common Lisp/Racket candidate or separately labelled work correction, plus an unchanged Mech AOT control. Startup, SBCL saved-core or Racket CS module loading, input, computation, full output to a regular file and shutdown are timed; compile-file/save-core or raco make compilation, validation and source/LOC audit are excluded. First full sample also establishes full-output validation. Native Apple Silicon, AC power, Low Power Mode off, published worker partitions, default GC/heap settings. Compiler and power changes invalidate samples. No published Linux timing reused, input-specific shortcuts, native benchmark executors, or compressed-source metric.')
    assert report['sample_count'] == args.samples

    def all_pins(current=None):
        pins(verify_binaries=current is None)
        if current is not None:
            assert sha(current['binary']) == current['binary_sha256']
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
                    all_pins(row)
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
