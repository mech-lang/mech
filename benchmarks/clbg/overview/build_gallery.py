#!/usr/bin/env python3
"""Build a presentation of preserved studies without changing any measurements."""
import hashlib
import html
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
STUDIES = [
    ('nbody', 'nbody', 'N = 50,000,000 steps', 'nbody/results/apple-m3-v04-native-optimized-50m.json',
     'Full cohort before the pairwise optimization; newer matched check available.'),
    ('mandelbrot', 'Mandelbrot', 'N = 16,000', 'mandelbrot/results/apple-m3-revised-corner-16000.json',
     'Revised complete Mech sources; batches normalized to their own Rust #4 control.'),
    ('knucleotide', 'k-nucleotide', 'FASTA N = 25,000,000', 'knucleotide/results/apple-m3-revised-corner-25000000.json',
     'Revised complete Mech sources; batches normalized to their own Rust #7 control.'),
    ('spectralnorm', 'Spectral norm', 'N = 5,500', 'spectralnorm/results/apple-m3-optimized-5500.json',
     'Latest full cohort, including integer denominators; five samples per build.'),
    ('revcomp', 'Reverse complement', 'FASTA N = 25,000,000', 'revcomp/results/apple-m3-final-25000000.json',
     'Final combined cohort; each build retains its original measurement batch.'),
    ('fannkuchredux', 'Fannkuch-redux', 'N = 12 · 479,001,600 permutations', 'fannkuchredux/results/apple-m3-final-12.json',
     'Final combined cohort; 3–5 samples; Rust includes the labeled ARM SIMD port.'),
    ('fasta', 'FASTA', 'N = 25,000,000 · 250,000,000 bases', 'fasta/results/apple-m3-streaming-20261008-25000000.json',
     'October 8 cycle replay and two-stage streaming; unchanged complete Mech sources; five matched samples; batches normalized to their C #2 LLVM control.'),
    ('binarytrees', 'Binary trees', 'N = 21 · allocation policies labeled', 'binarytrees/results/apple-m3-21.json',
     'Mech source uses recursion and named records over core planned arenas; allocation policies are labeled; three matched samples per build.'),
    ('fasta-strict', 'FASTA — strict cohort', 'N = 25,000,000 · complete output', 'fasta/results/apple-m3-streaming-20261008-25000000.json',
     'Strict cumulative-probability comparison; some smallest listed sources use historical algorithms and appear only in All listings.'),
    ('nbody-latest', 'nbody — latest matched check', 'N = 50,000,000 steps', 'nbody/results/apple-m3-chapel-tricks-50m.json',
     'Nine builds of Mech, Rust #3 and Chapel #3; five samples; includes pairwise Mech.'),
]


def refresh_preview(filename, fragment):
    """Keep the existing standalone guards while replacing its chart fragment."""
    path = HERE / filename
    if not path.exists():
        return
    wrapper = path.read_text()
    match = re.search(r'data-srcdoc="([^"]*)"', wrapper)
    assert match and wrapper.count('data-srcdoc=') == 1
    document = html.unescape(match.group(1))
    start = document.index(fragment.split('\n', 1)[0])
    tail = fragment[-200:]
    end = document.index(tail, start) + len(tail)
    document = document[:start] + fragment + document[end:]
    assert document.count(fragment) == 1
    path.write_text(wrapper[:match.start(1)] + html.escape(document, quote=True) + wrapper[match.end(1):])


def main():
    studies = []
    for key, name, workload, filename, note in STUDIES:
        path = CLBG / filename
        report = json.loads(path.read_text())
        rows = []
        for row in report['programs']:
            if key == 'fasta-strict' and not row['algorithm']['strict']: continue
            metric = row['source']
            record = dict(id=row['id'], lang=row['language'], seconds=row['median_seconds'],
                          score=row.get('comparison_time_ratio', row['median_seconds']),
                          loc=metric['nonblank_noncomment_lines'], gzip=metric['gzip1_bytes'],
                          samples=len(row['samples_seconds']),
                          batch=row.get('measurement_batch', path.name),
                          modified=bool(row.get('source_modified')), starred=bool(row.get('upstream_starred')))
            if 'algorithm' in row: record['algorithm'] = row['algorithm']
            assert record['seconds'] > 0 and record['loc'] > 0 and record['gzip'] > 0
            rows.append(record)
        studies.append(dict(key=key, name=name, workload=workload, note=note, rows=rows,
                            report=filename, sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    # Use the newer Mech nbody policies without mixing unadjusted batch times.
    # As in the Mandelbrot/k-nucleotide studies, each batch uses its own Rust
    # control. The original studies and raw elapsed seconds stay untouched.
    nbody, latest = studies[0], studies[-1]
    old_control = next(r['seconds'] for r in nbody['rows'] if r['id'] == 'nbody-rust-3')
    new_control = next(r['seconds'] for r in latest['rows'] if r['id'] == 'rust-3')
    merged = []
    for row in nbody['rows']:
        if row['lang'] == 'mech' or row['id'] in ('nbody-rust-3', 'nbody-chapel-3'):
            continue
        merged.append(dict(row, score=row['seconds'] / old_control))
    for row in latest['rows']:
        merged.append(dict(row, score=row['seconds'] / new_control))
    nbody['rows'] = merged
    nbody['note'] = 'Latest Mech sources plus preserved full cohort; batches normalized to their own Rust #3 control.'
    nbody['measurement_batches'] = [dict(report=nbody['report'], sha256=nbody['sha256'], control_seconds=old_control),
                                    dict(report=latest['report'], sha256=latest['sha256'], control_seconds=new_control)]
    compact_path = CLBG / 'compact/results/apple-m3-compact.json'
    if compact_path.exists():
        compact = json.loads(compact_path.read_text())
        for study in studies:
            benchmark = study['key'].replace('-strict', '')
            if study['key'] == 'nbody-latest': continue
            for row in compact['programs']:
                if row['benchmark'] != benchmark or row.get('excluded_full') or 'median_seconds' not in row: continue
                if study['key'] == 'fasta-strict' and not row['algorithm']['strict']: continue
                assert len(row['samples_seconds']) >= 3
                metric = row['source']
                study['rows'].append(dict(id=row['id'], lang=row['language'], seconds=row['median_seconds'],
                    score=row['comparison_time_ratio'], loc=metric['nonblank_noncomment_lines'], gzip=metric['gzip1_bytes'],
                    samples=len(row['samples_seconds']), batch=row['measurement_batch'], modified=False,
                    starred=row['upstream_starred'], algorithm=row['algorithm'], endpoint_scope='compact-only',
                    selection_reasons=row['selection_reasons'], source_url=row['url'], execution_mode=row['execution_mode']))
            if benchmark in compact['completed_benchmarks']:
                study['note'] += ' Added compact-only entries: minimum listed gzip, local gzip and LOC; no fastest-language claim. Matched controls rerun.'
                study['compact_report'] = dict(report=str(compact_path.relative_to(CLBG)), sha256=hashlib.sha256(compact_path.read_bytes()).hexdigest())
            if study['key'] == 'fasta':
                study['note'] += ' Full listed cohort includes labeled historical algorithms; Strict cohort view is available.'
    # A complete fresh run replaces every legacy timing together. Do not publish
    # a chart mixing fresh normal-power results with old untracked power states.
    fresh_path = CLBG / 'retime/results/apple-m3-ac-normal-20261008.json'
    timing_label = 'Original measurements · power state untracked'
    if fresh_path.exists():
        fresh = json.loads(fresh_path.read_text())
        if fresh['status'] == 'complete':
            audit = json.loads((fresh_path.parent / 'audit.json').read_text())
            assert audit['passed'] and audit['sha256'] == hashlib.sha256(fresh_path.read_bytes()).hexdigest(), 'Audit the fresh cohort before rebuilding charts'
            assert set(fresh['completed_benchmarks']) == set(s['key'] for s in studies if s['key'] not in ('nbody-latest', 'fasta-strict'))
            assert all(p == {'source': 'AC Power', 'low_power_mode': 0} for p in fresh['benchmark_power_profiles'].values())
            latest_ids = {r['id'] for r in latest['rows']}
            for study in studies:
                benchmark = study['key'].replace('-strict', '').replace('-latest', '')
                rows = []
                for row in fresh['programs']:
                    if row['benchmark'] != benchmark or row.get('excluded_full'): continue
                    if study['key'] == 'nbody-latest' and row['id'] not in latest_ids: continue
                    algorithm = row.get('algorithm', row.get('display_metadata', {}).get('algorithm'))
                    if study['key'] == 'fasta-strict' and not algorithm['strict']: continue
                    assert len(row['samples_seconds']) >= 3 and all(v['reference_match'] for v in row['sample_validation'])
                    metric = row['source']
                    record = dict(row.get('display_metadata', {}), id=row['id'], lang=row['language'],
                        seconds=row['median_seconds'], score=row['median_seconds'],
                        loc=metric['nonblank_noncomment_lines'], gzip=metric['gzip1_bytes'],
                        samples=len(row['samples_seconds']), batch=fresh_path.name,
                        modified=bool(row.get('source_modified')), starred=bool(row.get('upstream_starred')),
                        seconds_range=[min(row['samples_seconds']), max(row['samples_seconds'])])
                    if algorithm: record['algorithm'] = algorithm
                    for field in ('endpoint_scope', 'selection_reasons', 'execution_mode'):
                        if field in row: record[field] = row[field]
                    if 'url' in row: record['source_url'] = row['url']
                    rows.append(record)
                study.update(rows=rows, report=str(fresh_path.relative_to(CLBG)),
                    sha256=hashlib.sha256(fresh_path.read_bytes()).hexdigest(),
                    note='Fresh full-output measurements on AC power with Low Power Mode off; three sequential samples per build. Added runtimes are compact-only; algorithm and allocation differences are labeled.')
                study.pop('measurement_batches', None); study.pop('compact_report', None)
            timing_label = 'AC power · Low Power Mode off · fresh full-cohort measurements'
    main_studies = [s for s in studies if s['key'] not in ('nbody-latest', 'fasta-strict')]
    (HERE / 'normalized-results.json').write_text(json.dumps(main_studies, indent=2) + '\n')
    data = json.dumps(studies, separators=(',', ':')).replace('</', '<\\/')
    template = (HERE / 'benchmark-charts.template.html').read_text()
    assert template.count('/*__BENCHMARK_DATA__*/') == 1
    output = HERE / 'benchmark-charts.html'
    output.write_text(template.replace('/*__BENCHMARK_DATA__*/', data).replace('<!--__POWER_LABEL__-->', timing_label))
    refresh_preview('preview.html', output.read_text())
    assert output.stat().st_size < 1_000_000
    print(f'{len(main_studies)} benchmarks: {sum(len(s["rows"]) for s in main_studies)} plotted builds; separate {len(latest["rows"])}-build matched nbody check; {output.stat().st_size:,} bytes')
    mega_template = HERE / 'mega-benchmark-chart.template.html'
    if mega_template.exists():
        mega = HERE / 'mega-benchmark-chart.html'
        mega.write_text(mega_template.read_text().replace('/*__BENCHMARK_DATA__*/', json.dumps(main_studies, separators=(',', ':')).replace('</', '<\\/')).replace('<!--__POWER_LABEL__-->', timing_label))
        refresh_preview('mega-preview.html', mega.read_text())
        assert mega.stat().st_size < 1_000_000
        print(f'Mega chart: {mega.stat().st_size:,} bytes')


if __name__ == '__main__':
    main()
