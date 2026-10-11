#!/usr/bin/env python3
"""Audit completed scripting measurements and write LOC-only endpoint tables."""
import importlib.util
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
SPEC = importlib.util.spec_from_file_location('fast_scripting', HERE / 'run.py')
run = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run)


def main():
    path = HERE / 'results/apple-m3-ac-normal-20261009.json'
    report = json.loads(path.read_text())
    assert report['status'] == 'complete'
    assert set(report['completed_benchmarks']) == set(run.common.WORK)
    assert set(report['benchmark_power_profiles']) == set(run.common.WORK)
    assert all(p == run.POWER for p in report['benchmark_power_profiles'].values())
    assert report['inputs']['manifest_sha256'] == run.common.sha(run.MANIFEST)
    assert report['inputs']['previous_report_sha256'] == run.common.sha(run.PREVIOUS)
    for asset, digest in report['inputs']['portability_assets'].items():
        assert run.common.sha(HERE / asset) == digest
    assert run.common.sha(HERE / 'python_fork.py') == report['inputs']['python_launcher_sha256']
    for runtime in report['toolchains'].values():
        assert run.common.sha(runtime['path']) == runtime['sha256']
    policies_path = HERE / 'results/runtime-policies.json'
    policies = json.loads(policies_path.read_text()) if policies_path.exists() else None
    if policies:
        assert policies['status'] == 'complete'
        assert policies['primary_report_sha256'] == run.common.sha(path)
        for asset, digest in policies['inputs']['portability_assets'].items():
            assert run.common.sha(HERE / asset) == digest
        assert all(p == run.POWER for p in policies['benchmark_power_profiles'].values())
    accepted = [r for r in [*report['programs'], *(policies['programs'] if policies else [])] if not r.get('excluded_full')]
    assert len({(r['benchmark'], r['id']) for r in accepted}) == len(accepted)
    for row in accepted:
        assert run.common.sha(row['path']) == row['sha256'], row['id']
        assert len(row['samples_seconds']) >= 3, row['id']
        assert len(row['samples_seconds']) == len(row['sample_validation']), row['id']
        assert row['median_seconds'] == statistics.median(row['samples_seconds']), row['id']
        assert all(v['reference_match'] for v in row['sample_validation']), row['id']
        assert row['source']['nonblank_noncomment_lines'] > 0
    endpoints = []
    for benchmark in run.common.WORK:
        for language in run.LANGUAGES:
            rows = [r for r in accepted if r['benchmark'] == benchmark and r['language'] == language]
            assert rows, (benchmark, language)
            fast = min(rows, key=lambda r: (r['median_seconds'], r['source']['nonblank_noncomment_lines']))
            small = min(rows, key=lambda r: (r['source']['nonblank_noncomment_lines'], r['median_seconds']))
            endpoints.append(dict(benchmark=benchmark, language=language, measured_candidates=len(rows),
                fast=dict(id=fast['id'], seconds=fast['median_seconds'], loc=fast['source']['nonblank_noncomment_lines'], url=fast['url'], algorithm=fast['algorithm']),
                small=dict(id=small['id'], seconds=small['median_seconds'], loc=small['source']['nonblank_noncomment_lines'], url=small['url'], algorithm=small['algorithm'])))
    excluded_ids = {r['id'] for r in report['excluded']}
    for selection in report['selection_audit']:
        for source_id in selection['published_fastest_ids']:
            identifier = source_id + ('-luajit' if selection['language'] == 'luajit' else '')
            assert identifier in excluded_ids or any(r['id'] == identifier for r in accepted), identifier
    new = [r for r in accepted if not r.get('reused_measurement')]
    audit = dict(passed=True, report=path.name, sha256=run.common.sha(path),
        policies_sha256=run.common.sha(policies_path) if policies else None,
        measured_programs=len(accepted), new_programs=len(new),
        new_accepted_samples=sum(len(r['samples_seconds']) for r in new),
        reused_programs=len(accepted) - len(new),
        excluded_attempts=[*report['excluded'], *(policies['excluded'] if policies else [])],
        control_drift_ratios={r['benchmark']: r['drift_ratio'] for r in report['controls']},
        policy_control_drift_ratios={r['benchmark']: r['drift_ratio'] for r in policies['controls']} if policies else {},
        discarded_samples=len(report['discarded_samples']) + (len(policies['discarded_samples']) if policies else 0),
        scope=report['protocol']['fastest_claim'], endpoints=endpoints)
    (HERE / 'results/audit.json').write_text(json.dumps(audit, indent=2) + '\n')
    lines = ['Fastest means the fastest validated measurement among the selected listed candidates and documented runtime policies. Complete source uses nonblank, noncomment physical LOC.', '',
             '| Benchmark | Runtime | Fastest source | Time | LOC | Fewest LOC | Time |',
             '|---|---|---|---:|---:|---:|---:|']
    for endpoint in endpoints:
        fast, small = endpoint['fast'], endpoint['small']
        lines.append(f"| {endpoint['benchmark']} | {endpoint['language']} | [{fast['id']}]({fast['url']}) | {fast['seconds']:.3f} s | {fast['loc']} | {small['loc']} | {small['seconds']:.3f} s |")
    lines.extend(['', 'All accepted executions validate the full output. Per-program algorithm flags are retained in the JSON audit: some listed binary-trees sources share leaves, including PHP copy-on-write constant arrays, so allocation parity is not assumed. Some historical compact FASTA entries use probability subtraction instead of cumulative search.', '',
                  'Published-fastest sources that failed validation remain excluded with their diagnostics. PHP Mandelbrot #3 needs a 32 MB System V shared-memory segment; the initial macOS limit was 4 MB.'])
    (HERE / 'results/loc-endpoints.md').write_text('\n'.join(lines) + '\n')
    print('AUDIT PASSED', len(accepted), 'programs;', len(new), 'new;', audit['new_accepted_samples'], 'new full-output-validated samples')
    print('Control drift', audit['control_drift_ratios'])


if __name__ == '__main__':
    main()
