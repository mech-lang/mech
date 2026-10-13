#!/usr/bin/env python3
"""Queue only C3 grammar fixtures, without source counting or chart admission.

Queueing copies this proposal and two already-frozen language-neutral modules.
Every import, fixture, dependency hash and pin reconciliation holds the common
timing mutex. A failed job remains preserved and is never automatically retried.
"""
import argparse
import fcntl
import json
import os
import shutil
import subprocess
import sys
import traceback
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
PROPOSAL_FILES = ('c3_mask.py', 'foreign_c3.py', 'test_foreign_c3.py', 'REVIEW.md')
CORE_FILES = ('foreign_core.py', 'canonical.py')
EXPECTED_PYGMENTS = '2.19.2'
EXPECTED_FIXTURE_METHODS = 16


def now():
    return datetime.now(timezone.utc).isoformat()


def write_json(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def state(job, status, **fields):
    value = dict(status=status, updated_at=now(), pid=os.getpid(),
                 pgid=os.getpgrp(), automatic_retry=False,
                 source_counting_started=False, chart_admitted=False, **fields)
    temp = job / 'state.json.tmp'
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(job / 'state.json')


def locked_pins(job, dependency_root):
    # Called only after acquiring LOCK, before importing proposal/dependencies.
    import hashlib
    snapshot = job / 'snapshot'
    expected = set((*PROPOSAL_FILES, *CORE_FILES))
    if {path.name for path in snapshot.iterdir()} != expected:
        raise RuntimeError('Frozen snapshot inventory differs from reviewed files.')
    paths = [snapshot / name for name in sorted(expected)]
    paths += [job / 'worker.py', Path(sys.executable).resolve()]
    package = dependency_root / 'pygments'
    if not package.is_dir():
        raise RuntimeError('Reviewed Pygments dependency package is absent.')
    paths += sorted(package.rglob('*.py'))
    return {str(path.resolve()): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in paths}


def worker(job):
    config = json.loads((job / 'launch.json').read_text())
    # Job ownership is independent of the benchmark mutex. Duplicate workers
    # cannot execute or overwrite this preserved receipt.
    with (job / 'worker-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'fixture-receipt.json').exists():
            raise RuntimeError('This preserved fixture job already has a receipt.')
        state(job, 'waiting-for-common-timing-mutex', queued_at=config['queued_at'])
        print('WAIT LOCK: C3 proposal fixtures and dependency pins', flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            started_at = now()
            receipt = dict(status='failed-stop', started_at=started_at,
                common_timing_mutex=str(LOCK), exclusive_mutex_held=True,
                pid=os.getpid(), pgid=os.getpgrp(),
                python_executable=sys.executable, python_version=sys.version,
                frozen_sources=config['frozen_sources'],
                benchmark_processes_started=0, subprocesses_started_by_fixtures=0,
                source_counting_started=False, chart_admitted=False,
                automatic_retry=False, compiler_version='0.8.5',
                compiler_source_commit='640091d3303a393f9651d644063b837fc19d05f1')
            try:
                descriptor = os.fstat(lock.fileno())
                named = LOCK.stat()
                if (descriptor.st_dev, descriptor.st_ino) != (named.st_dev, named.st_ino):
                    raise RuntimeError('Mutex descriptor/path identity differs.')
                receipt['mutex_identity'] = dict(device=descriptor.st_dev,
                    inode=descriptor.st_ino, descriptor=lock.fileno())
                dependency_root = Path(config['dependency_root']).resolve()
                before = locked_pins(job, dependency_root)
                receipt['pins_before'] = before
                import importlib
                import unittest
                sys.path.insert(0, str(dependency_root))
                sys.path.insert(0, str(job / 'snapshot'))
                pygments = importlib.import_module('pygments')
                if pygments.__version__ != EXPECTED_PYGMENTS:
                    raise RuntimeError('Reviewed Pygments version differs.')
                if Path(pygments.__file__).resolve().parent != dependency_root / 'pygments':
                    raise RuntimeError('Pygments import escaped the reviewed dependency root.')
                receipt['pygments_version'] = pygments.__version__
                receipt['pygments_module'] = str(Path(pygments.__file__).resolve())
                module = importlib.import_module('test_foreign_c3')
                for name in ('test_foreign_c3', 'foreign_c3', 'c3_mask', 'foreign_core', 'canonical'):
                    loaded = sys.modules[name]
                    if Path(loaded.__file__).resolve() != job / 'snapshot' / (name + '.py'):
                        raise RuntimeError('Proposal import escaped immutable snapshot: ' + name)
                suite = unittest.defaultTestLoader.loadTestsFromModule(module)
                if suite.countTestCases() != EXPECTED_FIXTURE_METHODS:
                    raise RuntimeError('Reviewed fixture method inventory must be exactly sixteen.')
                state(job, 'fixtures-running', started_at=started_at,
                      expected_tests=EXPECTED_FIXTURE_METHODS)
                print('Acquired common timing mutex; testing frozen C3 proposal.', flush=True)
                with (job / 'fixtures.log').open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
                receipt.update(tests_run=result.testsRun, failures=len(result.failures),
                    errors=len(result.errors), suite='test_foreign_c3',
                    status='passed-awaiting-source-review' if result.wasSuccessful() else 'failed-fixtures')
                after = locked_pins(job, dependency_root)
                receipt['pins_after'] = after
                receipt['pin_reconciliation_passed'] = before == after
                if before != after:
                    raise RuntimeError('Frozen executed dependency bytes changed during fixtures.')
                # No authored benchmark source is imported, read or normalized.
                # This receipt qualifies only grammar fixtures for root review.
                receipt['finished_at'] = now()
                write_json(job / 'fixture-receipt.json', receipt)
                state(job, receipt['status'], tests_run=result.testsRun,
                      failures=len(result.failures), errors=len(result.errors))
                print(json.dumps(dict(status=receipt['status'], tests_run=result.testsRun,
                    failures=len(result.failures), errors=len(result.errors),
                    source_counting_started=False, chart_admitted=False)), flush=True)
                return 0 if result.wasSuccessful() else 1
            except BaseException as error:
                traceback.print_exc()
                receipt.update(status='failed-stop', error_type=type(error).__name__,
                               error=str(error), finished_at=now())
                if not (job / 'fixture-receipt.json').exists():
                    write_json(job / 'fixture-receipt.json', receipt)
                state(job, 'failed-stop', error_type=type(error).__name__, error=str(error))
                return 1


def queue(job, core_snapshot, dependency_root):
    job, core_snapshot = job.resolve(), core_snapshot.resolve()
    dependency_root = dependency_root.resolve()
    job.mkdir(parents=True, exist_ok=True)
    with (job / 'queue.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'launch.json').exists() or (job / 'snapshot').exists():
            raise SystemExit('Preserved C3 fixture job already queued; choose an additive revision.')
        sources = [HERE / name for name in PROPOSAL_FILES]
        sources += [core_snapshot / name for name in CORE_FILES]
        if any(not path.is_file() for path in sources):
            raise SystemExit('Reviewed snapshot source is absent.')
        snapshot = job / 'snapshot'
        snapshot.mkdir()
        for source in sources:
            target = snapshot / source.name
            shutil.copyfile(source, target)
            target.chmod(0o444)
        snapshot.chmod(0o555)
        runner = job / 'worker.py'
        shutil.copyfile(Path(__file__), runner)
        runner.chmod(0o444)
        write_json(job / 'launch.json', dict(queued_at=now(), mutex=str(LOCK),
            frozen_sources=[str(path) for path in sources], snapshot_files=[path.name for path in sources],
            frozen_worker_source=str(Path(__file__).resolve()),
            dependency_root=str(dependency_root), expected_pygments_version=EXPECTED_PYGMENTS,
            expected_fixture_methods=EXPECTED_FIXTURE_METHODS,
            python_executable=sys.executable, python_version=sys.version,
            automatic_retry=False, source_counting_started=False, chart_admitted=False))
        with (job / 'stdout.log').open('xb') as log:
            process = subprocess.Popen([sys.executable, '-B', '-u', str(runner), '--worker', str(job)],
                stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                start_new_session=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
        write_json(job / 'process.json', dict(pid=process.pid, pgid=process.pid,
            launched_at=now(), command=[sys.executable, '-B', '-u', str(runner), '--worker', str(job)],
            automatic_retry=False, source_counting_started=False, chart_admitted=False))
        print(json.dumps(dict(status='queued', pid=process.pid, job=str(job), mutex=str(LOCK))))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--job', type=Path, default=HERE / 'jobs/fixtures-v1')
    parser.add_argument('--core-snapshot', type=Path,
                        default=HERE.parent / 'jobs/boundary-audit-18/snapshot')
    parser.add_argument('--dependency-root', type=Path,
                        default=Path('/tmp/mech-clbg-compact-libs'))
    args = parser.parse_args()
    if args.worker: return worker(args.worker.resolve())
    queue(args.job, args.core_snapshot, args.dependency_root)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
