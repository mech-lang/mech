"""Validate complete local Lisp/Racket measurements and select measured endpoints."""
import json
import math
import statistics
from pathlib import Path
import run as runner

D = Path(__file__).resolve().parent
sha = runner.sha
LANGUAGES = ('lisp', 'racket')


def main():
    path = D / 'results/timings.json'
    report = json.loads(path.read_text())
    audit = json.loads((D / 'source-audit.json').read_text())
    final_loc = json.loads((D / 'results/final-loc-audit.json').read_text())
    assert report['status'] == 'complete' and set(report['completed_benchmarks']) == set(runner.BENCHES)
    assert final_loc['status'] == 'passed' and final_loc['timing_report_sha256'] == sha(path)
    assert final_loc['source_audit_sha256'] == sha(D / 'source-audit.json')
    assert final_loc['audit_script_sha256'] == sha(D / 'audit_loc_final.py')
    assert final_loc['loc_tool_sha256'] == sha(D / 'loc_final.py')
    assert final_loc['initial_loc_tool_sha256'] == sha(D / 'loc.py')
    for file, digest in report['source_pins'].items():
        assert sha(D / file) == digest, file
    for file, digest in audit['evidence'].items():
        assert sha(D / file) == digest, file
    for row in report['programs'].values():
        assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256']
        if row['language'] in LANGUAGES:
            assert row['small_validation']['reference_match'] and row['algorithm']['strict']
        assert row['full_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert row['median_seconds'] == statistics.median(row['samples_seconds'])
        assert row['sample_stdev_seconds'] == statistics.stdev(row['samples_seconds'])

    def point(row):
        return {key: row[key] for key in ('key', 'id', 'loc', 'source_sha256', 'binary_sha256', 'median_seconds', 'samples_seconds', 'sample_stdev_seconds')}

    rows, controls = [], {}
    for bench in runner.BENCHES:
        control = report['programs'][bench + ':mech-control']
        controls[bench] = dict(chart_key=control['published_chart_key'], source_sha256=control['source_sha256'], binary_sha256=control['binary_sha256'],
            previous_seconds=control['previous_seconds'], samples_seconds=control['samples_seconds'], median_seconds=control['median_seconds'],
            median_ratio=control['median_seconds'] / control['previous_seconds'])
        for language in LANGUAGES:
            candidates = [dict(r, loc=final_loc['rows'][r['key']]['normalized_loc']) for r in report['programs'].values()
                if r['benchmark'] == bench and r['language'] == language and r['eligible_for_chart']]
            assert candidates, (bench, language)
            fast = min(candidates, key=lambda r: (r['median_seconds'], r['loc']))
            small = min(candidates, key=lambda r: (r['loc'], r['median_seconds']))
            rows.append(dict(benchmark=bench, language=language, candidates=len(candidates), fast=point(fast), small=point(small),
                fastest_seconds_over_mech=fast['median_seconds'] / control['median_seconds'],
                smallest_seconds_over_mech=small['median_seconds'] / control['median_seconds'],
                fastest_seconds_over_chart_mech=fast['median_seconds'] / control['previous_seconds'],
                smallest_seconds_over_chart_mech=small['median_seconds'] / control['previous_seconds']))
    result = dict(status='passed', timing_report_sha256=sha(path), source_audit_sha256=sha(D / 'source-audit.json'),
        final_loc_audit_sha256=sha(D / 'results/final-loc-audit.json'), admitted_sources=59, rows=rows, controls=controls,
        excluded=report['excluded'], runtime_details=report['runtime_details'], protocol=report['protocol'], discarded_samples=len(report['discarded']),
        fastest_seconds_over_mech_geomeans={language: math.exp(statistics.mean(math.log(r['fastest_seconds_over_mech']) for r in rows if r['language'] == language)) for language in LANGUAGES})
    (D / 'results/summary.json').write_text(json.dumps(result, indent=2) + '\n')
    for row in rows:
        print(row['benchmark'], row['language'], 'fewest', row['small']['loc'], 'fastest', row['fast']['loc'],
            f"{row['fast']['median_seconds']:.6f}s", f"{row['fastest_seconds_over_mech']:.3f}x Mech")


if __name__ == '__main__':
    main()
