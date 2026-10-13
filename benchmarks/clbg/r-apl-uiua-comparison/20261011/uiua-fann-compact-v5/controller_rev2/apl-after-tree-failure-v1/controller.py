"""Unexecuted additive independent APL Nbody/Fann recovery; original75 preserved."""
import argparse
from contextlib import contextmanager
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import traceback

PACKET = Path(__file__).resolve().parent
PRIOR = PACKET.parent / 'apl-argv-v1'
N = PACKET.parents[2]
LOCK = Path('/tmp/mech-clbg-timing.lock')
COMMON = PACKET.parent / 'remaining_owner.py'
COMMON_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
PRIOR_MANIFEST_SHA = 'cfc246e46aa4e67db659dae89009259d5d9188381b007f486b0266cc973ed1b8'
OUT = PACKET / 'full-owner'
MANIFEST = OUT / 'manifest.json'
STAGES = [('apl', name) for name in ('nbody', 'fannkuchredux')]
STATIC = ('controller.py', 'test_controller.py', 'fixture_runner.py', 'REVIEW.md')
GATES = {'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged'}
SAFE = GATES - {'native_exited_normally', 'no_execution_error', 'no_resource_violation',
    'complete_output_reference_match', 'complete_output_bytes_and_hash'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def module(path, name):
    assert name not in sys.modules, 'Fresh source identity'
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    assert Path(value.__file__).resolve() == path.resolve()
    return value


def merge(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest, ('Conflicting pin', path)
            result[path] = digest
    return result


def isolated(guard, attempt):
    checks = attempt.get('qualification_checks')
    return (type(checks) is dict and set(checks) == GATES and
        all(type(value) is bool for value in checks.values()) and
        type(attempt.get('sampler_wait_status')) is int and attempt['sampler_wait_status'] == 0 and
        attempt.get('qualified') is False and guard.benchmark_failure(attempt) and
        all(checks[key] is True for key in SAFE))


def seed(old, status, failure, guard):
    assert status['status'] == 'stopped' and status['supervisor']['pid'] == 17798
    assert status['manifest_sha256'] == PRIOR_MANIFEST_SHA
    assert status['error_type'] == 'RuntimeError' and status['error'] == 'Remaining actual sample failed qualification; evidence retained and no retry'
    assert [(item['benchmark'], item.get('status')) for item in status['stages']] == [('fasta', 'complete'), ('knucleotide', 'complete'), ('binarytrees', None)]
    assert old['status'] == 'partial' and not old.get('active') and len(old['attempts']) == 75
    assert old['attempts'][-1] == failure and type(failure['attempt']) is int and failure['attempt'] == 75
    assert failure['id'] == 'binarytrees-apl-compact' and failure['workload_n'] == 21 and failure['full_workload'] is True
    assert failure['error_type'] == 'OutputValidationError' and failure['stdout_bytes'] == 0 and failure['exit_status'] == 0
    assert 'WS FULL+' in failure['stderr'] and 'Make[4]  Left←Make D-1' in failure['stderr']
    assert isolated(guard, failure)
    for benchmark in ('nbody', 'fannkuchredux'):
        selected = [row for row in old['programs'].values() if row['benchmark'] == benchmark]
        native = [row for row in selected if not row.get('timing_alias_of')]
        assert len(native) == 2 and {row['language'] for row in native} == {'apl', 'mech'}
        assert all(row['samples_seconds'] == [] and row['sample_validation'] == [] and not row.get('full_failure') for row in selected)
    result = copy.deepcopy(old)
    result['independent_recovery'] = dict(status='prepared-independent-unattempted-stages',
        preserved_attempts=75, failed_tree_attempt=75, failed_source_retried=False,
        stages=[dict(language=a, benchmark=b) for a, b in STAGES],
        original_report_mutated=False, original_sources_and_runtime_changed=False,
        aliases_remain_aliases=True, full_processes_planned=12, chart_admission=False,
        tree_control_pending=True, overall_suite_completion_pending=True)
    result['isolated_failed_profiles'] = []
    result['skipped_failed_profile_slots'] = []
    return result


def infrastructure(prior_manifest):
    assert sha(COMMON) == COMMON_SHA
    guard = module(COMMON, 'apl_recovery_frozen_native_guard')
    guard.verify(prior_manifest['pins'])
    base_path = PRIOR / 'continuation.py'
    descriptor_path = PRIOR / 'descriptor_guard.py'
    assert prior_manifest['pins'][str(base_path)] == sha(base_path)
    assert prior_manifest['pins'][str(descriptor_path)] == sha(descriptor_path)
    descriptor = module(descriptor_path, 'apl_recovery_high_descriptor_adapter')
    # The original adapter import is resolved only to its reviewed source.
    assert 'descriptor_guard' not in sys.modules
    sys.modules['descriptor_guard'] = descriptor
    base = module(base_path, 'apl_recovery_original_argv_controller')
    assert base.COMMON_SHA == COMMON_SHA and base.LOCK == LOCK
    guard.LOCK = descriptor.HighDescriptorLock(LOCK)
    guard.OUT, guard.MANIFEST, guard.STAGES = OUT, MANIFEST, STAGES
    return guard, base


def original_records(guard, manifest):
    report_path = PRIOR / 'full-owner/timings-apl.json'
    status_path = PRIOR / 'full-owner/status.json'
    old = json.loads(report_path.read_text())
    status = json.loads(status_path.read_text())
    failure = old['attempts'][-1]
    retained = {str(report_path): sha(report_path), str(status_path): sha(status_path)}
    historical_without_raw = []
    for attempt in old['attempts']:
        if 'result_path' not in attempt:
            # The original pre-controller producer saved some historical
            # outcomes only in its report. Preserve them as such, never invent
            # a raw receipt or claim a new twenty-gate qualification.
            assert 'supervisor_result_path' not in attempt
            assert type(attempt['attempt_number']) is int and attempt['attempt_number'] < 75
            historical_without_raw.append(attempt['attempt_number'])
            continue
        for key in ('result_path', 'supervisor_result_path'):
            path = Path(attempt[key])
            assert path.is_relative_to(N)
            retained[str(path)] = sha(path)
        enriched = json.loads(Path(attempt['supervisor_result_path']).read_text())
        # Report adds only its ledger fields after the immutable supervisor save.
        assert all(attempt[key] == value for key, value in enriched.items())
    assert json.loads(Path(failure['supervisor_result_path']).read_text())['stderr'] == failure['stderr']
    result = seed(old, status, failure, guard)
    result['independent_recovery']['historical_report_only_attempts_preserved'] = historical_without_raw
    return result, status, retained


def seal(lock):
    prior_path = PRIOR / 'full-owner/manifest.json'
    assert sha(prior_path) == PRIOR_MANIFEST_SHA
    prior = json.loads(prior_path.read_text())
    guard, base = infrastructure(prior)
    assert not OUT.exists(), 'Exclusive additive seed, never replace failed or completed evidence'
    with (N / 'results/coordinator-v4/owner-apl.lock').open('a+') as lease:
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        report, status, retained = original_records(guard, prior)
        assert guard.identity(17798) is None
        harness = guard.load_harness()
        paths = [PACKET / name for name in STATIC]
        pins = merge(prior['pins'], retained, {str(prior_path): PRIOR_MANIFEST_SHA}, {str(path): sha(path) for path in paths})
        contracts = base.preflight(report, harness, pins)
        selected_contracts = {key: value for key, value in contracts.items() if report['programs'][key]['benchmark'] in {'nbody', 'fannkuchredux'}}
        assert {report['programs'][key]['benchmark'] for key in selected_contracts} == {'nbody', 'fannkuchredux'}
        # The frozen order generator is applied to a new report only. Earlier
        # measured stages and their sample/order/failed records remain equal.
        prior_orders = copy.deepcopy(report['orders'])
        slots = []
        for language, benchmark in STAGES:
            for sample in range(3):
                rows = guard.ordered_candidates(report, language, benchmark, sample)
                assert len(rows) == 2
                slots.extend(dict(benchmark=benchmark, sample=sample + 1, key=row['key']) for row in rows)
        assert len(slots) == 12 and all(report['orders'][key] == value for key, value in prior_orders.items())
        OUT.mkdir(parents=True)
        helper = module(base.HELPER, 'apl_recovery_original_owned_helper')
        environment = dict(os.environ, PYTHONPATH=str(PACKET), PYTHONDONTWRITEBYTECODE='1')
        helper.owned_command(guard, harness, [sys.executable, '-B', str(PACKET / 'fixture_runner.py'),
            '--receipt', str(OUT / 'fixtures/inventory.json'), '--lock-fd', str(lock.fileno())],
            environment, OUT / 'fixtures', lock.fileno(), 30, 'three-mocked-recovery-fixtures')
        fixture = json.loads((OUT / 'fixtures/inventory.json').read_text())
        assert fixture['status'] == 'passed' and fixture['tests_run'] == 3
        pins = merge(pins, helper.pin_tree(OUT / 'fixtures'))
        before = OUT / 'apl-before.json'; report_path = OUT / 'timings-apl.json'
        new_json(before, report); new_json(report_path, report)
        pins[str(before)] = sha(before)
        guard.verify(pins)
        manifest = dict(prior, status='prepared-awaiting-independent-review', created_at=guard.now(),
            stages=[dict(language=a, benchmark=b) for a, b in STAGES], reports=dict(apl=str(report_path)),
            before_reports=dict(apl=dict(path=str(before), sha256=sha(before))), pins=pins,
            old_owner_identities=[*prior['old_owner_identities'], status['supervisor']],
            input_contracts=selected_contracts, planned_actual_full_processes=12, planned_slots=slots,
            preserved_terminal_failure=dict(attempt=75, original_report=str(PRIOR / 'full-owner/timings-apl.json'),
                original_report_sha256=retained[str(PRIOR / 'full-owner/timings-apl.json')],
                source_failure_isolation_exact_twenty_typed_gates=True, safety_gate_count=15,
                failed_tree_sources_and_all75_records_immutable=True),
            fixture_receipt=str(OUT / 'fixtures/inventory.json'), fixture_receipt_sha256=sha(OUT / 'fixtures/inventory.json'),
            no_duplicate_qualified_samples=True, no_automatic_retry=True, chart_admission=False,
            mutation_policy='Only new independent APL report/attempt/status lineage. All75 original entries, failed tree rows and completed K samples remain immutable.',
            failure_policy='Verified clean source failures retain failed rows and explicit later-slot skips while independent form/control/stages continue. Integrity/environment/ownership failures stop.')
        new_json(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), planned_actual_full_processes=12)), flush=True)


def run(digest):
    # Initial imports/pins and source binding own the same promoted description.
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(MANIFEST) == digest
        manifest = json.loads(MANIFEST.read_text())
        prior = json.loads((PRIOR / 'full-owner/manifest.json').read_text())
        guard, base = infrastructure(prior)
        guard.verify(manifest['pins'])
        assert manifest['planned_actual_full_processes'] == 12 and manifest['no_automatic_retry'] is True
        original = guard.record_attempt
        def preserve_independent(harness, report, row, attempt, sample, actual_digest):
            result = original(harness, report, row, attempt, sample, actual_digest)
            if result:
                assert type(attempt['qualification_checks']) is dict and set(attempt['qualification_checks']) == GATES
                assert all(type(value) is bool and value is True for value in attempt['qualification_checks'].values())
                return True
            if not isolated(guard, attempt):
                return False
            report['isolated_failed_profiles'].append(dict(key=row['key'], original_attempt=attempt['attempt'],
                source_sha256=row['source_sha256'], qualified=False, scored=False))
            report['skipped_failed_profile_slots'].extend(dict(key=row['key'], benchmark=row['benchmark'], sample=later,
                reason='Preserved earlier clean actual source failure; no automatic retry') for later in range(sample + 1, 4))
            return True
        guard.record_attempt = preserve_independent
    guard.execute_remaining(digest)


@contextmanager
def terminal(operation):
    # Final evidence pinning reacquires the mutex only after native guard cleanup.
    output = PACKET / 'controller-outcomes' / operation
    assert not output.exists(), 'Exclusive operation; explicit additive revision required after failure'
    output.mkdir(parents=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        new_json(output / 'started.json', dict(operation=operation, actual_argv=sys.argv, full_runs_started=False))
    result = dict(operation=operation, status='failed', original_evidence_mutated=False)
    try:
        yield
        result['status'] = 'completed'
    except BaseException as error:
        result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        actual_error = traceback.format_exc()
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            (output / 'actual-error.log').write_text(actual_error)
        raise
    finally:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            files = [path for root in (OUT, output) if root.exists() for path in root.rglob('*') if path.is_file()]
            result['terminal_evidence_pins'] = {str(path): sha(path) for path in files}
            result['common_mutex_held_during_final_pins'] = True
            new_json(output / 'outcome.json', result)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true'); mode.add_argument('--run', action='store_true')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    def interrupted(signum, frame): raise KeyboardInterrupt('APL independent recovery interrupted ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(signum, interrupted)
    with (PACKET / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        print('WAIT common mutex: APL independent recovery', flush=True)
        with terminal('seal' if args.seal else 'run'):
            if args.seal:
                with LOCK.open('a+') as lock:
                    duplicate = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD_CLOEXEC, 64)
                    with os.fdopen(duplicate, 'a+') as promoted:
                        fcntl.flock(promoted, fcntl.LOCK_EX); seal(promoted)
            else:
                assert args.manifest_sha256 and len(args.manifest_sha256) == 64
                run(args.manifest_sha256)


if __name__ == '__main__': main()
