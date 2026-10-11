"""Apply current measured Mech revisions and audited BQN/K source comparisons."""
import hashlib
import json
import math
from pathlib import Path
import statistics
from functional_data import add_functional_languages
from matlab_data import add_matlab_language
from erlang_data import add_erlang_language
from lisp_racket_data import add_lisp_racket_languages
from zig_odin_roc_elm_data import add_zig_odin_roc_elm_languages
from remaining_languages_data import add_remaining_languages
from nbody_compiler_data import add_nbody_compiler_audit
from d_data import add_d_language

APPLICATION = 'function-application-audit/20261009T171437Z/application-v3'
ARRAY_TIMINGS = 'array-comparison/nbody-20261009/comparison-mech-compact.json'
WORK_AUDIT = 'work-audit/20261009'
EXTRA_LANGUAGES = 'additional-languages/20261009-full-suite'
FASTA_FLOATING = 'fasta/investigations/20261009-floating-cdf'
SWIFT = 'swift-comparison/20261010'
FUTHARK_ALLOCATION = 'futhark-allocation/20261010'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def enrich_payload(root, data, array_audit_dir):
    programs, files = data['programs'], data['files']
    updates = []

    def load(relative):
        path = root / relative
        data['provenance'][relative] = sha(path)
        return json.loads(path.read_text())

    def add_file(path, expected, public_path=None):
        path = Path(path)
        digest = sha(path)
        assert digest == expected, (path, digest, expected)
        if digest not in files:
            files[digest] = dict(name=path.name, path=public_path or str(path.relative_to(root)),
                                 sha256=digest, code=path.read_text())
        return [digest]

    def measured(key, benchmark, lang, loc, samples, source, source_sha, report,
                 source_report=None, **extra):
        assert len(samples) >= 3 and all(x > 0 for x in samples)
        previous = programs.get(key, {})
        programs[key] = dict(previous, key=key, id=key.split(':', 1)[1], benchmark=benchmark,
                             language=lang, loc=loc, seconds=statistics.median(samples),
                             samples=samples, timing_sd=statistics.stdev(samples),
                             files=add_file(source, source_sha), report=report,
                             source_report=source_report or report, measurement_status='measured',
                             **extra)
        updates.append(key)

    summary = load(APPLICATION + '/summary.json')
    builds = load(APPLICATION + '/builds.json')
    assert summary['status'] == 'passed_with_documented_spectral_source_cost' and builds['builds_complete']
    assert summary['final_confirmation']['migration_source_hashes_match']
    for name, digest in summary['reports'].items():
        assert sha(root / APPLICATION / name) == digest, name
    load(APPLICATION + '/timings.json')
    load(APPLICATION + '/spectral-confirmation-10.json')
    build_index = {p['key']: p for p in builds['programs']}
    for row in summary['rows']:
        if row['benchmark'] == 'nbody':
            continue
        before = build_index[row['key']]['builds']['after']
        after = row['after']
        assert before['loc'] == after['loc']
        measured(row['key'], row['benchmark'], 'mech', after['loc'], after['samples_seconds'],
                 before['source'], before['source_sha256'], APPLICATION + '/' + row['timing_report'],
                 APPLICATION + '/builds.json')
        if row['benchmark'] == 'binarytrees':
            programs[row['key']]['algorithm'] = dict(reason='Current Mech match/comprehension source with general planned arenas.')

    corrections = load(WORK_AUDIT + '/timings.json')
    corrected_builds = load(WORK_AUDIT + '/builds.json')
    work_audit = load(WORK_AUDIT + '/audit.json')
    assert corrections['status'] == corrected_builds['status'] == 'passed'
    assert corrections['builds_sha256'] == sha(root / WORK_AUDIT / 'builds.json')
    assert work_audit['status'] == 'passed-with-corrections'
    for key, row in corrections['programs'].items():
        if row['variant'] not in ('floating', 'four-functions'):
            continue
        built = corrected_builds['programs'][key]
        assert row['source_sha256'] == built['source_sha256']
        assert row['binary_sha256'] == built['binary_sha256']
        assert row['full_validation']['reference_match']
        current_key = key.rsplit(':', 1)[0]
        measured(current_key, row['benchmark'], 'mech', row['loc'], row['samples_seconds'],
                 row['source'], row['source_sha256'], WORK_AUDIT + '/timings.json',
                 WORK_AUDIT + '/builds.json', algorithm=work_audit['benchmarks'][row['benchmark']],
                 runtime_policy='AOT', eligible_for_chart=True)
    for program in programs.values():
        if program['language'] == 'mech':
            program['algorithm'] = work_audit['benchmarks'][program['benchmark']]

    report = load(ARRAY_TIMINGS)
    assert report['status'] == 'complete' and report['full_steps'] == 50_000_000
    specs = [('mech', 'nbody:compact-baseline', 'mech'),
             ('mech-optimized', 'nbody:pairwise-mass-tails', 'mech'),
             ('bqn', 'nbody:array-bqn-compact', 'bqn'),
             ('bqn-optimized', 'nbody:array-bqn-optimized', 'bqn'),
             ('k', 'nbody:array-k-growler', 'k')]
    for record_key, key, lang in specs:
        row = report['programs'][record_key]
        measured(key, 'nbody', lang, row['normalized_loc'], [p['seconds'] for p in row['samples']],
                 row['source'], row['source_sha256'], ARRAY_TIMINGS,
                 source_provenance=row['source_provenance'], loc_metric='assignment-normalized',
                 physical_loc=row['normalized_loc'], runtime_policy={'mech':'AOT', 'bqn':'CBQN', 'k':'Growler'}[lang])
        assert programs[key]['seconds'] == row['median_seconds']

    data['languages'] += [dict(key='bqn', label='BQN (CBQN)', color='#2B7067'),
                          dict(key='k', label='K (Growler)', color='#8B668B')]
    nbody = next(b for b in data['benchmarks'] if b['key'] == 'nbody')
    nbody['groups'] += [dict(language='bqn', label='BQN (CBQN)',
                            programs=['nbody:array-bqn-compact', 'nbody:array-bqn-optimized']),
                       dict(language='k', label='K (Growler)', programs=['nbody:array-k-growler'])]

    audit_path = array_audit_dir / 'normalized-loc-audit.json'
    audit = json.loads(audit_path.read_text())
    audit_name = 'array-language-loc-audit'
    data['provenance'][audit_name] = sha(audit_path)
    source_only = []
    source_work_audit = load('array-comparison/all-20261009/source-audit.json')
    work_by_sha = {row['sha256']: row.get('algorithm', {}) for row in source_work_audit['rows']}
    for row in audit['rows']:
        if row['benchmark'] == 'nbody' or row['language'] not in ('bqn', 'growler.k'):
            continue
        lang = 'k' if row['language'] == 'growler.k' else 'bqn'
        key = row['benchmark'] + ':array-' + lang + '-compact'
        algorithm = work_by_sha.get(row['sha256'], {})
        if algorithm.get('strict') is not True:
            data['exclusions'].append(dict(key=key, benchmark=row['benchmark'], language=lang,
                reason=algorithm.get('reason', 'Required-work audit pending.')))
            continue
        path = array_audit_dir / row['file']
        lines = [s for s in path.read_text().splitlines() if s.strip() and
                 not s.lstrip().startswith(('#',) if lang == 'bqn' else ('/', '#!'))]
        assert len(lines) == row['physical_loc']
        assert row['normalized_loc'] == len(lines) + sum(e['extra_lines'] for e in row['expansions'])
        programs[key] = dict(key=key, id=key.split(':', 1)[1], benchmark=row['benchmark'], language=lang,
                             loc=row['normalized_loc'], physical_loc=row['physical_loc'],
                             loc_metric='assignment-normalized', loc_expansions=row['expansions'],
                             seconds=None, timing_sd=None, samples=[], measurement_status='source-only',
                             files=add_file(path, row['sha256'], 'array-language-sources/' + path.name),
                             source_url=row.get('upstream'), report=audit_name,
                             source_report=audit_name, source_provenance='Published array-language source; LOC audited; workload execution pending.')
        b = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        label = next(l['label'] for l in data['languages'] if l['key'] == lang)
        b['groups'].append(dict(language=lang, label=label, programs=[key]))
        source_only.append(key)

    # Full-size, required-work-audited BQN and K ports. Historical shortcuts
    # stay excluded; each current program has complete output checks and >=3 runs.
    full_report_name = 'array-comparison/all-20261009/results/faithful-timings.json'
    full_report = load(full_report_name)
    assert full_report['status'] == 'complete'
    assert full_report['source_audit_sha256'] == sha(root / 'array-comparison/all-20261009/source-audit.json')
    assert len(full_report['completed_benchmarks']) == 9
    for row in full_report['programs'].values():
        if row['language'] not in ('bqn', 'k'):
            continue
        assert row['algorithm']['strict'] and row['eligible_for_chart']
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert all(v['reference_match'] for v in row['sample_validation'])
        key = row['key']
        measured(key, row['benchmark'], row['language'], row['loc'], row['samples_seconds'],
                 row['source'], row['source_sha256'], full_report_name,
                 'array-comparison/all-20261009/source-audit.json', algorithm=row['algorithm'],
                 physical_loc=row['physical_loc'], loc_expansions=row['loc_expansions'],
                 runtime_sha256=row['runtime_sha256'], loc_metric='assignment-normalized',
                 source_provenance='Required-work-audited language port; full official workload; benchmark computation is in BQN or K.',
                 runtime_policy='CBQN' if row['language'] == 'bqn' else 'Growler')
        if row['benchmark'] in ('knucleotide', 'binarytrees', 'fannkuchredux', 'regexredux', 'pidigits'):
            runtime_files = ['stdlib/standard.' + ('bqn' if row['language'] == 'bqn' else 'k'), 'stdlib/native.c']
            for relative in runtime_files:
                path = root / 'array-comparison/all-20261009' / relative
                expected = full_report['library_pins'][relative]
                ids = add_file(path, expected)
                files[ids[0]]['role'] = 'runtime-support'
                files[ids[0]]['loc_note'] = 'General runtime/library support; excluded from application LOC. Benchmark algorithms are in the main source.'
                programs[key]['files'] += ids
            programs[key]['runtime_support'] = 'GMP arithmetic, PCRE2 regex, khash one-key get/set, and a generic process map executing language callbacks; used as needed. No native benchmark executors.'
        b = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        label = next(l['label'] for l in data['languages'] if l['key'] == row['language'])
        b['groups'].append(dict(language=row['language'], label=label, programs=[key]))

    # Native CPU ports: application algorithms stay in Mojo or Futhark.
    extra_audit = load(EXTRA_LANGUAGES + '/source-audit.json')
    extra_report_name = EXTRA_LANGUAGES + '/results/timings.json'
    extra_report = load(extra_report_name)
    assert extra_report['status'] == 'complete'
    assert extra_report['source_audit_sha256'] == sha(root / EXTRA_LANGUAGES / 'source-audit.json')
    assert set(extra_report['completed_benchmarks']) == {b['key'] for b in data['benchmarks']}
    assert extra_audit['maximum_workers'] == 8
    extra_profiles = load(EXTRA_LANGUAGES + '/profiles.json')
    extra_tests = load(EXTRA_LANGUAGES + '/results/stdlib-tests.json')
    extra_tree_audit = load(EXTRA_LANGUAGES + '/results/tree-work-audit.json')
    extra_summary = load(EXTRA_LANGUAGES + '/results/summary.json')
    assert extra_tests['status'] == extra_tree_audit['status'] == 'passed'
    assert sum(len(pairs) for pairs in extra_profiles['pairs'].values()) == 20
    assert extra_summary['status'] == 'passed-with-documented-allocation-gap'
    assert extra_summary['distinct_compact_performance_pairs'] == 19
    assert extra_summary['candidate_sources'] == 50
    assert extra_summary['timing_report_sha256'] == sha(root / extra_report_name)
    assert extra_summary['source_audit_sha256'] == sha(root / EXTRA_LANGUAGES / 'source-audit.json')
    assert len(extra_summary['coverage']['mojo']) == 10 and len(extra_summary['coverage']['futhark']) == 9
    assert extra_audit['unsupported'] == extra_report['unsupported'] == extra_summary['unsupported']
    allocation = load(FUTHARK_ALLOCATION + '/results/capabilities.json')
    assert allocation['status'] == 'unsupported-allocation-model'
    assert {r['backend'] for r in allocation['constructor_probes']} == {'c', 'multicore'}
    assert sha(allocation['compiler']['path']) == allocation['compiler']['sha256']
    for relative, expected in allocation['sources'].items():
        assert sha(root / FUTHARK_ALLOCATION / relative) == expected
    for row in allocation['constructor_probes']:
        assert sha(root / FUTHARK_ALLOCATION / row['evidence']) == row['evidence_sha256']
    for row in allocation['rejected_type_and_intrinsic_probes']:
        assert row['returncode'] == 2
        assert sha(root / FUTHARK_ALLOCATION / row['log']) == row['log_sha256']
    data['futhark_allocation_audit'] = allocation
    assert extra_tree_audit['futhark_binarytrees_admitted'] is False
    for file, expected in extra_report['review_pins'].items():
        assert sha(root / EXTRA_LANGUAGES / file) == expected
    load(EXTRA_LANGUAGES + '/provenance.json')
    load(EXTRA_LANGUAGES + '/results/builds.json')
    for relative, expected in extra_report['library_pins'].items():
        assert sha(root / EXTRA_LANGUAGES / relative) == expected
    assert extra_report['external_library_pins'] == extra_audit['external_library_pins']
    for path, expected in extra_report['external_library_pins'].items():
        assert sha(path) == expected
    for gap in extra_audit['unsupported']:
        data['exclusions'].append(dict(key=gap['benchmark'] + ':' + gap['language'] + '-allocation-gap', **gap))
    data['languages'] += [dict(key='mojo', label='Mojo', color='#ff4c1f'),
                          dict(key='futhark', label='Futhark', color='#5f021f',
                               coverage_note='Binary trees is excluded: the array representation omits required per-node allocations.',
                               unsupported_benchmarks=['binarytrees'])]
    for row in extra_report['programs'].values():
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds']) >= 3
        assert len(row['sample_validation']) == len(row['samples_seconds'])
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary']) == row['binary_sha256']
        if row['language'] == 'mech':
            key = row['chart_key']
            previous = programs[key]
            assert row['source_sha256'] == files[previous['files'][0]]['sha256']
            measured(key, row['benchmark'], 'mech', previous['loc'], row['samples_seconds'],
                     row['source'], row['source_sha256'], extra_report_name,
                     previous['source_report'], runtime_policy='AOT',
                     binary_sha256=row['binary_sha256'],
                     runtime_environment=row.get('runtime_environment', {}))
            continue
        assert row['language'] in ('mojo', 'futhark')
        assert row['algorithm']['strict'] and row['eligible_for_chart']
        assert row['algorithm']['native_benchmark_kernel'] is False
        key = row['key']
        runtime = extra_audit['runtimes'][row['language']]
        measured(key, row['benchmark'], row['language'], row['loc'], row['samples_seconds'],
                 row['source'], row['source_sha256'], extra_report_name,
                 EXTRA_LANGUAGES + '/source-audit.json', algorithm=row['algorithm'],
                 physical_loc=row['physical_loc'], loc_expansions=row['loc_expansions'],
                 loc_metric='assignment-normalized', binary_sha256=row['binary_sha256'],
                 source_provenance=row['source_provenance'],
                 runtime_policy=('AOT CPU · Mojo 1.1.0 · MAX 26.6.0' if row['language'] == 'mojo'
                                 else 'AOT multicore CPU · Futhark 0.27.1'),
                 compiler_sha256=runtime['sha256'], required_work_validated=True,
                 candidate_role=row['candidate_role'])
        for relative in row['stdlib_files']:
            path = root / EXTRA_LANGUAGES / relative
            ids = add_file(path, extra_report['library_pins'][relative])
            files[ids[0]]['role'] = 'runtime-support'
            files[ids[0]]['loc_note'] = 'General library or IO support; excluded from application LOC. All benchmark algorithms are in the main language source.'
            programs[key]['files'] += ids
        programs[key]['runtime_support'] = ('General IO/formatting, native containers, and GMP, PCRE2 or Apache APR pool bindings where used.'
            if row['language'] == 'mojo' else 'General Futhark IO formatting, unbounded signed integers, growing hash maps and byte regex; generic C adapter only handles arguments, stdin, context creation and stdout.')
        if row['language'] == 'mojo' and row['benchmark'] in ('nbody', 'spectralnorm'):
            programs[key]['source_url'] = ('https://github.com/mikeleppane/mojo-rust-python-perf'
                if row['benchmark'] == 'nbody' else 'https://github.com/cemrehancavdar/faster-python-bench')
        benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in benchmark['groups'] if g['language'] == row['language']), None)
        if group is None:
            group = dict(language=row['language'], label='Mojo' if row['language'] == 'mojo' else 'Futhark', programs=[])
            benchmark['groups'].append(group)
        group['programs'].append(key)

    # The approved local Rust #7 adaptation keeps the pipeline and restores
    # floating division/CDF comparisons. Preserve its complete source and use
    # the unchanged Mech binary's matching five-round control measurements.
    fasta_report_name = FASTA_FLOATING + '/results.json'
    fasta_report = load(fasta_report_name)
    assert fasta_report['status'] == 'passed' and fasta_report['workload'] == 25_000_000
    assert fasta_report['sample_count'] == 5
    assert fasta_report['required_power'] == dict(source='AC Power', low_power_mode=0)
    fasta = next(b for b in data['benchmarks'] if b['key'] == 'fasta')
    rust = fasta_report['programs']['rust7-floating-16']
    source = Path(rust['source'])
    assert sha(source) == rust['source_sha256'] and sha(rust['command'][0]) == rust['binary_sha256']
    assert rust['correctness_1000']['reference_match']
    assert len(rust['sample_validation']) == len(rust['samples_seconds']) == 5
    assert all(v['reference_match'] for v in rust['sample_validation'])
    code = source.read_text()
    assert '/*' not in code  # This source uses line comments only.
    loc = sum(bool(line.strip()) and not line.lstrip().startswith('//') for line in code.splitlines())
    key = 'fasta:rust7-floating-16-local'
    measured(key, 'fasta', 'rust', loc, rust['samples_seconds'], source,
             rust['source_sha256'], fasta_report_name, local_adaptation=True,
             source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/fasta-rust-7.html',
             source_provenance='Local adaptation of published Rust #7: unscaled f64 cumulative probabilities and direct floating division; threaded generator and output pipeline retained. Five full-size outputs match the reference.',
             algorithm=dict(strict=True, native_benchmark_kernel=False,
                 reason='Prescribed fresh LCG state per nucleotide, floating division, unscaled floating CDF comparisons, and complete FASTA output.'),
             binary_sha256=rust['binary_sha256'], compiler_sha256=fasta_report['rustc']['sha256'],
             required_work_validated=True, runtime_policy='AOT · two-thread source pipeline',
             eligible_for_chart=True)
    next(g for g in fasta['groups'] if g['language'] == 'rust')['programs'].append(key)
    mech_group = next(g for g in fasta['groups'] if g['language'] == 'mech')
    mech_key = min(mech_group['programs'], key=lambda k:programs[k]['seconds'])
    previous = programs[mech_key]
    control = fasta_report['programs']['mech-current-floating']
    assert previous['binary_sha256'] == control['binary_sha256']
    assert len(control['sample_validation']) == len(control['samples_seconds']) == 5
    assert all(v['reference_match'] for v in control['sample_validation'])
    source_file = files[previous['files'][0]]
    measured(mech_key, 'fasta', 'mech', previous['loc'], control['samples_seconds'],
             root / source_file['path'], source_file['sha256'], fasta_report_name,
             previous['source_report'], binary_sha256=control['binary_sha256'])

    # Every execution path uses the same complete Mech source candidates.
    modes_name = 'execution-modes/20261010/timings.json'
    modes = load(modes_name)
    mode_builds = load('execution-modes/20261010/builds.json')
    conformance = load('execution-modes/20261010/conformance.json')
    assert modes['status'] == mode_builds['status'] == 'complete'
    assert modes['sample_count'] == 3 and len(modes['completed_benchmarks']) == 10
    assert modes['power_profile'] == dict(source='AC Power', low_power_mode=0)
    assert modes['builds_sha256'] == sha(root / 'execution-modes/20261010/builds.json')
    assert len(conformance) == 40 and all(row['correct'] for row in conformance)
    assert all(row['identical_native_object'] for row in mode_builds['programs'].values())
    initial_modes_name = modes_name
    initial_aot_regressions = modes['aot_regressions']
    modes_name = 'execution-modes/20261010-final/timings.json'
    modes = load(modes_name)
    mode_objects = load('execution-modes/20261010-final/objects.json')
    conformance = load('execution-modes/20261010-final/conformance.json')
    runtime_control = load('execution-modes/20261010-final/aot-runtime-control.json')
    assert modes['status'] == mode_objects['status'] == 'complete'
    assert modes['sample_count'] == 3 and len(modes['completed_benchmarks']) == 10
    assert modes['power_profile'] == dict(source='AC Power', low_power_mode=0)
    assert modes['baseline_timing_sha256'] == sha(root / initial_modes_name)
    assert modes['builds_sha256'] == sha(root / 'execution-modes/20261010/builds.json')
    assert modes['optimization_objects_sha256'] == sha(root / 'execution-modes/20261010-final/objects.json')
    assert modes['aot_runtime_control_sha256'] == sha(root / 'execution-modes/20261010-final/aot-runtime-control.json')
    assert runtime_control['identical_native_object']
    assert sha(runtime_control['binary']) == runtime_control['binary_sha256']
    assert modes['runner_sha256'] == mode_objects['runner_sha256']
    assert len(conformance) == 54 and all(row['correct'] for row in conformance)
    assert len(mode_objects['programs']) == 15 and all(row['identical'] for row in mode_objects['programs'].values())
    mech_language = next(l for l in data['languages'] if l['key'] == 'mech')
    mech_language.update(family='mech', execution='AOT', line_style='solid')
    mech_index = data['languages'].index(mech_language)
    data['languages'][mech_index+1:mech_index+1] = [
        dict(key='mech-jit', label='Mech (JIT)', color=mech_language['color'], family='mech', execution='JIT', line_style='dashed'),
        dict(key='mech-resident', label='Mech (interpreted)', color=mech_language['color'], family='mech', execution='Interpreted', line_style='dotted')]
    for row in modes['programs'].values():
        if row['mode'] not in ('jit', 'resident'):
            continue
        assert row['required_work_validated'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary']) == row['binary_sha256']
        original = programs[row['source_key']]
        assert files[original['files'][0]]['sha256'] == row['source_sha256']
        label = next(l['label'] for l in data['languages'] if l['key'] == row['language'])
        measured(row['key'], row['benchmark'], row['language'], row['loc'], row['samples_seconds'],
                 row['source'], row['source_sha256'], modes_name,
                 source_report='execution-modes/20261010/builds.json',
                 physical_loc=row['physical_loc'], loc_expansions=row['loc_expansions'],
                 algorithm=original['algorithm'], required_work_validated=True,
                 binary_sha256=row['binary_sha256'], compiler_sha256=modes['runner_sha256'],
                 execution_mode=row['mode'], sample_details=row['sample_details'],
                 timing_boundary=row['timing_boundary'],
                 timing_summary=('Fresh process: startup, JIT compilation, execution and full I/O.' if row['mode']=='jit'
                    else 'Loaded interpretation: loading and activation excluded; input, execution, output and flush included.'),
                 runtime_policy=('JIT · same shared lowering and optimization passes as AOT' if row['mode']=='jit'
                    else 'Interpreted · loaded typed instruction graphs; general standard-library operations'),
                 source_provenance='Same audited Mech source candidates as AOT; full standard workloads and independently checked output. General parser reuse and batched interpreted scalar instructions; no coefficient or result caching.')
        b = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in b['groups'] if g['language'] == row['language']), None)
        if group is None:
            group = dict(language=row['language'], label=label, programs=[])
            b['groups'].append(group)
        group['programs'].append(row['key'])
    data['protocol']['mech_execution'] = modes['protocol']
    data['execution_mode_audit'] = dict(native_objects_identical=15, semantic_checks=54,
        timing_report=modes_name, previous_timing_report=initial_modes_name,
        aot_regressions=initial_aot_regressions,
        optimization_changes=modes['changes'], aot_runtime_control=runtime_control,
        optimization='One parsed canonical document for structural admission, released after admission; reuse admitted control IR in JIT emission. Resident scalar instructions execute in independent tiles after checked-arithmetic domain proofs, preserving reduction order and tail bounds. The standard counting container keeps batch table dispatch inlined with its caller, retaining every update.')

    # Swift: every linked published source considered, with explicit source
    # corrections where required work or modern platform APIs differ.
    swift_audit = load(SWIFT + '/source-audit.json')
    swift_builds = load(SWIFT + '/results/builds.json')
    swift_report_name = SWIFT + '/results/timings.json'
    swift_report = load(swift_report_name)
    swift_summary = load(SWIFT + '/results/summary.json')
    assert swift_audit['status'] == 'reviewed'
    assert swift_report['status'] == 'complete' and swift_summary['status'] == 'passed'
    assert swift_report['sample_count'] == 3
    assert set(swift_report['completed_benchmarks']) == {b['key'] for b in data['benchmarks']}
    assert swift_summary['timing_report_sha256'] == sha(root / swift_report_name)
    assert swift_summary['source_audit_sha256'] == sha(root / SWIFT / 'source-audit.json')
    assert swift_audit['builds_sha256'] == sha(root / SWIFT / 'results/builds.json')
    assert swift_report['power'] == dict(source='AC Power', low_power_mode=0)
    for relative, expected in swift_report['source_pins'].items():
        assert sha(root / SWIFT / relative) == expected
    load(SWIFT + '/manifest.json')
    load(SWIFT + '/corrections.json')
    swift_work = {r['key']:r for r in swift_audit['rows']}
    data['languages'].append(dict(key='swift', label='Swift', color='#F05138', execution='AOT', line_style='solid'))
    for row in swift_report['programs'].values():
        if row['language'] != 'swift' or not row['eligible_for_chart']:
            continue
        assert swift_work[row['key']]['admitted'] and row['algorithm']['strict']
        assert row['algorithm']['native_benchmark_kernel'] is False
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary']) == row['binary_sha256']
        assert row['loc'] == row['source_metrics']['nonblank_noncomment_lines']
        key = row['key']
        adapted = row.get('local_adaptation', False)
        measured(key, row['benchmark'], 'swift', row['loc'], row['samples_seconds'],
                 row['source'], row['source_sha256'], swift_report_name, SWIFT + '/source-audit.json',
                 algorithm=row['algorithm'], required_work_validated=True,
                 binary_sha256=row['binary_sha256'], compiler_sha256=swift_builds['compiler']['driver_sha256'],
                 physical_loc=row['physical_loc'], loc_expansions=[], loc_metric='statement-normalized',
                 source_url=row['source_url'], local_adaptation=adapted,
                 adaptations=row['adaptations'], upstream_id=row.get('upstream_id', row['id']),
                 source_provenance=('Locally corrected published Swift source: ' + ' '.join(row['adaptations'])
                    if adapted else 'Published Benchmarks Game Swift source; Darwin import port and packed statements expanded where needed.'),
                 runtime_policy='AOT · Apple Swift 5.10 · -Ounchecked -whole-module-optimization',
                 timing_boundary='Fresh-process elapsed time: startup, complete input, computation, complete stdout to a regular file, and teardown. Compilation and validation excluded.')
        for relative in row['support_files']:
            support = add_file(root / SWIFT / relative, swift_report['support_files'][relative])
            files[support[0]]['role'] = 'runtime-support'
            files[support[0]]['loc_note'] = 'Generic GMP header module map; arithmetic and digit-spigot control remain in Swift. Excluded from application LOC.'
            programs[key]['files'] += support
            programs[key]['runtime_support'] = 'Generic GMP arbitrary-precision arithmetic, bound through a Swift module map; the full spigot algorithm is in Swift.'
        benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
        group = next((g for g in benchmark['groups'] if g['language'] == 'swift'), None)
        if group is None:
            group = dict(language='swift', label='Swift', programs=[])
            benchmark['groups'].append(group)
        group['programs'].append(key)
    for row in swift_report['excluded']:
        data['exclusions'].append(dict(key=row['benchmark'] + ':' + row['id'], **row))
    data['swift_audit'] = dict(published_sources=37, corrected_sources=10,
        admitted_sources=swift_summary['admitted_sources'], output_exclusions=swift_summary['output_exclusions'],
        sample_count=3, compiler=swift_builds['compiler']['version'],
        timing_report=swift_report_name, source_audit=SWIFT + '/source-audit.json',
        controls=swift_summary['controls'], tree_review=swift_audit['tree_review'],
        scheduling=swift_audit['policy']['scheduling'])

    add_functional_languages(root,data,load,measured,sha)
    add_matlab_language(root,data,load,measured,add_file,sha)
    add_erlang_language(root,data,load,measured,sha)
    add_lisp_racket_languages(root,data,load,measured,sha)
    add_zig_odin_roc_elm_languages(root,data,load,measured,add_file,sha)
    add_remaining_languages(root,data,load,measured,add_file,sha)
    add_nbody_compiler_audit(root,data,load,measured,sha)
    add_d_language(root,data,load,measured,add_file,sha)

    # Retain endpoint programs only, selecting from the current revisions.
    for b in data['benchmarks']:
        for group in b['groups']:
            candidates = [programs[k] for k in group['programs']]
            timed = [p for p in candidates if p['seconds'] is not None]
            group['fast'] = min(timed, key=lambda p:(p['seconds'], p['loc']))['key'] if timed else None
            group['small'] = min(candidates, key=lambda p:(p['loc'], p['seconds'] if p['seconds'] is not None else math.inf))['key']
            group['programs'] = list(dict.fromkeys(k for k in (group['fast'], group['small']) if k))
        current = [programs[k] for g in b['groups'] for k in g['programs']]
        b['baseline_loc'] = min(p['loc'] for p in current)
        b['baseline_seconds'] = min(p['seconds'] for p in current if p['seconds'] is not None)
        for p in current:
            p['x'] = p['loc'] / b['baseline_loc']
            p['y'] = p['seconds'] / b['baseline_seconds'] if p['seconds'] is not None else None
    retained = {k for b in data['benchmarks'] for g in b['groups'] for k in g['programs']}
    data['programs'] = {k:p for k,p in programs.items() if k in retained}
    retained_files = {f for p in data['programs'].values() for f in p['files']}
    data['files'] = {k:f for k,f in files.items() if k in retained_files}

    def gm(values):
        return math.exp(sum(math.log(v) for v in values) / len(values))
    reference = []
    for language in data['languages']:
        inputs = []
        for b in data['benchmarks']:
            group = next((g for g in b['groups'] if g['language'] == language['key']), None)
            if group and group['fast'] and programs[group['small']]['seconds'] is not None:
                inputs.append(group)
        language['timed_benchmarks'] = [b['key'] for b in data['benchmarks'] if any(
            g['language'] == language['key'] and g['fast'] for g in b['groups'])]
        language['source_benchmarks'] = [b['key'] for b in data['benchmarks'] if any(
            g['language'] == language['key'] for g in b['groups'])]
        if len(inputs) == len(data['benchmarks']):
            reference.append(dict(lang=language['key'], label=language['label'],
                **{kind:{axis:gm([programs[g[kind]][axis] for g in inputs]) for axis in ('x','y')}
                   for kind in ('fast','small')}))
    data['reference_summary'] = reference
    data['schema_version'] = 3
    data['protocol']['measured'] = 'October 8–11, 2026'
    data['language_groups'] = [
        dict(key='dynamic', label='Dynamic', languages=['python','ruby','perl','php','javascript','lua','luajit','julia','bqn','k','matlab','erlang','lisp','racket','smalltalk']),
        dict(key='interpreted', label='Interpreted', languages=['python','ruby','perl','php','lua','bqn','k','mech-resident']),
        dict(key='array', label='Array', languages=['mech','mech-jit','mech-resident','julia','bqn','k','futhark','matlab']),
        dict(key='functional', label='Functional', languages=['haskell','ocaml','fsharp','fsharp-aot','futhark','mech','mech-jit','mech-resident','erlang','lisp','racket','roc','elm']),
        dict(key='aot', label='AOT', languages=['c','cpp','chapel','go','java','rust','mech','mojo','futhark','swift','haskell','ocaml','lisp','racket','zig','odin','roc','ada','pascal','fortran','csharp-aot','fsharp-aot','dart','d']),
        dict(key='jit', label='JIT', languages=['javascript','luajit','julia','mech-jit','matlab','erlang','elm','csharp','fsharp','dart-jit','smalltalk']),
    ]
    data['array_languages'] = ['bqn', 'k', 'futhark']
    data['array_source_benchmarks'] = [b['key'] for b in data['benchmarks'] if any(
        any(g['language'] == language for g in b['groups']) for language in data['array_languages'])]
    data['protocol']['metric'] = 'Nonblank, noncomment LOC; packed independent assignments/statements counted separately; semantic compiler pragmas included'
    data['protocol']['timing'] = 'Median of 3, 5, 7 or 10 samples by report. Fresh-process elapsed time includes startup and full output; Mech JIT also includes compilation. Mech interpreted reports loaded execution including input/output, with preparation excluded and full process timings retained separately.'
    data['protocol']['array_languages'] = 'BQN (CBQN) and K (Growler): all ten full workloads measured. N-body retains five samples; each of the other nine has three fresh samples and complete output checks. Previous spectral-norm and nucleotide shortcuts remain excluded.'
    data['protocol']['additional_languages'] = 'Mojo: ten admitted full CPU workloads. Futhark: nine; binary trees is excluded because flat/indexed arrays omit required per-node allocation. Twenty compact/performance pairs were attempted; nineteen pairs and fifty candidate sources are admitted, including valid baselines. At most eight workers and three fresh samples per source. Endpoints select measured winners from the admitted pool. Matching published Mojo N-body and spectral-norm sources were adapted; other Mojo programs and all Futhark ports were written locally. Algorithms remain in the named language. General libraries passed independent arithmetic, map, regex, wrapping and APR pool checks. Corrected Mojo trees allocate sixteen bytes per node using the same Apache APR library pool as the C reference; instrumentation verifies every allocation and clear. Earlier collapsed loops and array-allocation mismatches are archived and excluded. Nine unchanged benchmark cohorts retain their verified samples; corrected Mojo trees and their Mech control were freshly remeasured. Current Mech controls use original arguments and runtime settings.'
    data['protocol']['array_runtime'] = 'Benchmark algorithms are in the language source. General library bindings and process map support are pinned and inspectable separately from application LOC; no native benchmark executors.'
    data['protocol']['swift'] = 'Swift covers all ten full workloads with three shuffled fresh-process samples per admitted candidate. All 37 sources linked by the published measurement page were considered; originals are pinned, with ten separate source corrections for modern APIs, common binary64 sampling work, or required tree lifetime. Local compiler: Apple Swift 5.10 on Apple M3, using the published -Ounchecked and whole-module-optimization flags. Published Linux Swift 6.0.3 timings are not reused. Nonblank/noncomment LOC expands packed statements and includes the algorithm and lifecycle corrections. Swift Dispatch scheduling is retained on eight logical CPUs; task counts may exceed core counts. Current unchanged Mech AOT controls were remeasured alongside Swift; existing cohort samples remain unchanged.'
    data['protocol']['admission'] = 'Known work mismatches are excluded before endpoint selection. Correct output is necessary but insufficient. Mech uses the audited four-function spectral norm and floating-comparison FASTA builds.'
    data['protocol']['baseline'] = 'Fixed per-benchmark minima of the current endpoint dataset; visibility changes preserve these baselines.'
    data['protocol']['spread'] = 'Sample covariance in log2 LOC/time ratios across selected measured benchmarks, separately for fastest and fewest-LOC profiles. Inner contours use one standard deviation; optional lighter outer areas use two along both principal axes. Centers are geometric means. These areas describe benchmark variation, not timing uncertainty or confidence intervals. Covariance envelopes determine fixed upper limits whether displayed or hidden; the first labelled tick is always 1x and covariance tails below 1x are clipped without changing the statistics.'
    data['protocol']['fasta_rust'] = 'Rust fastest uses the locally adapted floating-CDF Rust #7 source with five full-size measurements. Mech FASTA uses the unchanged binary control from those same five shuffled rounds. The compact Rust endpoint remains its complete original source.'
    data['protocol']['language_groups'] = 'Groups overlap. Dynamic, Array and Functional describe language families; Interpreted, AOT and JIT describe the benchmark execution paths used here. Java uses GraalVM native AOT builds in this cohort.'
    years_path = Path(__file__).with_name('language-years.json')
    years = json.loads(years_path.read_text())
    data['provenance']['language-years.json'] = sha(years_path)
    data['protocol']['release_year'] = years['policy']
    variants = {
        'mech': ('mech', 'Mech', 'AOT', 0),
        'mech-jit': ('mech', 'Mech', 'JIT', 1),
        'mech-resident': ('mech', 'Mech', 'Interpreted', 2),
        'lua': ('lua', 'Lua', 'Interpreted', 0),
        'luajit': ('lua', 'Lua', 'LuaJIT', 1),
        'csharp-aot': ('csharp', 'C#', 'AOT', 0),
        'csharp': ('csharp', 'C#', 'JIT', 1),
        'fsharp-aot': ('fsharp', 'F#', 'AOT', 0),
        'fsharp': ('fsharp', 'F#', 'JIT', 1),
        'dart': ('dart', 'Dart', 'AOT', 0),
        'dart-jit': ('dart', 'Dart', 'JIT', 1),
    }
    for language in data['languages']:
        if language['key'] in variants:
            family, label, variant, order = variants[language['key']]
            language.update(family=family, family_label=label, variant_label=variant, variant_order=order)
        if language['key'] == 'lua':
            language.update(label='Lua (interpreted)', execution='Interpreted', line_style='solid',
                coverage_note='Nine recorded full workloads. No admitted Lua pi-digits timing is available.')
        if language['key'] == 'luajit':
            language.update(execution='JIT', line_style='dashed')
        key = language.get('family', language['key'])
        release = years['languages'][key]
        assert isinstance(release['year'], int) and release['source'].startswith('https://')
        language.update(release_year=release['year'], release_name=release['name'],
                        release_source=release['source'], release_note=release['note'])
    return dict(updated_mech_endpoints=len({k for k in updates if programs[k]['language'] == 'mech'}),
                array_timed_endpoints=sum(p['language'] in ('bqn','k') for p in data['programs'].values()), source_only_programs=source_only,
                all_benchmark_timed_languages=len(reference), array_audit_sha256=sha(audit_path),
                additional_language_endpoints=sum(p['language'] in ('mojo','futhark') for p in data['programs'].values()),
                required_work_exclusions=len(data['exclusions']),
                swift_endpoints=sum(p['language'] == 'swift' for p in data['programs'].values()),
                erlang_endpoints=sum(p['language'] == 'erlang' for p in data['programs'].values()),
                mech_execution_endpoints=sum(p['language'] in ('mech-jit','mech-resident') for p in data['programs'].values()))
