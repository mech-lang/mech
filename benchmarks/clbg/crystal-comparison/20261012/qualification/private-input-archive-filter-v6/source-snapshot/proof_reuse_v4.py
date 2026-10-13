"""Reuse exact Crystal layout-v2 binaries and bounded records, never rerun them."""
import hashlib
import json
import math
import os
from pathlib import Path
import tarfile

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-layout-v2'
PRIOR_PROPOSAL = HERE / 'proposals/private-layout-v2.json'
PRIOR_SHA = 'a3c202e1d21c052263f0bf7d1ed1aa2143ce4a3f9e1a77b3a1e75301aec57c93'
NUMERIC = {
    'nbody': (0, 1, 2, 10000),
    'mandelbrot': (0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 65, 129, 200),
    'spectralnorm': (1, 2, 3, 5, 9, 100),
    'binarytrees': (0, 4, 7, 9),
    'fannkuchredux': (1, 2, 3, 4, 5, 6, 7, 8),
    'fasta': (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000),
}
CASES = {benchmark: {'n-' + str(n) for n in sizes} for benchmark, sizes in NUMERIC.items()}
CASES['revcomp'] = {'iupac', 'comment-crlf-eof', 'tail', 'empty', 'header-eof'}
CASES['knucleotide'] = {'ties-and-stop', 'one-base', 'empty', 'missing', 'crlf-comment-eof',
                       'embedded-header-comment', 'arbitrary-widths', 'header-eof'}
PROFILES = {f'crystal-{benchmark}-{variant}' for benchmark in (*NUMERIC, 'revcomp')
            for variant in ('compact', 'optimized')} | {'crystal-knucleotide-compact'}
EXPECTED_GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}
OBSERVATION_FIELDS = ('exit_status', 'seconds', 'power_at_start', 'power_at_end',
    'competing_compilers', 'power_changes', 'timeout_seconds', 'maximum_rss_mib',
    'peak_process_group_rss_kib', 'valid_environment', 'pins_unchanged_at_end')
FATAL_FIELDS = ('error', 'resource_violation', 'monitor_errors', 'resource_monitor_errors',
    'cleanup_error', 'supervisor_cleanup_error', 'remaining_group_members', 'supervisor_remaining_members',
    'supervisor_pin_failure', 'source_work_failure', 'source_work_error', 'preparation_supervisor_failure',
    'sampler_failure', 'sampler_error', 'registration_error', 'supervisor_error',
    'monitor_finish_error', 'supervisor_cleanup')


def validate_observations(attempt):
    assert type(attempt['exit_status']) is int and attempt['exit_status'] == 0
    assert type(attempt['seconds']) in (int, float) and math.isfinite(attempt['seconds']) and attempt['seconds'] > 0
    assert attempt['power_at_start'] == attempt['power_at_end'] == dict(source='AC Power', low_power_mode=0)
    assert attempt['competing_compilers'] == attempt['power_changes'] == []
    assert attempt['timeout_seconds'] == 300 and attempt['maximum_rss_mib'] == 10240
    assert type(attempt['peak_process_group_rss_kib']) is int and 0 <= attempt['peak_process_group_rss_kib'] <= 10240 * 1024
    assert attempt['pins_unchanged_at_end'] is True and attempt['valid_environment'] is True
    assert not any(attempt.get(field) for field in FATAL_FIELDS)


def verify_saved_observations(original, enriched, ledger):
    for saved in (original, enriched, ledger):
        validate_observations(saved)
    assert all(original[field] == enriched[field] == ledger[field] for field in OBSERVATION_FIELDS)
    assert enriched['supervised_native_group'] == ledger['supervised_native_group']
    assert type(enriched['sampler_wait_status']) is int and enriched['sampler_wait_status'] == ledger['sampler_wait_status'] == 0


def validate_records(report):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == PRIOR_SHA, 'Do not relabel the original consumed proposal'
    assert report['error_type'] == 'FileExistsError' and 'crystal-knucleotide-compact/empty' in report['error']
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert report['never_chart_eligible'] is True and len(report['attempts']) == 112
    groups = {}
    for number, attempt in enumerate(report['attempts'], 1):
        validate_observations(attempt)
        identity = attempt['id']
        assert identity in PROFILES and attempt['attempt'] == number
        benchmark = attempt['benchmark']
        assert identity.startswith('crystal-' + benchmark + '-') and benchmark in CASES
        assert attempt['case'] in CASES[benchmark]
        if benchmark in NUMERIC:
            assert attempt['workload_n'] == int(attempt['case'][2:]), 'Never relabel a numeric bounded case'
        checks = attempt['qualification_checks']
        assert isinstance(checks, dict) and set(checks) == EXPECTED_GATES
        assert attempt['qualified'] is True and all(value is True for value in checks.values())
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
        assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
        assert type(attempt['seconds']) in (int, float) and math.isfinite(attempt['seconds']) and attempt['seconds'] > 0
        assert attempt['pins_unchanged_at_end'] is True and attempt['valid_environment'] is True
        assert attempt['power_at_start'] == attempt['power_at_end'] == dict(source='AC Power', low_power_mode=0)
        assert attempt['competing_compilers'] == attempt['power_changes'] == []
        assert attempt['timeout_seconds'] == 300 and attempt['maximum_rss_mib'] == 10240
        assert type(attempt['peak_process_group_rss_kib']) is int and 0 <= attempt['peak_process_group_rss_kib'] <= 10240 * 1024
        assert not any(attempt.get(field) for field in (
            'error', 'resource_violation', 'monitor_errors', 'resource_monitor_errors', 'cleanup_error',
            'supervisor_cleanup_error', 'remaining_group_members', 'supervisor_remaining_members',
            'supervisor_pin_failure', 'source_work_failure', 'preparation_supervisor_failure',
            'sampler_failure', 'sampler_error', 'registration_error', 'supervisor_error',
            'monitor_finish_error', 'supervisor_cleanup'))
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes']
        assert attempt['validation']['sha256'] == attempt['stdout_sha256']
        fingerprint = json.loads(attempt['execution_fingerprint'])
        command = fingerprint['command']
        assert len(command) == 2 and command[1] == '{n}'
        assert attempt['command'] == [command[0], str(attempt['workload_n'])]
        assert attempt['supervised_native_group']['command'] == attempt['command']
        assert attempt['supervised_native_group']['inherited_common_mutex'] is True
        native = attempt['supervised_native_group']
        assert type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
        assert fingerprint['source_sha256'] == attempt['source_sha256']
        assert fingerprint['environment'] == attempt['runtime_environment'] and fingerprint['cwd'] == attempt['cwd']
        group = groups.setdefault(identity, dict(benchmark=benchmark, cases={}, fingerprint=fingerprint))
        assert group['fingerprint'] == fingerprint, 'A profile may not silently change source/binary/environment'
        assert attempt['case'] not in group['cases'], 'Never replace or invent a duplicated case'
        group['cases'][attempt['case']] = attempt
    assert set(groups) == PROFILES and len(groups) == 15
    assert all(set(group['cases']) == CASES[group['benchmark']] for group in groups.values())
    return groups


def require_identity(fingerprint, source_sha256, binary_sha256):
    assert source_sha256 == fingerprint['source_sha256'], 'Changed source cannot reuse old output proof'
    assert binary_sha256 == fingerprint['binary_sha256'], 'Newly compiled binary cannot inherit old executions'


def archive_correspondence(preparation, archive, directory):
    """Check regular payloads and exact link metadata without following cycles."""
    pins = {}
    root = directory.resolve()
    expected_paths = set()
    with tarfile.open(archive) as bundle:
        for member in bundle:
            relative = Path(member.name)
            assert not relative.is_absolute() and '..' not in relative.parts, 'Archive member escapes its private root'
            target = root / relative
            assert target.is_relative_to(root)
            expected_paths.add(target)
            # A payload below a substituted directory symlink would otherwise
            # inspect a different tree even when its lexical name stays local.
            parent = target.parent
            while parent != root:
                assert parent.is_relative_to(root) and parent.is_dir() and not parent.is_symlink(), 'Archive member parent changed type'
                expected_paths.add(parent)
                parent = parent.parent
            if member.issym():
                # Official foreign-target lib_c entries can be self-referential.
                # Follow neither these links nor their targets while checking
                # their immutable archive metadata.
                assert target.is_symlink() and os.readlink(target) == member.linkname
                link = Path(member.linkname)
                assert not link.is_absolute(), 'Absolute archive symlink target'
                destination = Path(os.path.abspath(target.parent / link))
                assert destination.is_relative_to(root), 'Relative symlink target escapes private archive'
            elif member.isfile():
                assert target.is_file() and not target.is_symlink(), 'Regular archive payload changed file type'
                assert target.resolve().is_relative_to(root)
                with bundle.extractfile(member) as stream:
                    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
                assert preparation.sha(target) == digest, ('Retained release no longer matches its official archive', str(target))
                pins[str(target)] = pins[str(target.resolve())] = digest
            elif member.islnk():
                link = Path(member.linkname)
                assert not link.is_absolute() and '..' not in link.parts
                destination = root / link
                assert destination.is_relative_to(root) and destination.is_file() and not destination.is_symlink()
                assert destination.resolve().is_relative_to(root)
                assert target.is_file() and not target.is_symlink()
                assert os.path.samefile(target, destination), 'Archive hardlink topology changed'
                pins[str(target)] = preparation.sha(target)
            else:
                assert member.isdir() and target.is_dir() and not target.is_symlink(), 'Unexpected archive member type'
    observed_paths = set()
    def failed_walk(error):
        raise error
    # Walk link entries as names, never recurse through directory symlinks.
    for parent, directories, files in os.walk(root, followlinks=False, onerror=failed_walk):
        observed_paths.update(Path(parent) / name for name in directories + files)
    assert observed_paths == expected_paths, 'Retained official release has extra or missing entries'
    assert pins, 'Empty retained archive cannot establish compiler provenance'
    return pins


def row_from_evidence(prior):
    fingerprint = json.loads(prior['execution_fingerprint'])
    require_identity(fingerprint, prior['source_sha256'], prior['binary_sha256'])
    row = dict(prior['declaration'], key=prior['declaration']['benchmark'] + ':' + prior['declaration']['id'],
        language='crystal', source=prior['source'], source_sha256=prior['source_sha256'],
        binary=prior['binary'], binary_sha256=prior['binary_sha256'], command=prior['command'],
        cwd=prior['cwd'], runtime_environment=prior['runtime_environment'],
        qualification_origin='exact-unchanged-layout-v2-binary-and-records',
        prior_bounded_records=prior['prior_records'], new_native_cases=0)
    assert row['command'] == fingerprint['command'] and row['command'][0] == row['binary']
    assert row['cwd'] == fingerprint['cwd'] and row['runtime_environment'] == fingerprint['environment']
    return row


def reconstruct(preparation, guard, harness, output, lock_fd):
    """Metadata-only common-lock review of fifteen exact old binaries/112 rows."""
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(proposal['pins'])
    report_path = PRIOR / 'report.json'
    groups = validate_records(json.loads(report_path.read_text()))
    old_build = HERE / '.build/private-layout-v2'
    archive = old_build / 'crystal-1.21.1-1-darwin-universal.tar.gz'
    assert preparation.sha(archive) == preparation.ARCHIVE_SHA
    pins = preparation.merge(proposal['pins'], {str(PRIOR_PROPOSAL): PRIOR_SHA,
        str(report_path): preparation.sha(report_path), str(archive): preparation.ARCHIVE_SHA},
        archive_correspondence(preparation, archive, old_build / 'release'),
        preparation.pin_tree(PRIOR / 'source-snapshot'), preparation.pin_tree(PRIOR / 'builds'),
        preparation.pin_tree(PRIOR / 'bootstrap'))
    compiler, stdlib, libraries, wrapper = preparation.package_layout(old_build / 'release')
    version = json.loads((PRIOR / 'bootstrap/compiler-version.json').read_text())
    assert version['command'] == [str(compiler), '--version']
    assert version['exit_status'] == version['final_exit_status'] == 0 and version['inherited_common_mutex'] is True
    assert not version.get('error') and (PRIOR / 'bootstrap/compiler-version.stdout').read_text().startswith('Crystal 1.21.1')
    runtime_environment, environment = preparation.runtime_environments(stdlib, libraries)
    pins = preparation.merge(pins, preparation.native_images(guard, harness, compiler, environment,
        output / 'compiler-images', lock_fd, [old_build / 'release', preparation.GMP, preparation.PCRE2]))
    references = preparation.load_module('crystal_reuse_independent_references', preparation.V / 'references.py')
    dna = preparation.load_module('crystal_reuse_exact_dna', preparation.V / 'dna_oracles.py')
    references.knucleotide = dna.knucleotide
    evidence = dict(status='verified-exact-old-bounded-proof', prior_proposal=str(PRIOR_PROPOSAL),
        prior_proposal_sha256=PRIOR_SHA, prior_report=str(report_path), prior_report_sha256=pins[str(report_path)],
        preserved_terminal_failure='Input-case metadata collision before another native launch; not a native/output failure',
        profiles={}, native_cases_reexecuted=0, prior_bounded_cases=112, fresh_profile_builds_required=5,
        old_cases_relabelled_as_new_attempts=False, full_performance_runs=0, never_chart_eligible=True,
        retained_build=str(old_build), compiler=str(compiler), source_root=str(stdlib),
        library_paths=[str(p) for p in libraries], runtime_environment=runtime_environment)
    for declared in proposal['profiles']:
        identity = declared['id']
        if identity not in groups:
            continue
        group = groups[identity]
        fingerprint = group['fingerprint']
        source = PRIOR / 'source-snapshot' / declared['source']
        binary = old_build / 'bin' / identity
        assert fingerprint['command'][0] == str(binary)
        require_identity(fingerprint, preparation.sha(source), preparation.sha(binary))
        require_identity(fingerprint, preparation.sha(HERE / declared['source']), preparation.sha(binary))
        pins[str(binary)] = fingerprint['binary_sha256']
        build = json.loads((PRIOR / 'builds' / identity / 'build.json').read_text())
        assert build['exit_status'] == build['final_exit_status'] == 0 and build['inherited_common_mutex'] is True and not build.get('error')
        assert build['command'] == preparation.build_command(compiler, preparation.SDK, libraries, source, binary), 'Old compile flags/source/tool changed'
        pins = preparation.merge(pins, preparation.pin_tree(old_build / 'cache' / identity),
            preparation.native_images(guard, harness, binary, environment, output / identity / 'images', lock_fd,
                                     [old_build / 'release', preparation.GMP, preparation.PCRE2]))
        preserved = []
        for case, attempt in sorted(group['cases'].items()):
            directory = PRIOR / identity / case
            actual = (directory / 'actual.stdout').read_bytes()
            expected = (directory / 'expected.stdout').read_bytes()
            if group['benchmark'] in NUMERIC:
                canonical = getattr(references, group['benchmark'])(int(case[2:]))
            else:
                payload = dict(preparation.inputs(group['benchmark'], references))[case]
                canonical = getattr(references, group['benchmark'])(payload)
                assert (directory / 'stdin.fasta').read_bytes() == payload, 'Old bounded input bytes changed'
            assert expected == canonical, 'Saved oracle does not reconcile independently'
            if group['benchmark'] == 'nbody':
                harness.compact.common.validate(actual.decode(), [float(x) for x in expected.splitlines()])
            else:
                assert actual == canonical, 'Old complete output differs from its independent oracle'
            assert preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
            raw = Path(attempt['result_path'])
            supervisor = Path(attempt['supervisor_result_path'])
            assert raw.is_relative_to(PRIOR / 'attempts') and supervisor.is_relative_to(PRIOR / 'attempts')
            original, enriched = json.loads(raw.read_text()), json.loads(supervisor.read_text())
            verify_saved_observations(original, enriched, attempt)
            for saved in (original, enriched):
                for field in ('command', 'source_sha256', 'runtime_environment', 'execution_fingerprint', 'stdout_bytes', 'stdout_sha256'):
                    assert saved[field] == attempt[field], ('Saved actual row differs', identity, case, field)
                assert saved['exit_status'] == 0 and saved['pins_unchanged_at_end'] is True and saved['valid_environment'] is True
            assert enriched['qualified'] is True and enriched['qualification_checks'] == attempt['qualification_checks']
            assert attempt['runtime_environment'] == runtime_environment
            for path in list(directory.iterdir()) + [raw, supervisor]:
                if path.is_file():
                    pins[str(path)] = preparation.sha(path)
            preserved.append(dict(case=case, prior_attempt=attempt['attempt'], raw_result=str(raw),
                supervisor_result=str(supervisor), stdout_sha256=attempt['stdout_sha256']))
        row = dict(declared, key=declared['benchmark'] + ':' + identity, language='crystal',
            source=str(source), source_sha256=fingerprint['source_sha256'], binary=str(binary),
            binary_sha256=fingerprint['binary_sha256'], command=fingerprint['command'], cwd=fingerprint['cwd'],
            runtime_environment=fingerprint['environment'])
        assert json.loads(harness.execution_fingerprint(row)) == fingerprint, 'Reconstructed row changed the original fingerprint'
        evidence['profiles'][identity] = dict(declaration=declared, source=str(source), binary=str(binary),
            source_sha256=fingerprint['source_sha256'], binary_sha256=fingerprint['binary_sha256'],
            execution_fingerprint=json.dumps(fingerprint, sort_keys=True), command=fingerprint['command'],
            cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'], prior_records=preserved,
            original_build_command=build['command'])
    assert set(evidence['profiles']) == PROFILES
    pins = preparation.merge(pins, preparation.pin_tree(output))
    guard.verify(pins)
    evidence['pins'] = pins
    return evidence
