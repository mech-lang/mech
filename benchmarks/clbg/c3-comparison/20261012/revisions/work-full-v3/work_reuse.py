"""Reconcile preserved work-v2 Pi/maps; never replay native successful cases."""
import copy
import importlib.util
import json
from pathlib import Path
import sys

import references
import work_traces

HERE = Path(__file__).resolve().parents[2]
PRIOR = HERE / 'work-qualification/work-v2'
OLD_PACKET = HERE / 'revisions/work-full-v2'
INSPECTION = HERE.parent.parent / 'continuation/jobs/c3-tree-arm-inspection-v1'
PRIOR_REPORT_SHA = 'f0a09665cc11b5df929547110d06413df76a32114aedc9bd5e7481e348ae3ed3'
FIXTURE_SHA = '85d5226fa8b7aa17ff8e3941b7b05aa19a0efca013647d3580265090b632333a'
REUSED_IDS = {'c3-pidigits-compact', 'c3-pidigits-optimized',
              'c3-knucleotide-compact', 'c3-knucleotide-optimized'}


def validate_catalog(report, qualification, map_cases):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['error_type'] == 'AssertionError'
    assert report['error'] == 'Final native scored creator lacks actual malloc/calloc'
    assert report['qualification_revision'] == 'private-v3' and report['work_revision'] == 'work-v2'
    assert report['profiles'] == qualification['profiles']
    assert type(report['full_performance_runs']) is int and type(report['full_manifests_sealed']) is int
    assert report['full_performance_runs'] == report['full_manifests_sealed'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    assert 'pins' not in report, 'Do not invent a historical terminal evidence pin'
    assert len(report['attempts']) == len(report['proofs']) == 38
    assert len(report['source_reviews']) == 15
    expected = {('pi', 'c3-pidigits-' + form, 'real-divisions-n' + str(n))
                for form in ('compact', 'optimized') for n in (0,1,9,10,11,27,97)}
    expected |= {('maps', 'c3-knucleotide-' + form, 'complete-maps-' + case)
                 for form in ('compact', 'optimized') for case in map_cases}
    actual = [(p['kind'], p['profile'], p['case']) for p in report['proofs']]
    assert len(actual) == len(set(actual)) == len(expected) == 38 and set(actual) == expected
    assert {a['id'] for a in report['attempts']} == {identifier + '-work-trace' for identifier in REUSED_IDS}
    return expected


def validate_record(attempt, proof, observed, harness, ordinal, native_reuse):
    native_reuse.observations(attempt)
    assert type(attempt['attempt']) is int and attempt['attempt'] == ordinal
    assert type(attempt['workload_n']) is int and attempt['workload_n'] >= 0
    assert type(attempt['sampler_wait_status']) is int and attempt['sampler_wait_status'] == 0
    assert attempt['qualified'] is True
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == native_reuse.GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
    assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
    assert type(attempt['stdout_bytes']) is int and attempt['stdout_bytes'] >= 0
    assert type(attempt['validation']['bytes']) is int and attempt['validation']['bytes'] >= 0
    assert attempt['validation']['reference_match'] is True
    assert (attempt['stdout_bytes'], attempt['stdout_sha256']) == (attempt['validation']['bytes'], attempt['validation']['sha256'])
    group = attempt['supervised_native_group']
    assert group['type'] == 'native-group-started' and group['inherited_common_mutex'] is True
    assert type(group['pid']) is int and type(group['pgid']) is int and group['pid'] == group['pgid'] > 0
    assert group['command'] == attempt['command']
    assert proof['status'] == proof['reconciliation']['status'] == 'passed'
    assert proof['attempt'] == ordinal and type(proof['attempt']) is int
    assert attempt['id'] == proof['profile'] + '-work-trace' and attempt['case'] == proof['case']
    assert attempt['key'] == observed['key'] and attempt['benchmark'] == observed['benchmark']
    assert attempt['language'] == observed['language'] == 'c3'
    assert attempt['source_work_proof'] == proof['reconciliation']
    assert attempt['source_sha256'] == observed['source_sha256'] == proof['trace_source_sha256']
    assert attempt['command'] == [observed['command'][0], str(attempt['workload_n'])]
    assert attempt['cwd'] == observed['cwd'] and attempt['runtime_environment'] == observed['runtime_environment']
    assert json.loads(attempt['execution_fingerprint']) == json.loads(harness.execution_fingerprint(observed))


def command_receipt(path, command):
    receipt = json.loads(path.read_text())
    assert receipt['command'] == command
    assert type(receipt['exit_status']) is int and type(receipt['final_exit_status']) is int
    assert receipt['exit_status'] == receipt['final_exit_status'] == 0
    assert receipt['inherited_common_mutex'] is True
    assert receipt['preparation_only'] is True and receipt['never_chart_eligible'] is True
    assert type(receipt['pid']) is int and receipt['pid'] > 0
    assert type(receipt['identity']['pid']) is int and receipt['identity']['pid'] == receipt['pid']
    assert not any(receipt.get(key) for key in ('error', 'cleanup_error', 'cleanup_errors',
                   'output_drain_error', 'remaining_group_members'))
    return receipt


def preserved_module(name, path, pins, preparation, guard):
    assert name not in sys.modules, 'No cached preserved reader before its source binding'
    guard.verify(pins)
    assert preparation.sha(path) == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    assert sys.modules[name] is module and Path(module.__file__).resolve() == path.resolve()
    assert preparation.sha(path) == pins[str(path)]
    guard.verify(pins)
    return module


def reconstruct(preparation, guard, harness, qualification, native_reuse):
    report_path = PRIOR / 'report.json'
    assert preparation.sha(report_path) == PRIOR_REPORT_SHA
    prior = json.loads(report_path.read_text())
    assert prior['prerequisite'] == str(HERE / 'qualification/private-v3/report.json')
    assert preparation.sha(prior['prerequisite']) == prior['prerequisite_sha256']
    map_inputs = dict(preparation.inputs('knucleotide', references))
    validate_catalog(prior, qualification, map_inputs)
    fixture_path = PRIOR / 'fixtures/actual-inventory.json'
    assert prior['actual_mock_fixtures'] == dict(receipt=str(fixture_path), sha256=FIXTURE_SHA, methods=17, zero_skips=True)
    assert preparation.sha(fixture_path) == FIXTURE_SHA
    # The prior terminal report did not save these pins; capture and verify
    # exact preserved source bytes before importing, then recheck afterward.
    old_source_pins = preparation.pin_tree(OLD_PACKET)
    guard.verify(old_source_pins)
    old_fixtures = preserved_module('c3_preserved_work_v2_fixtures', OLD_PACKET / 'fixture_runner.py',
                                   old_source_pins, preparation, guard)
    old_fixtures.validate_receipt(json.loads(fixture_path.read_text()), 'work')
    old_traces = preserved_module('c3_preserved_work_v2_traces', OLD_PACKET / 'work_traces.py',
                                 old_source_pins, preparation, guard)
    assert Path(references.__file__).read_bytes() == (OLD_PACKET / 'references.py').read_bytes()
    inspection_path = INSPECTION / 'inspection.json'
    inspection = json.loads(inspection_path.read_text())
    assert inspection['status'] == 'saved-evidence-extracted-awaiting-call-review'
    assert inspection['actual_saved_proof_count'] == 38 and inspection['native_processes'] == 0
    assert inspection['exclusive_common_mutex_held'] is True and inspection['allocation_gate_changed'] is False
    guard.verify(inspection['pins'])
    assert inspection['pins'][str(report_path)] == PRIOR_REPORT_SHA
    rows = {row['id']: row for row in qualification['profiles']}
    assert [review['profile'] for review in prior['source_reviews']] == [row['id'] for row in qualification['profiles'][:15]]
    for review in prior['source_reviews']:
        row = rows[review['profile']]
        assert review['status'] == 'passed' and review['scored_source_sha256'] == row['source_sha256']
        assert review['scored_binary_sha256'] == row['binary_sha256'] and review['compiled_source_bundle'] == row['source_bundle']
        assert review['worker_cap'] == row['maximum_compute_workers']
        assert review['complete_bounded_cases'] == qualification['complete_bounded_case_counts'][row['id']]
        assert review['review'] == str(OLD_PACKET / 'REQUIRED_SOURCE_REVIEW.md')
    build_root = Path(qualification['build']) / 'source-work/work-v2'
    pins = preparation.merge(preparation.pin_tree(PRIOR), old_source_pins,
        preparation.pin_tree(build_root), preparation.pin_tree(INSPECTION), inspection['pins'])
    compiler, stdlib, sdk = (Path(qualification[key]) for key in ('compiler','stdlib','sdk'))
    observed_rows = {}
    for identifier in sorted(REUSED_IDS):
        row = rows[identifier]
        source = (build_root / identifier / 'observed.c3')
        binary = source.with_name('observed')
        cache = source.parent / 'cache'
        scored = Path(row['source']).read_text()
        if row['benchmark'] == 'pidigits':
            expected_source = old_traces.pi_source(scored)
            assert expected_source == work_traces.pi_source(scored)
        else:
            expected_source = old_traces.map_source(scored, row['variant'] == 'optimized')
            assert expected_source == work_traces.map_source(scored, row['variant'] == 'optimized')
        assert source.read_text() == expected_source
        sources = [source, *[Path(item['path']) for item in row['source_bundle'][1:]]]
        command = preparation.build_command(compiler, stdlib, sdk, sources, binary, cache, create_cache=False)
        build = PRIOR / 'builds' / identifier
        command_receipt(build / 'build.json', command)
        images = build / 'images'
        for label, command in (
            ('architecture', ['/usr/bin/lipo','-archs',str(binary)]),
            ('mach-header', ['/usr/bin/otool','-hv',str(binary)]),
            ('linked-images', ['/usr/bin/otool','-L',str(binary)])):
            command_receipt(images / (label + '.json'), command)
        assert (images / 'architecture.stdout').read_text().split() == ['arm64']
        assert 'ARM64' in (images / 'mach-header.stdout').read_text()
        dependencies = json.loads((images / 'dependency-review.json').read_text())
        assert dependencies['binary'] == str(binary) and dependencies['architectures'] == ['arm64']
        linked = [line.strip().split(' (',1)[0] for line in (images / 'linked-images.stdout').read_text().splitlines()[1:]]
        assert [item['install_name'] for item in dependencies['dependencies']] == linked
        for item in dependencies['dependencies']:
            if item['install_name'].startswith(('/usr/lib/','/System/Library/')):
                assert item == dict(install_name=item['install_name'],scope='OS-image-not-hashed')
            else:
                lookup = Path(item['lookup_path']); target = lookup.resolve(strict=True)
                assert str(target) == item['path']
                assert any(target.is_relative_to(root.resolve()) for root in
                           (compiler.parent, preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent))
                assert preparation.sha(lookup) == preparation.sha(target) == item['sha256']
                pins = preparation.merge(pins, {str(lookup):item['sha256'],str(target):item['sha256']})
        observed_rows[identifier] = dict(row, id=identifier + '-work-trace', source=str(source),
            source_sha256=preparation.sha(source), binary_sha256=preparation.sha(binary), command=[str(binary),'{n}'],
            compiled_source_bundle=[str(path) for path in sources],
            source_bundle=[dict(path=str(path),sha256=preparation.sha(path)) for path in sources],
            preparation_only=True, never_chart_eligible=True)
    for ordinal, (attempt, proof) in enumerate(zip(prior['attempts'], prior['proofs']), 1):
        row = rows[proof['profile']]
        directory = PRIOR / attempt['id'] / attempt['case']
        observed = copy.deepcopy(observed_rows[proof['profile']])
        observed['runtime_environment']['CLBG_TRACE'] = str(directory / 'trace')
        validate_record(attempt, proof, observed, harness, ordinal, native_reuse)
        assert proof['scored_source_sha256'] == row['source_sha256'] and proof['scored_binary_sha256'] == row['binary_sha256']
        raw = PRIOR / 'attempts' / ('%03d-%s.json' % (ordinal, attempt['id']))
        supervisor = raw.with_suffix('.supervisor.json')
        assert attempt['result_path'] == str(raw) and attempt['supervisor_result_path'] == str(supervisor)
        sampled, enriched = json.loads(raw.read_text()), json.loads(supervisor.read_text())
        for record in (sampled, enriched, attempt):
            assert record['full_workload'] is False
            assert type(record['stdout_bytes']) is int and record['stdout_bytes'] >= 0
            assert type(record['validation']['bytes']) is int and record['validation']['bytes'] >= 0
            assert record['validation']['reference_match'] is True
        native_reuse.validate_native_receipts(sampled, enriched, attempt)
        if proof['kind'] == 'pi':
            n = int(proof['case'].removeprefix('real-divisions-n'))
            assert attempt['workload_n'] == n
            expected = references.pidigits(n)
            actual_proof = old_traces.pi_trace(directory, n)
            assert actual_proof == work_traces.pi_trace(directory, n)
        else:
            case = proof['case'].removeprefix('complete-maps-')
            data = map_inputs[case]
            assert attempt['workload_n'] == harness.WORK['knucleotide'][0]
            assert (directory / 'stdin.fasta').read_bytes() == data
            expected = references.knucleotide(data)
            optimized = row['variant'] == 'optimized'
            actual_proof = old_traces.map_trace(directory, data, optimized, references, require_growth=case == 'arbitrary-widths')
            assert actual_proof == work_traces.map_trace(directory, data, optimized, references, require_growth=case == 'arbitrary-widths')
            assert proof['reviewed_source_loop'] == dict(status='passed',source=row['source'],source_sha256=row['source_sha256'],
                method='Exact rolling index loop with width warmup; no per-window event trace' if optimized else 'Exact compact nested start/width loops')
        assert actual_proof == proof['reconciliation']
        assert (directory / 'actual.stdout').read_bytes() == (directory / 'expected.stdout').read_bytes() == expected
        assert len(expected) == attempt['stdout_bytes'] and preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
    failed = PRIOR / 'scored-allocation/c3-binarytrees-compact'
    command_receipt(failed / 'final-scored-arm.json', ['/usr/bin/otool','-tvV',rows['c3-binarytrees-compact']['command'][0]])
    guard.verify(pins)
    return dict(status='verified-38-preserved-C3-Pi-and-map-work-proofs', prior_report=str(report_path),
        prior_report_sha256=PRIOR_REPORT_SHA, retained_proofs=38, native_cases_reexecuted=0,
        old_cases_relabelled_as_new_attempts=False, attempts=prior['attempts'], proofs=prior['proofs'],
        observed_profiles=observed_rows, preserved_failure=dict(error=prior['error'],
            raw_scored_arm=str(failed / 'final-scored-arm.stdout'), receipt=str(failed / 'final-scored-arm.json')),
        recorded_in_completed_work_pins=False, newly_captured_preserved_work_evidence=True,
        pins=pins, full_performance_runs=0, chart_admission=False)


def validate_packet(packet, preparation, guard, harness, qualification, native_reuse):
    guard.verify(packet['pins'])
    assert packet == reconstruct(preparation, guard, harness, qualification, native_reuse)
    return packet
