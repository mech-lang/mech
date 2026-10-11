"""Pin the required-work/LOC review, BEAM allocation evidence and unchanged controls."""
import json
import re
from pathlib import Path
import run as runner

D = Path(__file__).resolve().parent
C = D.parents[1]
sha = runner.sha
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')



def main():
    built = json.loads((D / 'results/builds.json').read_text())
    small = json.loads((D / 'results/small-checks.json').read_text())
    assert small['status'] == 'passed'
    data = json.loads(SITE.read_text())
    fixed = json.loads((C / 'work-audit/20261009/builds.json').read_text())['programs']
    controls = []
    for bench in runner.BENCHES:
        previous = min((r for r in data['programs'].values() if r['benchmark'] == bench and r['language'] == 'mech'), key=lambda r: r['seconds'])
        report = json.loads((C / previous['report']).read_text())
        records = report['programs']
        records = list(records.values()) if isinstance(records, dict) else records
        row = next(r.copy() for r in records if r.get('binary_sha256') == previous['binary_sha256'])
        if 'source' not in row:
            correction = next(r for r in fixed.values() if r['binary_sha256'] == row['binary_sha256'])
            row.update({k: correction[k] for k in ('source', 'source_sha256', 'binary')})
        binary = row.get('binary') or row['command'][0]
        assert sha(binary) == previous['binary_sha256'] and sha(row['source']) == row['source_sha256']
        controls.append(dict(id='mech-control-' + bench, key=bench + ':mech-control', benchmark=bench, language='mech',
            source=row['source'], source_sha256=row['source_sha256'], source_url='https://mech-lang.org', binary=binary,
            binary_sha256=sha(binary), command=row['command'], runtime_environment=row.get('runtime_environment', {}),
            loc=previous['loc'], published_chart_key=previous['key'], previous_seconds=previous['seconds'],
            previous_report=previous['report'], algorithm=previous['algorithm']))
    rows, evidence, files = [], [], {}
    for row in built['programs']:
        reason = runner.exclusion(row)
        metric = row['loc_audit']
        assert sha(row['source']) == row['source_sha256']
        if reason is None:
            assert small['programs'][row['key']]['status'] == 'passed'
        rows.append(dict(key=row['key'], id=row['id'], benchmark=row['benchmark'], language=row['language'], source=row['source'],
            sha256=row['source_sha256'], loc=metric['normalized_loc'], physical_loc=metric['physical_loc'], expansions=metric['expansions'],
            admitted=reason is None, reason=reason or runner.REASONS[row['benchmark']], source_url=row['source_url'],
            local_adaptation=row.get('local_adaptation', False)))
    evidence = json.loads((D / 'results/allocation-evidence.json').read_text())
    assert len(evidence) == 6
    for item in evidence:
        path = D / item['file']
        assert sha(path) == item['sha256']
        files[item['file']] = sha(path)
    files['support/allocation-probe.rkt'] = sha(D / 'support/allocation-probe.rkt')
    assert sum(r['admitted'] for r in rows) == 59
    report = dict(status='reviewed', builds_sha256=sha(D / 'results/builds.json'), small_checks_sha256=sha(D / 'results/small-checks.json'),
        rows=rows, controls=controls, allocation_evidence=evidence, evidence=files,
        policy=dict(output='Complete small/full output is checked against the same cohort reference; nbody energy tolerance 1e-8. Work review beyond printed answers is mandatory.',
            loc='Nonblank/noncomment LOC, including semantic module/export/compile/macro directives and all application helpers, plus additional packed independent Lisp/Racket body forms, initialized bindings and assignments. Structural collection, argument and guard separators do not inflate LOC. No compressed-size metric.',
            scheduling='Native SBCL AOT and Racket CS on the Apple M3; published task/worker partitioning retained. Default heap and garbage collection. No builds or browser tests during timed execution.',
            upstream='All 59 sources linked by the two published measurement pages were considered and preserved with their attribution and hashes. Nineteen separately named portability/work corrections are retained; no new benchmark executor. Only faithful locally timed sources may enter endpoint selection.'),
        requirements={b: 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/' + b + '.html' for b in runner.BENCHES})
    (D / 'source-audit.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Admitted 59 sources; 10 unchanged controls; 6 verified allocation paths')


if __name__ == '__main__':
    main()
