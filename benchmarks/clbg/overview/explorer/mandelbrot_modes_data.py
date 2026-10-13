"""Import freshly measured canonical Mandelbrot JIT/interpreted sources.

Old mode measurements retain their original sources and are archived separately.
Only the reviewed, complete eighteen-process study can replace chart endpoints.
"""
from copy import deepcopy
import json
import math
from pathlib import Path
import statistics

from mandelbrot_study_data import (
    EMPTY_FIELDS, ERROR_FIELDS, POWER, REFERENCE, _fingerprint,
    _preserved_endpoints, _support_pins,
)


STUDY = 'mandelbrot/experiments/20261011-array-state/source-modes-study'
MANIFEST = STUDY + '/full-study/manifest.json'
REPORT = STUDY + '/full-study/report.json'
# Independently reviewed against all 1,642 pins and the bounded qualification.
REVIEWED_MANIFEST_SHA256 = 'fbffba7b5674e889ac9e5088bbb2e2d8499df41f5d9b4c677be38d831091680f'
SOURCES = {
    11: '4e4b881e9c2368ba9ddfcbd2b8df49f78e6fd759833b26e7a10374b432de7b3a',
    16: 'baf1cc933c9d5f1741f9b20319d66a488fca177940fa5a23bafdd2917a333cde',
}
PROFILES = {
    'scalar11-jit': (11, 'jit', 'mech-jit'),
    'scalar11-resident': (11, 'resident', 'mech-resident'),
    'row16-jit': (16, 'jit', 'mech-jit'),
    'row16-resident': (16, 'resident', 'mech-resident'),
    'scalar11-aot-control': (11, 'aot-control', 'mech'),
    'row16-aot-control': (16, 'aot-control', 'mech'),
}
CHECKS = {
    'sampler_exited_normally', 'authoritative_native_group_received',
    'native_exited_normally', 'no_execution_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
    'no_monitor_finish_error', 'no_execute_cleanup_error',
    'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'valid_source_timing',
    'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash',
    'pins_unchanged',
}


def _number(value, positive=True):
    assert type(value) in (int, float) and math.isfinite(value)
    assert value > 0 if positive else value >= 0
    return value


def _timing(attempt, mode):
    """Reconcile the score with raw stderr and retained fresh-process elapsed."""
    wall = _number(attempt['wall_seconds'])
    if mode == 'aot-control':
        assert attempt['seconds'] == wall
        return
    records = [json.loads(line[7:]) for line in attempt['stderr'].splitlines()
               if line.startswith('TIMING ')]
    assert len(records) == 1
    timing = records[0]
    assert set(timing) == {'prepare_seconds', 'kernel_seconds'}
    prepare = _number(timing['prepare_seconds'], positive=False)
    kernel = _number(timing['kernel_seconds'])
    assert prepare + kernel <= wall + .001
    score = kernel if mode == 'resident' else wall
    assert attempt['prepare_seconds'] == prepare
    assert attempt['kernel_seconds'] == kernel
    assert attempt['seconds'] == score
    assert attempt['timing'] == dict(wall_seconds=wall, prepare_seconds=prepare,
                                    kernel_seconds=kernel, reported_seconds=score)


def validate_modes_study(manifest, report, digest):
    assert digest and report['manifest_sha256'] == digest
    assert report['status'] == 'complete' and report.get('active') is None
    assert report['pins_unchanged_at_end'] is True
    assert manifest['samples'] == 3 and manifest['size'] == 16000
    assert manifest['workers'] == 8 and manifest['required_power_profile'] == POWER
    assert manifest['full_output_reference'] == REFERENCE
    rows = {row['id']: row for row in manifest['profiles']}
    assert len(rows) == len(manifest['profiles']) == 6 and set(rows) == set(PROFILES)
    assert set(report['profiles']) == set(rows)
    assert report['orders'] == manifest['orders'] and len(manifest['orders']) == 3
    order = []
    for index, entry in enumerate(manifest['orders'], 1):
        assert entry['round'] == index
        assert len(entry['profiles']) == 6 and set(entry['profiles']) == set(rows)
        order.extend((index, identity) for identity in entry['profiles'])
    assert len(report['attempts']) == len(order) == 18
    grouped = {identity: [] for identity in rows}
    for identity, row in rows.items():
        loc, mode, language = PROFILES[identity]
        assert (row['loc'], row['mode'], row['language']) == (loc, mode, language)
        assert row['source_sha256'] == SOURCES[loc]
        assert row['benchmark'] == 'mandelbrot' and row['maximum_workers'] == 8
        assert row['binary_sha256'] == manifest['pins'][row['command'][0]]
        assert manifest['pins'][row['source']] == row['source_sha256']
        if mode == 'aot-control':
            assert row['command'][1:] == ['{n}', '8']
        else:
            flag = '--scalar' if loc == 11 else '--fp-contract-loop'
            assert row['command'][1:] == [row['source'], mode, '{n}', flag]
            assert row['runtime_environment']['RAYON_NUM_THREADS'] == '8'
            assert row['runtime_environment']['MECH_TIMINGS'] == '1'
    for number, (attempt, (round_index, identity)) in enumerate(zip(report['attempts'], order), 1):
        row = rows[identity]
        assert attempt['attempt'] == number and attempt['round'] == round_index
        for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd'):
            assert attempt[field] == row[field], (identity, field)
        command = [part.replace('{n}', '16000') for part in row['command']]
        assert attempt['command'] == command
        assert attempt['runtime_environment'] == row.get('runtime_environment', {})
        assert json.loads(attempt['execution_fingerprint']) == _fingerprint(row)
        assert attempt['qualified'] is True and attempt['valid_source_timing'] is True
        assert set(attempt['qualification_checks']) == CHECKS
        assert all(value is True for value in attempt['qualification_checks'].values())
        assert attempt['sampler_wait_status'] == attempt['exit_status'] == 0
        assert attempt['full_workload'] is True and attempt['workload_n'] == 16000
        assert attempt['canonical_full_workload'] is True
        assert attempt['never_chart_eligible'] is False
        assert attempt['valid_environment'] is True and attempt['pins_unchanged_at_end'] is True
        assert attempt['maximum_rss_mib'] == manifest['maximum_rss_mib']
        assert type(attempt['peak_process_group_rss_kib']) is int
        assert 0 <= attempt['peak_process_group_rss_kib'] <= manifest['maximum_rss_mib'] * 1024
        assert attempt['timeout_seconds'] == manifest['timeout_seconds']
        assert attempt['power_at_start'] == attempt['power_at_end'] == POWER
        assert all(attempt[field] == [] for field in EMPTY_FIELDS)
        assert not any(attempt.get(field) for field in ERROR_FIELDS)
        assert attempt['validation'] == dict(REFERENCE, reference_match=True)
        assert (attempt['stdout_bytes'], attempt['stdout_sha256']) == (REFERENCE['bytes'], REFERENCE['sha256'])
        native = attempt['supervised_native_group']
        assert type(native['pid']) is int and native['pid'] > 0
        assert native['pid'] == native['pgid'] and native['command'] == command
        _number(attempt['seconds'])
        _timing(attempt, row['mode'])
        grouped[identity].append(attempt)
    result = {}
    for identity, row in rows.items():
        recorded, attempts = report['profiles'][identity], grouped[identity]
        samples = [attempt['seconds'] for attempt in attempts]
        assert (recorded['loc'], recorded['mode'], recorded['state_layout'], recorded['timing_boundary']) == (
            row['loc'], row['mode'], row['state_layout'], row['timing_boundary'])
        assert len(samples) == len(recorded['sample_details']) == 3
        details = [{key: attempt[key] for key in ('seconds', 'wall_seconds', 'prepare_seconds',
                   'kernel_seconds', 'validation', 'supervised_native_group') if key in attempt}
                   for attempt in attempts]
        assert recorded['sample_details'] == details and recorded['samples_seconds'] == samples
        assert recorded['median_seconds'] == statistics.median(samples)
        assert recorded['sample_stdev_seconds'] == statistics.stdev(samples)
        result[identity] = dict(deepcopy(row), samples_seconds=samples,
            median_seconds=statistics.median(samples), sample_stdev_seconds=statistics.stdev(samples),
            sample_details=deepcopy(details), sample_execution_metadata=deepcopy(attempts),
            sample_validation=[deepcopy(attempt['validation']) for attempt in attempts])
    return result


def add_mandelbrot_modes(root, data, load, measured, add_file, sha):
    root = Path(root)
    if not (root / REPORT).is_file():
        return
    # Never load an in-progress report into publication provenance.
    report = json.loads((root / REPORT).read_text())
    if report['status'] != 'complete':
        return
    assert REVIEWED_MANIFEST_SHA256, 'Canonical mode manifest has not been independently reviewed'
    assert sha(root / MANIFEST) == REVIEWED_MANIFEST_SHA256
    manifest, report = load(MANIFEST), load(REPORT)
    assert Path(report['manifest']).resolve() == (root / MANIFEST).resolve()
    rows = validate_modes_study(manifest, report, REVIEWED_MANIFEST_SHA256)
    for row in rows.values():
        assert sha(row['source']) == row['source_sha256']
        assert sha(row['binary']) == row['binary_sha256']
    benchmark = next(row for row in data['benchmarks'] if row['key'] == 'mandelbrot')
    groups = {group['language']: group for group in benchmark['groups']}
    previous = {language: _preserved_endpoints(data, groups[language])
                for language in ('mech-jit', 'mech-resident')}
    control = rows['row16-aot-control']
    selected = {}
    for language in ('mech-jit', 'mech-resident'):
        candidates = [row for row in rows.values() if row['language'] == language]
        fast = min(candidates, key=lambda row: (row['median_seconds'], row['loc'], row['key']))
        small = min(candidates, key=lambda row: (row['loc'], row['median_seconds'], row['key']))
        selected[language] = dict(fast=fast['key'], small=small['key'])
        for row in {entry['key']: entry for entry in (fast, small)}.values():
            same_source = rows['scalar11-aot-control' if row['loc'] == 11 else 'row16-aot-control']
            measured(row['key'], 'mandelbrot', language, row['loc'], row['samples_seconds'],
                row['source'], row['source_sha256'], REPORT, MANIFEST,
                physical_loc=row['physical_loc'], loc_expansions=row['loc_expansions'],
                loc_metric='statement-normalized', execution_mode=row['mode'],
                binary_sha256=row['binary_sha256'], runtime_arguments=row['command'],
                runtime_environment=row['runtime_environment'], support_pins=_support_pins(row),
                execution_fingerprint=json.dumps(_fingerprint(row), sort_keys=True),
                study_manifest_sha256=REVIEWED_MANIFEST_SHA256,
                study_report_sha256=sha(root / REPORT), sample_details=row['sample_details'],
                sample_execution_metadata=row['sample_execution_metadata'],
                full_output_validation=row['sample_validation'], required_work_validated=True,
                timing_boundary=row['timing_boundary'], timing_summary=row['timing_boundary'],
                runtime_policy=('JIT · general source graph lowering' if row['mode'] == 'jit'
                                else 'Interpreted · loaded typed source graphs'),
                source_provenance='Counted canonical Mech source; authored array recurrences and general grid/PBM operations. Fresh complete N16000 image, at most eight compute workers.',
                algorithm=dict(strict=True, status='reviewed', native_benchmark_kernel=False,
                    full_work=dict(width=16000, height=16000, maximum_iterations=50),
                    reason='The Mech source authors complex recurrences and escape predicates. The generic executor parses and lowers that source; no native benchmark kernel.',
                    evidence=[dict(path=MANIFEST, sha256=REVIEWED_MANIFEST_SHA256)]),
                same_source_aot_control=deepcopy(same_source),
                mech_comparison=dict(scope='paired-control', seconds=control['median_seconds'],
                    control_key=control['key'], control_loc=control['loc'], source=control['source'],
                    source_sha256=control['source_sha256'], samples_seconds=control['samples_seconds'],
                    report=REPORT, source_audit=MANIFEST,
                    sample_execution_metadata=control['sample_execution_metadata'],
                    time_x_mech=row['median_seconds'] / control['median_seconds'],
                    control_loc_x_mech=row['loc'] / control['loc']))
        groups[language]['programs'] = list(dict.fromkeys((fast['key'], small['key'])))
    sources = {}
    for row in rows.values():
        for digest in add_file(row['source'], row['source_sha256']):
            sources[digest] = deepcopy(data['files'][digest])
    data['mandelbrot_modes_audit'] = dict(status='complete', manifest=MANIFEST,
        manifest_sha256=REVIEWED_MANIFEST_SHA256, report=REPORT, report_sha256=sha(root / REPORT),
        profiles=rows, selected=selected, previous_endpoints=previous, study_sources=sources,
        admission_rule='All eighteen fresh full processes reconciled; exactly three qualified rounds per profile.',
        comparison_policy='Mech multiples use the freshly measured canonical row16 AOT control; same-source AOT controls are retained separately. Old mode timings are never relabelled as new sources.')
    data['provenance']['mandelbrot_modes_data.py'] = sha(Path(__file__))
    data['protocol']['mandelbrot_source_modes'] = manifest['timing_boundary']
    if 'mandelbrot_studies' in data['protocol']:
        data['protocol']['mandelbrot_studies'] = data['protocol']['mandelbrot_studies'].replace(
            'Other workloads, Mech JIT and interpreted source measurements retain their existing cohorts.',
            'Other workloads retain their existing cohorts. Canonical Mandelbrot JIT and interpreted sources use a separate three-round study.')
