"""Retain actual v3 two-K plus v2 two-Pi work; never execute retained cases."""
import copy
import json
from pathlib import Path

import reuse_work_v2

OLD_MANIFEST_SHA = '00a83167f0d63f3f7ce0664096b70c841b2cd4fb77a34bc05ce2be8f71005df5'
OLD_REUSE_SHA = '3db14701f4ac1d3c270732f99230b533b233e4a0f700ab302978979b65ed44f0'
# Actual root common-job followup-digests-v1 receipt; no local hash at authoring.
OLD_REPORT_SHA = 'c683c6f669016eb70cd380455077c3589b8df86537ce448a655a40be44606ca3'
K_IDS = ('pony-knucleotide-compact-precedence-v4', 'pony-knucleotide-optimized-shift-types-v5')
TREE_IDS = ('pony-binarytrees-compact-precedence-v4', 'pony-binarytrees-optimized-precedence-v4')
IDENTITY_OLD = '    let identity = (digestof this).string()\n'
IDENTITY_NEW = '    let identity: String val = (digestof this).string()\n'


def failed_v4_seal(preparation):
    """The failed metadata seal ran no new native/work/fixture case."""
    cohort = preparation.HERE
    outcome = cohort / 'work-seal-outcomes/private-v8-work-v4'
    receipt_path, error_path = outcome/'receipt.json', outcome/'actual-error.log'
    value = json.loads(receipt_path.read_text())
    assert value['status'] == 'failed-stop' and value['revision'] == 'private-v8-work-v4'
    assert value['actual_common_mutex_held'] is True and value['chart_admission'] is False
    assert type(value['actual_native_subjects']) is int and value['actual_native_subjects'] == 0
    assert value['error'] == value['error_type'] == 'AssertionError'
    assert 'terminal_integrity_error' not in value
    error_sha = '2433658983082c3982585cda2d0f8a2e760a709b2f1a904dcae9379997d787d3'
    assert value['available_evidence_pins'] == {str(error_path): error_sha}
    assert preparation.sha(error_path) == error_sha
    error = error_path.read_text()
    assert 'reuse_work_v3.py", line 144, in reconstruct' in error
    assert "assert dict(proof, actual_record_mutation_rejections=mutations) == attempt['proof']" in error
    partial = cohort/'work-qualification/private-v8-work-v4'
    assert partial.is_dir() and not list(partial.iterdir()), 'No v4 partial native/sample evidence may be dropped'
    pins = preparation.pin_tree(outcome)
    return dict(path=str(receipt_path), sha256=pins[str(receipt_path)],
        error_log=str(error_path), error_log_sha256=error_sha, new_native_subjects=0,
        new_work_cases=0, reason='integer K width keys differ from their stored JSON string keys'), pins


def reconstruct(preparation, guard, harness, generator, checker, references, packet_pins):
    failed_metadata, metadata_pins = failed_v4_seal(preparation)
    cohort = preparation.HERE
    old = cohort / 'work-qualification/private-v8-work-v3'
    old_packet = cohort / 'revisions/work-full-v3'
    packet = Path(__file__).resolve().parent
    manifest_path, report_path = old / 'manifest.json', old / 'report.json'
    assert isinstance(OLD_REPORT_SHA, str) and len(OLD_REPORT_SHA) == 64
    assert preparation.sha(manifest_path) == OLD_MANIFEST_SHA
    assert preparation.sha(report_path) == OLD_REPORT_SHA
    manifest, prior = (json.loads(path.read_text()) for path in (manifest_path, report_path))
    assert manifest['status'] == 'prepared-awaiting-root-review'
    assert manifest['qualification'] == str(cohort / 'qualification/private-v8/report.json')
    assert manifest['qualification_sha256'] == reuse_work_v2.QUALIFIED_SHA
    assert preparation.sha(manifest['qualification']) == reuse_work_v2.QUALIFIED_SHA
    guard.verify(manifest['pins']); guard.verify(prior['pins'])
    assert prior['status'] == 'stopped' and prior['error_type'] == 'AssertionError'
    assert prior['error'] == "('Owned command failed', 'trace-build', 255)"
    assert 'terminal_integrity_error' not in prior
    assert prior['manifest'] == str(manifest_path) and prior['manifest_sha256'] == OLD_MANIFEST_SHA
    assert prior['observer_repair_revision'] == 'work-full-v3-isolated-map-observer'
    for key, expected in (('full_performance_runs', 0), ('fresh_actual_work_traces', 2), ('reused_actual_work_traces', 2)):
        assert type(prior[key]) is int and prior[key] == expected
    assert prior['old_attempts_relabelled_as_new'] is False
    assert tuple(item['id'] for item in prior['attempts']) == reuse_work_v2.PI_IDS + K_IDS
    assert 'scored_tree_raw_allocation_evidence' not in prior and not (old / 'scored-tree-allocation').exists()
    pins = preparation.merge(manifest['pins'], prior['pins'], preparation.pin_tree(old), metadata_pins)

    # Reconstruct the complete v2 Pi/14-fixture lineage with unchanged code.
    assert preparation.sha(packet / 'reuse_work_v2.py') == packet_pins[str(packet / 'reuse_work_v2.py')] == manifest['pins'][str(old_packet / 'reuse_work_v2.py')]
    earlier = reuse_work_v2.reconstruct(preparation, guard, harness, generator, checker, references, packet_pins)
    reuse_path = old / 'prior-work-reuse.json'
    assert manifest['prior_work_reuse'] == prior['prior_work_reuse'] == dict(path=str(reuse_path), sha256=OLD_REUSE_SHA)
    assert preparation.sha(reuse_path) == OLD_REUSE_SHA
    assert json.loads(reuse_path.read_text()) == earlier
    assert prior['attempts'][:2] == earlier['attempts']
    pins = preparation.merge(pins, earlier['pins'])

    unchanged = ('generate_traces_base.py', 'check_traces.py', 'check_traces_base.py', 'static_review.py',
        'allocation_evidence.py', 'preparation_service.py', 'owned_service_work_v2.py', 'full_prerequisite.py')
    for name in unchanged + reuse_work_v2.FIXTURE_CLOSURE:
        assert preparation.sha(packet / name) == packet_pins[str(packet / name)] == manifest['pins'][str(old_packet / name)]
    old_generator = (old_packet / 'generate_traces.py').read_text()
    assert old_generator.count(IDENTITY_OLD) == 1 and IDENTITY_NEW not in old_generator
    assert (packet / 'generate_traces.py').read_text() == old_generator.replace(
        '"""Additive v3 observer repair; scored sources and work order are unchanged."""',
        '"""Additive v4 diagnostic String capability repair; scored work is unchanged."""').replace(IDENTITY_OLD, IDENTITY_NEW)

    rows = {row['id']: row for row in manifest['rows']}
    assert set(rows) == set(reuse_work_v2.PI_IDS + K_IDS + TREE_IDS) and len(rows) == len(manifest['rows']) == 6
    qualified = json.loads(Path(manifest['qualification']).read_text())
    scored = {row['id']: row for row in qualified['profiles']}
    assert len(scored) == len(qualified['profiles']) == 20
    for key, row in rows.items():
        assert row['source'] == scored[key]['source'] and row['source_sha256'] == scored[key]['source_sha256']
        source, trace = Path(row['source']), Path(row['trace_source'])
        assert preparation.sha(source) == row['source_sha256'] == pins[str(source)]
        generated, _ = generator.instrument(source.read_text(), row['benchmark'], row['variant'])
        if key in TREE_IDS:
            assert generated.count(IDENTITY_NEW) == 1 and IDENTITY_OLD not in generated
            assert generated.replace(IDENTITY_NEW, IDENTITY_OLD) == trace.read_text()
        else:
            assert generated == trace.read_text()
        assert preparation.sha(trace) == pins[str(trace)]

    toolchain = manifest['toolchain']
    compiler, sdk, regex, gmp = (Path(toolchain[name]) for name in ('compiler', 'sdk', 'regex_root', 'gmp_root'))
    failed_id = TREE_IDS[0] + '-trace'
    failed_source = Path(rows[TREE_IDS[0]]['trace_source'])
    failed_build = old / 'builds' / failed_id / 'trace-build.json'
    command = preparation.build_command(compiler, sdk, regex, gmp, failed_source, old / 'bin' / failed_id, failed_id)
    reuse_work_v2.receipt(failed_build, command, 255)
    failure_stderr = failed_build.with_suffix('.stderr')
    text = failure_stderr.read_text()
    assert text.count('\nError:\n') == 3 and text.count('argument type is String iso') == 3
    assert text.count('parameter type requires String box^') == 3 and text.count('iso is not a subcap of box^') == 3
    for line in (':33:56:', ':36:49:', ':38:49:'):
        assert str(failed_source) + line in text
    assert IDENTITY_OLD in failed_source.read_text()
    assert not (old / 'executions' / failed_id).exists() and not (old / 'bin' / failed_id / failed_id).exists()
    assert {path.name for path in (old / 'builds').iterdir()} == {key + '-trace' for key in K_IDS} | {failed_id}
    assert {path.name for path in (old / 'images').iterdir()} == {key + '-trace' for key in K_IDS}
    assert {path.name for path in (old / 'executions').iterdir()} == {key + '-trace' for key in K_IDS}

    retained = copy.deepcopy(earlier['attempts'])
    data_path = Path(manifest['fixtures']['knucleotide'])
    assert preparation.sha(data_path) == pins[str(data_path)]
    data = data_path.read_bytes()
    assert data == dict(references.dna_fixtures())['arbitrary-widths']
    for attempt in prior['attempts'][2:]:
        row = rows[attempt['id']]; identifier = row['id'] + '-trace'
        assert attempt['benchmark'] == row['benchmark'] == 'knucleotide'
        assert attempt['source'] == row['source'] and attempt['source_sha256'] == row['source_sha256']
        trace = Path(row['trace_source']); binary = old / 'bin' / identifier / identifier
        assert attempt['trace_source'] == str(trace)
        assert preparation.sha(trace) == attempt['trace_source_sha256'] == pins[str(trace)]
        assert preparation.sha(binary) == attempt['binary_sha256'] == pins[str(binary)]
        build = preparation.build_command(compiler, sdk, regex, gmp, trace, binary.parent, identifier)
        reuse_work_v2.receipt(old / 'builds' / identifier / 'trace-build.json', build, 0)
        image = old / 'images' / identifier
        reuse_work_v2.receipt(image / 'architecture.json', ['/usr/bin/lipo', '-archs', str(binary)], 0)
        reuse_work_v2.receipt(image / 'linked-images.json', ['/usr/bin/otool', '-L', str(binary)], 0)
        review = json.loads((image / 'dependency-review.json').read_text())
        assert review['binary'] == str(binary) and 'arm64' in review['architectures']
        for item in review['dependencies']:
            if item.get('scope') == 'OS-image-not-hashed':
                assert item['install_name'].startswith(('/usr/lib/', '/System/Library/'))
            else:
                assert preparation.sha(item['path']) == item['sha256'] == pins[item['path']]
        execution = old / 'executions' / identifier
        native_command = [str(binary), '1000', '--ponymaxthreads', '8']
        native = reuse_work_v2.receipt(execution / 'trace-native.json', native_command, 0)
        actual = attempt['observations']
        assert actual == json.loads((execution / 'observations.json').read_text())
        assert actual['power_at_start'] == actual['power_at_end'] == harness.REQUIRED
        assert not any(actual[key] for key in ('competing_compilers', 'power_changes', 'monitor_errors', 'resource_monitor_errors', 'resource_violation'))
        assert actual['native']['command'] == native_command
        assert type(actual['native']['pid']) is type(actual['native']['pgid']) is int
        assert actual['native']['pid'] == actual['native']['pgid'] == native['pid']
        assert actual['native']['inherited_common_mutex'] is actual['native']['observer_registered_after_owned_active_journal'] is True
        assert type(attempt['input_n']) is int and attempt['input_n'] == 1000 and attempt['stdin'] == str(data_path)
        assert attempt['qualified'] is attempt['complete_output_match'] is True
        raw = execution / 'trace-native.stderr'
        assert attempt['trace'] == str(raw) and attempt['trace_sha256'] == preparation.sha(raw) == pins[str(raw)]
        assert (execution / 'trace-native.stdout').read_bytes() == references.knucleotide(data)
        proof = checker.check('knucleotide', raw.read_bytes(), 1000, data, references, row['variant'])
        mutations = ['missing-work-record', 'too-many-scheduler-identities']
        # report.json writes integer object keys as strings. Convert only this
        # proved seven-width map to that representation; no proof value changes.
        entries = proof['map_entries']
        assert type(entries) is dict and set(entries) == set(references.WIDTHS)
        assert len(entries) == 7 and all(type(width) is type(count) is int for width, count in entries.items())
        stored_entries = attempt['proof']['map_entries']
        assert type(stored_entries) is dict and set(stored_entries) == {str(width) for width in references.WIDTHS}
        assert all(type(width) is str and type(count) is int for width, count in stored_entries.items())
        persisted = dict(proof, map_entries={str(width): count for width, count in entries.items()},
            actual_record_mutation_rejections=mutations)
        assert persisted == attempt['proof']
        lines = raw.read_bytes().splitlines(keepends=True)
        first = next(index for index, line in enumerate(lines) if not line.startswith((b'THREAD\t', b'FINAL\t')))
        invalid = (b''.join(lines[:first] + lines[first + 1:]),
            raw.read_bytes() + b''.join(b'THREAD\t' + str(index).encode() + b'\n' for index in range(1001, 1010)))
        for changed in invalid:
            try:
                checker.check('knucleotide', changed, 1000, data, references, row['variant'])
            except (AssertionError, KeyError, IndexError):
                pass
            else:
                raise AssertionError('Retained K mutation must remain rejected')
        retained.append(copy.deepcopy(attempt))
    guard.verify(pins)
    return dict(status='verified-four-actual-Pi-K-work-proofs-and-fourteen-fixtures',
        prior_report=dict(path=str(report_path), sha256=OLD_REPORT_SHA),
        prior_manifest=dict(path=str(manifest_path), sha256=OLD_MANIFEST_SHA),
        prior_reuse=dict(path=str(reuse_path), sha256=OLD_REUSE_SHA),
        preserved_failed_v4_metadata=failed_metadata,
        metadata_repair_revision='work-full-v5-canonical-JSON-K-width-keys',
        failed_compact_tree_build=dict(path=str(failed_build), sha256=pins[str(failed_build)],
            stderr=str(failure_stderr), stderr_sha256=pins[str(failure_stderr)]),
        attempts=retained, reused_actual_work_traces=4, new_actual_work_traces_required=2,
        fixtures=earlier['fixtures'], reused_fixture_tests=14, production_cases_reexecuted=0,
        old_attempts_relabelled_as_new=False, prior_tree_traces=0, prior_scored_ARM_IR_operations=0,
        pins=pins, chart_admission=False)
