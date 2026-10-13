"""Reuse the actual six-fixture seal for the original bounded Rebol intake."""
import argparse
from contextlib import ExitStack
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASE = P.with_name('runtime-v1')
BASE_SHA = '7e2d3cf696273b8d96f9a31b752e7170da69ea0240b577de6c9dbe118f8ec117'
PROPOSAL = H / 'proposals/runtime-v1.json'
PROPOSAL_SHA = 'c50f70950c75fd054d1b5f50c859b3359b8971027ccfeb4b2b3f99d91e10ea06'
SEAL = H / 'proposal-seals/runtime-v1'
REPORT_SHA = '0d8da0373e5877f5520c51f5aab2bb954aa42d546996a0b6c2b42665072c3440'
OWNER = C / 'owned-jobs/rebol-runtime6-fixture-seal-v1'
SOURCE_SHAS = {
    'bootstrap_runtime.py': BASE_SHA,
    'contracts.py': '6f31e09de8127e364e5d3a8441d9a240c91592fd5d35b2795da82879193c1ad3',
    'fixture_runner.py': 'cae13d0967f820949c9eea820ad6c4b74bf6d6aee24dd41378357fb71433e24e',
    'test_runtime.py': '0b7c55b8db20baed19eb5e2a1572ca97ed128d10e21f4d1ee4bb23c77b175458',
    'runtime_probe.reb': '3155db11b58322a31a13246491d28fcdd91b8831c024b51c903f9baa47590f63',
    'REVIEW.md': 'af3f2f21005f8ae13c504428dc853db4346381cafa6b478fbd43614a601e5442',
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert sha(path) == digest and (str(path) not in pins or pins[str(path)] == digest), str(path)
    pins[str(path)] = digest
    resolved = path.resolve(strict=True)
    if resolved != path:
        assert sha(resolved) == digest and (str(resolved) not in pins or pins[str(resolved)] == digest)
        pins[str(resolved)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def cold(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    return module


def retain_seal(args, pins):
    report = read(SEAL / 'report.json', REPORT_SHA, pins)
    ended = read(SEAL / 'terminal-pin.json', args.seal_terminal_sha256, pins)
    assert report['status'] == ended['status'] == 'sealed-awaiting-root-review'
    assert report['terminal_evidence_complete'] is True and ended['report_sha256'] == REPORT_SHA
    assert ended['report'] == str(SEAL / 'report.json') and ended['partial_evidence_pins'] == report['pins']
    assert report['actual_fixture_count'] == 6 and report['downloads_started'] == report['native_commands_started'] == 0
    assert report['proposal'] == str(PROPOSAL) and report['proposal_sha256'] == PROPOSAL_SHA
    assert report['chart_admission'] is report['runtime_configuration_admitted'] is report['source_work_qualified'] is False
    assert report['full_processes_authorized'] == report['benchmark_processes_authorized'] == 0
    for path, digest in report['pins'].items():
        bind(path, digest, pins)
    proposal = read(PROPOSAL, PROPOSAL_SHA, pins)
    for path, digest in proposal['pins'].items():
        bind(path, digest, pins)
    done = read(OWNER / 'terminal.json', args.owner_terminal_sha256, pins)
    request, launch, worker, started = [read(OWNER / name, sha(OWNER / name), pins) for name in
        ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
    command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
        '-E', '-S', '-B', '-u', str(BASE / 'bootstrap_runtime.py'), '--seal']
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3]) for item in (request, launch, worker, started))
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert done['status'] == 'command-exited-zero-awaiting-report-review' and type(done['exit_code']) is int and done['exit_code'] == 0
    assert done['actual_command_started'] is True and done['stop_signals'] == [] and not done.get('error')
    assert request['automatic_retry'] is request['qualification_granted'] is done['automatic_retry'] is done['qualification_granted'] is False
    assert done['worker_identity'] == worker['worker_identity'] and done['command_identity'] == started['command_identity']
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert done['command_pid'] == started['command_pid'] == 40179 and worker['worker_identity']['pid'] == 40175
    assert launch['worker_identity']['pid'] == 40175 and launch['worker_identity']['listing'].split()[2:] == worker['worker_identity']['listing'].split()[2:]
    assert done['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    observed = started['command_identity']['listing'].split(None, 7)
    assert observed[:2] == ['40179', '40175'] and observed[7].split() == command
    for name, digest in SOURCE_SHAS.items():
        assert pins[str(BASE / name)] == digest
        bind(BASE / name, digest, pins)
    base = cold(BASE / 'bootstrap_runtime.py', 'rebol_retained_original_runtime_entry', pins)
    contracts = cold(BASE / 'contracts.py', 'rebol_retained_fixture_contracts', pins)
    contracts.authorize('run', proposal)
    fixture_path = SEAL / 'fixtures/fixture-receipt.json'
    fixture = read(fixture_path, pins[str(fixture_path)], pins)
    assert fixture == proposal['fixture_receipt'] and fixture['loader_errors'] == []
    assert fixture['python_executable'] == command[0] and fixture['execution_completed'] is fixture['inherited_common_mutex'] is True
    assert fixture['expected_test_ids'] == contracts.IDS and fixture['tests_run'] == fixture['discovered_tests'] == 6
    for key in ('actual_test_ids', 'discovered_test_ids'):
        assert len(fixture[key]) == len(set(fixture[key])) == 6 and set(fixture[key]) == set(contracts.IDS)
    assert all(fixture[key] == [] for key in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
    for name in ('contracts.py', 'test_runtime.py'):
        snapshot = SEAL / 'fixtures/fixture-sources' / name
        assert pins[str(snapshot)] == SOURCE_SHAS[name]
        bind(snapshot, SOURCE_SHAS[name], pins)
    base.verify(pins)
    return base


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'seal-terminal', 'owner-terminal'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    print('WAIT common mutex: retained-six-fixture Rebol runtime intake only', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        lease = leases.enter_context((OWNER / 'owner.lock').open('r+'))
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        held, named = os.fstat(lease.fileno()), (OWNER / 'owner.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        output = H / 'runtime-qualification/retained-intake-v1'
        assert not output.exists() and not (H / '.build/runtime-v1').exists(), 'No old intake retry or output overwrite'
        for target in (output, H / '.build/runtime-v1'):
            assert target.resolve(strict=False).is_relative_to(H), 'Prospective intake path escapes cohort'
            ancestor = target
            while ancestor != H:
                assert not ancestor.is_symlink(), 'Symlinked intake output or private-root ancestor'
                ancestor = ancestor.parent
        output.mkdir(parents=True)
        pins, base, failure, inherited = {}, None, None, None
        report = dict(status='retained-seal-intake-started', inherited_common_mutex=True,
            actual_fixture_methods_reexecuted=0, benchmark_processes_authorized=0, full_processes_authorized=0,
            chart_admission=False, runtime_configuration_admitted=False, source_work_qualified=False,
            header_generation_executed=False, extension_build_executed=False, prior_fixture_bytes_preserved=True)
        try:
            for path, digest in ((P / 'intake.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            assert all(not getattr(item, '__file__', None) or not Path(item.__file__).resolve().is_relative_to(H.parents[1]) or
                Path(item.__file__).resolve() == P / 'intake.py' for item in list(sys.modules.values()))
            base = retain_seal(args, pins)
            inherited = fcntl.fcntl(common.fileno(), fcntl.F_DUPFD, 64)
            os.set_inheritable(inherited, True)
            report['retained_seal'] = dict(proposal=str(PROPOSAL), proposal_sha256=PROPOSAL_SHA,
                report=str(SEAL / 'report.json'), report_sha256=REPORT_SHA,
                body_terminal_sha256=args.seal_terminal_sha256, durable_terminal_sha256=args.owner_terminal_sha256,
                original_runtime_fixtures=6, original_owned_service_fixtures=11, fixture_reexecutions=0)
            # Only the existing runtime branch is called; its seal/fixtures/main
            # and SDK/header/extension/application routes are unreachable.
            base.bootstrap(argparse.Namespace(proposal_sha256=PROPOSAL_SHA), output, pins, inherited, report)
            assert report['status'] == 'runtime-discovered-awaiting-source-header-and-primitive-review'
            assert report['native_command_count'] == 5 and report['source_header_generation_executed'] is False
            assert report['runtime_configuration_admitted'] is report['source_tree_commit_independently_verified'] is False
        except BaseException as error:
            failure = error
            try:
                with (output / 'error.log').open('x') as stream:
                    stream.write(traceback.format_exc())
            except BaseException as journal_error:
                report['exception_journal_error'] = str(journal_error)
            raise
        finally:
            try:
                if base is not None:
                    if 'private_root' in report:
                        for path in sorted(Path(report['private_root']).rglob('*')):
                            if path.is_file():
                                try:
                                    pins = base.merge(pins, {str(path.resolve()): sha(path)})
                                except BaseException as error:
                                    report.setdefault('private_root_pin_errors', []).append(dict(path=str(path), error=str(error)))
                                    if failure is None:
                                        failure = error
                    base.terminal(output, report, pins, failure)
                else:
                    available, errors = {}, []
                    for path, expected in pins.items():
                        try:
                            available[path] = sha(path)
                            assert available[path] == expected
                        except BaseException as error:
                            errors.append(dict(path=path, error=str(error)))
                    for path in output.rglob('*'):
                        if path.is_file():
                            try:
                                available[str(path)] = sha(path)
                            except BaseException as error:
                                errors.append(dict(path=str(path), error=str(error)))
                    report.update(status='failed-before-runtime-service', error=str(failure), pins=pins,
                        available_evidence_pins=available, terminal_artifact_errors=errors, terminal_evidence_complete=not errors)
                    with (output / 'report.json').open('x') as stream:
                        json.dump(report, stream, indent=2)
                    with (output / 'terminal-pin.json').open('x') as stream:
                        json.dump(dict(status=report['status'], report=str(output / 'report.json'),
                            report_sha256=sha(output / 'report.json'), partial_evidence_pins=pins,
                            available_evidence_pins=available, inherited_common_mutex=True,
                            chart_admission=False, full_processes_authorized=0), stream, indent=2)
                if failure is not None:
                    raise failure
            finally:
                if inherited is not None:
                    os.close(inherited)


if __name__ == '__main__':
    main()
