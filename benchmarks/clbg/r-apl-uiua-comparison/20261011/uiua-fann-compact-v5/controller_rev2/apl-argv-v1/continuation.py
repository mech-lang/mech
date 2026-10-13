"""Additive APL argv correction; frozen native ownership/validation is reused.

Preparation is bounded and unscored. Root reviews its exact full manifest before
--detach. Original reports/audits/sources and every prior attempt stay immutable.
"""
import argparse
from contextlib import ExitStack
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
from descriptor_guard import HighDescriptorLock, NativeObservation

HERE = Path(__file__).resolve().parent
PRIOR = HERE.parent
N = PRIOR.parent.parent
W = N.parents[3]
COMMON = PRIOR / 'remaining_owner.py'
COMMON_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
PRIOR_MANIFEST_SHA = 'affba5b243da43d1a15c28e5364741828e4f684bb1b43d15d81ad720b28ef39f'
OLD_OWNER = 52680
LOCK = Path('/tmp/mech-clbg-timing.lock')
PROTECTED_LOCK = HighDescriptorLock(LOCK)
OUT = HERE / 'full-owner'
MANIFEST = OUT / 'manifest.json'
ORIGINAL_REPORT = N / 'results/timings-apl.json'
ORIGINAL_AUDIT = N / 'source-audit-apl.json'
KEY = 'fasta:fasta-apl-affine-hoisted-optimized-v3'
STAGES = [('apl', name) for name in ('fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')]
SIZES = (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000)
HELPER = W / 'benchmarks/clbg/nim-comparison/20261012/guarded_prepare_v2.py'
HELPER_PROPOSAL = HELPER.parent / 'proposals/private-v2.json'
HELPER_SHA = '76e37273079d8a9e8c8541cbe67a36b96a64753d337670a03bf63cd01c65da52'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def guard():
    assert sha(COMMON) == COMMON_SHA
    return module(COMMON, 'apl_argv_frozen_native_guard')


def merge(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest
            result[path] = digest
    return result


def corrected(report, audit):
    """Only missing numeric delivery changes; retain the failed old disposition."""
    result, revised = copy.deepcopy(report), copy.deepcopy(audit)
    row = result['programs'][KEY]
    assert row['benchmark'] == 'fasta' and row['language'] == 'apl'
    assert row['samples_seconds'] == [] and row['sample_validation'] == []
    assert row['command'] == [str(N / 'run-apl.sh'), '--cc', '8', '-f', row['source']]
    failure = row['full_failure']
    assert failure['error_type'] == 'OutputValidationError' and failure['stdout_bytes'] == 0 and failure['exit_status'] == 0
    assert report['attempts'][-1]['attempt'] == 58 and report['attempts'][-1]['key'] == KEY
    assert report['attempts'][-1]['qualified'] is False
    assert report['attempts'][-1]['command'] == row['command']
    row['command_metadata_failure_lineage'] = copy.deepcopy(failure)
    row.pop('full_failure')
    row['measurement_status'] = 'pending-corrected-argv'
    row['command'] += ['--', '{n}']
    declared = [entry for entry in revised['programs'] if entry['key'] == KEY]
    assert len(declared) == 1 and declared[0]['command'] == report['programs'][KEY]['command']
    declared[0]['command'] += ['--', '{n}']
    assert result['attempts'] == report['attempts'] and result['orders'] == report['orders']
    assert result['completed_benchmarks'] == report['completed_benchmarks']
    assert all(result['programs'][key] == value for key, value in report['programs'].items() if key != KEY)
    result['argv_revision'] = dict(status='corrected-command-only-awaiting-new-fulls', key=KEY,
        old_command=report['programs'][KEY]['command'], new_command=row['command'],
        original_failed_attempt=58, original_samples_preserved=True, sources_changed=False)
    result.pop('active', None)
    result['status'] = 'partial'
    revised['argv_revision'] = copy.deepcopy(result['argv_revision'])
    return result, revised


def preflight(report, harness, pins):
    """Tie each reviewed remaining command to actual argv or pinned stdin."""
    contracts = {}
    selected = {name for _, name in STAGES}
    for key, row in report['programs'].items():
        if row['benchmark'] not in selected:
            continue
        command = row['command']
        assert type(command) is list and all(type(part) is str and part for part in command)
        assert row['source'] in pins and pins[row['source']] == row['source_sha256']
        assert '{source}' not in command and not any('{' in part and part != '{n}' for part in command)
        if row['language'] == 'apl':
            assert command[:4] == [str(N / 'run-apl.sh'), '--cc', command[2], '-f']
            assert command[2] in ('1', '2', '3', '4', '5', '6', '7', '8') and command[4] == row['source']
            prefix = 5
        else:
            assert row['language'] == 'mech' and command[0] in pins
            assert row['binary_sha256'] == pins[command[0]]
            prefix = 1
        if row['benchmark'] == 'knucleotide':
            assert len(command) == prefix and '{n}' not in command
            files = harness.INPUTS[row['benchmark']]
            assert len(files) == 2 and all(str(path) in pins for path in files)
            contracts[key] = dict(mode='stdin', small=str(files[0]), full=str(files[1]),
                small_sha256=pins[str(files[0])], full_sha256=pins[str(files[1])])
        else:
            assert command[prefix:] == (['--', '{n}'] if row['language'] == 'apl' else ['{n}'])
            assert command.count('{n}') == 1 and row['benchmark'] not in harness.INPUTS
            contracts[key] = dict(mode='numeric-argv', small=harness.WORK[row['benchmark']][0],
                                  full=harness.WORK[row['benchmark']][1])
    assert {report['programs'][key]['benchmark'] for key in contracts} == selected
    return contracts


def prepare():
    print('WAIT LOCK: bounded APL argv correction and immutable continuation seal', flush=True)
    with PROTECTED_LOCK.open('a+') as lock, ExitStack() as leases:
        fcntl.flock(lock, fcntl.LOCK_EX)
        g = guard()
        assert g.identity(OLD_OWNER) is None and g.identity(31810) is None and g.identity(54174) is None
        lease = leases.enter_context((N / 'results/coordinator-v4/owner-apl.lock').open('a+'))
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not MANIFEST.exists() and not (HERE / 'qualification').exists()
        old_manifest = PRIOR / 'remaining-owner/manifest.json'
        assert sha(old_manifest) == PRIOR_MANIFEST_SHA
        old = json.loads(old_manifest.read_text())
        g.verify(old['pins'])
        old_status = json.loads((PRIOR / 'remaining-owner/status.json').read_text())
        assert old_status['status'] == 'stopped' and old_status['supervisor']['pid'] == OLD_OWNER
        assert sha(HELPER_PROPOSAL) == HELPER_SHA
        helper_proposal = json.loads(HELPER_PROPOSAL.read_text())
        g.verify(helper_proposal['pins'])
        assert helper_proposal['pins'][str(HELPER)] == sha(HELPER)
        helper = module(HELPER, 'apl_argv_exact_owned_helper')
        harness = g.load_harness()
        harness.install_interrupt_handlers()
        original = json.loads(ORIGINAL_REPORT.read_text())
        audit = json.loads(ORIGINAL_AUDIT.read_text())
        assert not original.get('active') and original['source_audit_sha256'] == sha(ORIGINAL_AUDIT)
        assert set(original['completed_benchmarks']) == {'spectralnorm', 'mandelbrot', 'revcomp', 'regexredux', 'pidigits'}
        r_report = N / 'results/timings-r.json'
        assert len(json.loads(r_report.read_text())['completed_benchmarks']) == 10
        report, revised = corrected(original, audit)
        paths = [Path(__file__).resolve(), HERE / 'descriptor_guard.py', HERE / 'test_continuation.py', HERE / 'PLAN.md',
                 Path('/usr/sbin/lsof'), ORIGINAL_REPORT,
                 ORIGINAL_AUDIT, old_manifest, PRIOR / 'remaining-owner/status.json', r_report]
        pins = merge(old['pins'], helper_proposal['pins'], audit['pins'], {str(path): sha(path) for path in paths})
        pins = merge(pins, helper.pin_tree(PRIOR / 'remaining-owner/attempts'))
        contracts = preflight(report, harness, pins)
        assert all(harness.execution_fingerprint(report['programs'][row['key']]) == harness.execution_fingerprint(row)
                   for row in revised['programs'] + revised['controls'])
        fixtures = HERE / 'fixtures'
        helper.owned_command(g, harness, [sys.executable, '-B', str(HERE / 'test_continuation.py')],
                             dict(os.environ, MECH_TIMING_LOCK_FD=str(lock.fileno())),
                             fixtures, lock.fileno(), 30, 'mock-checks')
        pins = merge(pins, helper.pin_tree(fixtures))
        qualification = HERE / 'qualification'
        qualification.mkdir()
        g.OUT, g.REPORT = qualification, qualification / 'report.json'
        bounded = dict(status='preparing', attempts=[], active=None, full_performance_runs=0,
                       chart_admission=False, never_chart_eligible=True, no_automatic_retry=True)
        g.save(g.REPORT, bounded)
        references = module(HELPER.parent / 'references.py', 'apl_argv_independent_fasta')
        try:
            for n in SIZES:
                observer = None
                if n == SIZES[-1]:
                    observer = NativeObservation(g, helper, harness, bounded, report['programs'][KEY], n,
                                                 lock.fileno(), qualification / 'native-lock-observation', LOCK)
                try:
                    helper.probe(g, harness, report['programs'][KEY], n, None, references.fasta(n), pins,
                                 qualification, lock.fileno(), bounded, 'n-' + str(n),
                                 trace=(lambda directory: observer.finish()) if observer is not None else None)
                finally:
                    if observer is not None:
                        observer.stop.set()
                        observer.thread.join(timeout=12)
                        assert not observer.thread.is_alive(), 'Descriptor observer cleanup did not finish'
            assert len(bounded['attempts']) == len(SIZES) and all(row['qualified'] is True for row in bounded['attempts'])
            bounded.update(status='bounded-corrected-command-qualified', finished_at=g.now(), sources_changed=False)
            g.save(g.REPORT, bounded)
        except BaseException as error:
            bounded.update(status='stopped-unqualified', error=str(error), error_type=type(error).__name__, finished_at=g.now())
            g.save(g.REPORT, bounded)
            raise
        pins = merge(pins, helper.pin_tree(qualification))
        OUT.mkdir()
        overlay = OUT / 'source-audit-apl-argv.json'
        revised.update(status='reviewed', command_delivery_qualified=True,
                       original_audit=str(ORIGINAL_AUDIT), original_audit_sha256=sha(ORIGINAL_AUDIT))
        g.save(overlay, revised)
        pins[str(overlay)] = sha(overlay)
        report.update(source_audit=str(overlay), source_audit_sha256=sha(overlay))
        report['argv_revision'].update(original_report=str(ORIGINAL_REPORT), original_report_sha256=sha(ORIGINAL_REPORT),
                                      bounded_qualification=str(g.REPORT), bounded_qualification_sha256=sha(g.REPORT))
        new_report = OUT / 'timings-apl.json'
        before = OUT / 'apl-before.json'
        g.save(new_report, report)
        before.write_bytes(new_report.read_bytes())
        pins[str(before)] = sha(before)
        g.verify(pins)
        manifest = dict(status='prepared-awaiting-independent-review', created_at=g.now(), pins=pins,
            stages=[dict(language=language, benchmark=benchmark) for language, benchmark in STAGES], samples=3,
            reports=dict(apl=str(new_report)), audits=dict(apl=dict(path=str(overlay), sha256=sha(overlay))),
            before_reports=dict(apl=dict(path=str(before), sha256=sha(before))),
            old_owner_identities=[old_status['supervisor']] + old['old_owner_identities'],
            owner_leases=[str(N / 'results/coordinator-v4/owner-apl.lock')],
            timeout_seconds=14400, maximum_rss_mib=10240, shared_timing_lock=str(LOCK), input_contracts=contracts,
            bounded_qualification=dict(path=str(g.REPORT), sha256=sha(g.REPORT), cases=len(SIZES)),
            prior_failed_attempt=58, original_report_immutable=True, original_sources_and_audits_immutable=True,
            no_duplicate_qualified_samples=True, no_automatic_retry=True,
            planned_actual_full_processes=sum(max(0, 3 - len(row['samples_seconds'])) for row in report['programs'].values()
                if row['benchmark'] in {name for _, name in STAGES} and not row.get('timing_alias_of') and not row.get('full_failure')),
            native_policy=old['native_policy'], timing_policy=old['timing_policy'], failure_policy=old['failure_policy'],
            mutation_policy='Only new additive APL report/attempt/status/log lineage; original R/APL/Uiua files remain immutable.')
        g.save(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), runner_sha256=sha(__file__)), indent=2), flush=True)


def run(digest):
    g = guard()
    g.OUT, g.MANIFEST, g.STAGES, g.LOCK = OUT, MANIFEST, STAGES, PROTECTED_LOCK
    with PROTECTED_LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(MANIFEST) == digest
        manifest = json.loads(MANIFEST.read_text())
        g.verify(manifest['pins'])
        report = json.loads(Path(manifest['reports']['apl']).read_text())
        assert preflight(report, g.load_harness(), manifest['pins']) == manifest['input_contracts']
    g.execute_remaining(digest)


def cleanup_waiter(process, harness, detail):
    """Parent holds the common mutex; this child cannot have begun a trial."""
    try:
        if process.poll() is None:
            process.terminate()
            detail['owned_waiting_controller_term_sent'] = True
        try:
            process.wait(timeout=8)
        except subprocess.TimeoutExpired:
            process.kill()
            detail['owned_waiting_controller_kill_sent'] = True
            process.wait(timeout=5)
    finally:
        # Even a second parent interruption cannot leave an alive controller
        # able to acquire the mutex and launch after this failed dispatch.
        if process.poll() is None:
            process.kill()
            detail['owned_waiting_controller_kill_sent'] = True
            process.wait(timeout=5)
    detail['owned_waiting_controller_exit_status'] = process.returncode
    detail['remaining_owned_group'] = harness.group_members(process.pid)
    assert not detail['remaining_owned_group'], 'Owned launcher child group did not drain'


def detach(digest):
    with PROTECTED_LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        g = guard()
        assert sha(MANIFEST) == digest
        manifest = json.loads(MANIFEST.read_text())
        g.verify(manifest['pins'])
        assert not any((OUT / name).exists() for name in ('launch.json', 'status.json', 'launch-failure.json'))
        process = None
        failure = dict(status='detached-launch-failed', manifest_sha256=digest,
                       common_mutex_held_through_owned_cleanup=True, native_trial_started=False)
        try:
            harness = g.load_harness()
            with (OUT / 'supervisor.log').open('xb') as stream:
                process = subprocess.Popen([sys.executable, '-B', '-u', str(Path(__file__).resolve()), '--run', '--manifest-sha256', digest],
                    stdin=subprocess.DEVNULL, stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
            identity = g.identity(process.pid)
            assert identity is not None and process.poll() is None
            launch = dict(pid=process.pid, identity=identity, manifest_sha256=digest, launched_at=g.now())
            with (OUT / 'launch.json').open('x') as stream:
                json.dump(launch, stream, indent=2)
                stream.write('\n')
        except BaseException as error:
            failure.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            try:
                if process is not None:
                    failure['pid'] = process.pid
                    cleanup_waiter(process, harness, failure)
            except BaseException as cleanup_error:
                failure.update(cleanup_error=str(cleanup_error) or type(cleanup_error).__name__,
                               cleanup_error_type=type(cleanup_error).__name__)
                raise
            finally:
                failure['finished_at'] = g.now()
                with (OUT / 'launch-failure.json').open('x') as stream:
                    json.dump(failure, stream, indent=2)
                    stream.write('\n')
            raise
        print(json.dumps(launch, indent=2), flush=True)


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('APL argv continuation interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare', action='store_true')
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--detach', action='store_true')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    if args.prepare:
        prepare()
    else:
        assert args.manifest_sha256
        (run if args.run else detach)(args.manifest_sha256)


if __name__ == '__main__':
    main()
