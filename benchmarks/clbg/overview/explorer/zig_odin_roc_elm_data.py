"""Import the pinned, measured Zig/Odin/Roc/Elm candidate cohort."""
from pathlib import Path

COHORT = 'zig-odin-roc-elm-comparison/20261010'
LANGUAGES = {
    'zig': dict(label='Zig', color='#ec915c', execution='AOT',
        policy='Zig 0.17.0 · LLVM AOT · ReleaseFast · native CPU'),
    'odin': dict(label='Odin', color='#60AFFE', execution='AOT',
        policy='Odin dev-2026-10 · AOT · -o:speed -no-bounds-check · native CPU'),
    'roc': dict(label='Roc', color='#7c38f5', execution='AOT',
        policy='Roc alpha4 LLVM compiler · 2025-09-09 d73ea109cc2 · --optimize'),
    'elm': dict(label='Elm', color='#60B5CC', execution='JIT',
        policy='Elm 0.19.3 --optimize · Node 24.19.0'),
}


def add_zig_odin_roc_elm_languages(root, data, load, measured, add_file, sha):
    audit_name = COHORT + '/source-audit-complete.json'
    report_name = COHORT + '/results/timings-complete.json'
    summary_name = COHORT + '/results/summary.json'
    audit = load(audit_name)
    report = load(report_name)
    summary = load(summary_name)
    builds = load(COHORT + '/results/builds-complete.json')
    published = load(COHORT + '/published.json')
    bootstrap = load(COHORT + '/bootstrap.json')
    colors = load(COHORT + '/support/language-colors.json')
    exclusions = load(COHORT + '/exclusions.json')
    assert audit['status'] == 'reviewed' and report['status'] == 'complete' and summary['status'] == 'passed'
    assert report['source_audit_sha256'] == summary['source_audit_sha256'] == sha(root / audit_name)
    assert summary['timing_report_sha256'] == sha(root / report_name)
    assert summary['summary_script_sha256'] == sha(root / COHORT / 'summarize.py')
    assert summary['loc_tool_sha256'] == sha(root / COHORT / 'loc.py')
    assert report['sample_count'] == 3 and report['power'] == dict(source='AC Power', low_power_mode=0)
    assert set(report['completed_benchmarks']) == {b['key'] for b in data['benchmarks']}
    assert summary['admitted_sources'] == len(audit['programs']) == 61
    assert summary['completed_samples'] == summary['expected_samples'] == 213
    assert not summary['pending'] and len(summary['comparisons']) == 40
    assert all(r['build_status'] == 'passed' for r in builds['programs'])
    assert len(builds['programs']) == 61 and len(published['programs']) == 20
    for filename, expected in {**audit['pin_files'], **audit['libraries']}.items():
        assert sha(filename) == expected, filename
    for language, runtime in audit['runtimes'].items():
        assert sha(runtime['path']) == runtime['sha256']
        assert colors['languages'][language]['color'] == LANGUAGES[language]['color']
    retained = report['retained_report']
    assert retained['samples'] == 75 and sha(root / COHORT / retained['path']) == retained['sha256']
    retained_source = report['retained_source_report']
    assert retained_source['samples'] == 198 and sha(root / COHORT / retained_source['path']) == retained_source['sha256']
    retained_initial = report['retained_initial_source_report']
    assert retained_initial['samples'] == 75 and sha(root / COHORT / retained_initial['path']) == retained_initial['sha256']
    assert summary['timing_report'] == 'results/timings-complete.json'
    assert summary['source_audit'] == 'source-audit-complete.json'
    downloads = {}
    for extension in ('csv', 'md'):
        target = Path(__file__).parent / 'dist' / ('zig-odin-roc-elm-mech.' + extension)
        target.write_bytes((root / COHORT / 'results' / ('summary.' + extension)).read_bytes())
        data['provenance'][target.name] = sha(target)
        downloads[extension] = './' + target.name
    assert len(report['work_exclusions']) == 2
    assert all(row['key'] not in report['programs'] and row['samples_excluded'] == 0 for row in report['work_exclusions'])
    license_source = root / COHORT / 'upstream/hanabi/LICENSE'
    license_path = Path(__file__).parent / 'dist/programming-language-benchmarks-license.txt'
    assert sha(license_source) == sha(license_path)
    data['provenance']['programming-language-benchmarks-license.txt'] = sha(license_path)
    for language, meta in LANGUAGES.items():
        data['languages'].append(dict(key=language, label=meta['label'], color=meta['color'],
            execution=meta['execution'], line_style='solid', runtime_policy=meta['policy']))
    review = {r['key']:r for r in audit['programs']}
    added = []
    support_ids = {}
    for row in report['programs'].values():
        if row['language'] not in LANGUAGES:
            continue
        check = review[row['key']]
        assert check['eligible_for_chart'] and row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['source_sha256'] == check['source_sha256'] and row['binary_sha256'] == check['binary_sha256']
        assert row['small_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256']
        loc = summary['final_loc'][row['key']]
        assert loc['source_sha256'] == row['source_sha256'] and loc['normalized_loc'] == row['loc']
        assert loc['physical_loc'] == row['physical_loc'] and loc['expansions'] == row['loc_expansions']
        upstream = row.get('upstream_source')
        if upstream:
            assert sha(upstream) == row['upstream_sha256']
        meta = LANGUAGES[row['language']]
        runtime = audit['runtimes'][row['language']]
        provenance = ('Locally corrected published Programming-Language-Benchmarks source: ' + ' '.join(row['adaptations'])
            if upstream and row['local_adaptation'] else
            'Published Programming-Language-Benchmarks source, locally compiled and timed.' if upstream else
            'Locally authored ' + meta['label'] + ' benchmark. ' + ' '.join(row['adaptations']))
        support = 'Ordinary standard-library arrays, growable maps, threads and process I/O. General GMP or PCRE2 arithmetic/matching libraries where used; all application sequencing and FFI declarations count toward LOC. No native benchmark executor.'
        if row['language'] == 'roc':
            support = 'Pinned basic-cli 0.20.0 provides generic process I/O and allocation. The complete benchmark and general pure-Roc big-integer/ASCII-regex helpers are in displayed Roc source and count toward LOC. This measures the established alpha4 LLVM compiler, not the newer compiler rewrite.'
        if row['language'] == 'elm':
            support = 'All benchmark work is Elm, using ordinary Elm packages for unbounded integer arithmetic, regular expressions, arrays and growable hash maps. The separately inspectable Node adapter only reads argv/stdin and writes port output; it contains no algorithm or arithmetic. Application helpers and port boilerplate count toward LOC; generic library/adapter code is excluded.'
        control = summary['controls'][row['benchmark']]
        measured(row['key'], row['benchmark'], row['language'], row['loc'], row['samples_seconds'],
            row['source'], row['source_sha256'], report_name, audit_name,
            algorithm=row['algorithm'], required_work_validated=True, full_output_validation=row['sample_validation'],
            binary_sha256=row['binary_sha256'], compiler_sha256=runtime['sha256'],
            runtime_sha256=audit['libraries']['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'] if row['language']=='elm' else runtime['sha256'],
            physical_loc=row['physical_loc'], loc_expansions=row['loc_expansions'], loc_metric='statement-normalized', loc_report=summary_name,
            source_url=row['source_url'], license_url='./programming-language-benchmarks-license.txt' if upstream else None,
            source_link_label='Published source' if upstream else 'Benchmark requirements',
            upstream_source_sha256=row.get('upstream_sha256'), local_adaptation=row['local_adaptation'], adaptations=row['adaptations'],
            source_provenance=provenance, execution_mode=meta['execution'].lower(), runtime_policy=meta['policy'],
            runtime_arguments=row['command'][1:], runtime_support=support,
            mech_comparison=dict(seconds=control['seconds'], fastest_loc=control['fastest_loc'], smallest_loc=control['smallest_loc'],
                time_x_mech=row['median_seconds']/control['seconds'],
                fastest_loc_x_mech=row['loc']/control['fastest_loc'], smallest_loc_x_mech=row['loc']/control['smallest_loc']),
            timing_boundary=audit['timing_boundary'], timing_summary=audit['timing_boundary'])
        if row['language'] == 'elm':
            if not support_ids:
                for relative, role, note in [('support/elm-runner.cjs','runtime-support','Generic argv/stdin/stdout port adapter only; excluded from application LOC. All algorithms remain in the Elm application.'),
                    ('sources/elm/elm.json','runtime-support','Pinned ordinary Elm package dependencies; excluded from application LOC.')]:
                    path = root / COHORT / relative
                    file_id = add_file(path, audit['libraries'][str(path)])[0]
                    data['files'][file_id].update(role=role, loc_note=note)
                    support_ids[relative] = file_id
            data['programs'][row['key']]['files'] += list(support_ids.values())
        benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in benchmark['groups'] if g['language'] == row['language']), None)
        if group is None:
            group = dict(language=row['language'], label=meta['label'], programs=[])
            benchmark['groups'].append(group)
        group['programs'].append(row['key'])
        added.append(row['key'])
    assert len(added) == 61
    for language in LANGUAGES:
        assert {r['benchmark'] for r in review.values() if r['language']==language} == {b['key'] for b in data['benchmarks']}
    # Original unsupported/noncomparable implementations never enter endpoint selection.
    for row in exclusions['excluded']:
        assert sha(row['path']) == row['sha256']
        data['exclusions'].append(dict(key=row['benchmark'] + ':' + row['id'], **row))
    for row in report['work_exclusions']:
        assert sha(row['source']) == row['source_sha256']
        data['exclusions'].append(dict(benchmark=row['key'].split(':')[0], language='elm', **row))
    data['zig_odin_roc_elm_audit'] = dict(admitted_sources=61, published_source_candidates=20,
        locally_authored_source_candidates=41, sample_count=3, completed_samples=213,
        timing_report=report_name, source_audit=audit_name, summary=summary_name,
        comparison_downloads=downloads,
        controls=summary['controls'], comparisons_to_mech=summary['comparisons'],
        ratio_policy=summary['ratio_policy'], endpoint_policy=summary['endpoint_policy'],
        runtimes={key:{k:v for k,v in runtime.items() if k in ('version','sha256','source_url','archive_sha256')}
                  for key,runtime in audit['runtimes'].items()},
        libraries=audit['libraries'], allocation_evidence=audit['allocation_evidence'],
        general_helper_tests=audit['general_helper_tests'], elm_packing_evidence=audit['elm_packing_evidence'],
        nucleotide_key_evidence=audit['nucleotide_key_evidence'],
        rejected_full_outputs=summary['rejected_full_outputs'],
        retained_report=retained, retained_source_report=retained_source,
        retained_initial_source_report=retained_initial, work_exclusions=report['work_exclusions'],
        upstream_repository=bootstrap['repository'], upstream_commit=bootstrap['commit'],
        color_source=colors['source_url'])
    data['provenance']['zig_odin_roc_elm_data.py'] = sha(Path(__file__))
    data['protocol']['zig_odin_roc_elm'] = audit['timing_boundary'] + ' All four languages cover ten required workloads. Twenty pinned community sources are ported and work-corrected where needed; forty-one candidates are locally authored. Endpoints select only measured fastest and smallest normalized-LOC sources. Roc uses the established alpha4 LLVM compiler from September 2025. Elm uses optimized compiled JavaScript under Node; the generic adapter performs process I/O only. Pure-Roc big-integer and byte-regex helpers count toward application LOC and pass 228 independent checks, including the same fourteen regex cases on both search implementations. Odin additionally measures the ordinary PCRE2 JIT interface; the optimized Roc engine derives its first-byte table from the general parsed regex AST. Elm additionally measures exact base-sixteen packed permutations/counters for up to thirteen elements, with a complete counted list/array fallback for larger inputs and nine separate smaller-input comparisons. Odin additionally measures ordinary map_entry value pointers and seven native workers, each building a complete growable map for its width. Elm also measures its ordinary core Dict with full thirty-six-bit numeric keys against the original generic hash-map implementation. Actual compiled Elm tree ordering, optimized Roc allocation/lifetime IR, native arena allocation counters, and the actual timed native machine code verify distinct equal-size leaf/interior nodes and retained long-lived roots. An unmeasured early-lifetime Elm tree source and inefficient pi multiplication operand order are preserved and superseded by corrected sources. All earlier explorer sources and timing samples remain unchanged.'
