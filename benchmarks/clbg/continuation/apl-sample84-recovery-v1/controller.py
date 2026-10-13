"""Harvest actual orphaned APL sample84 and run only its three missing slots.

The kernel observer supplies its literal exit status, never synthetic waitpid.
All original reports/raw files stay immutable. Native dispatch/ledger functions
and the high descriptor adapter are the exact original frozen implementations.
"""
import argparse
from contextlib import ExitStack
import copy
from datetime import datetime
import fcntl
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import sys
import traceback

PACKET = Path(__file__).resolve().parent
SOURCE = Path(__file__).resolve()
CLBG = PACKET.parents[1]
N = CLBG / 'r-apl-uiua-comparison/20261011'
OLD_PACKET = N / 'uiua-fann-compact-v5/controller_rev2/apl-after-tree-failure-v1'
OLD = OLD_PACKET / 'full-owner'
OLD_MANIFEST = OLD / 'manifest.json'
OLD_MANIFEST_SHA = 'afe92e55cea508e497e943143ae1934531330abd023c67020d03bf5e2c71e1ed'
OLD_CONTROLLER = OLD_PACKET / 'controller.py'
OLD_CONTROLLER_SHA = 'd36a58c5c87b9dd1e69348705b406a03c8a2874279a834aa1e8f6345b0d09745'
COMMON = OLD_PACKET.parent / 'remaining_owner.py'
COMMON_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
OLD_REPORT = OLD / 'timings-apl.json'
OLD_STATUS = OLD / 'status.json'
RAW84 = OLD / 'attempts/084-fannkuchredux-apl-compact.json'
OBSERVER = CLBG / 'continuation/jobs/apl-sample84-kqueue-observer-v1'
OBSERVER_SOURCE = CLBG / 'continuation/observe_apl_exit_v1.py'
OUT = PACKET / 'full-owner'
LOCK = Path('/tmp/mech-clbg-timing.lock')
SHARED_LEASE = N / 'results/coordinator-v4/owner-apl.lock'
KEY = 'fannkuchredux:fannkuchredux-apl-compact'
CONTROL = 'fannkuchredux:mech-control-for-apl'
EXPECTED_BIRTHS = {68659: 'Tue Oct 13 01:11:26 2026', 68666: 'Tue Oct 13 01:11:27 2026'}
EXPECTED84_SECONDS = 7797.184570125


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Recovery input changed', path)


def load(path, name):
    assert name not in sys.modules
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    spec.loader.exec_module(result)
    assert Path(result.__file__).resolve() == path.resolve()
    return result


def observed_statuses(active):
    registered = json.loads((OBSERVER / 'registered.json').read_text())
    terminal = json.loads((OBSERVER / 'terminal.json').read_text())
    assert registered['status'] == 'both-live-identities-registered'
    assert registered['observer_pid'] == terminal['observer_pid'] == 83359
    assert registered['report'] == str(OLD_REPORT) and registered['saved_active'] == active
    assert registered['note_exit'] == 0x80000000 and registered['note_exitstatus'] == 0x04000000
    assert terminal['status'] == 'both-kernel-exit-statuses-captured-awaiting-raw-pin-review'
    assert terminal['exit_status_origin'] == 'Darwin EVFILT_PROC NOTE_EXITSTATUS; not synthetic waitpid'
    assert set(terminal['exit_events']) == {str(pid) for pid in EXPECTED_BIRTHS}
    for document in (registered, terminal):
        assert document['signals_sent'] == document['benchmarks_started'] == 0
        assert document['debugger_attached'] is document['qualification_granted'] is False
        assert 'error' not in document and 'failure_drain_error' not in document
    statuses = {}
    raw_paths = set()
    for pid, birth in EXPECTED_BIRTHS.items():
        assert registered['identities'][str(pid)].startswith(birth)
        event = json.loads((OBSERVER / ('exit-%d.json' % pid)).read_text())
        assert event == terminal['exit_events'][str(pid)]
        raw_path = Path(event['raw_event'])
        assert raw_path.parent == OBSERVER and raw_path.name.startswith('raw-event-')
        raw_paths.add(raw_path)
        raw = json.loads(raw_path.read_text())
        assert type(event['pid']) is int and event['pid'] == raw['ident'] == pid
        for field in ('filter', 'flags', 'fflags'):
            assert type(event[field]) is type(raw[field]) is int and event[field] == raw[field]
        assert event['filter'] == -5 and not event['flags'] & 0x4000
        assert event['fflags'] & 0x80000000 and event['fflags'] & 0x04000000
        status = raw['data']
        assert type(status) is type(event['kernel_wait_status']) is int
        assert status == event['kernel_wait_status'] == 0
        assert os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0
        assert event['exited_normally'] is True and event['signalled'] is False
        assert type(event['exit_code']) is int and event['exit_code'] == 0
        assert event['signal_number'] is None
        assert event['qualification_granted'] is raw['qualification_granted'] is False
        statuses[pid] = status
    assert raw_paths == {OBSERVER / 'raw-event-0001.json', OBSERVER / 'raw-event-0002.json'}
    assert 'apl-after-tree-failure-v1/controller.py' in registered['identities']['68659']
    assert 'fannkuchredux.compact.apl -- 12' in registered['identities']['68666']
    return statuses, registered, terminal


def literal_review(guard, controller, harness, attempt, row, manifest):
    assert type(attempt['sampler_wait_status']) is type(attempt['exit_status']) is int
    native = attempt['supervised_native_group']
    assert type(native['pid']) is type(native['pgid']) is int and native['pid'] == native['pgid'] > 0
    assert native['type'] == 'native-group-started' and native['inherited_common_mutex'] is True
    assert attempt['execution_fingerprint'] == harness.execution_fingerprint(row)
    assert attempt['source_sha256'] == row['source_sha256'] and attempt['key'] == row['key']
    assert attempt['id'] == row['id'] and attempt['benchmark'] == row['benchmark']
    assert attempt['language'] == row['language'] and attempt['cwd'] == row['cwd']
    assert attempt['runtime_environment'] == row.get('runtime_environment', {})
    assert attempt['timeout_seconds'] == manifest['timeout_seconds']
    assert attempt['maximum_rss_mib'] == manifest['maximum_rss_mib']
    assert type(attempt['seconds']) in (int, float) and math.isfinite(attempt['seconds']) and attempt['seconds'] > 0
    for field in ('monitor_errors', 'resource_monitor_errors', 'remaining_group_members',
            'supervisor_remaining_members', 'competing_compilers', 'power_changes'):
        assert type(attempt[field]) is list and attempt[field] == []
    assert attempt['resource_violation'] is None
    assert attempt['power_at_start'] == attempt['power_at_end'] == harness.REQUIRED
    reference = manifest['full_output_reference']
    validation = attempt['validation']
    assert type(attempt['stdout_bytes']) is type(validation['bytes']) is int
    assert attempt['stdout_bytes'] == validation['bytes'] == reference['bytes']
    assert attempt['stdout_sha256'] == validation['sha256'] == reference['sha256']
    replay = guard.qualification_checks(attempt, native, manifest, row, attempt['sampler_wait_status'])
    assert set(replay) == controller.GATES and len(replay) == 20
    assert all(type(value) is bool and value is True for value in replay.values())
    if 'qualification_checks' in attempt:
        assert attempt['qualification_checks'] == replay and attempt['qualified'] is True
    return replay


def run():
    created = False
    guard = None
    report = None
    status = dict(status='preparing-sample84-recovery', old_sample_signals_sent=0,
        original_report_mutated=False, original_raw_mutated=False, no_automatic_retry=True,
        chart_admission=False, fresh_full_processes=0, required_fresh_full_processes=3)
    with ExitStack() as leases:
        for path in (OLD_PACKET / 'owner.lock', SHARED_LEASE):
            lease = leases.enter_context(path.open('a+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            print('WAIT common mutex: actual APL sample84 recovery', flush=True)
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                assert not OUT.exists(), 'Exclusive additive recovery; no replacement/resume/retry'
                OUT.mkdir(parents=True)
                created = True
                new_json(OUT / 'started.json', dict(status, actual_argv=sys.argv, common_descriptor=lock.fileno()))
                pins = {str(SOURCE): sha(SOURCE), str(OLD_MANIFEST): OLD_MANIFEST_SHA,
                    str(OLD_CONTROLLER): OLD_CONTROLLER_SHA, str(COMMON): COMMON_SHA}
                verify(pins)
                prior = json.loads(OLD_MANIFEST.read_text())
                assert prior['no_automatic_retry'] is True and prior['owner_leases'] == [str(SHARED_LEASE)]
                for path, digest in prior['pins'].items():
                    assert path not in pins or pins[path] == digest
                    pins[path] = digest
                verify(pins)
                sys.dont_write_bytecode = True
                controller = load(OLD_CONTROLLER, 'apl_sample84_original_recovery_controller')
                argv_path = controller.PRIOR / 'full-owner/manifest.json'
                assert pins[str(argv_path)] == controller.PRIOR_MANIFEST_SHA
                argv_manifest = json.loads(argv_path.read_text())
                guard, base = controller.infrastructure(argv_manifest)
                assert controller.COMMON_SHA == base.COMMON_SHA == COMMON_SHA
                harness = guard.load_harness()
                assert all(guard.identity(pid) is None for pid in (32432, 68659, 68666))
                assert harness.group_members(68666) == []
                old = json.loads(OLD_REPORT.read_text())
                old_status = json.loads(OLD_STATUS.read_text())
                assert old_status['status'] == 'running' and old_status['supervisor']['pid'] == 32432
                assert old_status['manifest_sha256'] == OLD_MANIFEST_SHA
                assert old['status'] == 'partial' and len(old['attempts']) == 83
                active = old['active']
                assert active['attempt'] == 84 and active['sample'] == active['round'] == 2
                assert active['key'] == KEY and active['id'] == 'fannkuchredux-apl-compact'
                assert active['sampler_pid'] == 68659 and active['sampler_inherited_common_mutex'] is True
                assert active['native']['pid'] == active['native']['pgid'] == 68666
                assert active['result_path'] == str(RAW84)
                assert not RAW84.with_suffix('.supervisor.json').exists(), 'Original supervisor already enriched this sample'
                statuses, registered, terminal = observed_statuses(active)
                # Bind every available original raw/supervisor file; historical
                # report-only outcomes stay report-only, with no invented files.
                for attempt in old['attempts']:
                    for field in ('result_path', 'supervisor_result_path'):
                        if field in attempt:
                            path = Path(attempt[field])
                            assert path.is_relative_to(N)
                            pins[str(path)] = sha(path)
                for path in (OLD_REPORT, OLD_STATUS, RAW84, OBSERVER_SOURCE,
                        *(path for path in OBSERVER.iterdir() if path.is_file())):
                    pins[str(path)] = sha(path)
                verify(pins)
                report = copy.deepcopy(old)
                rows = report['programs']
                assert rows[KEY]['samples_seconds'] == [7919.4648885] and rows[KEY]['attempts'] == [82]
                assert rows[CONTROL]['samples_seconds'] == [1.707343] and rows[CONTROL]['attempts'] == [83]
                assert rows['fannkuchredux:fannkuchredux-apl-optimized']['timing_alias_of'] == KEY
                contracts = base.preflight(report, harness, pins)
                assert all(contracts[key] == prior['input_contracts'][key]
                    for key in prior['input_contracts'] if key.startswith('fannkuchredux:'))
                sample_manifest = dict(prior, size=12, canonical_full_workload=True,
                    full_output_reference=guard.full_reference(harness, 'fannkuchredux'))
                retained_checks = {}
                for number in (82, 83):
                    attempt = old['attempts'][number - 1]
                    assert attempt['attempt'] == attempt['attempt_number'] == number and attempt['sample'] == 1
                    supervisor = json.loads(Path(attempt['supervisor_result_path']).read_text())
                    assert all(attempt[key] == value for key, value in supervisor.items())
                    retained_checks[str(number)] = literal_review(guard, controller, harness, attempt,
                        rows[attempt['key']], sample_manifest)
                raw84 = json.loads(RAW84.read_text())
                assert raw84['seconds'] == EXPECTED84_SECONDS
                assert raw84['exit_status'] == os.WEXITSTATUS(statuses[68666])
                assert datetime.fromisoformat(active['started_at']) <= datetime.fromisoformat(raw84['started_at'])
                assert datetime.fromisoformat(raw84['started_at']) < datetime.fromisoformat(registered['registered_at'])
                assert datetime.fromisoformat(raw84['finished_at']) <= datetime.fromisoformat(terminal['exit_events']['68659']['observed_at'])
                recovered = copy.deepcopy(raw84)
                native = copy.deepcopy(active['native'])
                native.pop('identity')
                recovered.update(attempt=84, round=2, sampler_wait_status=statuses[68659], result_path=str(RAW84),
                    supervised_native_group=native, supervisor_remaining_members=[],
                    sampler_wait_status_origin='Literal Darwin NOTE_EXITSTATUS kernel event for exact sampler68659; no waitpid call',
                    native_wait_status_origin='Literal Darwin NOTE_EXITSTATUS kernel event for exact native68666',
                    recovered_after_supervisor_exit=True)
                recovered['qualification_checks'] = literal_review(guard, controller, harness, recovered, rows[KEY], sample_manifest)
                recovered['qualified'] = True
                supervisor84 = OUT / 'recovered-084.supervisor.json'
                recovered['supervisor_result_path'] = str(supervisor84)
                new_json(supervisor84, recovered)
                assert guard.record_attempt(harness, report, rows[KEY], recovered, 2, OLD_MANIFEST_SHA)
                assert report['attempts'][:83] == old['attempts'] and len(report['attempts']) == 84
                assert all(report['programs'][key] == value for key, value in old['programs'].items()
                    if value['benchmark'] != 'fannkuchredux')
                pins[str(supervisor84)] = sha(supervisor84)
                guard.STAGES = [('apl', 'fannkuchredux')]
                slots = []
                for sample in range(3):
                    for row in guard.ordered_candidates(report, 'apl', 'fannkuchredux', sample):
                        if len(row['samples_seconds']) <= sample and not row.get('full_failure'):
                            slots.append(dict(sample=sample + 1, key=row['key']))
                assert slots == [dict(sample=2, key=CONTROL), dict(sample=3, key=CONTROL), dict(sample=3, key=KEY)]
                snapshot = OUT / 'original-snapshot'
                snapshot.mkdir()
                for label, path in (('manifest.json', OLD_MANIFEST), ('timings-apl.json', OLD_REPORT),
                        ('status.json', OLD_STATUS), ('raw-084.json', RAW84), ('observer-registered.json', OBSERVER / 'registered.json'),
                        ('observer-terminal.json', OBSERVER / 'terminal.json')):
                    target = snapshot / label
                    with target.open('xb') as stream:
                        stream.write(path.read_bytes())
                    assert target.read_bytes() == path.read_bytes()
                    pins[str(target)] = sha(target)
                receipt = OUT / 'recovery-admission.json'
                new_json(receipt, dict(status='passed-exact-kernel-status-and-original-twenty-gate-replay',
                    original_manifest=dict(path=str(OLD_MANIFEST), sha256=OLD_MANIFEST_SHA),
                    original_report=dict(path=str(OLD_REPORT), sha256=pins[str(OLD_REPORT)]),
                    original_raw84=dict(path=str(RAW84), sha256=pins[str(RAW84)]),
                    observer=dict(path=str(OBSERVER / 'terminal.json'), sha256=pins[str(OBSERVER / 'terminal.json')]),
                    literal_kernel_wait_statuses=statuses, original82_83_replayed_checks=retained_checks,
                    recovered84_checks=recovered['qualification_checks'], retained_seconds=[7919.4648885, raw84['seconds']],
                    recovered_supervisor=dict(path=str(supervisor84), sha256=pins[str(supervisor84)]),
                    missing_slots=slots, already_running_sample_restarted=False, old_process_signals_sent=0,
                    fresh_full_processes=0, chart_admission=False))
                pins[str(receipt)] = sha(receipt)
                before = OUT / 'apl-before.json'
                new_json(before, report)
                pins[str(before)] = sha(before)
                manifest = dict(prior, status='prepared-explicit-sample84-recovery', stages=[dict(language='apl', benchmark='fannkuchredux')],
                    reports=dict(apl=str(OUT / 'timings-apl.json')), before_reports=dict(apl=dict(path=str(before), sha256=pins[str(before)])),
                    pins=dict(pins), planned_actual_full_processes=3, planned_slots=slots, no_automatic_retry=True,
                    chart_admission=False, original_report_immutable=True, recovered_sample84=str(receipt))
                new_json(OUT / 'manifest.json', manifest)
                digest = sha(OUT / 'manifest.json')
                pins[str(OUT / 'manifest.json')] = digest
                manifest['pins'] = dict(pins)
                guard.OUT, guard.REPORT, guard.MANIFEST = OUT, OUT / 'timings-apl.json', OUT / 'manifest.json'
                report['sample84_recovery'] = dict(admission=str(receipt), preserved_ledger_entries=83,
                    recovered_original_attempt=84, fresh_attempts=0, original_report_mutated=False,
                    original_sources_changed=False, aliases_remain_aliases=True, chart_admission=False)
                guard.save(guard.REPORT, report)
                status.update(status='running', started_at=guard.now(), supervisor=guard.identity(os.getpid()),
                    manifest_sha256=digest, literal_kernel_wait_statuses=statuses, planned_slots=slots)
                guard.save(OUT / 'status.json', status)
            harness.install_interrupt_handlers()
            for slot in slots:
                row = report['programs'][slot['key']]
                with guard.LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    assert lock.fileno() >= 64, 'Original GNU APL high descriptor policy'
                    verify(pins)
                    harness.compact.wait_for_environment(harness.REQUIRED)
                    sample_manifest = dict(manifest, size=12, canonical_full_workload=True,
                        full_output_reference=guard.full_reference(harness, 'fannkuchredux'))
                    print('FULL RECOVERED APL REMAINING', slot['sample'], row['id'], flush=True)
                    attempt = guard.one_attempt(harness, row, sample_manifest, report, slot['sample'], lock.fileno())
                    accepted = guard.record_attempt(harness, report, row, attempt, slot['sample'], digest)
                    guard.save(guard.REPORT, report)
                    assert accepted, 'Actual new sample failed; retained with zero retries'
                    literal_review(guard, controller, harness, attempt, row, sample_manifest)
                    for field in ('result_path', 'supervisor_result_path'):
                        path = Path(attempt[field])
                        assert path.parent == OUT / 'attempts'
                        pins[str(path)] = sha(path)
                    manifest['pins'] = dict(pins)
                    status['fresh_full_processes'] += 1
                    report['sample84_recovery']['fresh_attempts'] += 1
                    guard.save(guard.REPORT, report)
                    guard.save(OUT / 'status.json', status)
            with guard.LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                verify(pins)
                assert status['fresh_full_processes'] == 3 and len(report['attempts']) == 87
                assert report['attempts'][:83] == old['attempts']
                assert all(len(row['samples_seconds']) == 3 for row in report['programs'].values()
                    if row['benchmark'] == 'fannkuchredux')
                assert all(report['programs'][key] == value for key, value in old['programs'].items()
                    if value['benchmark'] != 'fannkuchredux')
                guard.refresh_report_status(report)
                report['sample84_recovery'].update(status='complete', pins_unchanged_at_end=True,
                    completed_at=guard.now(), overall_suite_completion_pending=True)
                guard.save(guard.REPORT, report)
                status.update(status='complete', finished_at=guard.now(), pins_unchanged_at_end=True,
                    retained_input_and_actual_result_pins=pins, timing_report=str(guard.REPORT))
                guard.save(OUT / 'status.json', status)
        except BaseException as error:
            if created:
                with LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    status.update(status='stopped', error=str(error) or type(error).__name__, error_type=type(error).__name__)
                    if guard is not None:
                        guard.save(OUT / 'status.json', status)
                        if report is not None:
                            guard.save(OUT / 'timings-apl.json', report)
                    else:
                        new_json(OUT / 'failure.json', status)
                    try:
                        with (OUT / 'actual-error.log').open('x') as stream:
                            stream.write(traceback.format_exc())
                    except OSError:
                        pass
            raise
    print(json.dumps(dict(status=status['status'], report=str(OUT / 'timings-apl.json'),
        recovered_original_sample84=True, fresh_full_processes=3, chart_admission=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--run', action='store_true', required=True)
    parser.parse_args()
    run()


if __name__ == '__main__':
    main()
