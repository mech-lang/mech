"""Reuse completed seals with the exact recorded framework-worker transition.

The original v1 failures and all downstream prerequisites remain unchanged.
Only root-reviewed, byte-pinned Python 3.14.8 launcher/framework linkage and
the authoritative actual worker-started record can qualify this transition.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import time
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON_LAUNCHER = '/opt/homebrew/opt/python@3.14/bin/python3.14'
PYTHON_LAUNCHER_IMAGE = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/bin/python3.14'
PYTHON_FRAMEWORK_IMAGE = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
PYTHON_LAUNCHER_SHA = 'e401ac7a95cda75dee34a0cfe1e4faf0a6df1a501cf67887f177fb78cd4eb60a'
PYTHON_STARTUP_SOURCE = 'https://raw.githubusercontent.com/python/cpython/v3.14.8/Mac/Tools/pythonw.c'


def read(path): return json.loads(Path(path).read_text())


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed exact next-stage prerequisite', path)


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest, ('Conflicting prerequisite pin', path)
        pins[path] = digest


def bound_read(path, pins, expected=None):
    path = Path(path)
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    assert expected is None or digest == expected, ('Changed actual witness', str(path))
    merge(pins, {str(path): digest})
    return json.loads(raw)


parser = argparse.ArgumentParser()
parser.add_argument('--stage', choices=('pony-work', 'prolog-runtime'), required=True)
parser.add_argument('--host-linkage', type=Path, required=True)
parser.add_argument('--host-linkage-sha256', required=True)
args = parser.parse_args()
dependency = {'pony-work': 'pony-work-v5-seal', 'prolog-runtime': 'prolog-runtime-v4-seal'}[args.stage]
terminal = HERE / 'owned-jobs' / dependency / 'terminal.json'
output = HERE / 'jobs' / (args.stage + '-reviewed-next-stage-v2')
output.parent.mkdir(parents=True, exist_ok=True)
output.mkdir()  # A failed or incomplete v2 attempt is never resumed.
state = dict(status='waiting-for-exact-metadata-dependency', dependency=dependency,
    automatic_retry=False, benchmark_timing_authorized=False, chart_admission=False)
command = None
try:
    # Waiting holds no timing mutex. The dependency may itself be waiting for
    # a long benchmark. This owner never restarts or duplicates that job.
    print('WAIT completed metadata dependency:', dependency, flush=True)
    deadline = time.monotonic() + 12 * 3600
    # The durable worker writes terminal JSON before releasing its lease.
    # Waiting for that lease avoids reading a partially written terminal.
    with (terminal.parent / 'owner.lock').open('a+') as dependency_lease:
        while True:
            try:
                fcntl.flock(dependency_lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise TimeoutError('Exact metadata dependency did not finish in twelve hours')
                time.sleep(15)
        assert terminal.is_file(), 'Dependency released its lease without a terminal'
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        pins = {str(Path(__file__).resolve()): sha(__file__)}
        completed = bound_read(terminal, pins)
        assert completed['status'] == 'command-exited-zero-awaiting-report-review'
        assert completed['actual_command_started'] is True
        assert type(completed['exit_code']) is int and completed['exit_code'] == 0
        assert completed['stop_signals'] == []
        assert not any(completed.get(key) for key in
            ('error', 'error_type', 'error_log_write_failure', 'exception_cleanup'))
        request = bound_read(terminal.parent / 'request.json', pins)
        started = bound_read(terminal.parent / 'command-started.json', pins)
        launch = bound_read(terminal.parent / 'launch.json', pins)
        worker_started = bound_read(terminal.parent / 'worker-started.json', pins)
        linkage_path = args.host_linkage.resolve(strict=True)
        linkage = bound_read(linkage_path, pins, args.host_linkage_sha256)
        assert linkage['status'] == 'root-reviewed-exact-python-framework-linkage'
        assert linkage['python_version'] == '3.14.8'
        assert linkage['official_startup_source'] == PYTHON_STARTUP_SOURCE
        assert linkage['same_image_bytes_claimed'] is False
        assert linkage['benchmark_runtime_image_changed'] is False
        assert linkage['launcher'] == dict(path=PYTHON_LAUNCHER, resolved_path=PYTHON_LAUNCHER_IMAGE,
            sha256=PYTHON_LAUNCHER_SHA)
        framework = linkage['framework']
        assert set(framework) == {'path', 'sha256'} and framework['path'] == PYTHON_FRAMEWORK_IMAGE
        assert type(framework['sha256']) is str and len(framework['sha256']) == 64 and all(
            value in '0123456789abcdef' for value in framework['sha256'])
        assert framework['sha256'] != PYTHON_LAUNCHER_SHA
        assert Path(PYTHON_LAUNCHER).resolve(strict=True) == Path(PYTHON_LAUNCHER_IMAGE)
        assert not Path(PYTHON_FRAMEWORK_IMAGE).is_symlink()
        assert sha(PYTHON_LAUNCHER_IMAGE) == PYTHON_LAUNCHER_SHA
        assert sha(PYTHON_FRAMEWORK_IMAGE) == framework['sha256']
        merge(pins, {PYTHON_LAUNCHER_IMAGE: PYTHON_LAUNCHER_SHA,
                     PYTHON_FRAMEWORK_IMAGE: framework['sha256']})
        root = str(CLBG.parents[1])
        if args.stage == 'pony-work':
            expected = ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u',
                str(CLBG / 'pony-comparison/20261012/revisions/work-full-v5/work_qualify.py'),
                '--seal', '--revision', 'private-v8-work-v5', '--qualification', 'private-v8']
        else:
            expected = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
                '-E', '-S', '-B', '-u', str(CLBG / 'prolog-comparison/20261013/revisions/runtime-v4/seal_runtime_v4.py'),
                '--seal', '--proposal-sha256', '5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88']
        assert request['command'] == started['command'] == launch['command'] == expected
        assert request['cwd'] == started['cwd'] == launch['cwd'] == root
        assert worker_started['command'] == expected and worker_started['cwd'] == root
        assert worker_started['inherited_private_lease'] is True and worker_started['detached_session'] is True
        assert worker_started['common_mutex_owned_by_target_command'] is True
        assert worker_started['qualification_granted'] is False
        assert request['common_mutex_owned_by_target_command'] is True
        assert request['automatic_retry'] is request['qualification_granted'] is False
        assert type(completed['command_pid']) is int and completed['command_pid'] > 0
        assert completed['command_pid'] == started['command_pid']
        assert completed['command_identity'] == started['command_identity']
        assert completed['command_identity']['pid'] == completed['command_pid']
        assert completed['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
        assert completed['worker_identity']['pid'] == launch['worker_identity']['pid']
        # The initial macOS bin launcher explicitly SETEXECs the app image
        # with argv[0] changed. Bind the actual post-startup worker to terminal
        # exactly, and allow only this byte-pinned pair in the launch record.
        assert worker_started['worker_identity'] == completed['worker_identity']
        initial_listing = launch['worker_identity']['listing'].split(None, 8)
        final_listing = completed['worker_identity']['listing'].split(None, 8)
        assert len(initial_listing) == len(final_listing) == 9
        assert initial_listing[0] == final_listing[0] == str(completed['worker_identity']['pid'])
        assert initial_listing[2:7] == final_listing[2:7]
        assert initial_listing[7] == PYTHON_LAUNCHER and final_listing[7] == PYTHON_FRAMEWORK_IMAGE
        expected_worker_arguments = '-B -u '+str(HERE / 'launch_durable_v1.py')+' --worker '+str(terminal.parent)+' --lease-fd 3 --go-fd 4'
        assert initial_listing[8] == final_listing[8] == expected_worker_arguments
        assert initial_listing[1].isdigit() and final_listing[1] == '1'
        previous = HERE / 'owned-jobs' / {
            'pony-work': 'pony-two-tree-reviewed-next-stage-v1',
            'prolog-runtime': 'prolog-private-runtime-reviewed-next-stage-v1'}[args.stage]
        failed_terminal = bound_read(previous / 'terminal.json', pins)
        failed_state = bound_read(HERE / 'jobs' / (args.stage + '-reviewed-next-stage-v1') / 'receipt.json', pins)
        assert failed_terminal['status'] == 'command-failed' and type(failed_terminal['exit_code']) is int and failed_terminal['exit_code'] == 1
        assert failed_terminal['actual_command_started'] is True and failed_terminal['stop_signals'] == []
        assert failed_state['status'] == 'stopped-before-next-stage' and failed_state['dependency'] == dependency
        assert failed_state['automatic_retry'] is failed_state['benchmark_timing_authorized'] is failed_state['chart_admission'] is False
        failed_trace_path = previous / 'command.stderr'
        failed_trace = failed_trace_path.read_bytes()
        assert failed_trace.endswith(b'AssertionError\n') and b"assert initial_listing[2] == final_listing[2]" in failed_trace
        merge(pins, {str(failed_trace_path): hashlib.sha256(failed_trace).hexdigest()})
        state['worker_framework_transition'] = dict(launcher=linkage['launcher'], framework=framework,
            same_image_bytes_claimed=False, actual_worker_started_identity=worker_started['worker_identity'],
            launcher_record=str(terminal.parent / 'launch.json'),
            worker_started_record=str(terminal.parent / 'worker-started.json'),
            terminal_record=str(terminal), official_startup_source=PYTHON_STARTUP_SOURCE,
            birth_and_remaining_argv_unchanged=True, downstream_command_identity_unchanged=True)
        if args.stage == 'pony-work':
            cohort = CLBG / 'pony-comparison/20261012'
            manifest = cohort / 'work-qualification/private-v8-work-v5/manifest.json'
            seal_receipt = cohort / 'work-seal-outcomes/private-v8-work-v5/receipt.json'
            sealed = bound_read(seal_receipt, pins)
            assert sealed['status'] == 'completed-metadata-awaiting-root-review'
            assert sealed['revision'] == 'private-v8-work-v5'
            assert sealed['actual_native_subjects'] == 0 and sealed['chart_admission'] is False
            verify(sealed['available_evidence_pins']); merge(pins, sealed['available_evidence_pins'])
            value = bound_read(manifest, pins, sealed['available_evidence_pins'][str(manifest)])
            assert value['status'] == 'prepared-awaiting-root-review'
            assert value['shared_lock'] == str(LOCK)
            assert value['source_work_profiles'] == 20 and value['source_work_trace_profiles'] == 6
            assert value['reused_actual_work_traces'] == 4
            assert value['new_actual_work_traces_required'] == value['new_native_cases_required'] == 2
            assert value['reused_fixture_tests'] == 14 and value['fresh_fixture_tests'] == 0
            assert value['full_performance_runs'] == 0
            assert value['production_operations_preserved'] is True
            assert value['workload_code_delegated_to_native'] is False
            assert value['qualification_sha256'] == '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
            verify(value['pins']); merge(pins, value['pins'])
            retained = bound_read(value['prior_work_reuse']['path'], pins,
                value['prior_work_reuse']['sha256'])
            assert retained['status'] == 'verified-four-actual-Pi-K-work-proofs-and-fourteen-fixtures'
            assert retained['reused_actual_work_traces'] == 4 and retained['new_actual_work_traces_required'] == 2
            assert retained['old_attempts_relabelled_as_new'] is False
            assert len(retained['attempts']) == 4
            verify(retained['pins']); merge(pins, retained['pins'])
            target = cohort / 'revisions/work-full-v5/work_qualify.py'
            assert sha(target) == value['pins'][str(target)]
            digest = sha(manifest); pins[str(manifest)] = digest
            entry = HERE / 'pony-work-entry-v1/replay.py'
            assert entry.is_file() and not entry.is_symlink()
            pins[str(entry)] = sha(entry)
            command = [sys.executable, '-B', '-u', str(entry), '--manifest-sha256', digest]
            state['authorized_scope'] = 'two new bounded tree observations and original scored allocation evidence; zero full timing samples'
        else:
            cohort = CLBG / 'prolog-comparison/20261013'
            proposal = cohort / 'proposals/runtime-v4.json'
            compatibility = cohort / 'proposals/runtime-v2.json'
            seal_terminal = cohort / 'proposal-seals/runtime-v4/terminal-pin.json'
            original_witness = bound_read(seal_terminal, pins)
            assert original_witness['status'] == 'sealed-runtime-bootstrap-awaiting-root-review'
            assert original_witness['inherited_common_mutex'] is True
            original_report = Path(original_witness['report'])
            assert original_report == cohort / 'proposal-seals/runtime-v4/report.json'
            report_value = bound_read(original_report, pins, original_witness['report_sha256'])
            assert report_value['pins'] == original_witness['partial_evidence_pins']
            value = bound_read(proposal, pins, original_witness['report_sha256'])
            assert value == report_value
            assert value['qualification_stage'] == 'runtime-v4'
            assert value['status'] == 'sealed-runtime-bootstrap-awaiting-root-review'
            assert value['terminal_evidence_complete'] is True
            assert value['retained_runtime_fixture_count'] == 6
            assert value['runtime_configuration_admitted'] is value['source_work_qualified'] is False
            assert value['actual_native_subjects'] == value['full_processes_authorized'] == 0
            assert value['fixture_python_host']['actual_pyexpat_builtin_after_fixtures'] is True
            assert value['fixture_python_host']['actual_pyexpat_spec_origin'] == 'built-in'
            assert value['prolog_proposal_sha256'] == '5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88'
            verify(value['pins']); merge(pins, value['pins'])
            publication = cohort / 'proposal-seals/runtime-v4/metadata-publication-outcome.json'
            publication_terminal = bound_read(cohort / 'proposal-seals/runtime-v4/metadata-publication-terminal.json', pins)
            assert publication_terminal['status'] == 'completed'
            assert publication_terminal['outcome'] == str(publication)
            assert publication_terminal['success_requires_actual_command_exit_zero'] is True
            actual = bound_read(publication, pins, publication_terminal['outcome_sha256'])
            assert actual['status'] == 'completed' and actual['qualification_stage'] == 'runtime-v4'
            assert actual['actual_proposal_pins'] == publication_terminal['actual_proposal_pins']
            assert set(actual['actual_proposal_pins']) == {str(proposal), str(compatibility)}
            verify(actual['actual_proposal_pins']); merge(pins, actual['actual_proposal_pins'])
            assert proposal.read_bytes() == compatibility.read_bytes()
            target = cohort / 'revisions/runtime-v2/bootstrap_runtime_v2.py'
            assert sha(target) == value['pins'][str(target)]
            python = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12')
            assert sha(python.resolve()) == value['fixture_python_host']['executable_sha256']
            command = [str(python), '-E', '-S', '-B', '-u', str(target), '--bootstrap',
                '--proposal-sha256', value['prolog_proposal_sha256'],
                '--runtime-proposal-sha256', actual['actual_proposal_pins'][str(compatibility)]]
            pins[str(publication)] = sha(publication)
            state['authorized_scope'] = 'one private exact SWI10.0.2 runtime bootstrap; zero application benchmark runs; runtime admission remains separate'
        verify(pins)
        assert Path(PYTHON_LAUNCHER).resolve(strict=True) == Path(PYTHON_LAUNCHER_IMAGE)
        assert not Path(PYTHON_FRAMEWORK_IMAGE).is_symlink()
        state.update(status='exact-next-stage-prerequisites-reconciled', command=command,
            dependency_terminal=str(terminal), dependency_terminal_sha256=pins[str(terminal)],
            exact_prerequisite_pins=pins, common_timing_mutex_held_for_review=True)
except BaseException as failure:
    state.update(status='stopped-before-next-stage', error=str(failure) or type(failure).__name__,
        error_type=type(failure).__name__)
    try:
        (output / 'failure.log').write_text(traceback.format_exc())
    except BaseException as log_error:
        state['failure_log_write_error'] = str(log_error) or type(log_error).__name__
    if args.stage == 'prolog-runtime':
        # V4 leaves any failed partial publication in place. Capture all
        # available proposal/alias bytes incrementally without promoting the
        # failed fixture/publication or repeating any runtime operation.
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            held, named = os.fstat(lock.fileno()), LOCK.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
            cohort = CLBG / 'prolog-comparison/20261013'
            available, absent, errors = {}, [], []
            for path in [cohort / 'proposals/runtime-v4.json', cohort / 'proposals/runtime-v2.json',
                *[cohort / 'proposal-seals/runtime-v4' / name for name in
                  ('report.json', 'terminal-pin.json', 'metadata-publication-outcome.json', 'metadata-publication-terminal.json')]]:
                try:
                    if path.is_file(): available[str(path)] = sha(path)
                    else: absent.append(str(path))
                except BaseException as error:
                    errors.append(dict(path=str(path), error=str(error) or type(error).__name__))
            state['failed_dependency_publication_bytes'] = dict(available_pins=available,
                absent_paths=absent, pin_errors=errors, original_status_changed=False,
                retries=0, runtime_admission=False)
    raise
finally:
    with (output / 'receipt.json').open('x') as stream:
        json.dump(state, stream, indent=2); stream.write('\n')
assert command is not None
# The reviewed original target owns its own common descriptor. Close ours
# before entering it; preserve this exact durable owner PID with exec.
os.execv(command[0], command)
