"""Two owned halt-only controls against the closed, pinned SWI environment."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
PRIOR = H / '.build/swi-runtime-probe-v1'
OWNER = C / 'owned-jobs/prolog-runtime-probe-images-v1'
REVIEW = C / 'jobs/prolog-runtime-probe-closed-review-v1/review.json'
LEGACY = H / 'revisions/runtime-directory-v1/bootstrap_runtime_directory_v1.py'
REPORT_SHA = 'f143a7b6627af26bc46741ac287c50ef80980d355411be468b1104ded09cfe66'
TERMINAL_SHA = '1f534082e617e804a300e5f3abe887746b8a2e0d20511e0ce525f037d3c7f878'
OWNER_SHA = '3d2240941209d7557bc4a4dc99e6dab87dd7e06ae06efdd31af78c57c0eba58e'
PROPOSAL_SHA = '5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'helper', 'review', 'prior-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    with open('/tmp/mech-clbg-timing.lock', 'r+') as common:
        fcntl.flock(common, fcntl.LOCK_EX)
        held = os.fstat(common.fileno())
        named = Path('/tmp/mech-clbg-timing.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        assert held.st_ino == 79859134 and sys.dont_write_bytecode
        with (OWNER / 'owner.lock').open('r+') as prior_lease:
            fcntl.flock(prior_lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            lease_held, lease_named = os.fstat(prior_lease.fileno()), (OWNER / 'owner.lock').stat()
            assert (lease_held.st_dev, lease_held.st_ino) == (lease_named.st_dev, lease_named.st_ino)
            output = H / '.build/swi-halt-diagnostic-v1'
            output.mkdir()
            pins = {}
            legacy = None
            failure = None
            report = dict(status='running', chart_admission=False,
                          runtime_configuration_admitted=False, actual_benchmark_runs=0,
                          fixture_reexecutions=0, version_reexecutions=0,
                          feature_reexecutions=0, image_commands=0, controls=[])

            def read(path, digest):
                assert sha(path) == digest, str(path)
                pins[str(path)] = digest
                return json.loads(path.read_text())

            try:
                for name, digest in [('run_controls.py', args.worker_sha256),
                                     ('diagnose_halt.pl', args.helper_sha256),
                                     ('REVIEW.md', args.review_sha256)]:
                    path = P / name
                    assert not path.stat().st_mode & 0o222
                    assert sha(path) == digest
                    pins[str(path)] = digest
                review = read(REVIEW, args.prior_review_sha256)
                assert review['status'] == 'closed-probe-exit1-preserved'
                assert review['report_sha256'] == REPORT_SHA
                assert review['durable_terminal_sha256'] == OWNER_SHA
                assert review['verified_pins'] == 13213
                prior = read(PRIOR / 'report.json', REPORT_SHA)
                terminal = read(PRIOR / 'terminal-pin.json', TERMINAL_SHA)
                assert prior['status'] == terminal['status'] == 'stopped-unqualified'
                assert prior['terminal_evidence_complete'] is True
                assert terminal['report_sha256'] == REPORT_SHA
                assert prior['pins'] == terminal['partial_evidence_pins']
                for path, digest in prior['pins'].items():
                    assert path not in pins or pins[path] == digest
                    assert sha(path) == digest, path
                    pins[path] = digest
                done = read(OWNER / 'terminal.json', OWNER_SHA)
                request, started, worker = [read(OWNER / name, sha(OWNER / name)) for name in
                                           ('request.json', 'command-started.json', 'worker-started.json')]
                assert done['exit_code'] == 1 and done['stop_signals'] == []
                assert done['actual_command_started'] is True
                assert not any(done.get(k) for k in ('error', 'exception_cleanup'))
                assert done['command_identity'] == started['command_identity']
                assert done['worker_identity'] == worker['worker_identity']
                assert request['command'] == started['command'] == worker['command']
                assert started['command_identity']['listing'].split()[7:] == request['command']
                spec = importlib.util.spec_from_file_location('halt_control_legacy', LEGACY)
                legacy = importlib.util.module_from_spec(spec)
                sys.modules[spec.name] = legacy
                raw = LEGACY.read_bytes()
                assert hashlib.sha256(raw).hexdigest() == pins[str(LEGACY)]
                exec(compile(raw, str(LEGACY), 'exec'), legacy.__dict__)
                preparation, sealed, pins = legacy.load_preparation(
                    SimpleNamespace(proposal_sha256=PROPOSAL_SHA), pins)
                service, references, guard, harness, pins = legacy.reused_service(preparation, sealed, pins)
                guard.OUT, guard.REPORT = output, output / 'report.json'
                harness.install_interrupt_handlers()
                harness.compact.wait_for_environment(harness.REQUIRED)
                receipt = read(PRIOR / 'runtime/feature-probe.json', review['probe_receipt_sha256'])
                actual = read(PRIOR / 'runtime/feature-result.json', review['probe_result_sha256'])
                binary, probe = Path(receipt['command'][0]), H / 'revisions/runtime-probe-v1/runtime_probe.pl'
                assert receipt['command'][1:8] == preparation.prefix(binary)[1:] + ['-l', str(probe)]
                assert sha(binary) == pins[str(binary)]
                environment = dict(harness.compact.ENV)
                environment['SWI_HOME_DIR'] = actual['runtime']['home']
                for mode, expected in [('direct', 0), ('wrapped', 1)]:
                    events = output / (mode + '.events')
                    assert not events.exists()
                    command = preparation.prefix(binary) + ['-l', str(probe), '-l', str(P / 'diagnose_halt.pl'),
                        '-g', 'clbg_halt_diagnostic:' + mode, '--', str(events)]
                    caught = None
                    try:
                        service.owned_command(guard, harness, command, environment,
                                              output / mode, common.fileno(), 30, mode)
                    except AssertionError as error:
                        caught = error
                    detail = json.loads((output / mode / (mode + '.json')).read_text())
                    assert detail['command'] == command
                    assert type(detail['exit_status']) is type(detail['final_exit_status']) is int
                    assert detail['exit_status'] == detail['final_exit_status'] == expected
                    assert detail['identity'] is not None and not detail.get('cleanup_errors')
                    if expected:
                        assert caught is not None and detail['error_type'] == 'AssertionError'
                        assert caught.args == (('Owned command failed', mode, expected),)
                    else:
                        assert caught is None and not detail.get('error')
                    observed = events.read_text().splitlines()
                    expected_events = [f'started({mode}).', 'before_halt(0).', 'cleanup(exception(unwind(halt(0)))).']
                    if expected:
                        expected_events += ['recovery(unwind(halt(0))).', 'before_recovery_halt(1).']
                    assert observed == expected_events, (mode, observed)
                    assert (output / mode / (mode + '.stderr')).read_bytes() == b''
                    assert (output / mode / (mode + '.stdout')).read_bytes() == b''
                    report['controls'].append(dict(mode=mode, exit_status=expected, events=observed,
                        owned_receipt=str(output / mode / (mode + '.json'))))
                report.update(status='halt-recovery-exit-replacement-confirmed',
                              prior_failure_relabelled_as_success=False,
                              next_action='Move successful halt outside ordinary-error catch; separately qualify feature and images.')
            except BaseException as error:
                failure = error
            finally:
                if legacy is not None:
                    legacy.terminal(output, report, pins, failure)
                else:
                    report.update(status='failed-before-service', error=str(failure), pins=pins)
                    save(output / 'report.json', report)
            if failure is not None:
                raise failure
            print(json.dumps(dict(status=report['status'], controls=report['controls'], report=str(output / 'report.json'))), flush=True)


if __name__ == '__main__':
    main()
