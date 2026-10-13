"""Admit the two separately guarded Mandelbrot source studies.

The original cohorts remain immutable. Only complete, independently reconciled
study records may supply the current AOT and Uiua endpoint measurements.
"""
from copy import deepcopy
import json
import math
from pathlib import Path
import statistics


MECH = 'mandelbrot/experiments/20261011-array-state'
UIUA = 'r-apl-uiua-comparison/20261011/uiua-mandel-parallel-v4'
REFERENCE = dict(bytes=32_000_015,
                 sha256='609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be')
POWER = dict(source='AC Power', low_power_mode=0)
STUDIES = {
    'mech': dict(manifest=MECH + '/full-study/manifest.json',
                 report=MECH + '/full-study/report.json',
                 digest='a8f8d3069a25ffe47ab1bec2e6922108570432baeb5b467fc17d4a44734f55a9',
                 locs={'scalar11': 11, 'five16-loop-fma': 16,
                       'five16-rows-loop-fma': 16, 'frozen20': 20, 'frozen29-audited': 29}),
    'uiua': dict(manifest=UIUA + '/full-study/manifest.json',
                 report=UIUA + '/results/full-study.json',
                 digest='d286d36a47de8eda58a8999c6d4c00c498308eda09ad8de89a7ea616b495d1a6',
                 locs={'whole11': 11, 'active14': 14, 'whole16-parallel': 16,
                       'active19-parallel': 19, 'mech29-control': 29}),
}
REQUIRED_CHECKS = {
    'sampler_exited_normally', 'authoritative_native_group_received',
    'native_exited_normally', 'no_execution_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
    'no_monitor_finish_error', 'no_execute_cleanup_error',
    'no_remaining_execute_group', 'no_supervisor_cleanup', 'valid_environment',
    'complete_output_reference_match', 'pins_unchanged',
}
EMPTY_FIELDS = ('remaining_group_members', 'competing_compilers', 'power_changes',
                'monitor_errors', 'resource_monitor_errors', 'supervisor_remaining_members')
ERROR_FIELDS = ('error', 'resource_violation', 'cleanup_error', 'monitor_finish_error',
                'supervisor_cleanup', 'sampler_failure')


def _support_pins(value):
    """Reconstruct the frozen harness fingerprint without importing its runner."""
    result = {}
    if isinstance(value, dict):
        if value.get('path') and isinstance(value.get('sha256'), str):
            result[value['path']] = value['sha256']
        for name, child in value.items():
            if name in ('pins', 'libraries', 'pin_files', 'support_pins') and isinstance(child, dict):
                for filename, digest in child.items():
                    if isinstance(digest, str) and len(digest) == 64:
                        result[str(Path(filename).absolute())] = digest
                        result[str(Path(filename).resolve())] = digest
            if isinstance(child, (dict, list)):
                result.update(_support_pins(child))
    elif isinstance(value, list):
        for child in value:
            result.update(_support_pins(child))
    return result


def _fingerprint(row):
    return dict(source_sha256=row['source_sha256'], binary_sha256=row.get('binary_sha256'),
                command=[part.replace(row['source'], '{source}') for part in row['command']],
                cwd=row['cwd'], environment=row.get('runtime_environment', {}),
                support=_support_pins(row))


def validate_study(manifest, report, digest, locs):
    """Return reconciled rows; completion/qualification flags alone are insufficient."""
    assert report['status'] == 'complete' and report.get('active') is None
    assert report['pins_unchanged_at_end'] is True
    assert report['manifest_sha256'] == digest
    assert manifest['samples'] == 3 and manifest['size'] == 16000
    assert manifest['required_power_profile'] == POWER
    assert manifest['full_output_reference'] == REFERENCE
    assert manifest.get('workers', manifest.get('maximum_compute_workers')) == 8
    rows = {row['id']: row for row in manifest['profiles']}
    assert len(rows) == len(manifest['profiles']) == 5 and set(rows) == set(locs)
    assert set(report['profiles']) == set(rows)
    assert report['orders'] == manifest['orders']
    expected_order = []
    for index, ordered in enumerate(manifest['orders'], 1):
        assert ordered['round'] == index
        assert len(ordered['profiles']) == 5 and set(ordered['profiles']) == set(rows)
        expected_order.extend((index, identity) for identity in ordered['profiles'])
    assert len(expected_order) == len(report['attempts']) == 15
    attempts = {identity: [] for identity in rows}
    for number, (attempt, (round_index, identity)) in enumerate(zip(report['attempts'], expected_order), 1):
        row = rows[identity]
        assert row['loc'] == locs[identity]
        assert row['benchmark'] == 'mandelbrot'
        if row['language'] == 'mech':
            assert row['command'][1:] == ['{n}', '8']
        else:
            assert row['language'] == 'uiua' and row['maximum_workers'] in (1, 8)
            assert row['maximum_workers'] == (8 if identity.endswith('parallel') else 1)
        assert attempt['attempt'] == number and attempt['round'] == round_index
        for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd'):
            assert attempt[field] == row[field], (identity, field)
        command = [part.replace('{n}', '16000') for part in row['command']]
        assert attempt['command'] == command
        assert attempt['runtime_environment'] == row.get('runtime_environment', {})
        assert json.loads(attempt['execution_fingerprint']) == _fingerprint(row)
        assert attempt['qualified'] is True
        assert attempt['sampler_wait_status'] == attempt['exit_status'] == 0
        assert attempt['maximum_rss_mib'] == manifest['maximum_rss_mib']
        assert type(attempt['peak_process_group_rss_kib']) is int
        assert 0 <= attempt['peak_process_group_rss_kib'] <= manifest['maximum_rss_mib'] * 1024
        assert attempt['timeout_seconds'] == manifest['timeout_seconds']
        assert attempt['full_workload'] is True and attempt['workload_n'] == 16000
        assert attempt['valid_environment'] is True and attempt['pins_unchanged_at_end'] is True
        assert all(attempt[field] == [] for field in EMPTY_FIELDS)
        assert not any(attempt.get(field) for field in ERROR_FIELDS)
        assert attempt['power_at_start'] == attempt['power_at_end'] == POWER
        assert attempt['validation'] == dict(REFERENCE, reference_match=True)
        assert attempt['stdout_bytes'] == REFERENCE['bytes']
        assert attempt['stdout_sha256'] == REFERENCE['sha256']
        native = attempt['supervised_native_group']
        assert type(native['pid']) is int and native['pid'] > 0
        assert native['pid'] == native['pgid'] and native['command'] == command
        if 'qualification_checks' in attempt:
            assert set(attempt['qualification_checks']) == REQUIRED_CHECKS
            assert all(value is True for value in attempt['qualification_checks'].values())
        else:
            assert attempt['fatal_qualification_fields'] == []
            assert native['inherited_common_mutex'] is True
        seconds = attempt['seconds']
        assert type(seconds) in (int, float) and math.isfinite(seconds) and seconds > 0
        attempts[identity].append(attempt)
    result = {}
    for identity, row in rows.items():
        recorded = report['profiles'][identity]
        samples = [attempt['seconds'] for attempt in attempts[identity]]
        assert recorded['loc'] == row['loc'] and recorded['samples_seconds'] == samples
        assert len(samples) == 3
        assert recorded['median_seconds'] == statistics.median(samples)
        assert recorded['sample_stdev_seconds'] == statistics.stdev(samples)
        result[identity] = dict(deepcopy(row), samples_seconds=samples,
                               median_seconds=statistics.median(samples),
                               sample_stdev_seconds=statistics.stdev(samples),
                               sample_validation=[deepcopy(a['validation']) for a in attempts[identity]],
                               sample_execution_metadata=deepcopy(attempts[identity]))
    return result


def _preserved_endpoints(data, group):
    previous = [deepcopy(data['programs'][key]) for key in group['programs']]
    files = {digest: deepcopy(data['files'][digest]) for row in previous for digest in row['files']}
    return dict(programs=previous, files=files)


def add_mandelbrot_studies(root, data, load, measured, add_file, sha):
    """Add reviewed current endpoints while archiving the preceding source/sample bytes."""
    root = Path(root)
    if not all((root / spec['report']).is_file() for spec in STUDIES.values()):
        return
    # A running study contributes no partial endpoint. Load directly until both
    # are complete so live reports never become packaging provenance.
    reports = {name: json.loads((root / spec['report']).read_text()) for name, spec in STUDIES.items()}
    if any(report['status'] != 'complete' for report in reports.values()):
        return
    checked, studies = {}, {}
    for name, spec in STUDIES.items():
        assert sha(root / spec['manifest']) == spec['digest'], 'Reviewed manifest changed'
        manifest, report = load(spec['manifest']), load(spec['report'])
        assert Path(report['manifest']).resolve() == (root / spec['manifest']).resolve()
        rows = validate_study(manifest, report, spec['digest'], spec['locs'])
        for row in rows.values():
            for path, digest in ((row['source'], row['source_sha256']),
                                 (row['command'][0], row.get('binary_sha256') or row['runtime_sha256'])):
                assert manifest['pins'][path] == digest and sha(path) == digest, ('Artifact pin differs', path)
        checked[name] = rows
        studies[name] = dict(manifest=spec['manifest'], manifest_sha256=spec['digest'],
                             report=spec['report'], report_sha256=sha(root / spec['report']),
                             timing_boundary=manifest['timing_boundary'],
                             profiles=rows, required_power=POWER, full_output_reference=REFERENCE,
                             qualification='All fifteen fresh full processes independently reconciled; no partial scores.')
    benchmark = next(row for row in data['benchmarks'] if row['key'] == 'mandelbrot')
    groups = {group['language']: group for group in benchmark['groups']}
    previous = {language: _preserved_endpoints(data, groups[language]) for language in ('mech', 'uiua')}
    selected = {}
    candidates = {'mech': [checked['mech'][key] for key in ('scalar11', 'five16-loop-fma', 'five16-rows-loop-fma')],
                  'uiua': [checked['uiua'][key] for key in ('whole11', 'active14', 'whole16-parallel', 'active19-parallel')]}
    control = checked['uiua']['mech29-control']
    for language, rows in candidates.items():
        fast = min(rows, key=lambda row: (row['median_seconds'], row['loc'], row['key']))
        small = min(rows, key=lambda row: (row['loc'], row['median_seconds'], row['key']))
        selected[language] = dict(fast=fast['key'], small=small['key'])
        spec = STUDIES[language]
        for row in {entry['key']: entry for entry in (fast, small)}.values():
            extra = dict(physical_loc=row.get('physical_loc', row['loc']), loc_expansions=row.get('loc_expansions', []),
                         loc_metric='statement-normalized', full_output_validation=row['sample_validation'],
                         sample_execution_metadata=row['sample_execution_metadata'],
                         runtime_arguments=row['command'], runtime_environment=row.get('runtime_environment', {}),
                         execution_fingerprint=json.dumps(_fingerprint(row), sort_keys=True),
                         support_pins=_support_pins(row),
                         study_manifest_sha256=spec['digest'], study_report_sha256=studies[language]['report_sha256'],
                         timing_boundary=studies[language]['timing_boundary'],
                         timing_summary=studies[language]['timing_boundary'], required_work_validated=True,
                         source_provenance='Counted application source; general arrays and authored recurrences, complete N16000 PBM, at most eight compute workers.',
                         algorithm=dict(strict=True, status='reviewed', native_benchmark_kernel=False,
                                        full_work=dict(width=16000, height=16000, maximum_iterations=50),
                                        evidence=[dict(path=spec['manifest'], sha256=spec['digest'])]))
            if language == 'mech':
                extra.update(execution_mode='aot', runtime_policy='AOT', binary_sha256=row['binary_sha256'],
                             native_object_sha256=row['native_object_sha256'],
                             generation_command=row['emission']['command'],
                             native_flags=[part for part in row['emission']['command'] if part.startswith('--')])
                extra['algorithm']['reason'] = (
                    'The Mech application authors ordinary complex recurrences and escape predicates in numeric arrays. '
                    'General grid mapping, native lowering and PBM packing execute the complete image; no native workload kernel.')
            else:
                extra.update(execution_mode='interpreted', runtime_policy='Uiua 0.19.1 · official native ARM64 interpreter',
                             runtime=row['runtime'], runtime_sha256=row['runtime_sha256'],
                             maximum_workers=row['maximum_workers'],
                             mech_comparison=dict(scope='paired-control', seconds=control['median_seconds'],
                                 control_key=control['key'], control_loc=control['loc'],
                                 source=control['source'], source_sha256=control['source_sha256'],
                                 samples_seconds=control['samples_seconds'], report=STUDIES['uiua']['report'],
                                 source_audit=STUDIES['uiua']['manifest'],
                                 sample_execution_metadata=control['sample_execution_metadata'],
                                 time_x_mech=row['median_seconds'] / control['median_seconds'],
                                 control_loc_x_mech=row['loc'] / control['loc']))
                extra['algorithm']['reason'] = (
                    'The Uiua application authors row recurrences, persistent escape masks and standard array packing. '
                    + ('Ordinary Spawn/Wait partitions complete rows among at most eight compute workers; '
                       if row['maximum_workers'] == 8 else 'Complete rows execute sequentially; ')
                    + 'no native workload kernel.')
            measured(row['key'], 'mandelbrot', language, row['loc'], row['samples_seconds'],
                     row['source'], row['source_sha256'], spec['report'], spec['manifest'], **extra)
        groups[language]['programs'] = list(dict.fromkeys([fast['key'], small['key']]))
    # Preserve exact control source bytes independently of the endpoint file
    # pruning performed later by augment_data. They never become extra options.
    control_files = {}
    for rows in checked.values():
        for row in rows.values():
            for digest in add_file(row['source'], row['source_sha256']):
                control_files[digest] = deepcopy(data['files'][digest])
    data['mandelbrot_study_audit'] = dict(status='complete', studies=studies, selected=selected,
        previous_endpoints=previous, study_sources=control_files,
        admission_rule='Exactly three full qualified rounds for every profile in both complete guarded studies.',
        comparison_policy='Uiua time and LOC ratios name the unchanged paired 29-LOC Mech AOT control; its samples are never relabelled as the new 16-LOC source.')
    data['provenance']['mandelbrot_study_data.py'] = sha(Path(__file__))
    data['protocol']['mandelbrot_studies'] = (
        'Separate guarded three-round N16000 source studies retain old controls and measurements. '
        'Mech AOT selects the fewest-LOC and fastest qualified canonical array-state sources. '
        'Uiua selects the sequential 11-LOC compact source and ordinary Spawn/Wait 16-LOC parallel source. '
        'Complete PBM, startup and shutdown are timed. The paired Uiua Mech control remains its measured 29-LOC source. '
        'Other workloads, Mech JIT and interpreted source measurements retain their existing cohorts.')
