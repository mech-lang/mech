"""Import the complete integrated interpreter cohort and preserve its history."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import math
from pathlib import Path
import statistics

COHORT='interpreter-integration/20261011'
DEFAULT_FANNKUCH='fannkuchredux:mech-fannkuch-default'
WORKLOADS = {
    'nbody': [1000, 50000000], 'mandelbrot': [200, 16000],
    'knucleotide': [1000, 25000000], 'spectralnorm': [100, 5500],
    'revcomp': [1000, 25000000], 'fannkuchredux': [7, 12],
    'fasta': [1000, 25000000], 'binarytrees': [10, 21],
    'regexredux': [30, 5000000], 'pidigits': [30, 10000],
}
ROW_FIELDS = ('source_key', 'benchmark', 'mode', 'language', 'source',
              'source_sha256', 'loc', 'physical_loc', 'loc_expansions',
              'runtime_environment', 'timing_boundary')
FULL_OUTPUTS = {
    'mandelbrot': (32000015, '609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be'),
    'knucleotide': (259, 'f8c042fe770dda8f870b967605ea40ca2e353f2bdf92e89a327209dd31d445c7'),
    'revcomp': (254166745, 'e5730d4fd55c464f276e5c6062c80f46e0a735281de15d8c957e2731a9baf70e'),
    'fasta': (254166745, '3fcf4f78104c8a65ef210fe1d469f4e473456c791225f2f1f9114f4986aa09fa'),
    'binarytrees': (455, '341de11a51feab3d8122b4b5d6a68b038a2d14434aa9bc2372f39300bf5f48e1'),
}


def _number(value, positive=False):
    assert isinstance(value, (int, float)) and not isinstance(value, bool)
    assert math.isfinite(value) and (value > 0 if positive else value >= 0)
    return value


def _same_number(value, expected):
    assert math.isclose(_number(value), expected, rel_tol=1e-12, abs_tol=1e-12)


def _validate_full_output(root, benchmark, validation, sha):
    assert validation['reference_match'] is True
    assert isinstance(validation['bytes'], int) and not isinstance(validation['bytes'], bool)
    assert validation['bytes'] > 0
    digest = validation['sha256']
    assert isinstance(digest, str) and len(digest) == 64
    assert all(c in '0123456789abcdef' for c in digest)
    if benchmark == 'nbody':
        energies = validation['energies']
        assert len(energies) == 2 and validation['tolerance'] == 1e-8
        for actual, expected in zip(energies, (-0.169075164, -0.169059907)):
            assert isinstance(actual, (int, float)) and not isinstance(actual, bool)
            assert math.isfinite(actual) and abs(actual - expected) <= 1e-8
    else:
        if benchmark in FULL_OUTPUTS:
            expected = FULL_OUTPUTS[benchmark]
        elif benchmark in ('spectralnorm', 'fannkuchredux'):
            text = ('1.274224153\n' if benchmark == 'spectralnorm'
                    else '3968050\nPfannkuchen(12) = 65\n').encode()
            expected = (len(text), hashlib.sha256(text).hexdigest())
        else:
            reference = root / benchmark / 'inputs/output-full.txt'
            expected = (reference.stat().st_size, sha(reference))
        assert (validation['bytes'], digest) == expected, benchmark


def _validate_samples(root, row, samples, sha):
    timings, validations, details = (row[field] for field in
        ('samples_seconds', 'sample_validation', 'sample_details'))
    assert len(timings) == len(validations) == len(details) == samples, row['key']
    for seconds, validation, detail in zip(timings, validations, details):
        _number(seconds, positive=True)
        _validate_full_output(root, row['benchmark'], validation, sha)
        _same_number(detail['seconds'], seconds)
        _same_number(detail['kernel_seconds'], seconds)
        _number(detail['prepare_seconds'])
        assert _number(detail['wall_seconds'], positive=True) >= seconds
    _same_number(row['median_seconds'], statistics.median(timings))
    _same_number(row['sample_stdev_seconds'], statistics.stdev(timings))


def validate_complete_report(root, report, baseline, sha):
    """Check existing execution records without adding retrospective run pins."""
    assert report['status'] == 'complete'
    samples = report['sample_count']
    assert isinstance(samples, int) and not isinstance(samples, bool) and samples >= 3
    assert report['workloads'] == WORKLOADS
    completed = report['completed_benchmarks']
    assert len(completed) == len(set(completed)) and set(completed) == set(WORKLOADS)
    assert report['power_profile'] == dict(source='AC Power', low_power_mode=0)
    assert report['code_path_audit'] == baseline['code_path_audit']
    assert report['code_path_audit']['direct_benchmark_dispatch_changed'] is False
    assert report['code_path_audit']['benchmark_source_changes'] is False

    sources = baseline['programs']
    source_keys = [source['source_key'] for source in sources]
    assert len(source_keys) == len(set(source_keys)) == 15
    assert set(source['benchmark'] for source in sources) == set(WORKLOADS)
    assert set(report['programs']) == {
        key + ':' + phase for key in source_keys for phase in ('before', 'after')}
    assert set(report['changes']) == set(source_keys)
    repo = root.parents[1]
    for relative, digest in baseline['source_files'].items():
        assert sha(repo / relative) == digest, relative
    verified = {}
    for source in sources:
        assert source['mode'] == 'resident' and source['language'] == 'mech-resident'
        assert source['key'] == source['source_key'] + ':resident'
        assert source['required_work_validated'] is True and source['eligible_for_chart'] is True
        for phase in ('before', 'after'):
            key = source['source_key'] + ':' + phase
            row = report['programs'][key]
            binary = baseline['binary' if phase == 'before' else 'after_binary']
            digest = baseline['binary_sha256' if phase == 'before' else 'after_binary_sha256']
            expected = {field: source[field] for field in ROW_FIELDS}
            expected.update(key=key, phase=phase, binary=binary, binary_sha256=digest,
                            command=[binary] + source['command'][1:])
            assert all(row[field] == value for field, value in expected.items()), key
            for path, pin in ((row['source'], row['source_sha256']), (binary, digest)):
                if path not in verified:
                    verified[path] = sha(path)
                assert verified[path] == pin, path
            _validate_samples(root, row, samples, sha)
            assert row['full_validation'] == row['sample_validation'][-1]
        before = report['programs'][source['source_key'] + ':before']
        after = report['programs'][source['source_key'] + ':after']
        change = report['changes'][source['source_key']]
        assert change['benchmark'] == source['benchmark'] and change['loc'] == source['loc']
        for field, expected in (
                ('before_seconds', before['median_seconds']),
                ('after_seconds', after['median_seconds']),
                ('after_over_before', after['median_seconds'] / before['median_seconds']),
                ('before_stdev_seconds', before['sample_stdev_seconds']),
                ('after_stdev_seconds', after['sample_stdev_seconds'])):
            _same_number(change[field], expected)
    return samples, len(sources), len(report['programs']) * samples


def _after_candidates(data, report, baseline, report_name, baseline_name):
    """Reconcile all old candidates before changing any point or its history."""
    keys = {source['source_key'] + ':resident' for source in baseline['programs']}
    assert {key for key, point in data['programs'].items()
            if point['language'] == 'mech-resident'} == keys
    replacements = {}
    for source in baseline['programs']:
        key = source['source_key'] + ':resident'
        point = data['programs'][key]
        for field in ('key', 'benchmark', 'language', 'loc', 'physical_loc', 'loc_expansions',
                      'timing_boundary'):
            assert point[field] == source[field], (key, field)
        assert point['execution_mode'] == 'resident'
        assert point['required_work_validated'] is True and point['algorithm']['strict'] is True
        digest = source['source_sha256']
        assert point['files'] == [digest] and data['files'][digest]['sha256'] == digest, key
        assert hashlib.sha256(data['files'][digest]['code'].encode()).hexdigest() == digest, key
        assert point['samples'] == source['samples_seconds'], key
        assert point['sample_details'] == source['sample_details'], key
        _same_number(point['seconds'], source['median_seconds'])
        _same_number(point['timing_sd'], source['sample_stdev_seconds'])
        assert point['binary_sha256'] == source['binary_sha256'], key
        after = report['programs'][source['source_key'] + ':after']
        previous = deepcopy(point)
        # The baseline also preserves execution-row fields absent from the old UI point.
        previous['execution_record'] = deepcopy(source)
        replacement = deepcopy(point)
        replacement.update(seconds=after['median_seconds'], samples=deepcopy(after['samples_seconds']),
            timing_sd=after['sample_stdev_seconds'], sample_details=deepcopy(after['sample_details']),
            report=report_name, source_report=baseline_name, previous_measurement=previous,
            binary=after['binary'], binary_sha256=after['binary_sha256'],
            compiler_sha256=after['binary_sha256'], runtime_arguments=deepcopy(after['command']),
            runtime_environment=deepcopy(after['runtime_environment']),
            environment_scope='Recorded per-row overrides; ambient variables were inherited and not fully captured.',
            full_output_validation=deepcopy(after['sample_validation']),
            full_validation=deepcopy(after['full_validation']), measurement_status='measured',
            measurement_cohort=COHORT, measurement_phase='after',
            integration_comparison=deepcopy(report['changes'][source['source_key']]))
        replacements[key] = replacement
    return replacements


def _confirmation(root, report, load, sha):
    """Optional independent evidence is never a source of chart samples."""
    name = COHORT + '/fannkuch-confirmation.json'
    if not (root / name).is_file():
        return None
    confirmation = load(name)
    if confirmation['status'] != 'complete':
        return dict(status=confirmation['status'], report=name, used_for_chart=False)
    samples = confirmation['sample_count']
    assert isinstance(samples, int) and not isinstance(samples, bool) and samples >= 3
    assert confirmation['workload_n'] == WORKLOADS['fannkuchredux'][1]
    assert confirmation['power_profile'] == report['power_profile']
    pins = confirmation['pins']
    assert pins[str(root / (COHORT + '/baseline.json'))] == sha(root / (COHORT + '/baseline.json'))
    assert pins[str(root / (COHORT + '/timings.json'))] == sha(root / (COHORT + '/timings.json'))
    for path, digest in pins.items():
        assert sha(path) == digest, path
    assert set(confirmation['programs']) == {'before', 'after'}
    assert len(confirmation['orders']) == samples
    assert all(len(order) == 2 and set(order) == {'before', 'after'}
               for order in confirmation['orders'])
    for phase in ('before', 'after'):
        row = confirmation['programs'][phase]
        original = report['programs'][DEFAULT_FANNKUCH + ':' + phase]
        for field in ROW_FIELDS + ('key', 'phase', 'binary', 'binary_sha256', 'command'):
            assert row[field] == original[field], (phase, field)
        assert pins[row['source']] == row['source_sha256']
        assert pins[row['binary']] == row['binary_sha256']
        _validate_samples(root, row, samples, sha)
        for detail, validation in zip(row['sample_details'], row['sample_validation']):
            assert detail['validation'] == validation
            assert detail['competing_compilers'] == [] and detail['power_changes'] == []
    before, after = (confirmation['programs'][phase]['median_seconds']
                     for phase in ('before', 'after'))
    comparison = confirmation['comparison']
    for field, expected in (('before_seconds', before), ('after_seconds', after),
                            ('after_x_before', after / before)):
        _same_number(comparison[field], expected)
    return dict(confirmation, report=name, report_sha256=sha(root / name), used_for_chart=False)


def _codegen_evidence(root, report, load, sha):
    name = COHORT + '/codegen-inspection.json'
    if not (root / name).is_file():
        return None
    inspection = load(name)
    assert set(inspection['rows']) == {'before', 'after'}
    files = {}
    for phase, row in inspection['rows'].items():
        original = report['programs'][DEFAULT_FANNKUCH + ':' + phase]
        assert row['binary'] == original['binary']
        assert row['binary_sha256'] == original['binary_sha256'] == sha(row['binary'])
        assert row['symbol'] == '<mech_gpu::control::resident::Machine>::call'
        assert sha(row['disassembly_file']) == row['disassembly_sha256']
        relative = str(Path(row['disassembly_file']).relative_to(root))
        files[relative] = row['disassembly_sha256']
    before, after = (inspection['rows'][phase] for phase in ('before', 'after'))
    assert before['disassembly_sha256'] != after['disassembly_sha256']
    assert before['size_to_next_text_symbol'] != after['size_to_next_text_symbol']
    return dict(inspection, report=name, report_sha256=sha(root / name),
                disassembly_pins=files, llvm_generated_code_changed=True,
                causal_attribution_proven=False)


def add_interpreter_integration_audit(root,data,load,sha):
    report_name=COHORT+'/timings.json'
    if not (root/report_name).is_file():
        return
    report=load(report_name)
    if report['status']!='complete':
        return
    baseline_name=COHORT+'/baseline.json'
    baseline=load(baseline_name)
    assert report['baseline_sha256']==sha(root/baseline_name)
    samples,sources,total=validate_complete_report(root,report,baseline,sha)
    replacements = _after_candidates(data,report,baseline,report_name,baseline_name)
    confirmation = _confirmation(root,report,load,sha)
    codegen = _codegen_evidence(root,report,load,sha)
    helpers = (COHORT + '/run.py', 'execution-modes/20261010-final/run.py',
               'compact/run.py', 'nbody/run.py')
    verification = dict(captured_at=datetime.now(timezone.utc).isoformat(),
        scope='Verification-time helper snapshot; these hashes were not pinned at execution time for this existing run.',
        sha256={name:sha(root/name) for name in helpers})
    change = report['changes'][DEFAULT_FANNKUCH]
    result_policy = (f'The complete canonical after cohort replaces all {sources} Mech interpreted '
        f'candidate measurements ({sources * samples} validated full-workload samples); the matched '
        f'before/after audit retains all {total} samples. Sources, LOC and direct benchmark dispatch '
        'are unchanged, but LLVM codegen partitioning and generated interpreter code changed. '
        'The uncalled f32 executor does not establish timing neutrality. '
        f'Default fannkuch is {(change["after_over_before"] - 1) * 100:+.2f}% in the main cohort. ')
    if confirmation and confirmation['status'] == 'complete':
        ratio = confirmation['comparison']['after_x_before']
        result_policy += (f'Independent confirmation measured {(ratio - 1) * 100:+.2f}% '
                          'and is retained separately; it does not replace chart samples. ')
    else:
        result_policy += 'Independent confirmation is pending and is not used for chart samples. '
    result_policy += ('The cause of the observed timing change is not proven. All prior measurements '
        'and source reports remain archived on each point; AOT and JIT are unchanged. Helper '
        'snapshots are verification-time provenance; inherited ambient environment was not fully captured.')
    data['programs'].update(replacements)
    data['interpreter_integration_audit']=dict(status='complete',sample_count=samples,
        sources=sources,completed_samples=total,report=report_name,
        baseline=baseline_name, report_sha256=sha(root/report_name),
        baseline_sha256=sha(root/baseline_name), selected_phase='after',
        selected_samples=sources*samples, selected_programs=sorted(replacements),
        code_path=report['code_path_audit'],changes=report['changes'],
        imported_commit=baseline['interpreter_commit'],
        confirmation=confirmation, codegen_inspection=codegen,
        verification_time_helpers=verification,
        result_policy=result_policy)
    if codegen:
        data['provenance'].update(codegen['disassembly_pins'])
    data['protocol']['interpreter_integration']=data['interpreter_integration_audit']['result_policy']
    data['provenance']['interpreter_integration_data.py']=sha(Path(__file__))
