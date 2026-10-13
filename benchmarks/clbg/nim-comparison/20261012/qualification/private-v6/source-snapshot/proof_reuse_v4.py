"""Reuse exact prior bounded executions, never a newly compiled binary."""
import hashlib
import json
import math
from pathlib import Path
import tarfile

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v3'
PRIOR_PROPOSAL = HERE / 'proposals/private-v3.json'
PRIOR_SHA = 'e7969a5420712e71c0b4b26a2302b5a7993077a8585e645c89144f089fa0bc16'
import proof_reuse_v3 as original
import library_compat_v3 as compatibility
NEW_PROFILES = {f'nim-{benchmark}-{variant}' for benchmark in ('spectralnorm', 'mandelbrot', 'fasta')
                for variant in ('compact', 'optimized')} | {'nim-pidigits-optimized', 'nim-knucleotide-compact'}
PROFILES = NEW_PROFILES | original.PROFILES
NUMERIC = {'pidigits': (0, 1, 9, 10, 11, 20, 21, 27, 97),
           'spectralnorm': (1, 2, 3, 5, 9, 100),
           'mandelbrot': (1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 65, 129, 200),
           'fasta': (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000)}
CASES = {benchmark: {'n-' + str(n) for n in values} for benchmark, values in NUMERIC.items()}
CASES['knucleotide'] = {'ties-and-stop', 'one-base', 'empty', 'missing', 'crlf-comment-eof',
                       'embedded-header-comment', 'arbitrary-widths', 'header-eof'}
PENDING_REUSED_CASES = {'nim-knucleotide-compact': ['empty-input', 'exact-token-cr', 'exact-token-controls', 'tail']}
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
    assert report['error_type'] == 'FileExistsError' and 'nim-knucleotide-compact/empty' in report['error']
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert len(report['attempts']) == 77, 'Only the exact seventy-seven v3 cases are reusable'
    groups = {}
    for number, attempt in enumerate(report['attempts'], 1):
        assert attempt['attempt'] == number
        validate_observations(attempt)
        identifier = attempt['id']
        assert identifier in NEW_PROFILES
        benchmark = attempt['benchmark']
        assert identifier.startswith('nim-' + benchmark + '-') and benchmark in CASES
        assert attempt['case'] in CASES[benchmark]
        if benchmark in NUMERIC:
            assert attempt['workload_n'] == int(attempt['case'][2:]), 'Numeric label must match actual input'
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


def archive_correspondence(preparation, archive, directory, records=None):
    """Verify the old extracted tool/package bytes against their retained archive."""
    files = {}
    with tarfile.open(archive) as bundle:
        for member in bundle:
            target = directory / member.name
            assert target.resolve().is_relative_to(directory.resolve()), 'Archive path escaped its private root'
            if member.isfile():
                with bundle.extractfile(member) as stream:
                    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
                if records is not None and str(target.resolve()) in records:
                    record = records[str(target.resolve())]
                    assert record['original_sha256'] == digest
                    original_path = Path(record['original_copy'])
                    assert preparation.sha(original_path) == digest
                    with bundle.extractfile(member) as stream:
                        original_bytes = stream.read()
                    assert original_path.read_bytes() == original_bytes
                    filename = next(name for name in compatibility.UNIONS if str(target).endswith('/' + name))
                    transformed = compatibility.normalize(original_bytes.decode(), filename).encode()
                    changed = hashlib.sha256(transformed).hexdigest()
                    assert record['changed_sha256'] == changed and preparation.sha(target) == changed
                    assert target.read_bytes() == transformed
                    files[str(target.resolve())] = changed
                else:
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


def compatibility_records(preparation):
    complete = json.loads((PRIOR / 'bootstrap/compatibility-complete.json').read_text())
    plan = json.loads((PRIOR / 'bootstrap/compatibility-plan.json').read_text())
    assert complete['status'] == 'exact-two-union-pragma-moves' and plan['status'] == 'planned-two-union-pragma-moves'
    assert complete['files'] == plan['files'] and len(complete['files']) == 2
    assert complete['production_source_changed'] is False and complete['general_library_compatibility_only'] is True
    assert complete['public_package_commit'] == '4e9a6a05bfc274abd02a241f2d7c9162b2300191'
    records = {}
    for record in complete['files']:
        target = Path(record['path']).resolve()
        assert target.is_relative_to(HERE / '.build/private-v3/nim-gmp')
        filename = next(name for name in compatibility.UNIONS if str(target).endswith('/' + name))
        assert (record['old_declaration'], record['new_declaration']) == compatibility.UNIONS[filename]
        assert record['arithmetic_changed'] is False and record['ffi_symbols_types_argument_order_changed'] is False
        retained = Path(record['original_copy'])
        assert retained.is_relative_to(PRIOR / 'bootstrap')
        assert preparation.sha(retained) == record['original_sha256']
        assert preparation.sha(target) == record['changed_sha256']
        assert str(target) not in records
        records[str(target)] = record
    assert len(records) == 2
    return records


def reconstruct_new(preparation, guard, harness, output, lock_fd):
    """Caller owns mutex; new image inspections reference exact old binaries."""
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    prior_proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(prior_proposal['pins'])
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    groups = validate_records(report)
    old_build = HERE / '.build/private-v3'
    pins = preparation.merge(prior_proposal['pins'], {str(PRIOR_PROPOSAL): PRIOR_SHA,
        str(report_path): preparation.sha(report_path)}, preparation.pin_tree(PRIOR / 'source-snapshot'),
        preparation.pin_tree(PRIOR / 'builds'), preparation.pin_tree(PRIOR / 'bootstrap'))
    archive = old_build / 'nim-2.2.12-macosx_arm64.tar.xz'
    assert preparation.sha(archive) == preparation.NIM_SHA
    for name, directory in (('nim-2.2.12-macosx_arm64.tar.xz', 'nim'),
                            ('bigints-source.tar.gz', 'bigints'), ('nim-gmp-source.tar.gz', 'nim-gmp')):
        retained_archive = old_build / name
        pins[str(retained_archive)] = preparation.sha(retained_archive)
        pins = preparation.merge(pins, archive_correspondence(preparation, retained_archive, old_build / directory,
            compatibility_records(preparation) if directory == 'nim-gmp' else None))
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
            execution_fingerprint=attempt['execution_fingerprint'], command=fingerprint['command'],
            cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'],
            original_build_command=build['command'], prior_records=records,
            qualification_origin='exact-unchanged-v3-binary-and-records', new_native_cases=0)
    pins = preparation.merge(pins, preparation.pin_tree(output))
    guard.verify(pins)
    evidence['pins'] = pins
    return evidence


def reconstruct(preparation, guard, harness, output, lock_fd):
    first = original.reconstruct(preparation, guard, harness, output / 'v2', lock_fd)
    second = reconstruct_new(preparation, guard, harness, output / 'v3', lock_fd)
    assert set(first['profiles']) == original.PROFILES and set(second['profiles']) == NEW_PROFILES
    assert not set(first['profiles']) & set(second['profiles'])
    pins = preparation.merge(first['pins'], second['pins'])
    guard.verify(pins)
    profiles = dict(first['profiles'], **second['profiles'])
    assert set(profiles) == PROFILES and sum(len(row['prior_records']) for row in profiles.values()) == 106
    import references
    for identifier, profile in profiles.items():
        row = profile_row(profile)
        assert json.loads(harness.execution_fingerprint(row)) == json.loads(profile['execution_fingerprint'])
        benchmark = profile['declaration']['benchmark']
        for record in profile['prior_records']:
            case = record['case']
            raw = Path(record['raw_result'])
            directory = raw.parent.parent / identifier / case
            if benchmark in preparation.NUMERIC:
                expected = getattr(references, benchmark)(int(case[2:]))
            else:
                payload = dict(preparation.inputs(benchmark, references))[case]
                assert (directory / 'stdin.fasta').read_bytes() == payload
                expected = getattr(references, benchmark)(payload)
            assert (directory / 'expected.stdout').read_bytes() == expected
            assert (directory / 'actual.stdout').read_bytes() == expected
    return dict(status='verified-exact-old-bounded-proof', prior_reports=[first['prior_report'], second['prior_report']],
        lineage=[{key: evidence[key] for key in ('prior_proposal', 'prior_proposal_sha256', 'prior_report', 'prior_report_sha256')}
                 for evidence in (first, second)], profiles=profiles, pins=pins, reused_bounded_cases=106,
        pending_reused_profile_cases=PENDING_REUSED_CASES, native_cases_reexecuted=0,
        old_cases_relabelled_as_new_attempts=False, full_performance_runs=0, never_chart_eligible=True)


def validate_prior_reports(reuse):
    assert len(reuse['prior_reports']) == 2
    original.validate_records(json.loads(Path(reuse['prior_reports'][0]).read_text()))
    validate_records(json.loads(Path(reuse['prior_reports'][1]).read_text()))
