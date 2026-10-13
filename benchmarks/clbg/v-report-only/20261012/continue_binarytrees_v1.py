"""Add the eight missing V tree samples, retaining the exact first sample.

An explicit additive continuation, with the original owner lease and sampler.
The original report/raw bytes remain unchanged. No retries or chart admission.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import statistics
import sys
import traceback

HERE = Path(__file__).resolve().parent
SOURCE = Path(__file__).resolve()
ORIGINAL = HERE / 'studies/binarytrees/full-study'
MANIFEST = ORIGINAL / 'manifest.json'
MANIFEST_SHA = '8180cecca83fd5c6c48d19cdb613dd530bdeb2047962e80c6cd03c5eb77d88f3'
OLD_REPORT = ORIGINAL / 'report.json'
OLD_RAW = ORIGINAL / 'attempts/001-v-binarytrees-compact-input-v2.json'
RUNNER = HERE / 'full_study.py'
RUNNER_SHA = '578a49d343fae3d6d67317594c6d820eb604556e8de5904b3d725706b15cc7c4'
HELPER = HERE.parents[1] / 'r-apl-uiua-comparison/20261011/uiua-mandel-parallel-v4/full_study.py'
HELPER_SHA = '7a48690bef2f36375c72619f8c74170735273804d089dafb55e6813189613fe5'
LOCK = Path('/tmp/mech-clbg-timing.lock')
FIRST_ID = 'v-binarytrees-compact-input-v2'
FIRST_SECONDS = 6.152344917
REFERENCE = dict(bytes=455, sha256='341de11a51feab3d8122b4b5d6a68b038a2d14434aa9bc2372f39300bf5f48e1')
GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}


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
        assert sha(path) == digest, ('Continuation input changed', path)


def checks(attempt, row, manifest, harness):
    """Derive typed gates without changing any saved legacy sampler result."""
    native = attempt.get('supervised_native_group') or {}
    validation = attempt.get('validation') or {}
    command = [part.replace('{n}', str(manifest['size'])) for part in row['command']]

    def absent(key):
        return key not in attempt or attempt[key] is None or attempt[key] is False

    def empty(key):
        return type(attempt.get(key)) is list and attempt[key] == []

    exact = (attempt.get('id') == row['id'] and attempt.get('key') == row['key']
        and attempt.get('benchmark') == row['benchmark'] and attempt.get('language') == row['language']
        and attempt.get('source_sha256') == row['source_sha256']
        and attempt.get('command') == native.get('command') == command
        and attempt.get('cwd') == row['cwd']
        and attempt.get('runtime_environment') == row.get('runtime_environment', {})
        and attempt.get('timeout_seconds') == manifest['timeout_seconds']
        and attempt.get('maximum_rss_mib') == manifest['maximum_rss_mib']
        and attempt.get('execution_fingerprint') == harness.execution_fingerprint(row)
        and type(native.get('pid')) is int and native['pid'] > 0
        and type(native.get('pgid')) is int and native['pgid'] == native['pid'])
    gates = dict(
        sampler_exited_normally=type(attempt.get('sampler_wait_status')) is int and attempt['sampler_wait_status'] == 0,
        authoritative_native_group_received=type(native) is dict and native.get('type') == 'native-group-started',
        native_inherited_common_mutex=native.get('inherited_common_mutex') is True,
        native_exited_normally=type(attempt.get('exit_status')) is int and attempt['exit_status'] == 0,
        no_execution_error=absent('error') and absent('error_type'),
        no_wrapper_integrity_error=all(absent(key) for key in ('sampler_failure', 'sampler_error',
            'supervisor_failure', 'supervisor_error', 'supervisor_error_type', 'supervisor_registration_error',
            'registration_error', 'supervisor_pin_failure')),
        no_resource_violation=attempt.get('resource_violation') is None,
        no_resource_monitor_errors=empty('resource_monitor_errors'),
        no_compiler_power_monitor_errors=empty('monitor_errors') and empty('competing_compilers') and empty('power_changes'),
        no_monitor_finish_error=absent('monitor_finish_error'),
        no_execute_cleanup_error=absent('cleanup_error'),
        no_remaining_execute_group=empty('remaining_group_members'),
        no_supervisor_cleanup=absent('supervisor_cleanup'),
        no_supervisor_remaining_group=empty('supervisor_remaining_members'),
        valid_environment=attempt.get('valid_environment') is True
            and attempt.get('power_at_start') == attempt.get('power_at_end') == harness.REQUIRED,
        actual_full_workload=attempt.get('full_workload') is True
            and type(attempt.get('workload_n')) is int and attempt['workload_n'] == manifest['size'] == 21,
        exact_command_and_group=exact,
        complete_output_reference_match=validation.get('reference_match') is True,
        complete_output_bytes_and_hash=type(attempt.get('stdout_bytes')) is int
            and type(validation.get('bytes')) is int
            and attempt['stdout_bytes'] == validation['bytes'] == REFERENCE['bytes']
            and attempt.get('stdout_sha256') == validation.get('sha256') == REFERENCE['sha256'],
        pins_unchanged=attempt.get('pins_unchanged_at_end') is True)
    assert set(gates) == GATES and all(type(value) is bool for value in gates.values())
    return gates


def qualified(attempt, gates):
    return (attempt.get('qualified') is True and type(attempt.get('fatal_qualification_fields')) is list
        and attempt['fatal_qualification_fields'] == [] and all(gates.values())
        and type(attempt.get('seconds')) in (int, float) and math.isfinite(attempt['seconds'])
        and attempt['seconds'] > 0)


def run(revision):
    output = HERE / 'studies/binarytrees' / revision
    report = dict(status='preparing-explicit-additive-continuation', active=None, attempts=[],
        report_only=True, never_chart_eligible=True, chart_admission=False,
        no_automatic_retry=True, retained_attempt_count=1, fresh_attempt_count=0,
        required_fresh_attempts=8, original_report_mutated=False)
    pins = {}
    module = None
    created = False
    # This is the original lease, shared with the original full-study runner.
    with (ORIGINAL / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            print('WAIT common mutex: explicit V binarytrees continuation', flush=True)
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                assert not output.exists(), 'Exclusive additive revision; no overwrite/resume/retry'
                output.mkdir(parents=True)
                created = True
                pins = {str(SOURCE): sha(SOURCE), str(MANIFEST): MANIFEST_SHA,
                    str(RUNNER): RUNNER_SHA, str(HELPER): HELPER_SHA}
                verify(pins)
                manifest = json.loads(MANIFEST.read_text())
                assert manifest['pins'][str(RUNNER)] == RUNNER_SHA
                assert manifest['pins'][str(HELPER)] == HELPER_SHA
                pins.update(manifest['pins'])
                verify(pins)
                # Imports happen only after the original source pins are checked
                # and while this process owns the common mutex.
                sys.dont_write_bytecode = True
                spec = importlib.util.spec_from_file_location('v_explicit_continuation_runner', RUNNER)
                runner = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(runner)
                module = runner.configure('binarytrees')
                assert module.LOCK == LOCK and module.OUT == ORIGINAL
                assert runner.review_manifest(module, MANIFEST_SHA, 'binarytrees') == manifest
                harness = module.load_harness()
                assert tuple(harness.compact.HASHES['binarytrees'][1]) == (REFERENCE['bytes'], REFERENCE['sha256'])
                old = json.loads(OLD_REPORT.read_text())
                raw = json.loads(OLD_RAW.read_text())
                assert old['status'] == 'running' and old['active'] is None and len(old['attempts']) == 1
                assert old['manifest'] == str(MANIFEST) and old['manifest_sha256'] == MANIFEST_SHA
                assert old['supervisor']['pid'] == 37064 and module.identity(37064) is None
                assert old['orders'] == manifest['orders']
                rows = {row['id']: row for row in manifest['profiles']}
                assert len(rows) == 3 and set(old['profiles']) == set(rows)
                first = old['attempts'][0]
                original_raw = copy.deepcopy(first)
                assert original_raw.pop('environment_attempt') == 1 and original_raw == raw
                assert first['id'] == FIRST_ID and first['attempt'] == first['round'] == 1
                assert type(first['attempt']) is type(first['round']) is int
                assert first['seconds'] == FIRST_SECONDS and first['result_path'] == str(OLD_RAW)
                assert first['supervised_native_group']['pid'] == first['supervised_native_group']['pgid'] == 68619
                assert harness.group_members(68619) == []
                first_checks = checks(first, rows[FIRST_ID], manifest, harness)
                assert qualified(first, first_checks), 'Original sample fails typed twenty-gate retention'
                for identity, row in rows.items():
                    assert old['profiles'][identity] == dict(loc=row['loc'], role=row['role'],
                        samples_seconds=[FIRST_SECONDS] if identity == FIRST_ID else [])
                slots = [(order['round'], identity) for order in manifest['orders'] for identity in order['profiles']]
                assert len(slots) == 9 and slots[0] == (1, FIRST_ID) and len(set(slots)) == 9
                pins.update({str(OLD_REPORT): sha(OLD_REPORT), str(OLD_RAW): sha(OLD_RAW)})
                frozen = output / 'original-snapshot'
                frozen.mkdir()
                for label, source in (('manifest.json', MANIFEST), ('report.json', OLD_REPORT),
                        ('first-attempt.json', OLD_RAW), ('v-full_study.py', RUNNER), ('shared-full_study.py', HELPER)):
                    target = frozen / label
                    with target.open('xb') as stream:
                        stream.write(source.read_bytes())
                    assert target.read_bytes() == source.read_bytes()
                    pins[str(target)] = sha(target)
                receipt = output / 'retention.json'
                new_json(receipt, dict(status='passed-exact-original-sample-retention',
                    original_report=dict(path=str(OLD_REPORT), sha256=pins[str(OLD_REPORT)]),
                    original_raw=dict(path=str(OLD_RAW), sha256=pins[str(OLD_RAW)]),
                    manifest=dict(path=str(MANIFEST), sha256=MANIFEST_SHA), qualification_checks=first_checks,
                    retained_seconds=FIRST_SECONDS, old_supervisor_absent=True, original_native_group_empty=True,
                    remaining_slots=[dict(round=round_index, id=identity) for round_index, identity in slots[1:]],
                    common_descriptor=lock.fileno(), original_owner_descriptor=owner.fileno(),
                    full_performance_runs=0, chart_admission=False, report_only=True))
                pins[str(receipt)] = sha(receipt)
                # The frozen original profiles/orders/commands stay byte exact;
                # the additive pins extend the sampler's pre/post integrity guard.
                execution = copy.deepcopy(manifest)
                execution['pins'] = dict(pins)
                report.update(status='running', created_at=module.now(), manifest=str(MANIFEST),
                    manifest_sha256=MANIFEST_SHA, original_report=dict(path=str(OLD_REPORT), sha256=pins[str(OLD_REPORT)]),
                    supervisor=module.identity(os.getpid()), orders=copy.deepcopy(old['orders']),
                    profiles=copy.deepcopy(old['profiles']), attempts=copy.deepcopy(old['attempts']),
                    retention_receipt=dict(path=str(receipt), sha256=pins[str(receipt)]),
                    qualification_receipts=[], pins=dict(pins))
                module.OUT = output
                module.REPORT = output / 'report.json'
                module.save(module.REPORT, report)
            harness.install_interrupt_handlers()
            for round_index, identity in slots[1:]:
                with LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    harness.compact.wait_for_environment(harness.REQUIRED)
                    verify(pins)
                    print('FULL REMAINING V TREE', round_index, identity, 21, flush=True)
                    attempt = module.one_attempt(harness, rows[identity], execution, report, round_index, lock.fileno())
                    attempt['environment_attempt'] = 1
                    report['attempts'].append(attempt)
                    report['fresh_attempt_count'] += 1
                    report['active'] = None
                    module.save(module.REPORT, report)
                    verify(pins)
                    gates = checks(attempt, rows[identity], execution, harness)
                    raw_path = Path(attempt['result_path'])
                    assert raw_path.parent == output / 'attempts'
                    raw_sha = sha(raw_path)
                    gate_path = output / 'checks' / ('%03d-%s.json' % (attempt['attempt'], identity))
                    new_json(gate_path, dict(status='passed' if qualified(attempt, gates) else 'failed',
                        attempt=attempt['attempt'], round=round_index, id=identity,
                        raw_result=dict(path=str(raw_path), sha256=raw_sha), qualification_checks=gates,
                        report_only=True, never_chart_eligible=True, chart_admission=False))
                    gate_sha = sha(gate_path)
                    report['qualification_receipts'].append(dict(path=str(gate_path), sha256=gate_sha))
                    pins.update({str(raw_path): raw_sha, str(gate_path): gate_sha})
                    report['pins'] = dict(pins)
                    execution['pins'] = dict(pins)
                    if qualified(attempt, gates):
                        report['profiles'][identity]['samples_seconds'].append(attempt['seconds'])
                    module.save(module.REPORT, report)
                    assert qualified(attempt, gates), 'Unqualified sample retained; zero retries; continuation stopped'
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                verify(pins)
                assert report['fresh_attempt_count'] == 8 and len(report['attempts']) == 9
                assert report['attempts'][0] == old['attempts'][0]
                for profile in report['profiles'].values():
                    assert len(profile['samples_seconds']) == 3
                    profile['median_seconds'] = statistics.median(profile['samples_seconds'])
                    profile['sample_stdev_seconds'] = statistics.stdev(profile['samples_seconds'])
                report.update(status='complete', finished_at=module.now(), pins_unchanged_at_end=True)
                module.save(module.REPORT, report)
        except BaseException as error:
            if created:
                with LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    report.update(status='stopped', error=str(error) or type(error).__name__,
                        error_type=type(error).__name__, original_report_mutated=False)
                    if module is not None:
                        module.save(output / 'report.json', report)
                    else:
                        new_json(output / 'failure.json', report)
                    with (output / 'actual-error.log').open('x') as stream:
                        stream.write(traceback.format_exc())
            raise
    print(json.dumps(dict(report=str(output / 'report.json'), status=report['status'],
        retained_attempts=1, fresh_attempts=8, report_only=True, chart_admission=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert args.revision != 'full-study'
    run(args.revision)


if __name__ == '__main__':
    main()
