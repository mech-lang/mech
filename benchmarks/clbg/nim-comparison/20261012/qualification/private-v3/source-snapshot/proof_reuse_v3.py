"""Reuse exact prior bounded executions, never a newly compiled binary."""
import hashlib
import json
from pathlib import Path
import tarfile

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v2'
PRIOR_PROPOSAL = HERE / 'proposals/private-v2.json'
PRIOR_SHA = '76e37273079d8a9e8c8541cbe67a36b96a64753d337670a03bf63cd01c65da52'
PROFILES = {f'nim-{benchmark}-{variant}' for benchmark in ('regexredux', 'revcomp')
            for variant in ('compact', 'optimized')} | {'nim-pidigits-compact'}
CASES = {'regexredux': {'all-replacements', 'records', 'header-eof', 'replacement-boundaries', 'empty'},
         'revcomp': {'iupac', 'comment-crlf-eof', 'tail', 'empty', 'header-eof'},
         'pidigits': {'n-' + str(n) for n in (0, 1, 9, 10, 11, 20, 21, 27, 97)}}
EXPECTED_GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}


def validate_records(report):
    assert report['status'] == 'stopped-unqualified', 'Preserve the terminal failed revision'
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert len(report['attempts']) == 29, 'Only the exact twenty-nine prior cases are reusable'
    groups = {}
    for attempt in report['attempts']:
        identifier = attempt['id']
        assert identifier in PROFILES
        benchmark = attempt['benchmark']
        assert identifier.startswith('nim-' + benchmark + '-') and benchmark in CASES
        assert attempt['case'] in CASES[benchmark]
        if benchmark == 'pidigits':
            assert attempt['workload_n'] == int(attempt['case'][2:]), 'Do not relabel a Pi size'
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
        assert command[1:] == ['{n}']
        assert attempt['command'] == [command[0], str(attempt['workload_n'])]
        assert attempt['supervised_native_group']['command'] == attempt['command']
        assert attempt['supervised_native_group']['inherited_common_mutex'] is True
        assert fingerprint['source_sha256'] == attempt['source_sha256']
        assert fingerprint['environment'] == attempt['runtime_environment']
        assert fingerprint['cwd'] == attempt['cwd']
        group = groups.setdefault(identifier, dict(benchmark=benchmark, cases={}, fingerprint=fingerprint))
        assert group['fingerprint'] == fingerprint
        assert attempt['case'] not in group['cases'], 'Never replace a case with a duplicate'
        group['cases'][attempt['case']] = attempt
    assert set(groups) == PROFILES
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
    return dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='nim',
                source=prior['source'], source_sha256=prior['source_sha256'],
                binary_sha256=prior['binary_sha256'], command=prior['command'], cwd=prior['cwd'],
                runtime_environment=prior['runtime_environment'],
                qualification_origin=prior['qualification_origin'],
                prior_bounded_records=prior['prior_records'], new_native_cases=0)


def reconstruct(preparation, guard, harness, output, lock_fd):
    """Caller owns mutex; new image inspections reference exact old binaries."""
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    prior_proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(prior_proposal['pins'])
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    groups = validate_records(report)
    old_build = HERE / '.build/private-v2'
    pins = preparation.merge(prior_proposal['pins'], {str(PRIOR_PROPOSAL): PRIOR_SHA,
        str(report_path): preparation.sha(report_path)}, preparation.pin_tree(PRIOR / 'source-snapshot'),
        preparation.pin_tree(PRIOR / 'builds'), preparation.pin_tree(PRIOR / 'bootstrap'))
    archive = old_build / 'nim-2.2.12-macosx_arm64.tar.xz'
    assert preparation.sha(archive) == preparation.NIM_SHA
    for name, directory in (('nim-2.2.12-macosx_arm64.tar.xz', 'nim'),
                            ('bigints-source.tar.gz', 'bigints'), ('nim-gmp-source.tar.gz', 'nim-gmp')):
        retained_archive = old_build / name
        pins[str(retained_archive)] = preparation.sha(retained_archive)
        pins = preparation.merge(pins, archive_correspondence(preparation, retained_archive, old_build / directory))
    compiler = old_build / 'nim/nim-2.2.12/bin/nim'
    toolchain = compiler.parent.parent
    bigint = old_build / 'bigints/bigints-3b5a106b88ec1baa904a059a1430c8335d885075'
    gmp_wrapper = old_build / 'nim-gmp/nim-gmp-4e9a6a05bfc274abd02a241f2d7c9162b2300191'
    version = json.loads((PRIOR / 'bootstrap/nim-version.json').read_text())
    assert version['command'] == [str(compiler), '--version']
    assert version['exit_status'] == version['final_exit_status'] == 0
    assert version['inherited_common_mutex'] is True and not version.get('error')
    assert (PRIOR / 'bootstrap/nim-version.stdout').read_text().startswith('Nim Compiler Version 2.2.12')
    environment = dict(LC_ALL='C', DYLD_LIBRARY_PATH=preparation.LIBRARY_PATH)
    roots = [old_build / 'nim', preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent]
    pins = preparation.merge(pins, preparation.native_images(guard, harness, compiler, environment,
        output / 'compiler-images', lock_fd, roots))
    evidence = dict(status='verified-exact-old-bounded-proof', prior_proposal=str(PRIOR_PROPOSAL),
        prior_proposal_sha256=PRIOR_SHA, prior_report=str(report_path),
        prior_report_sha256=pins[str(report_path)], native_cases_reexecuted=0,
        old_cases_relabelled_as_new_attempts=False, profiles={},
        full_performance_runs=0, never_chart_eligible=True)
    for declared in prior_proposal['profiles']:
        identifier = declared['id']
        if identifier not in groups:
            continue
        group = groups[identifier]
        fingerprint = group['fingerprint']
        source = PRIOR / 'source-snapshot' / declared['source']
        binary = old_build / 'bin' / identifier
        assert fingerprint['command'][0] == str(binary)
        require_identity(fingerprint, preparation.sha(source), preparation.sha(binary))
        require_identity(fingerprint, preparation.sha(HERE / declared['source']), preparation.sha(binary))
        pins[str(binary)] = fingerprint['binary_sha256']
        build = json.loads((PRIOR / 'builds' / identifier / 'build.json').read_text())
        assert build['exit_status'] == build['final_exit_status'] == 0
        assert build['inherited_common_mutex'] is True and not build.get('error')
        expected_command = preparation.build_command(compiler, toolchain, bigint, gmp_wrapper,
            Path(prior_proposal['sdk']), source, binary, old_build / 'cache' / identifier)
        assert build['command'] == expected_command, 'Old compiler/package/source command changed'
        pins = preparation.merge(pins, preparation.native_images(guard, harness, binary,
            fingerprint['environment'], output / identifier / 'images', lock_fd, roots))
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
            execution_fingerprint=attempt['execution_fingerprint'], command=fingerprint['command'],
            cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'],
            original_build_command=build['command'], prior_records=records,
            qualification_origin='exact-unchanged-v2-binary-and-records', new_native_cases=0)
    pins = preparation.merge(pins, preparation.pin_tree(output))
    guard.verify(pins)
    evidence['pins'] = pins
    return evidence
