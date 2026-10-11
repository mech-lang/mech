"""Audit completed measurements and export raw values and ratios to Mech AOT."""
import argparse
import csv
import json
import math
import statistics
from pathlib import Path
from bootstrap import D, sha
from loc import normalize, fixtures

LANGUAGES = ('zig', 'odin', 'roc', 'elm')
LABELS = dict(zig='Zig', odin='Odin', roc='Roc', elm='Elm')
BENCHMARKS = ('nbody', 'mandelbrot', 'knucleotide', 'spectralnorm', 'revcomp',
              'fannkuchredux', 'fasta', 'binarytrees', 'regexredux', 'pidigits')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--partial', action='store_true', help='Write a separate progress snapshot; never admit incomplete endpoints.')
    parser.add_argument('--audit', default='source-audit.json')
    parser.add_argument('--report', default='results/timings.json')
    args = parser.parse_args()
    fixtures()
    audit_path = D / args.audit
    report_path = D / args.report
    previous_path = D / 'results/explorer-before.json'
    audit = json.loads(audit_path.read_text())
    report = json.loads(report_path.read_text())
    previous = json.loads(previous_path.read_text())
    assert audit['status'] == 'reviewed'
    assert report['source_audit_sha256'] == sha(audit_path)
    assert report['sample_count'] == 3 and report['power'] == dict(source='AC Power', low_power_mode=0)
    if not args.partial:
        assert report['status'] == 'complete'
        assert set(report['completed_benchmarks']) == set(BENCHMARKS)
    for filename, expected in {**audit['pin_files'], **audit['libraries']}.items():
        assert sha(filename) == expected, filename
    for runtime in audit['runtimes'].values():
        assert sha(runtime['path']) == runtime['sha256']
    reviewed = {r['key']: r for r in audit['programs'] + audit['controls']}
    assert set(report['programs']) == set(reviewed)
    final_loc = {}
    for key, row in report['programs'].items():
        check = reviewed[key]
        assert row['source_sha256'] == check['source_sha256']
        assert row['binary_sha256'] == check['binary_sha256']
        assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256'], key
        assert row['algorithm']['strict']
        if row['language'] in LANGUAGES:
            assert row['algorithm']['native_benchmark_kernel'] is False
            assert row['small_validation']['reference_match'], key
        samples, validations = row['samples_seconds'], row['sample_validation']
        assert len(samples) == len(validations) and len(samples) <= 3, key
        assert all(math.isfinite(s) and s > 0 for s in samples), key
        assert all(v['reference_match'] for v in validations), key
        if not args.partial:
            assert len(samples) == 3, key
        if samples:
            assert row['median_seconds'] == statistics.median(samples)
        if row['language'] in LANGUAGES:
            metric = normalize(Path(row['source']).read_text(), row['language'])
            assert metric['normalized_loc'] == row['loc'] and metric['physical_loc'] == row['physical_loc'], key
            assert metric['expansions'] == row['loc_expansions'], key
            final_loc[key] = dict(source_sha256=row['source_sha256'], **metric)
    controls = {}
    for check in audit['controls']:
        row = report['programs'][check['key']]
        benchmark = next(b for b in previous['benchmarks'] if b['key'] == row['benchmark'])
        group = next(g for g in benchmark['groups'] if g['language'] == 'mech')
        fastest = previous['programs'][group['fast']]
        smallest = previous['programs'][group['small']]
        assert row['loc'] == fastest['loc'], row['benchmark']
        assert row['source_sha256'] in fastest['files'], row['benchmark']
        seconds = statistics.median(row['samples_seconds']) if len(row['samples_seconds']) == 3 else None
        controls[row['benchmark']] = dict(key=row['key'], chart_key=group['fast'], seconds=seconds,
            samples=row['samples_seconds'], fastest_loc=row['loc'], smallest_loc=smallest['loc'],
            smallest_key=smallest['key'], smallest_source_sha256=smallest['files'],
            source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
            preserved_chart_seconds=fastest['seconds'], ratio_to_preserved_chart=seconds / fastest['seconds'] if seconds else None)
    comparisons = []
    pending = []
    for language in LANGUAGES:
        for benchmark in BENCHMARKS:
            candidates = [r for r in report['programs'].values() if r['language'] == language and r['benchmark'] == benchmark]
            assert candidates, (language, benchmark)
            incomplete = [r['key'] for r in candidates if len(r['samples_seconds']) < 3]
            if incomplete or controls[benchmark]['seconds'] is None:
                pending.append(dict(language=language, benchmark=benchmark, keys=incomplete))
                continue
            fastest = min(candidates, key=lambda p: (p['median_seconds'], p['loc']))
            smallest = min(candidates, key=lambda p: (p['loc'], p['median_seconds']))
            control = controls[benchmark]
            def endpoint(row, kind):
                reference_loc = control['fastest_loc' if kind == 'fastest' else 'smallest_loc']
                return dict(key=row['key'], source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                    seconds=row['median_seconds'], samples_seconds=row['samples_seconds'],
                    sample_stdev_seconds=statistics.stdev(row['samples_seconds']),
                    time_x_mech=row['median_seconds'] / control['seconds'], loc=row['loc'], physical_loc=row['physical_loc'],
                    loc_x_mech=row['loc'] / reference_loc, mech_loc=reference_loc, mech_seconds=control['seconds'])
            comparisons.append(dict(language=language, benchmark=benchmark, candidates=len(candidates),
                fastest=endpoint(fastest, 'fastest'), smallest=endpoint(smallest, 'smallest'),
                same_source=fastest['key'] == smallest['key']))
    summary = dict(status='partial' if args.partial else 'passed', admitted_sources=len(audit['programs']),
        sample_count=3, completed_samples=sum(len(r['samples_seconds']) for r in report['programs'].values()),
        expected_samples=3 * len(reviewed), comparisons=comparisons, pending=pending, controls=controls,
        ratio_policy='Time ratios use the fresh unchanged fastest Mech AOT control for that workload for both profiles. Fastest LOC ratios use the fastest Mech source; smallest LOC ratios use the smallest Mech source in the preserved explorer dataset. Below 1x means faster or fewer LOC than that Mech reference.',
        endpoint_policy='Measured fastest and smallest normalized-LOC endpoints selected from this cohort, with runtime and then LOC as tie breakers. A single source may occupy both endpoints. These are measured candidate winners, not a claim of an absolute language speed limit.',
        timing_report_sha256=sha(report_path), source_audit_sha256=sha(audit_path), previous_explorer_sha256=sha(previous_path),
        summary_script_sha256=sha(Path(__file__)), loc_tool_sha256=sha(D/'loc.py'),
        final_loc=final_loc, runtimes=audit['runtimes'], allocation_evidence=audit['allocation_evidence'],
        general_helper_tests=audit['general_helper_tests'], power=report['power'], machine=report['machine'],
        rejected_full_outputs=json.loads((D/'results/full-check-failures.json').read_text()),
        retained_report=report.get('retained_report'), discarded_samples=len(report['discarded']))
    summary['timing_report'] = args.report
    summary['source_audit'] = args.audit
    summary['retained_source_report'] = report.get('retained_source_report')
    summary['work_exclusions'] = report.get('work_exclusions', [])
    name = 'progress' if args.partial else 'summary'
    (D/f'results/{name}.json').write_text(json.dumps(summary, indent=2)+'\n')
    with (D/f'results/{name}.csv').open('w', newline='') as stream:
        fields = ['language','benchmark','profile','seconds','time_x_mech','loc','loc_x_mech','mech_seconds','mech_loc','key']
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for comparison in comparisons:
            for profile in ('fastest','smallest'):
                row = comparison[profile]
                writer.writerow(dict(language=comparison['language'], benchmark=comparison['benchmark'], profile=profile,
                    **{key:row[key] for key in fields if key in row}))
    lines = ['# Zig, Odin, Roc and Elm compared with Mech', '', summary['ratio_policy'], '',
             'Values are full-workload medians of three fresh-process runs. LOC includes the complete application and its helpers/FFI declarations; general library implementation and the generic process I/O adapter are excluded.', '',
             'Roc uses the pinned alpha4 LLVM compiler from September 2025. Elm uses the pinned optimized compiler and Node; all benchmark algorithms remain in Elm.', '']
    for language in LANGUAGES:
        lines += [f'## {LABELS[language]}', '', '| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |',
                  '|---|---|---:|---:|---:|---:|']
        for comparison in comparisons:
            if comparison['language'] != language:
                continue
            for profile in ('fastest','smallest'):
                row = comparison[profile]
                lines.append(f"| {comparison['benchmark']} | {profile} | {row['seconds']:.6f} | {row['time_x_mech']:.3f}× | {row['loc']} | {row['loc_x_mech']:.3f}× |")
        lines.append('')
    if pending:
        lines += ['Incomplete candidate groups are omitted until all three required full-output samples and their Mech control are available.', '']
    (D/f'results/{name}.md').write_text('\n'.join(lines))
    print(json.dumps(dict(status=summary['status'], comparisons=len(comparisons), completed_samples=summary['completed_samples'],
        expected_samples=summary['expected_samples'], pending_groups=len(pending))))


if __name__ == '__main__':
    main()
