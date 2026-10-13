"""Wait for actual23 fixtures, then enter the reviewed original-input replay.

No fixture retry, native command or canonical LOC admission. The durable target
lease spans the wait and exact exec handoff. Dependency completion is read only
after its private owner lease closes, before acquiring the common timing lock.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
DEPENDENCY = HERE/'owned-jobs/c3-data-owner23-fixtures-v5'
FIXTURES = CLBG/'loc-normalization/20261012/c3-proposal-v5/jobs/fixtures-v5'
PACKET = CLBG/'loc-normalization/20261012/c3-source-replay-v2'
PRIOR = CLBG/'loc-normalization/20261012/c3-source-replay-v1/jobs/c3-v4-source-replay-restored-v2'
OUT = HERE/'jobs/c3-grammar5-source24-handoff-v1'


def require(condition, message):
    if not condition: raise ValueError(message)


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def read(path, pins, expected=None):
    path = Path(path).resolve(strict=True)
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    require(expected is None or expected == digest, 'Changed actual prerequisite bytes')
    require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting prerequisite pin')
    pins[str(path)] = digest
    return raw, digest


def main():
    print('WAIT dependency lease: C3 actual23 grammar fixtures', flush=True)
    with (DEPENDENCY/'owner.lock').open('a+') as dependency:
        fcntl.flock(dependency, fcntl.LOCK_EX)
        held_lease, named_lease = os.fstat(dependency.fileno()), (DEPENDENCY/'owner.lock').stat()
        require((held_lease.st_dev, held_lease.st_ino) == (named_lease.st_dev, named_lease.st_ino),
            'Dependency owner lease pathname differs from the held descriptor')
        # A terminal pathname alone can refer to a partly written JSON file.
        # Lease closure is the authoritative dependency completion boundary.
        require((DEPENDENCY/'terminal.json').is_file(), 'Dependency ended without a terminal receipt')
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            actual, named = os.fstat(lock.fileno()), LOCK.stat()
            require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino), 'Different common mutex')
            OUT.mkdir(parents=True, exist_ok=False)
            pins, argv = {}, None
            try:
                terminal = json.loads(read(DEPENDENCY/'terminal.json', pins)[0])
                require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
                    type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
                    terminal['actual_command_started'] is True and
                    terminal['command_identity_observation_missing'] is False and
                    terminal['stop_signals'] == [] and
                    not any(terminal.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure')),
                    'Fixture durable target did not finish normally')
                request = json.loads(read(DEPENDENCY/'request.json', pins)[0])
                started = json.loads(read(DEPENDENCY/'command-started.json', pins)[0])
                require(request['command'] == started['command'] and request['cwd'] == started['cwd'] and
                    started['command_pid'] == terminal['command_pid'] and
                    started['command_identity'] == terminal['command_identity'], 'Changed original fixture command identity')
                receipt_raw, receipt_sha = read(FIXTURES/'fixture-receipt.json', pins)
                receipt = json.loads(receipt_raw)
                pin_raw, pin_sha = read(FIXTURES/'fixture-receipt-pin.json', pins)
                pin = json.loads(pin_raw)
                require(receipt['status'] == 'passed-awaiting-source-review' and
                    receipt['tests_run'] == 23 and type(receipt['tests_run']) is int and
                    receipt['exact_test_execution'] is receipt['pin_reconciliation_passed'] is True and
                    receipt['exclusive_mutex_held'] is True and
                    receipt['pins_before'] == receipt['pins_after'] and
                    pin['terminal_worker_status'] == pin['saved_receipt_status'] == receipt['status'] and
                    pin['common_mutex_held'] is True and pin['receipt'] == str(FIXTURES/'fixture-receipt.json') and
                    pin['receipt_sha256'] == receipt_sha and
                    pin['terminal_artifact_pins'][str(FIXTURES/'fixture-receipt.json')] == receipt_sha,
                    'Actual passed fixture and terminal receipt are inconsistent')
                require(all(type(receipt[key]) is int and receipt[key] == 0 for key in
                    ('failures','errors','skipped','expected_failures','unexpected_successes',
                     'benchmark_processes_started','subprocesses_started_by_fixtures')),
                    'Fixture outcome includes unsuccessful or native work')
                for path, digest in receipt['pins_before'].items(): read(path, pins, digest)
                for path, digest in pin['terminal_artifact_pins'].items(): read(path, pins, digest)
                _, raw_sha = read(FIXTURES/'fixtures.log', pins)
                require(receipt['raw_fixture_log'] == dict(path=str(FIXTURES/'fixtures.log'), sha256=raw_sha),
                    'Original actual fixture log differs')
                _, prior_sha = read(PRIOR/'terminal-pins.json', pins)
                for path in (PACKET/'qualify_replay.py', PACKET/'REVIEW.md', Path(__file__).resolve()):
                    require(not path.stat().st_mode & 0o222, 'Reviewed replay source must be frozen before handoff')
                    read(path, pins)
                read(Path(sys.executable).resolve(), pins)
                argv = [sys.executable, '-B', '-u', str(PACKET/'qualify_replay.py'),
                    '--revision', 'grammar5-source24-v1', '--fixture-sha256', receipt_sha,
                    '--raw-fixture-sha256', raw_sha, '--fixture-terminal-sha256', pin_sha,
                    '--prior-replay-terminal-sha256', prior_sha]
                save(OUT/'handoff.json', dict(status='reviewed-prerequisites-entering-source-replay',
                    command=argv, source_or_timing_mutations=0, pins=pins, common_mutex_held=True,
                    canonical_LOC_admitted=False, chart_admission=False, automatic_retry=False))
            except BaseException as error:
                (OUT/'error.log').write_text(traceback.format_exc())
                save(OUT/'failed-handoff.json', dict(status='failed-handoff-precondition',
                    error=str(error) or type(error).__name__, error_type=type(error).__name__,
                    available_input_pins=pins, common_mutex_held=True, automatic_retry=False,
                    canonical_LOC_admitted=False, chart_admission=False))
                raise
    # The replay reacquires and rechecks all actual prerequisite/source bytes.
    # Keep the same durable command PID/lease; do not spawn a second owner.
    os.execv(sys.executable, argv)


if __name__ == '__main__': main()
