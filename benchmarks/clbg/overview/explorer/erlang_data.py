"""Import the complete locally measured and audited published Erlang source pool."""
from pathlib import Path

COHORT = 'erlang-comparison/20261010'


def add_erlang_language(root, data, load, measured, sha):
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
    assert final_loc['loc_tool_sha256'] == sha(root / COHORT / 'support/loc_probe.erl')
    assert summary['final_loc_audit_sha256'] == sha(root / loc_report)
    assert len(manifest['programs']) == 21 and len(corrections['programs']) == 11
    license_path = Path(__file__).parent / 'dist/benchmarks-game-license.txt'
    assert sha(license_path) == manifest['license_sha256']
    data['provenance']['benchmarks-game-license.txt'] = sha(license_path)
    assert built['runtime_details']['jit'] == 'jit' and built['runtime_details']['schedulers'] == 8
    for relative, expected in report['source_pins'].items():
        assert sha(root / COHORT / relative) == expected
    for relative, expected in audit['evidence'].items():
        assert sha(root / COHORT / relative) == expected
    runtime_amendments = []
    for entry in report.get('runtime_amendments', []):
        assert sha(root / COHORT / entry['file']) == entry['sha256']
        amendment = load(COHORT + '/' + entry['file'])
        for snapshot in amendment['snapshots'].values():
            assert sha(root / COHORT / snapshot['file']) == snapshot['sha256']
        previous = load(COHORT + '/' + amendment['snapshots']['results/timings.json']['file'])
        assert sum(len(r['samples_seconds']) for r in previous['programs'].values()) == amendment['preserved_samples'] == 42
        for key, old in previous['programs'].items():
            current = report['programs'][key]
            assert current['source_sha256'] == old['source_sha256'] and current['binary_sha256'] == old['binary_sha256']
            assert current['samples_seconds'][:len(old['samples_seconds'])] == old['samples_seconds']
        for change in amendment['changes']:
            assert change['key'].startswith('regexredux:')
            assert [x for x in change['after'] if x != '-noinput'] == change['before']
        runtime_amendments.append(dict(file=entry['file'], sha256=entry['sha256'], reason=amendment['reason'], preserved_samples=42))
    review = {r['key']: r for r in audit['rows']}
    data['languages'].append(dict(key='erlang', label='Erlang', color='#B83998', execution='JIT', line_style='solid'))
    added = []
    for row in report['programs'].values():
        if row['language'] != 'erlang' or not row['eligible_for_chart']:
            continue
        assert review[row['key']]['admitted'] and row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary']) == row['binary_sha256']
        metric = final_loc['rows'][row['key']]
        assert metric['source_sha256'] == row['source_sha256'] and metric['initial_normalized_loc'] == row['loc']
        adapted = row.get('local_adaptation', False)
        measured(row['key'], row['benchmark'], 'erlang', metric['normalized_loc'], row['samples_seconds'], row['source'],
            row['source_sha256'], report_name, COHORT + '/source-audit.json',
            algorithm=row['algorithm'], required_work_validated=True, full_output_validation=row['sample_validation'], binary_sha256=row['binary_sha256'],
            compiler_sha256=built['runtimes']['compiler']['sha256'], runtime_sha256=built['runtimes']['engine']['sha256'],
            physical_loc=metric['physical_loc'], loc_expansions=[dict(e, extra_lines=e['extra']) for e in metric['expansions']], loc_metric='statement-normalized', loc_report=loc_report,
            source_url=row['source_url'], license_url='./benchmarks-game-license.txt', local_adaptation=adapted, adaptations=row['adaptations'], upstream_id=row.get('upstream_id', row['id']),
            source_provenance=('Locally corrected published Erlang source: ' + ' '.join(row['adaptations']) if adapted
                else 'Published Benchmarks Game Erlang source, locally compiled and timed.'),
            execution_mode='jit', runtime_policy=built['runtime_policy'], runtime_arguments=row['command'][1:],
            runtime_support='Ordinary Erlang/OTP binary, ETS hash-map, arbitrary-precision integer, regex and process libraries. Algorithm and sequencing remain in the displayed Erlang source; no native benchmark executor.',
            timing_boundary=built['timing_boundary'], timing_summary=built['timing_boundary'])
        benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in benchmark['groups'] if g['language'] == 'erlang'), None)
        if group is None:
            group = dict(language='erlang', label='Erlang', programs=[])
            benchmark['groups'].append(group)
        group['programs'].append(row['key'])
        added.append(row['key'])
    assert len(added) == 21
    for row in report['excluded']:
        data['exclusions'].append(dict(key=row['benchmark'] + ':' + row['id'], **row))
    data['erlang_audit'] = dict(published_sources=21, corrected_sources=11, admitted_sources=21,
        sample_count=3, runtimes=built['runtimes'], runtime_details=built['runtime_details'], runtime_policy=built['runtime_policy'],
        timing_report=report_name, source_audit=COHORT + '/source-audit.json', final_loc_audit=loc_report,
        controls=summary['controls'], allocation_evidence=audit['allocation_evidence'], exclusions=summary['excluded'],
        runtime_amendments=runtime_amendments,
        measurement_page='https://benchmarksgame-team.pages.debian.net/benchmarksgame/measurements/erlang.html',
        color_source='https://github.com/github-linguist/linguist/blob/master/lib/linguist/languages.yml')
    data['provenance']['erlang_data.py'] = sha(Path(__file__))
    data['protocol']['erlang'] = 'All ten full workloads, three shuffled fresh-process samples per admitted candidate. All 21 published Erlang sources were considered and preserved; eleven separately labelled corrections repair common required work or output ordering and count toward LOC. Erlang/OTP ' + built['runtime_details']['release'] + ' uses native Apple Silicon BEAM load-time JIT, eight online schedulers, original task partitioning and default GC/heap settings. Startup, module loading, JIT, complete input/computation/output and shutdown are timed; erlc bytecode compilation is excluded. Compiler evidence verifies fresh leaf/interior allocation and long-lived tree retention. Floating-CDF FASTA, all seven nucleotide maps and whole-input regex work follow the shared cohort policy. Unchanged Mech AOT controls were remeasured alongside Erlang; earlier chart samples are preserved.'
