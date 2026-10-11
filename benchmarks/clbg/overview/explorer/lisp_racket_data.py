"""Import the fully measured, work-audited Common Lisp and Racket source pool."""
from pathlib import Path

COHORT = 'lisp-racket-comparison/20261010'
LANGUAGES = {
    'lisp': dict(label='Common Lisp (SBCL)', color='#3fb68b', runtime='sbcl'),
    'racket': dict(label='Racket', color='#3c5caa', runtime='racket'),
}


def add_lisp_racket_languages(root, data, load, measured, sha):
    audit = load(COHORT + '/source-audit.json')
    built = load(COHORT + '/results/builds.json')
    report_name = COHORT + '/results/timings.json'
    report = load(report_name)
    summary = load(COHORT + '/results/summary.json')
    loc_report = COHORT + '/results/final-loc-audit.json'
    final_loc = load(loc_report)
    manifest = load(COHORT + '/manifest.json')
    corrections = load(COHORT + '/corrections.json')
    assert audit['status'] == 'reviewed' and report['status'] == 'complete' and summary['status'] == 'passed'
    assert report['sample_count'] == 3 and report['power'] == dict(source='AC Power', low_power_mode=0)
    assert set(report['completed_benchmarks']) == {b['key'] for b in data['benchmarks']}
    assert audit['builds_sha256'] == sha(root / COHORT / 'results/builds.json')
    assert summary['timing_report_sha256'] == sha(root / report_name)
    assert summary['source_audit_sha256'] == sha(root / COHORT / 'source-audit.json')
    assert final_loc['status'] == 'passed' and final_loc['timing_report_sha256'] == sha(root / report_name)
    assert final_loc['source_audit_sha256'] == sha(root / COHORT / 'source-audit.json')
    assert final_loc['audit_script_sha256'] == sha(root / COHORT / 'audit_loc_final.py')
    assert final_loc['loc_tool_sha256'] == sha(root / COHORT / 'loc_final.py')
    assert final_loc['initial_loc_tool_sha256'] == sha(root / COHORT / 'loc.py')
    assert summary['final_loc_audit_sha256'] == sha(root / loc_report)
    assert len(manifest['programs']) == 59 and len(corrections['programs']) == 19
    license_path = Path(__file__).parent / 'dist/benchmarks-game-license.txt'
    assert sha(license_path) == manifest['license_sha256']
    data['provenance']['benchmarks-game-license.txt'] = sha(license_path)
    for relative, expected in report['source_pins'].items():
        assert sha(root / COHORT / relative) == expected, relative
    for relative, expected in audit['evidence'].items():
        assert sha(root / COHORT / relative) == expected, relative
    review = {r['key']: r for r in audit['rows']}
    for key, language in LANGUAGES.items():
        data['languages'].append(dict(key=key, label=language['label'], color=language['color'], execution='AOT', line_style='solid'))
    added = []
    for row in report['programs'].values():
        if row['language'] not in LANGUAGES or not row['eligible_for_chart']:
            continue
        assert review[row['key']]['admitted'] and row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary']) == row['binary_sha256']
        metric = final_loc['rows'][row['key']]
        assert metric['source_sha256'] == row['source_sha256'] and metric['initial_normalized_loc'] == row['loc']
        adapted = row.get('local_adaptation', row['id'].endswith('-faithful'))
        language = LANGUAGES[row['language']]
        runtime = built['runtimes'][language['runtime']]
        measured(row['key'], row['benchmark'], row['language'], metric['normalized_loc'], row['samples_seconds'], row['source'],
            row['source_sha256'], report_name, COHORT + '/source-audit.json',
            algorithm=row['algorithm'], required_work_validated=True, full_output_validation=row['sample_validation'], binary_sha256=row['binary_sha256'],
            compiler_sha256=runtime['sha256'], runtime_sha256=runtime['sha256'],
            physical_loc=metric['physical_loc'], loc_expansions=[dict(e, extra_lines=e['extra']) for e in metric['expansions']], loc_metric='statement-normalized', loc_report=loc_report,
            source_url=row['source_url'], license_url='./benchmarks-game-license.txt', local_adaptation=adapted, adaptations=row['adaptations'], upstream_id=row.get('upstream_id', row['id']),
            source_provenance=('Locally corrected published ' + language['label'] + ' source: ' + ' '.join(row['adaptations']) if adapted
                else 'Published Benchmarks Game ' + language['label'] + ' source, locally compiled and timed.'),
            execution_mode='aot', runtime_policy=built['runtime_policy'], runtime_arguments=row['command'][1:],
            runtime_support='Ordinary language libraries, threads/futures, growable hash maps, and general GMP integer or CL-PPCRE regex libraries where used. Application helpers, macros and FFI bindings count toward LOC; benchmark algorithms and sequencing remain in the displayed language source. No native benchmark executor.',
            timing_boundary=built['timing_boundary'], timing_summary=built['timing_boundary'])
        benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in benchmark['groups'] if g['language'] == row['language']), None)
        if group is None:
            group = dict(language=row['language'], label=language['label'], programs=[])
            benchmark['groups'].append(group)
        group['programs'].append(row['key'])
        added.append(row['key'])
    assert len(added) == 59
    for row in report['excluded']:
        data['exclusions'].append(dict(key=row['benchmark'] + ':' + row['id'], **row))
    data['lisp_racket_audit'] = dict(published_sources=59, corrected_sources=19, admitted_sources=59,
        published_counts={'lisp': 33, 'racket': 26}, sample_count=3,
        runtimes=built['runtimes'], runtime_details=built['runtime_details'], runtime_policy=built['runtime_policy'],
        timing_report=report_name, source_audit=COHORT + '/source-audit.json', final_loc_audit=loc_report,
        controls=summary['controls'], allocation_evidence=audit['allocation_evidence'], exclusions=summary['excluded'],
        loc_corrections=final_loc['corrections'], loc_fixtures=final_loc['fixtures_passed'],
        measurement_pages={key: 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/measurements/' + language['runtime'] + '.html' for key,language in LANGUAGES.items()},
        color_source='https://github.com/github-linguist/linguist/blob/master/lib/linguist/languages.yml')
    data['provenance']['lisp_racket_data.py'] = sha(Path(__file__))
    data['protocol']['lisp_racket'] = report['protocol'] + ' All 59 published sources were considered and preserved; nineteen separately labelled corrections repair common required work, output ordering or platform compatibility and count toward LOC. SBCL saved cores and Racket CS modules use native precompilation, default GC/heap settings, and published source worker partitions. General GMP integer arithmetic and CL-PPCRE matching are allowed libraries; their application bindings and macros are counted. Allocation evidence checks distinct leaf/interior nodes and long-lived roots. Racket syntax-template ellipses are structural tokens, not independent statements. Both languages cover all ten tests. Fresh unchanged Mech AOT controls are retained for comparison; previous chart samples are preserved.'
