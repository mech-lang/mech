"""Additive exact four-profile/24-case reuse; no invocation at import time."""
import copy
import hashlib
import json
import math
from pathlib import Path
import re
import tarfile

HERE = Path(__file__).resolve().parents[2]
PRIOR = HERE / 'qualification/private-v1'
BUILD = HERE / '.build/private-v1'
FIRST = HERE / 'proposals/private-v1.json'
FIRST_SHA = '5b0ce9e490fa4e013a71e732b6a556a028d64bb7f93a0ef9aee77e3b56a2dce7'
IDENTITIES = ('clojure-binarytrees-compact', 'clojure-binarytrees-optimized',
    'clojure-fannkuchredux-compact', 'clojure-fannkuchredux-optimized')
OBSERVATIONS = ('exit_status', 'seconds', 'power_at_start', 'power_at_end', 'timeout_seconds',
    'maximum_rss_mib', 'peak_process_group_rss_kib', 'valid_environment', 'pins_unchanged_at_end',
    'command', 'source_sha256', 'execution_fingerprint', 'runtime_environment', 'stdout_bytes', 'stdout_sha256',
    'workload_n', 'full_workload')
FATAL = ('error', 'resource_violation', 'monitor_errors', 'resource_monitor_errors', 'cleanup_error',
    'supervisor_cleanup_error', 'remaining_group_members', 'supervisor_remaining_members', 'sampler_failure',
    'sampler_error', 'registration_error', 'supervisor_error', 'monitor_finish_error', 'supervisor_cleanup',
    'supervisor_pin_failure', 'preparation_supervisor_failure', 'source_work_error', 'source_work_failure')


def source_changes(candidate):
    old = json.loads((HERE / 'profiles.json').read_text())
    new = json.loads((candidate.REVISION_SOURCE / 'profiles.json').read_text())
    expected = copy.deepcopy(old)
    for row in expected['profiles']:
        if row['benchmark'] == 'nbody':
            before = row['source']
            after = 'revisions/private-v2/' + before
            row['source'] = after
            row['compiled_source_bundle'] = [after, 'support/clbg/support.clj']
            original = (HERE / before).read_text()
            marker = '(* dz dz))))))))]'
            assert original.count(marker) == 1
            assert (HERE / after).read_text() == original.replace(marker, '(* dz dz)))))))))]', 1)
    assert new == expected, 'Exactly two parenthesis-only main aliases; all metadata/helpers/other18 mains unchanged'
    return candidate.profiles(new)


def observations(record):
    assert type(record['exit_status']) is int and record['exit_status'] == 0
    assert type(record['seconds']) in (float, int) and math.isfinite(record['seconds']) and record['seconds'] > 0
    assert record['power_at_start'] == record['power_at_end'] == dict(source='AC Power', low_power_mode=0)
    assert record['timeout_seconds'] == 120 and record['maximum_rss_mib'] == 10240
    assert type(record['peak_process_group_rss_kib']) is int and 0 <= record['peak_process_group_rss_kib'] <= 10240*1024
    assert record['valid_environment'] is True and record['pins_unchanged_at_end'] is True
    assert type(record['workload_n']) is int and record['workload_n'] >= 0
    assert type(record['stdout_bytes']) is int and record['stdout_bytes'] >= 0
    assert record['full_workload'] is False
    assert record['competing_compilers'] == record['power_changes'] == []
    assert not any(record.get(key) for key in FATAL)


def validate_records(candidate, report, rows, fingerprints):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == FIRST_SHA and report['full_performance_runs'] == 0
    assert report['chart_admission'] is report['canonical_LOC_admitted'] is False
    assert report['no_automatic_retry'] is True and report['error_type'] == 'AssertionError'
    assert report['error'] == "('Owned command failed', 'compile', 1)"
    assert set(rows) == set(fingerprints) == set(IDENTITIES) and len(report['attempts']) == 24
    expected = [(identity, n) for identity in IDENTITIES for n in ((0, 4, 7, 9) if 'binarytrees' in identity else range(1, 9))]
    for index, (attempt, (identity, n)) in enumerate(zip(report['attempts'], expected), 1):
        row = rows[identity]
        assert type(attempt['attempt']) is type(attempt['workload_n']) is int
        assert attempt['id'] == identity and attempt['attempt'] == index and attempt['case'] == 'n-' + str(n)
        assert attempt['benchmark'] == row['benchmark'] and attempt['key'] == row['key'] and attempt['language'] == 'clojure'
        assert attempt['source_sha256'] == row['source_sha256'] and attempt['cwd'] == row['cwd']
        assert attempt['runtime_environment'] == row['runtime_environment']
        assert attempt['command'] == [part.replace('{n}', str(n)) for part in row['command']]
        assert attempt['execution_fingerprint'] == fingerprints[identity]
        assert attempt['workload_n'] == n and attempt['full_workload'] is False
        assert attempt['preparation_only'] is attempt['never_chart_eligible'] is True and attempt['full_performance_run'] is False
        candidate.exact_true_checks(attempt)
        observations(attempt)
        native = attempt['supervised_native_group']
        assert native['type'] == 'native-group-started' and type(native['pid']) is int and native['pid'] > 0
        assert native['pgid'] == native['pid'] and native['command'] == attempt['command'] and native['inherited_common_mutex'] is True
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes'] and attempt['validation']['sha256'] == attempt['stdout_sha256']
    return copy.deepcopy(report['attempts'])


def archive_correspondence(candidate, archive, root):
    """Bind retained extraction to the original reviewed official payload."""
    expected, pins = set(), {}
    with tarfile.open(archive, 'r:gz') as bundle:
        for member in bundle.getmembers():
            name = Path(member.name)
            assert not name.is_absolute() and '..' not in name.parts
            target = root / name
            assert target.resolve().is_relative_to(root.resolve())
            assert member.isdir() or member.isfile() or member.issym()
            if member.isfile():
                assert target.is_file() and not target.is_symlink()
                digest = hashlib.file_digest(bundle.extractfile(member), 'sha256').hexdigest()
                assert candidate.sha(target) == digest
                pins[str(target)] = digest
                expected.add(target)
            elif member.issym():
                assert target.is_symlink() and target.readlink() == Path(member.linkname)
                assert target.resolve(strict=True).is_relative_to(root.resolve())
                pins[str(target)] = candidate.sha(target)
                expected.add(target)
    assert expected == {path for path in root.rglob('*') if path.is_file() or path.is_symlink()}, 'No unaccounted JDK input'
    return pins


def receipt(path, command, exit_status=0):
    value = json.loads(path.read_text())
    assert value['command'] == command and value['inherited_common_mutex'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == exit_status
    assert not any(value.get(key) for key in ('cleanup_error', 'cleanup_errors', 'output_drain_error', 'remaining_group_members'))
    if exit_status == 0:
        assert not value.get('error') and not value.get('error_type')
    assert path.with_suffix('.stdout').is_file() and path.with_suffix('.stderr').is_file()
    return value


def reconstruct(candidate, service, guard, harness):
    assert candidate.sha(FIRST) == FIRST_SHA
    first = json.loads(FIRST.read_text())
    guard.verify(first['pins'])
    assert first['status'] == 'sealed-awaiting-root-review'
    source_changes(candidate)
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    runtime_path = PRIOR / 'runtime-receipt.json'
    runtime = json.loads(runtime_path.read_text())
    assert runtime['status'] == 'qualified-native-arm64-jvm-identity'
    assert runtime['compiler_version'] == candidate.CLOJURE_VERSION and runtime['java_runtime_version'] == candidate.JDK_VERSION
    assert runtime['measured_architecture'] == 'arm64' and runtime['collector'] == 'G1'
    assert runtime['backend'] == 'JVM bytecode then HotSpot JIT' and runtime['flags'] == candidate.JVM_FLAGS
    java_home, java = Path(runtime['java_home']), Path(runtime['java'])
    assert java == java_home / 'bin/java' and java_home.is_relative_to(BUILD / 'jdk')
    environment = candidate.controlled_environment(java_home)
    archive = BUILD / candidate.JDK_ARCHIVE
    assert candidate.sha(archive) == first['jdk_archive_sha256']
    pins = service.merge(first['pins'], {str(FIRST): FIRST_SHA, str(archive): candidate.sha(archive)},
        archive_correspondence(candidate, archive, BUILD / 'jdk'), service.pin_tree(PRIOR), service.pin_tree(BUILD))
    jars = [Path(entry['path']) for entry in runtime['vendor_jars']]
    assert len(jars) == 3 and jars == [BUILD / 'vendor' / (name + '-' + version + '.jar') for name, version in candidate.ARTIFACTS]
    for declared, entry in zip(first['artifacts'], runtime['vendor_jars']):
        path = Path(entry['path'])
        assert candidate.sha(path) == entry['sha256']
        with path.open('rb') as stream:
            assert hashlib.file_digest(stream, 'sha1').hexdigest() == declared['publisher_sha1']
    tools = PRIOR / 'runtime-identity'
    for executable in ('java', 'javac', 'javap'):
        receipt(tools / (executable + '-version.json'), [str(java_home / ('bin/' + executable)), '-version'])
    assert candidate.JDK_VERSION in (tools / 'java-version.stderr').read_text()
    receipt(tools / 'jvm-properties.json', [str(java), *candidate.JVM_FLAGS, '-XshowSettings:properties', '-version'])
    properties = (tools / 'jvm-properties.stdout').read_text() + (tools / 'jvm-properties.stderr').read_text()
    assert 'os.arch = aarch64' in properties and 'java.runtime.version = ' + candidate.JDK_VERSION in properties and 'Eclipse Adoptium' in properties
    language_command = [str(java), *candidate.JVM_FLAGS, '-cp', candidate.os.pathsep.join(map(str, jars)), 'clojure.main', '-e',
        '(assert (= "1.12.6" (clojure-version))) (assert (= 8 (.availableProcessors (Runtime/getRuntime)))) (doseq [g (java.lang.management.ManagementFactory/getGarbageCollectorMXBeans)] (println (.getName g)))']
    receipt(tools / 'language-collector-identity.json', language_command)
    collector = (tools / 'language-collector-identity.stdout').read_text()
    assert 'G1 Young Generation' in collector and 'G1 Old Generation' in collector
    for name, image in (('java', java), ('libjvm', java_home / 'lib/server/libjvm.dylib')):
        directory = PRIOR / 'runtime-images' / name
        for label, command in (('architecture', ['/usr/bin/lipo', '-archs', str(image)]),
                ('mach-header', ['/usr/bin/otool', '-hv', str(image)]), ('linked-images', ['/usr/bin/otool', '-L', str(image)])):
            receipt(directory / (label + '.json'), command)
        assert (directory / 'architecture.stdout').read_text().split() == ['arm64']
        assert 'ARM64' in (directory / 'mach-header.stdout').read_text()
        review = json.loads((directory / 'dependency-review.json').read_text())
        assert review['binary'] == str(image) and review['architectures'] == ['arm64']
        assert [entry['install_name'] for entry in review['dependencies']] == [line.strip().split(' (', 1)[0]
            for line in (directory / 'linked-images.stdout').read_text().splitlines()[1:]]
        for entry in review['dependencies']:
            if entry['install_name'].startswith(('/usr/lib/', '/System/Library/')):
                assert entry == dict(install_name=entry['install_name'], scope='OS-image-not-hashed')
            else:
                target = Path(entry['lookup_path']).resolve(strict=True)
                assert str(target) == entry['path'] and target.is_relative_to(java_home)
                assert candidate.sha(entry['lookup_path']) == candidate.sha(target) == entry['sha256']
                pins = service.merge(pins, {entry['lookup_path']: entry['sha256'], str(target): entry['sha256']})
    declarations = {row['id']: row for row in first['profiles']}
    rows = {}
    for identity in IDENTITIES:
        declared = declarations[identity]
        path = PRIOR / 'builds' / identity / 'compiler-input-inventory.json'
        inventory = json.loads(path.read_text())
        package, classes = BUILD / 'packages' / identity, BUILD / 'classes' / identity
        inputs, artifacts = inventory['source_bundle'], inventory['compiled_artifacts']
        command = candidate.compile_command(java, package, classes, jars, declared['namespace'])
        build_receipt = PRIOR / 'builds' / identity / 'compile.json'
        receipt(build_receipt, command)
        assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == identity
        assert inventory['source'] == inputs[0]['path'] and inventory['source_sha256'] == inputs[0]['sha256']
        assert inventory['build_command'] == command and inventory['compiler_jar'] == str(jars[0]) and inventory['compiler_sha256'] == candidate.sha(jars[0])
        assert inventory['runtime_classpath'] == [str(classes), *map(str, jars)]
        assert artifacts == service.pin_tree(classes) and artifacts and all(Path(path).suffix == '.class' for path in artifacts)
        assert len(inputs) == 2 and [item['role'] for item in inputs] == ['application', 'application-helper']
        for item, name in zip(inputs, declared['compiled_source_bundle']):
            assert item['path'] == str(package / name) and item['origin'] == str(PRIOR / 'source-snapshot' / name)
            assert candidate.sha(item['path']) == candidate.sha(item['origin']) == first['pins'][str(HERE / name)] == item['sha256']
        rows[identity] = dict(declared, key=declared['benchmark'] + ':' + identity, language='clojure',
            source=inputs[0]['path'], source_sha256=inputs[0]['sha256'], source_bundle=inputs,
            binary=str(java), binary_sha256=candidate.sha(java), command=candidate.runtime_command(java, classes, jars, declared['namespace']),
            cwd=str(HERE), runtime_environment=environment, compiler_version=candidate.CLOJURE_VERSION,
            runtime_version=candidate.JDK_VERSION, measured_architecture='arm64', backend='JVM HotSpot JIT',
            support=service.merge(artifacts, service.pin_tree(java_home), {str(path): candidate.sha(path) for path in jars},
                {item['path']: item['sha256'] for item in inputs[1:]}),
            compiled_input_inventory=str(path), compiled_input_inventory_sha256=candidate.sha(path),
            compiled_artifacts=artifacts, runtime_classpath=inventory['runtime_classpath'],
            compile_receipt=str(build_receipt), compile_receipt_sha256=candidate.sha(build_receipt), bounded_origin='retained-private-v1')
    fingerprints = {identity: harness.execution_fingerprint(row) for identity, row in rows.items()}
    records = validate_records(candidate, report, rows, fingerprints)
    references = candidate.module('clojure_reuse_independent_references', candidate.REFERENCE)
    for attempt in records:
        directory = PRIOR / attempt['id'] / attempt['case']
        raw, supervisor = Path(attempt['result_path']), Path(attempt['supervisor_result_path'])
        assert raw == PRIOR / 'attempts' / ('%03d-%s.json' % (attempt['attempt'], attempt['id'])) and supervisor == raw.with_suffix('.supervisor.json')
        actual = (directory / 'actual.stdout').read_bytes()
        assert actual == (directory / 'expected.stdout').read_bytes() == getattr(references, attempt['benchmark'])(attempt['workload_n'])
        assert candidate.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
        sampled, enriched = json.loads(raw.read_text()), json.loads(supervisor.read_text())
        observations(sampled); observations(enriched)
        # The immutable child producer has no ordinal field; the exact retained
        # result pathname binds its ordinal. Parent enrichment does add it.
        assert 'attempt' not in sampled
        assert type(enriched['attempt']) is int and enriched['attempt'] == attempt['attempt']
        assert type(enriched['sampler_wait_status']) is int and enriched['sampler_wait_status'] == 0
        for key in OBSERVATIONS:
            assert sampled[key] == enriched[key] == attempt[key]
        for key in ('qualified', 'qualification_checks', 'supervised_native_group', 'sampler_wait_status'):
            assert enriched[key] == attempt[key]
    failed = PRIOR / 'builds/clojure-nbody-compact'
    failed_command = candidate.compile_command(java, BUILD / 'packages/clojure-nbody-compact',
        BUILD / 'classes/clojure-nbody-compact', jars, 'clbg.nbody.compact')
    failure = receipt(failed / 'compile.json', failed_command, 1)
    assert failure['error_type'] == 'AssertionError' and failure['error'] == "('Owned command failed', 'compile', 1)"
    diagnostic = (failed / 'compile.stderr').read_text()
    assert 'clbg/nbody/compact.clj:26:131' in diagnostic and 'Unmatched delimiter: ]' in diagnostic
    assert not (failed / 'compiler-input-inventory.json').exists()
    guard.verify(pins)
    return dict(status='verified-exact-old-Clojure-four-profile24-case-proof', profiles=rows, attempts=records,
        prior_report=str(report_path), prior_report_sha256=candidate.sha(report_path), first_proposal=str(FIRST), first_proposal_sha256=FIRST_SHA,
        runtime_receipt=str(runtime_path), runtime_receipt_sha256=candidate.sha(runtime_path),
        runtime=runtime, preserved_failed_build=str(failed / 'compile.json'), preserved_failed_build_sha256=candidate.sha(failed / 'compile.json'),
        reused_profiles=4, reused_bounded_cases=24, old_cases_relabelled_as_new_attempts=False, native_cases_reexecuted=0,
        newly_captured_terminal_evidence_pins=True,
        pin_basis='The stopped v1 report did not persist its local rows/pins. Reconstruct from preserved original producer compiler-input/build/runtime receipts, actual24 raw/supervisor/output records and reviewed official JDK payload correspondence; current receipt hashes are newly captured, never claimed historical report pins.',
        pins=pins)
