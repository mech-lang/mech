"""Additive MATLAB fixture/runtime-input LOC proposals; no native subject launch."""
import argparse
import ast
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import sys
import sysconfig
import traceback

HERE = Path(__file__).resolve().parent
LOC = HERE.parent
CLBG = LOC.parents[1]
MAT = CLBG / 'matlab-comparison/20261012/regex-case-sensitive-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
OLD_FIXTURE = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
OLD_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
QUALIFICATION = MAT / 'qualification-v1/report.json'
QUALIFICATION_SHA = 'd192fcb065f229a7069816c515667004155fe16b813c7c2c5c9a8f8ab7b94b23'
PROPOSAL = MAT / 'proposal-sealed-v1.json'
PROPOSAL_SHA = '446e83d95ab413865c55ba6b27158e4873cb323df2679f2cad5e67883fff67bc'
MANIFEST = MAT / 'studies/full-v1/regexredux/manifest.json'
MANIFEST_SHA = '8e35f8f769c5d751348245b9a614c0ada15c62265d721b3d89abe7bc203ee443'
INDEX = MAT / 'studies/full-v1/index.json'
INDEX_SHA = 'c6a523e68d01e3d2280dca846ad5f98165e31528476d6365f7261efeb37c2a47'
FULL_REPORT = MANIFEST.parent / 'report.json'
SOURCE_DIGESTS = {
    'matlab-case-compact': 'd41b8687b3661174be4b7c58014f0cfaaf331f6a11024135cc5dc8a561bced99',
    'matlab-case-optimized': '0a59dbb911fd4e4aaa0aac8edd7380c8d5c834f4f0f3338337850b7ee3ee0846',
}
PACKET = ('foreign_matlab.py', 'test_foreign_matlab.py', 'qualify_replay.py', 'REVIEW.md', 'GRAMMAR_REVIEW.md')
METHODS = (
    'test_function_headers_parameters_and_written_closes',
    'test_for_inline_expanded_and_nested_owners',
    'test_actual_char_read_transpose_and_nonconjugate_transpose',
    'test_quoted_regex_text_and_doubled_quotes_keep_exact_spans',
    'test_line_and_alone_block_comments_do_not_own_code',
    'test_array_matrix_and_argument_separators_are_structural',
    'test_wrapped_literal_data_rows_are_preserved',
    'test_command_comma_and_semicolon_expose_independent_work',
    'test_initialized_bindings_clear_and_field_writes_are_work',
    'test_index_end_does_not_close_a_function_or_loop',
    'test_function_signature_and_iterator_cannot_hide_effect',
    'test_array_owner_cannot_hide_assignment_call_or_callback',
    'test_callbacks_and_dynamic_readers_are_diagnostic',
    'test_unsupported_controls_and_malformed_owners_are_gated',
    'test_unselected_continuation_ambiguous_quote_and_unclosed_text_gate',
    'test_selected_regex_shapes_keep_both_loops_and_all_body_work',
)
IDS = sorted('test_foreign_matlab.MatlabRegexpFixtures.' + method for method in METHODS)
GATES = {'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged'}
EMPTY = ('remaining_group_members', 'competing_compilers', 'power_changes', 'monitor_errors', 'resource_monitor_errors')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Exact input/evidence changed', path)


def merge(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest, ('Conflicting pin', path)
            result[path] = digest
    return result


def read(path):
    return json.loads(Path(path).read_text())


def new(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def now():
    return datetime.now(timezone.utc).isoformat()


@contextmanager
def terminal(revision, state):
    with LOCK.open('a+') as lock:
        print('WAIT common mutex: MATLAB grammar fixtures/exact runtime source replay', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = HERE / 'jobs' / revision
        output.parent.mkdir(parents=True, exist_ok=True)
        output.mkdir()  # Exclusive revision, including failures before imports.
        error = None
        try:
            yield output, lock
        except BaseException as failure:
            error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            with (output / 'terminal-error.log').open('x') as log:
                log.write(traceback.format_exc())
            raise
        finally:
            collected, pin_error = {}, None
            candidates = dict(state.get('pins', {}))
            candidates.update({str(path): None for path in output.rglob('*') if path.is_file()})
            for path, expected in candidates.items():
                try:
                    actual = sha(path)
                    if expected is not None: assert actual == expected, ('Terminal changed pin', path)
                    collected[path] = actual
                except BaseException as failure:
                    if pin_error is None:
                        pin_error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__, path=path)
            status = 'failed-terminal-integrity' if pin_error else ('failed-proposal-stage' if error else 'completed-proposal-stage')
            new(output / 'terminal.json', dict(status=status, body_error=error, integrity_error=pin_error,
                pins=collected, available_bytes_not_fully_pinned=pin_error is not None,
                exclusive_mutex_held=True, common_timing_mutex=str(LOCK), common_mutex_fd=lock.fileno(),
                actual_native_subjects=0, actual_signals_sent=0, canonical_LOC_admitted=False, chart_admission=False,
                finished_at=now()))
            if pin_error is not None: raise RuntimeError('Terminal integrity failure retained under common mutex: ' + str(pin_error))


def test_inventory(path):
    tree = ast.parse(Path(path).read_text())
    classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
    assert [node.name for node in classes] == ['MatlabRegexpFixtures']
    methods = [node.name for node in classes[0].body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    assert len(methods) == len(set(methods)) == 16
    assert sorted('test_foreign_matlab.MatlabRegexpFixtures.' + name for name in methods) == IDS


def fixture_valid(receipt):
    assert receipt['status'] == 'passed' and receipt['successful'] is True
    assert type(receipt['tests_run']) is type(receipt['discovered_tests']) is int
    assert receipt['tests_run'] == receipt['discovered_tests'] == 16
    for name in ('expected_test_ids', 'discovered_test_ids', 'actual_test_ids'):
        assert sorted(receipt[name]) == IDS and len(receipt[name]) == len(set(receipt[name])) == 16
    for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'): assert receipt[name] == []
    assert receipt['exclusive_mutex_held'] is True and receipt['common_timing_mutex'] == str(LOCK)
    assert type(receipt['actual_native_subjects']) is type(receipt['actual_signals_sent']) is int
    assert receipt['actual_native_subjects'] == receipt['actual_signals_sent'] == 0
    assert receipt['canonical_LOC_admitted'] is receipt['chart_admission'] is False


def copied(source, target, pins, expected=None):
    digest = sha(source)
    if expected is not None: assert digest == expected
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    assert sha(target) == digest
    pins[str(source)] = pins[str(target)] = digest
    target.chmod(0o444)


def prepare_modules(output, packet_pins, stage, fixture_path=None, fixture_sha=None, state=None):
    assert set(packet_pins) == {str(HERE / name) for name in PACKET}
    verify(packet_pins)
    assert sha(OLD_FIXTURE) == OLD_SHA
    old = read(OLD_FIXTURE)
    assert old['status'] == 'passed' and old['tests_run'] == 672 and old['failures'] == old['errors'] == 0
    assert old['exclusive_mutex_held'] is True and old['common_timing_mutex'] == str(LOCK)
    assert sys.version.split()[0] == old['python_version'].split()[0] and sys.version_info[:2] == (3, 14)
    assert old['pygments_version'] == '2.19.2'
    qualified = old['frozen_dependency_pins']
    dependencies = {str(OLD_FIXTURE.parent / 'snapshot' / name): qualified[str(OLD_FIXTURE.parent / 'snapshot' / name)]
                    for name in ('foreign_core.py', 'canonical.py')}
    verify(dependencies)
    pygments = Path(old['pygments_module']).resolve(strict=True)
    assert sha(pygments) == old['pygments_module_sha256']
    pins = merge(state.get('pins', {}) if state is not None else {}, packet_pins, dependencies,
                 {str(OLD_FIXTURE): OLD_SHA, str(pygments): old['pygments_module_sha256']})
    if state is not None: state['pins'] = pins
    if stage == 'replay':
        assert fixture_path.resolve().parent.parent == HERE / 'jobs'
        assert sha(fixture_path) == fixture_sha
        receipt = read(fixture_path); fixture_valid(receipt)
        assert receipt['python_version'].split()[0] == sys.version.split()[0]
        assert receipt['reviewed_packet_pins'] == packet_pins
        terminal_path = fixture_path.parent / 'terminal.json'
        completed = read(terminal_path)
        assert completed['status'] == 'completed-proposal-stage'
        assert completed['body_error'] is completed['integrity_error'] is None
        assert completed['available_bytes_not_fully_pinned'] is False
        assert completed['exclusive_mutex_held'] is True and completed['common_timing_mutex'] == str(LOCK)
        assert type(completed['actual_native_subjects']) is type(completed['actual_signals_sent']) is int
        assert completed['actual_native_subjects'] == completed['actual_signals_sent'] == 0
        assert completed['canonical_LOC_admitted'] is completed['chart_admission'] is False
        assert completed['pins'][str(fixture_path)] == fixture_sha
        assert completed['pins'][str(fixture_path.parent / 'fixtures.log')] == receipt['frozen_dependency_pins'][str(fixture_path.parent / 'fixtures.log')]
        assert all(completed['pins'][path] == digest for path, digest in receipt['frozen_dependency_pins'].items())
        verify(completed['pins'])
        verify(receipt['frozen_dependency_pins'])
        pins = merge(pins, receipt['frozen_dependency_pins'], completed['pins'],
            {str(fixture_path): fixture_sha, str(terminal_path): sha(terminal_path)})
        snapshot = fixture_path.parent / 'snapshot'
        neutral = fixture_path.parent / 'neutral-dependencies'
        unittest_snapshot = fixture_path.parent / 'python-dependencies/unittest'
    else:
        snapshot, neutral = output / 'snapshot', output / 'neutral-dependencies'
        for name in PACKET: copied(HERE / name, snapshot / name, pins, packet_pins[str(HERE / name)])
        for source, digest in dependencies.items(): copied(Path(source), neutral / Path(source).name, pins, digest)
        unittest_root = Path(sysconfig.get_path('stdlib')).resolve(strict=True) / 'unittest'
        unittest_snapshot = output / 'python-dependencies/unittest'
        for source in sorted(unittest_root.rglob('*.py')):
            copied(source, unittest_snapshot / source.relative_to(unittest_root), pins)
    pins = merge(pins, {str(path): sha(path) for path in pygments.parent.rglob('*.py')})
    if state is not None: state['pins'] = pins
    python = Path(sys.executable).resolve(strict=True)
    pins[str(python)] = sha(python)
    if stage == 'replay': assert pins[str(python)] == receipt['python_executable_sha256']
    assert not any(name in sys.modules for name in ('foreign_matlab', 'test_foreign_matlab', 'foreign_core', 'canonical', 'pygments'))
    assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
    assert not list(unittest_snapshot.rglob('*.pyc'))
    sys.dont_write_bytecode = True
    sys.path[:0] = [str(snapshot), str(neutral), str(unittest_snapshot.parent), str(pygments.parent.parent)]
    unittest = importlib.import_module('unittest')
    assert Path(unittest.__file__).resolve() == (unittest_snapshot / '__init__.py').resolve()
    normalizer = importlib.import_module('foreign_matlab')
    tests = importlib.import_module('test_foreign_matlab') if stage == 'fixtures' else None
    assert Path(normalizer.__file__).resolve() == (snapshot / 'foreign_matlab.py').resolve()
    if tests is not None: assert Path(tests.__file__).resolve() == (snapshot / 'test_foreign_matlab.py').resolve()
    for name in ('foreign_core', 'canonical'): assert Path(sys.modules[name].__file__).resolve() == (neutral / (name + '.py')).resolve()
    assert Path(sys.modules['pygments'].__file__).resolve() == pygments and sys.modules['pygments'].__version__ == '2.19.2'
    test_inventory(snapshot / 'test_foreign_matlab.py')
    verify(pins)  # Fresh import/discovery cannot change pinned source bytes.
    def identities():
        result = {}
        for name, module in tuple(sys.modules.items()):
            if name == 'unittest' or name.startswith('unittest.'):
                path = Path(module.__file__).resolve(strict=True)
                assert path.is_relative_to(unittest_snapshot) and str(path) in pins
                result[name] = str(path)
        assert result
        return result
    identities()
    return normalizer, tests, unittest, identities, pins, python


def fingerprint(row):
    # The frozen harness records {} here for its plain path->hash support map.
    # Actual launcher/native/JRE support pins are independently mandatory below.
    return json.dumps(dict(binary_sha256=row['binary_sha256'], command=row['command'], cwd=row.get('cwd'),
        environment=row.get('runtime_environment', {}), source_sha256=row['source_sha256'], support={}), sort_keys=True)


def native_lock_record(text, descriptor):
    # Bind engine name, descriptor and lock path within the same actual lsof
    # process/FD record, rather than combining facts from separate processes.
    records, current, fd = [], None, None
    for line in text.splitlines():
        if line.startswith('p'):
            current = dict(pid=line[1:], command=None, descriptors={})
            records.append(current); fd = None
        elif current is not None and line.startswith('c'):
            current['command'] = line[1:]
        elif current is not None and line.startswith('f'):
            fd = line[1:]; current['descriptors'][fd] = []
        elif current is not None and fd is not None and line.startswith('n'):
            current['descriptors'][fd].append(line[1:])
    return any(item['pid'].isdigit() and int(item['pid']) > 0 and item['command'] == 'MATLAB'
        and any(name in (str(LOCK), str(LOCK.resolve())) for name in item['descriptors'].get(str(descriptor), [])) for item in records)


def actual_attempt(attempt, row, full, ordinal, directory, pins):
    assert type(attempt['attempt']) is int and attempt['attempt'] == ordinal
    assert type(attempt['round']) is int and attempt['round'] == (0 if not full else ((ordinal - 1) // 3 + 1))
    assert type(attempt['workload_n']) is int and attempt['workload_n'] == (5000000 if full else 1000)
    assert attempt['full_workload'] is full and attempt['full_performance_run'] is full and attempt['chart_admission'] is False
    assert attempt['id'] == row['id'] and attempt['key'] == row['key'] and attempt['source_sha256'] == row['source_sha256']
    assert attempt['command'] == row['command'] and attempt['cwd'] == row['cwd']
    assert attempt['runtime_environment'] == row['runtime_environment'] and attempt['execution_fingerprint'] == fingerprint(row)
    assert attempt['qualified'] is True and set(attempt['qualification_checks']) == GATES
    assert all(value is True for value in attempt['qualification_checks'].values())
    assert type(attempt['sampler_wait_status']) is int and attempt['sampler_wait_status'] == 0
    raw_path = directory / 'attempts' / f'{ordinal:03d}-{row["id"]}.json'
    supervisor_path = raw_path.with_suffix('.supervisor.json')
    assert attempt['result_path'] == str(raw_path) and attempt['supervisor_result_path'] == str(supervisor_path)
    raw, supervisor = read(raw_path), read(supervisor_path)
    additions = {'case', 'preparation_only', 'full_performance_run', 'chart_admission'}
    assert supervisor == {key: value for key, value in attempt.items() if key not in additions}
    assert all(key in supervisor and supervisor[key] == value and type(supervisor[key]) is type(value) for key, value in raw.items())
    for record in (raw, supervisor, attempt):
        assert type(record['exit_status']) is int and record['exit_status'] == 0
        assert type(record['workload_n']) is int and record['workload_n'] == (5000000 if full else 1000)
        assert record['full_workload'] is full and record['valid_environment'] is True and record['pins_unchanged_at_end'] is True
        assert record['validation']['reference_match'] is True
        assert type(record['stdout_bytes']) is type(record['validation']['bytes']) is int
        assert record['stdout_bytes'] >= 0 and record['stdout_bytes'] == record['validation']['bytes']
        assert record['stdout_sha256'] == record['validation']['sha256']
        assert type(record['seconds']) in (int, float) and math.isfinite(record['seconds']) and record['seconds'] > 0
        assert type(record['peak_process_group_rss_kib']) is int and record['peak_process_group_rss_kib'] > 0
        assert all(record[name] == [] for name in EMPTY) and record['resource_violation'] is None
        assert all(not record.get(name) for name in ('error', 'execute_error', 'execute_cleanup_error', 'monitor_finish_error', 'wrapper_error', 'supervisor_error'))
    group = supervisor['supervised_native_group']
    assert group['type'] == 'native-group-started' and group['command'] == row['command'] and group['inherited_common_mutex'] is True
    assert type(group['pid']) is type(group['pgid']) is int and group['pid'] == group['pgid'] > 0
    assert supervisor['supervisor_remaining_members'] == [] and not supervisor.get('supervisor_cleanup')
    captured = {str(raw_path): sha(raw_path), str(supervisor_path): sha(supervisor_path)}
    for path, digest in captured.items():
        assert path not in pins or pins[path] == digest
        pins[path] = digest
    if full:
        assert raw['stdout_bytes'] == 298 and raw['stdout_sha256'] == 'a37e6da1d43b332d5c0f4fc25b76090e9095d4c97c25f1a7186385e87706e4af'
    else:
        case = directory / 'cases' / row['id'] / attempt['case']
        for name in ('stdin', 'expected.stdout', 'actual.stdout'):
            path = case / name
            assert str(path) in pins and sha(path) == pins[str(path)]
        assert (case / 'actual.stdout').read_bytes() == (case / 'expected.stdout').read_bytes()
        assert sha(case / 'actual.stdout') == raw['stdout_sha256']
        if row['language'] == 'matlab':
            proof = raw['bounded_mutex_fd_evidence']
            assert type(proof['descriptor']) is int and proof['descriptor'] >= 64
            assert proof['native_MATLAB_engine_observed'] is True and proof['errors'] == []
            observations = proof['observations']; assert observations
            assert all(type(item['native_group']) is int and item['native_group'] == group['pgid'] for item in observations)
            assert any(native_lock_record(item['lsof'], proof['descriptor']) for item in observations)
            saved_observations = sorted((case / 'descriptor-observer').glob('*.stdout'))
            assert saved_observations and all(str(path) in pins for path in saved_observations)
            assert all(any(path.read_text() == item['lsof'] for path in saved_observations) for item in observations)


def source_inventory(full_sha):
    immutable = {str(QUALIFICATION): QUALIFICATION_SHA, str(PROPOSAL): PROPOSAL_SHA,
                 str(MANIFEST): MANIFEST_SHA, str(INDEX): INDEX_SHA, str(FULL_REPORT): full_sha}
    assert re.fullmatch('[0-9a-f]{64}', full_sha)
    verify(immutable)
    qualified, proposal, manifest, index, full = (read(path) for path in (QUALIFICATION, PROPOSAL, MANIFEST, INDEX, FULL_REPORT))
    assert qualified['status'] == 'semantic-output-runtime-ownership-passed-awaiting-full-review-and-LOC'
    assert qualified['proposal'] == str(PROPOSAL) and qualified['proposal_sha256'] == PROPOSAL_SHA
    assert qualified['active'] is None and qualified['pins_unchanged_at_end'] is True
    assert qualified['full_performance_runs'] == 0 and qualified['chart_admission'] is False
    assert qualified['profiles'] == proposal['profiles']
    assert manifest['qualified_semantics'] == str(QUALIFICATION) and manifest['qualified_semantics_sha256'] == QUALIFICATION_SHA
    assert index['benchmarks'] == {'regexredux': {'manifest': str(MANIFEST), 'sha256': MANIFEST_SHA}}
    assert index['samples'] == 3 and index['full_processes'] == manifest['full_processes'] == 9
    assert full['status'] == 'complete' and full['manifest'] == str(MANIFEST) and full['manifest_sha256'] == MANIFEST_SHA
    assert full['active'] is None and full['pins_unchanged_at_end'] is True and full['skipped_failed_profile_slots'] == []
    pins = merge(immutable, qualified['pins'], manifest['pins'])
    verify(pins)
    rows = {row['id']: row for row in proposal['profiles']}
    assert len(rows) == len(proposal['profiles']) == 3 and set(rows) == {*SOURCE_DIGESTS, 'mech-aot-control'}
    source_paths = {MAT / 'sources' / ('regexredux_' + variant + '.m') for variant in ('compact', 'optimized')}
    assert set((MAT / 'sources').rglob('*')) == source_paths, 'No competing local helper, private/package/class, P/MEX/MLX source in the runtime folder'
    assert all(path.is_file() and not path.is_symlink() for path in source_paths)
    for row in proposal['profiles']:
        assert sha(row['source']) == row['source_sha256'] == pins[row['source']]
        assert sha(row['binary']) == row['binary_sha256'] == pins[row['binary']]
        if row['id'] in SOURCE_DIGESTS:
            variant = row['id'].rsplit('-', 1)[1]
            assert row['source'] == str(MAT / 'sources' / ('regexredux_' + variant + '.m'))
            assert row['source_sha256'] == SOURCE_DIGESTS[row['id']] and row['maximum_workers'] == 8
            assert row['command'] == ['/bin/sh', str(CLBG / 'matlab-comparison/20261010/matlab-stdin.sh'),
                '/Applications/MATLAB_R2026b.app/bin/matlab', '-licmode', 'onlinelicensing', '-nodisplay',
                '-sd', str(MAT / 'sources'), '-batch', 'regexredux_' + variant + '()']
            assert row['runtime_environment']['LC_ALL'] == 'C' and row['runtime_environment']['MATLABPATH'] == ''
            assert len(row['support']) == 5 and all(pins[path] == digest and sha(path) == digest for path, digest in row['support'].items())
        native = full['profiles'][row['id']]
        for key in ('key', 'id', 'source', 'source_sha256', 'binary', 'binary_sha256', 'command', 'cwd', 'runtime_environment'):
            assert native[key] == row[key]
    diagnostic = dict(rows['matlab-case-compact'])
    diagnostic.update(id='matlab-runtime-identity', key='regexredux:matlab-runtime-identity',
                      source=str(MAT / 'runtime_identity.m'), source_sha256=pins[str(MAT / 'runtime_identity.m')])
    diagnostic['command'] = list(diagnostic['command'])
    diagnostic['command'][7], diagnostic['command'][-1] = str(MAT), 'runtime_identity()'
    old_rows = []
    for original in proposal['original_profiles']:
        old = dict(original); old.update(id='old-witness-' + original['profile'], key='regexredux:old-witness-' + original['profile'], cwd=str(MAT))
        old_rows.append(old)
    bounded_rows = dict(rows, **{row['id']: row for row in [diagnostic, *old_rows]})
    expected = {(row['id'], case) for row in rows.values() for case in proposal['cases']}
    expected.update({('matlab-runtime-identity', 'runtime-identity'), ('old-witness-compact', 'first-pattern-case'), ('old-witness-optimized', 'first-pattern-case')})
    assert len(qualified['attempts']) == len(expected) == 18
    assert {(item['id'], item['case']) for item in qualified['attempts']} == expected
    for ordinal, attempt in enumerate(qualified['attempts'], 1):
        actual_attempt(attempt, bounded_rows[attempt['id']], False, ordinal, QUALIFICATION.parent, pins)
    identity = qualified['attempts'][0]
    assert identity['id'] == 'matlab-runtime-identity' and identity['stderr'] == 'CLBG_MATLAB_VERSION=26.2.0.3386108 (R2026b)\nCLBG_MATLAB_WORKERS=8\n'
    assert (QUALIFICATION.parent / 'cases/matlab-runtime-identity/runtime-identity/actual.stdout').read_bytes() == b'MATLAB R2026b native arm64 <=8 compute verified\n'
    orders = [dict(round=number + 1, profiles=list(rows)[number:] + list(rows)[:number]) for number in range(3)]
    assert manifest['orders'] == full['orders'] == orders and len(full['attempts']) == 9
    flattened = [identifier for order in orders for identifier in order['profiles']]
    assert [attempt['id'] for attempt in full['attempts']] == flattened
    assert set(full['profiles']) == set(rows)
    for ordinal, attempt in enumerate(full['attempts'], 1):
        actual_attempt(attempt, rows[attempt['id']], True, ordinal, MANIFEST.parent, pins)
    for identifier, row in full['profiles'].items():
        assert identifier in rows and row['measurement_status'] == 'measured'
        attempts = [item for item in full['attempts'] if item['id'] == identifier]
        assert len(attempts) == len(row['samples_seconds']) == len(row['sample_validation']) == 3
        assert row['samples_seconds'] == [item['seconds'] for item in attempts]
        assert row['sample_validation'] == [item['validation'] for item in attempts]
        assert row['attempts'] == [item['attempt'] for item in attempts]
    evidence = {identifier: dict(status='actual-runtime-function-inputs-bound-for-unadmitted-source-replay',
        source=rows[identifier]['source'], source_sha256=rows[identifier]['source_sha256'], authored_files=[dict(path=rows[identifier]['source'], sha256=rows[identifier]['source_sha256'], role='application')],
        command=rows[identifier]['command'], runtime_environment=rows[identifier]['runtime_environment'], runtime_support_pins=rows[identifier]['support'],
        complete_runtime_source_folder=[dict(path=str(path), sha256=pins[str(path)]) for path in sorted(source_paths)],
        runtime_qualification=dict(path=str(QUALIFICATION), sha256=QUALIFICATION_SHA, actual_bounded_attempts=18),
        full_study=dict(path=str(FULL_REPORT), sha256=full_sha, actual_full_samples=9),
        native_MATLAB_identity_source=dict(path=str(MAT / 'runtime_identity.m'), sha256=pins[str(MAT / 'runtime_identity.m')]),
        compiler_input_claim=False, source_function_resolution_basis='Exact -sd folder and batch function, no competing folder inputs, actual bounded/full source and native runtime receipts')
        for identifier in SOURCE_DIGESTS}
    verify(pins)
    return rows, sorted(source_paths), evidence, pins


def run(args):
    state = {'pins': {}}
    with terminal(args.revision, state) as (output, lock):
        packet_path = args.packet_pins.resolve(strict=True)
        state['pins'][str(packet_path)] = args.packet_pins_sha256
        assert sha(packet_path) == args.packet_pins_sha256
        packet_pins = read(packet_path)
        state['pins'] = merge(state['pins'], packet_pins)
        normalizer, tests, unittest, identities, pins, python = prepare_modules(output, packet_pins,
            args.stage, args.fixture_receipt, args.fixture_sha256, state)
        pins[str(packet_path)] = args.packet_pins_sha256
        state['pins'] = pins
        if args.stage == 'fixtures':
            receipt = dict(status='failed-fixtures', started_at=now(), expected_test_ids=IDS, actual_test_ids=[],
                discovered_test_ids=[], tests_run=0, discovered_tests=0, successful=False,
                common_timing_mutex=str(LOCK), exclusive_mutex_held=True, common_mutex_fd=lock.fileno(),
                python_version=sys.version, python_executable=str(python), python_executable_sha256=pins[str(python)],
                unittest_module_identities=identities(), reviewed_packet_pins=packet_pins, frozen_dependency_pins=pins,
                actual_native_subjects=0, actual_signals_sent=0, canonical_LOC_admitted=False, chart_admission=False)
            class RecordedResult(unittest.TextTestResult):
                def __init__(self, *values):
                    super().__init__(*values); self.started_ids = []
                def startTest(self, test):
                    self.started_ids.append(test.id()); super().startTest(test)
            try:
                suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
                def discovered(value):
                    for item in value:
                        if isinstance(item, unittest.TestSuite): yield from discovered(item)
                        else: yield item.id()
                receipt.update(discovered_tests=suite.countTestCases(), discovered_test_ids=list(discovered(suite)))
                assert sorted(receipt['discovered_test_ids']) == IDS
                verify(pins)
                log = output / 'fixtures.log'
                with log.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=RecordedResult).run(suite)
                pins[str(log)] = sha(log)
                receipt.update(tests_run=result.testsRun, actual_test_ids=result.started_ids, successful=result.wasSuccessful(),
                    failures=[(test.id(), detail) for test, detail in result.failures], errors=[(test.id(), detail) for test, detail in result.errors],
                    skipped=[(test.id(), detail) for test, detail in result.skipped], expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                    unexpected_successes=[test.id() for test in result.unexpectedSuccesses], unittest_module_identities=identities())
                receipt['status'] = 'passed'; fixture_valid(receipt); verify(pins)
                receipt['finished_at'] = now()
            except BaseException as error:
                receipt.update(status='failed-fixtures', error=str(error) or type(error).__name__, error_type=type(error).__name__, finished_at=now())
                raise
            finally:
                new(output / 'fixture-receipt.json', receipt)
            print(json.dumps(dict(fixture=str(output / 'fixture-receipt.json'), fixture_sha256=sha(output / 'fixture-receipt.json'), actual_tests=16, canonical_LOC_admitted=False)), flush=True)
            return
        rows, paths, runtime_evidence, native_pins = source_inventory(args.full_report_sha256)
        pins = merge(pins, native_pins); state['pins'] = pins
        metrics = {}
        for path in paths:
            metric = normalizer.normalize(path.read_text(), 'matlab')
            assert metric['proposal_only'] is True and metric['eligible_for_chart'] is False
            metrics[str(path)] = dict(source=str(path), source_sha256=pins[str(path)], **metric)
        unresolved = [dict(source=path, diagnostic=diagnostic) for path, metric in metrics.items() for diagnostic in metric['diagnostics']]
        identities(); verify(pins)
        new(output / 'source-proposals.json', dict(status='source-proposals-needing-grammar-review' if unresolved else 'source-proposals-awaiting-semantic-review',
            language='matlab', cohort='regex-case-sensitive-v1', unique_source_count=2, profile_count=2,
            fixture=dict(path=str(args.fixture_receipt), sha256=args.fixture_sha256), pins=pins,
            profiles=[dict(id=identifier, benchmark='regexredux', variant=identifier.rsplit('-', 1)[1], producer_loc=None,
                source_bundle=[metrics[rows[identifier]['source']]], physical_loc=metrics[rows[identifier]['source']]['physical_loc'],
                normalized_loc=metrics[rows[identifier]['source']]['normalized_loc']) for identifier in SOURCE_DIGESTS],
            runtime_input_evidence=runtime_evidence, unresolved=unresolved, created_at=now(),
            stream_launcher_role='Uniform stream transport only; exact preserved helper bytes pinned. fopen/fread/fclose authored in each counted MATLAB main.',
            fresh_mech_control=dict(id='mech-aot-control', source=rows['mech-aot-control']['source'], source_sha256=rows['mech-aot-control']['source_sha256'],
                binary=rows['mech-aot-control']['binary'], binary_sha256=rows['mech-aot-control']['binary_sha256'], correction_cohort='regex-case-sensitive-v1',
                preserved_archival_control_for=rows['mech-aot-control']['control_for']),
            actual_native_subjects=0, actual_signals_sent=0, full_performance_runs=0,
            canonical_LOC_admitted=False, chart_admission=False, producer_LOC_mutated=False, parser_pass_is_not_semantic_certification=True))
        path = output / 'source-proposals.json'
        print(json.dumps(dict(replay=str(path), replay_sha256=sha(path), actual_unique_sources=2, actual_profiles=2, unresolved=len(unresolved), canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--stage', choices=('fixtures', 'replay'), required=True)
    parser.add_argument('--packet-pins', type=Path, required=True)
    parser.add_argument('--packet-pins-sha256', required=True)
    parser.add_argument('--fixture-receipt', type=Path)
    parser.add_argument('--fixture-sha256')
    parser.add_argument('--full-report-sha256')
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', args.revision)
    if args.stage == 'replay': assert all((args.fixture_receipt, args.fixture_sha256, args.full_report_sha256))
    run(args)


if __name__ == '__main__':
    main()
