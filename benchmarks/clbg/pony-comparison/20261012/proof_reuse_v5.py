"""Reuse exact prior bounded executions, never a newly compiled binary."""
import hashlib
import json
import math
from pathlib import Path
import tarfile

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v4'
PRIOR_PROPOSAL = HERE / 'proposals/private-v4.json'
PRIOR_SHA = '76bc19e092899272a24dc3c8d73269022beffd46801f4e8ba9d5843e7963419f'
NEW_PROFILES = {'pony-knucleotide-compact-precedence-v4'}
import proof_reuse_v4 as original
PROFILES = NEW_PROFILES | original.PROFILES
CASES = {'knucleotide': {'ties-and-stop', 'one-base', 'empty', 'missing', 'crlf-comment-eof', 'embedded-header-comment', 'arbitrary-widths',
                        'header-eof', 'empty-input', 'exact-token-cr', 'exact-token-controls', 'tail'}}
EXPECTED_GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}


OBSERVATION_FIELDS = ('exit_status', 'seconds', 'power_at_start', 'power_at_end', 'competing_compilers',
                      'power_changes', 'timeout_seconds', 'maximum_rss_mib', 'peak_process_group_rss_kib',
                      'valid_environment', 'pins_unchanged_at_end')
FATAL_FIELDS = ('error', 'resource_violation', 'monitor_errors', 'resource_monitor_errors', 'cleanup_error',
                'supervisor_cleanup_error', 'remaining_group_members', 'supervisor_remaining_members',
                'supervisor_pin_failure', 'source_work_error', 'source_work_failure', 'preparation_supervisor_failure',
                'sampler_failure', 'sampler_error', 'registration_error', 'supervisor_error', 'monitor_finish_error',
                'supervisor_cleanup', 'execute_group_members_after_cleanup', 'monitor_finish')


def validate_observations(attempt):
    assert type(attempt['exit_status']) is int and attempt['exit_status'] == 0
    assert type(attempt['seconds']) in (int, float) and math.isfinite(attempt['seconds']) and attempt['seconds'] > 0
    assert attempt['power_at_start'] == attempt['power_at_end'] == dict(source='AC Power', low_power_mode=0)
    assert attempt['competing_compilers'] == attempt['power_changes'] == []
    assert attempt['timeout_seconds'] == 120 and attempt['maximum_rss_mib'] == 10240
    rss = attempt['peak_process_group_rss_kib']
    assert type(rss) is int and 0 <= rss <= 10240 * 1024
    assert attempt['valid_environment'] is True and attempt['pins_unchanged_at_end'] is True
    assert not any(attempt.get(field) for field in FATAL_FIELDS)


def validate_records(report):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == PRIOR_SHA, 'Preserve the actual consumed proposal identity'
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert len(report['attempts']) == 12, 'Only the exact twelve v4 compact-K cases are reusable'
    groups = {}
    for number, attempt in enumerate(report['attempts'], 1):
        assert attempt['attempt'] == number
        validate_observations(attempt)
        identifier = attempt['id']
        assert identifier in NEW_PROFILES
        benchmark = attempt['benchmark']
        assert identifier == 'pony-knucleotide-compact-precedence-v4' and benchmark == 'knucleotide'
        assert attempt['case'] in CASES[benchmark]
        assert attempt['workload_n'] == 1000, 'Preserve the original bounded stdin-workload declaration'
        checks = attempt['qualification_checks']
        assert isinstance(checks, dict) and set(checks) == EXPECTED_GATES
        assert attempt['qualified'] is True and all(value is True for value in checks.values())
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
        assert attempt['pins_unchanged_at_end'] is True and attempt['valid_environment'] is True
        assert not attempt.get('error') and not attempt.get('resource_violation')
        assert not any(attempt.get(field) for field in (
            'monitor_errors', 'resource_monitor_errors', 'cleanup_error', 'supervisor_cleanup_error',
            'remaining_group_members', 'supervisor_remaining_members', 'supervisor_pin_failure'))
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes']
        assert attempt['validation']['sha256'] == attempt['stdout_sha256']
        fingerprint = json.loads(attempt['execution_fingerprint'])
        command = fingerprint['command']
        assert command[1:] == ['{n}', '--ponymaxthreads', '8']
        assert attempt['command'] == [command[0], str(attempt['workload_n']), '--ponymaxthreads', '8']
        assert attempt['supervised_native_group']['command'] == attempt['command']
        assert attempt['supervised_native_group']['inherited_common_mutex'] is True
        native = attempt['supervised_native_group']
        assert type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
        assert fingerprint['source_sha256'] == attempt['source_sha256']
        assert fingerprint['environment'] == attempt['runtime_environment']
        assert fingerprint['cwd'] == attempt['cwd']
        group = groups.setdefault(identifier, dict(benchmark=benchmark, cases={}, fingerprint=fingerprint))
        assert group['fingerprint'] == fingerprint
        assert attempt['case'] not in group['cases'], 'Never replace a case with a duplicate'
        group['cases'][attempt['case']] = attempt
    assert set(groups) == NEW_PROFILES
    assert all(set(group['cases']) == CASES[group['benchmark']] for group in groups.values())
    return groups


def require_identity(fingerprint, source_sha256, binary_sha256):
    assert source_sha256 == fingerprint['source_sha256'], 'Changed source cannot reuse outputs'
    assert binary_sha256 == fingerprint['binary_sha256'], 'A different binary cannot reuse outputs'


def archive_correspondence(preparation, archive, directory):
    """Verify the old extracted tool/package bytes against their retained archive."""
    files = {}
    with tarfile.open(archive) as bundle:
        for member in bundle:
            target = directory / member.name
            assert target.resolve().is_relative_to(directory.resolve()), 'Archive path escaped its private root'
            if member.isfile():
                with bundle.extractfile(member) as stream:
                    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
                assert preparation.sha(target) == digest, ('Old extracted archive bytes changed', str(target))
                files[str(target.resolve())] = digest
            else:
                assert member.isdir() or member.issym() or member.islnk(), 'Unexpected archive member type'
                if member.issym():
                    assert target.is_symlink() and target.readlink() == Path(member.linkname)
    assert files, 'An empty archive is not compiler/package provenance'
    return files


def profile_row(prior):
    """Preserve the original command/source/environment; do not create an attempt."""
    fingerprint = json.loads(prior['execution_fingerprint'])
    require_identity(fingerprint, prior['source_sha256'], prior['binary_sha256'])
    assert prior['command'] == fingerprint['command']
    assert prior['cwd'] == fingerprint['cwd']
    assert prior['runtime_environment'] == fingerprint['environment']
    assert prior['command'][0] == prior['binary']
    declared = prior['declaration']
    return dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='pony',
                source=prior['source'], source_sha256=prior['source_sha256'],
                binary_sha256=prior['binary_sha256'], command=prior['command'], cwd=prior['cwd'],
                runtime_environment=prior['runtime_environment'],
                qualification_origin=prior['qualification_origin'],
                prior_bounded_records=prior['prior_records'], new_native_cases=0)


def reconstruct_new(preparation, guard):
    """Caller owns the common mutex for every original hash/pin check."""
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    prior_proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(prior_proposal['pins'])
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    groups = validate_records(report)
    pins = preparation.merge(prior_proposal['pins'], {str(PRIOR_PROPOSAL): PRIOR_SHA,
        str(report_path): preparation.sha(report_path)},
        preparation.pin_tree(HERE / '.build/private-v4/release'),
        preparation.pin_tree(HERE / '.build/private-v4/regex'),
        preparation.pin_tree(HERE / '.build/private-v4/gmp4pony'),
        preparation.pin_tree(PRIOR / 'source-snapshot'), preparation.pin_tree(PRIOR / 'builds'))
    old_build = HERE / '.build/private-v4'
    archive = old_build / 'ponyc-arm64-apple-darwin.tar.gz'
    assert preparation.sha(archive) == preparation.PONY_SHA
    pins[str(archive)] = preparation.PONY_SHA
    for name, directory in (('ponyc-arm64-apple-darwin.tar.gz', 'release'),
                            ('regex-source.tar.gz', 'regex'), ('gmp4pony-source.tar.gz', 'gmp4pony')):
        retained_archive = old_build / name
        pins[str(retained_archive)] = preparation.sha(retained_archive)
        pins = preparation.merge(pins, archive_correspondence(preparation, retained_archive, old_build / directory))
    compiler_image = PRIOR / 'bootstrap/images/dependency-review.json'
    image = json.loads(compiler_image.read_text())
    assert 'arm64' in image['architectures']
    compiler = old_build / 'release/0.74.0-95ad180/bin/ponyc'
    assert image['binary'] == str(compiler)
    compiler_receipt = json.loads((PRIOR / 'bootstrap/ponyc-version.json').read_text())
    assert compiler_receipt['command'] == [str(compiler), '--version']
    assert compiler_receipt['exit_status'] == compiler_receipt['final_exit_status'] == 0
    assert compiler_receipt['inherited_common_mutex'] is True and not compiler_receipt.get('error')
    assert '0.74.0' in (PRIOR / 'bootstrap/ponyc-version.stdout').read_text()
    pins = preparation.merge(pins, preparation.pin_tree(PRIOR / 'bootstrap'))
    evidence = dict(status='verified-exact-old-bounded-proof', prior_proposal=str(PRIOR_PROPOSAL),
                    prior_proposal_sha256=PRIOR_SHA, prior_report=str(report_path),
                    prior_report_sha256=pins[str(report_path)], native_cases_reexecuted=0,
                    old_cases_relabelled_as_new_attempts=False, profiles={}, pins=pins,
                    full_performance_runs=0, never_chart_eligible=True)
    for declared in prior_proposal['profiles']:
        identifier = declared['id']
        if identifier not in groups:
            continue
        group = groups[identifier]
        fingerprint = group['fingerprint']
        source = PRIOR / 'source-snapshot' / declared['source']
        binary = HERE / '.build/private-v4/bin' / identifier / identifier
        assert fingerprint['command'][0] == str(binary)
        require_identity(fingerprint, preparation.sha(source), preparation.sha(binary))
        require_identity(fingerprint, preparation.sha(HERE / declared['source']), preparation.sha(binary))
        pins[str(binary)] = fingerprint['binary_sha256']
        build_directory = PRIOR / 'builds' / identifier
        build_receipt = build_directory / 'build.json'
        build = json.loads(build_receipt.read_text())
        assert build['exit_status'] == build['final_exit_status'] == 0
        assert build['inherited_common_mutex'] is True and not build.get('error')
        expected_build_command = [str(compiler), '--strip', '--triple', 'aarch64-apple-macosx14.4.0',
            '--sysroot', prior_proposal['sdk'], '--lib-path', str(Path(prior_proposal['sdk']) / 'usr/lib'),
            '--lib-path', str(preparation.PCRE / 'lib'), '--lib-path', str(preparation.GMP / 'lib'),
            '--path', str(old_build / 'regex/regex-589eb008db250aab17d4c073a41c5562ac5b8dc7'),
            '--path', str(old_build / 'gmp4pony/gmp4pony-b14db4ba347f6a2b840f135e1602e19f85625c39'),
            '--path', str(PRIOR / 'source-snapshot/support-v2'),
            '--output', str(binary.parent), '--bin-name', identifier, str(source.parent)]
        assert build['command'] == expected_build_command, 'Old compiler/package/source command changed'
        image_path = build_directory / 'images/dependency-review.json'
        images = json.loads(image_path.read_text())
        assert images['binary'] == str(binary) and 'arm64' in images['architectures']
        for dependency in images['dependencies']:
            if 'path' in dependency:
                for path in (dependency['path'], dependency['lookup_path']):
                    assert preparation.sha(path) == dependency['sha256']
                    pins[path] = dependency['sha256']
        for path in build_directory.rglob('*'):
            if path.is_file():
                pins[str(path)] = preparation.sha(path)
        records = []
        for case, attempt in sorted(group['cases'].items()):
            directory = PRIOR / identifier / case
            assert preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
            assert preparation.sha(directory / 'expected.stdout') == attempt['stdout_sha256']
            raw = Path(attempt['result_path'])
            supervisor = Path(attempt['supervisor_result_path'])
            assert raw.is_relative_to(PRIOR / 'attempts') and supervisor.is_relative_to(PRIOR / 'attempts')
            original = json.loads(raw.read_text())
            enriched = json.loads(supervisor.read_text())
            validate_observations(original)
            validate_observations(enriched)
            assert all(original[key] == enriched[key] == attempt[key] for key in OBSERVATION_FIELDS)
            assert enriched['supervised_native_group'] == attempt['supervised_native_group']
            assert enriched['sampler_wait_status'] == attempt['sampler_wait_status'] == 0
            assert original['command'] == attempt['command']
            assert original['exit_status'] == 0 and original['stdout_sha256'] == attempt['stdout_sha256']
            assert original['source_sha256'] == attempt['source_sha256']
            assert original['runtime_environment'] == attempt['runtime_environment']
            assert original['execution_fingerprint'] == attempt['execution_fingerprint']
            assert original['pins_unchanged_at_end'] is True and original['valid_environment'] is True
            assert enriched['qualified'] is True and enriched['execution_fingerprint'] == attempt['execution_fingerprint']
            assert enriched['qualification_checks'] == attempt['qualification_checks']
            for path in list(directory.iterdir()) + [raw, supervisor]:
                if path.is_file():
                    pins[str(path)] = preparation.sha(path)
            records.append(dict(case=case, prior_attempt=attempt['attempt'], raw_result=str(raw),
                                supervisor_result=str(supervisor), stdout_sha256=attempt['stdout_sha256']))
        evidence['profiles'][identifier] = dict(declaration=declared, source=str(source), binary=str(binary),
            source_sha256=fingerprint['source_sha256'], binary_sha256=fingerprint['binary_sha256'],
            execution_fingerprint=attempt['execution_fingerprint'],
            command=fingerprint['command'], cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'],
            original_build_command=build['command'], prior_records=records,
            qualification_origin='exact-unchanged-v4-binary-and-records', new_native_cases=0)
    guard.verify(pins)
    return evidence


def reconstruct(preparation, guard):
    """Combine disjoint immutable v1/v3 proofs and v4 compact-K without new executions."""
    first = original.reconstruct(preparation, guard)
    second = reconstruct_new(preparation, guard)
    assert set(first['profiles']) == original.PROFILES
    assert set(second['profiles']) == NEW_PROFILES
    assert not set(first['profiles']) & set(second['profiles'])
    pins = preparation.merge(first['pins'], second['pins'])
    guard.verify(pins)
    profiles = dict(first['profiles'], **second['profiles'])
    assert set(profiles) == PROFILES and sum(len(row['prior_records']) for row in profiles.values()) == 110
    return dict(status='verified-exact-old-bounded-proof', prior_reports=first['prior_reports'] + [second['prior_report']],
        lineage=first['lineage'] + [{key: second[key] for key in ('prior_proposal', 'prior_proposal_sha256', 'prior_report', 'prior_report_sha256')}], profiles=profiles, pins=pins, reused_bounded_cases=110,
        native_cases_reexecuted=0, old_cases_relabelled_as_new_attempts=False,
        full_performance_runs=0, never_chart_eligible=True)


def validate_prior_reports(reuse):
    assert len(reuse['prior_reports']) == 3
    original.validate_prior_reports(dict(prior_reports=reuse['prior_reports'][:2]))
    validate_records(json.loads(Path(reuse['prior_reports'][2]).read_text()))
