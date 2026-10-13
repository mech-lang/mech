#!/usr/bin/env python3
"""Freeze and queue one fixtures-to-audit job at the shared timing boundary.

Queueing copies only this proposal's small code files. All fixture execution,
benchmark source/report hashing and lexical work starts after the common flock.
There is no retry, benchmark invocation, chart admission or publication here.
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
MODULES = ('audit.py', 'normalizer.py', 'lexical.py', 'canonical.py',
           'pools.py', 'comparisons.py', 'test_normalizer.py', 'test_comparisons.py',
           'prior_array_loc.py', 'test_prior_array_loc.py',
           'apl_closer_approved.py','test_apl_closer_approved.py',
           'value_forms.py','test_value_forms.py','test_value_integration.py',
           'c_type_environment.py','test_c_type_environment.py','test_semantic_imports.py','test_c_type_integration.py',
           'compiler_attributes.py','test_compiler_attributes.py',
           'c_named_types.py','test_c_named_types.py',
           'pending_minima.py','test_pending_minima.py',
           'pending_minima_records.py','test_pending_minima_records.py',
           'record_fields.py','test_record_fields.py','test_record_integration.py',
           'foreign_dispatch.py','foreign_core.py','foreign_braced.py','test_foreign_braced.py',
           'zig_odin.py','test_zig_odin.py','foreign_keyword.py','test_foreign_keyword.py',
           'foreign_layout.py','test_foreign_layout.py','foreign_smalltalk.py','test_foreign_smalltalk.py')
FUTURE_MODULES=('foreign_future_mask.py','test_foreign_future_mask.py',
                'foreign_nim.py','test_foreign_nim.py',
                'foreign_crystal.py','test_foreign_crystal.py',
                'foreign_pony.py','test_foreign_pony.py')
ARTIFACTS = ('apl-closer-approval-job07.json','policy-scope-correction-20261012.json')
EXPLORER_MODULES = ('legacy_loc_data.py', 'test_legacy_loc_data.py',
                    'isolated_study_data.py','mandelbrot_study_data.py')


def now():
    return datetime.now(timezone.utc).isoformat()


def write_state(job, status, **fields):
    value = dict(status=status, updated_at=now(), pid=os.getpid(), **fields)
    temp = job / 'state.json.tmp'
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(job / 'state.json')


def worker(job):
    config = json.loads((job / 'launch.json').read_text())
    write_state(job, 'waiting-for-common-timing-mutex', queued_at=config['queued_at'])
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        write_state(job, 'fixtures-running', mutex_acquired_at=now())
        print('Acquired common timing mutex; starting frozen fixtures.', flush=True)
        try:
            import importlib
            import unittest
            dependency_root = Path(config['dependency_root'])
            sys.path.insert(0, str(dependency_root))
            pygments = importlib.import_module('pygments')
            if pygments.__version__ != config['expected_pygments_version']:
                raise RuntimeError('Pinned Pygments version differs from launch metadata.')
            if Path(pygments.__file__).resolve().parent != dependency_root / 'pygments':
                raise RuntimeError('Pygments loaded from an unexpected dependency root.')
            print(json.dumps(dict(python_executable=sys.executable,
                python_version=sys.version, pygments_version=pygments.__version__,
                pygments_module=str(Path(pygments.__file__).resolve()))), flush=True)
            os.environ['MECH_LOC_CLBG_ROOT'] = config['root']
            sys.path.insert(0, str(job / 'snapshot'))
            modules = [importlib.import_module(name) for name in
                       ('test_normalizer', 'test_legacy_loc_data', 'test_comparisons',
                        'test_prior_array_loc','test_apl_closer_approved',
                        'test_value_forms','test_value_integration','test_foreign_braced',
                        'test_c_type_environment','test_semantic_imports','test_c_type_integration',
                        'test_compiler_attributes',
                        'test_c_named_types',
                        'test_pending_minima','test_pending_minima_records',
                        'test_record_fields','test_record_integration',
                        'test_zig_odin','test_foreign_keyword','test_foreign_layout','test_foreign_smalltalk',
                        'test_foreign_future_mask','test_foreign_nim','test_foreign_crystal','test_foreign_pony')]
            modules[1].MUTEX_FD = lock.fileno()
            suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(m)
                                       for m in modules)
            with (job / 'fixtures.log').open('x') as stream:
                result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
            # Retain an executed dependency receipt even when a fixture fails
            # before the audit's normal tool-pin section can be produced.
            import hashlib
            receipt=dict(status='passed' if result.wasSuccessful() else 'failed-fixtures',
                tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                common_timing_mutex=str(LOCK),exclusive_mutex_held=True,
                python_executable=sys.executable,python_version=sys.version,
                pygments_version=pygments.__version__,pygments_module=str(Path(pygments.__file__).resolve()),
                pygments_module_sha256=hashlib.sha256(Path(pygments.__file__).read_bytes()).hexdigest(),
                frozen_dependency_pins={str(path):hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in sorted((job/'snapshot').iterdir()) if path.is_file()},
                suites=[module.__name__ for module in modules],benchmark_processes_started=0,
                source_pool_audit_started=False,chart_admitted=False,automatic_retry=False,
                completed_at=now())
            with (job/'fixture-receipt.json').open('x') as stream:
                json.dump(receipt,stream,indent=2);stream.write('\n')
            if not result.wasSuccessful():
                write_state(job, 'failed-fixtures', tests_run=result.testsRun,
                            failures=len(result.failures), errors=len(result.errors),
                            audit_started=False, automatic_retry=False)
                print('Fixtures failed; stopped before source-pool audit. See fixtures.log.', flush=True)
                return 1
            write_state(job, 'audit-running', tests_run=result.testsRun)
            audit = importlib.import_module('audit')
            value = audit.produce(Path(config['root']), Path(config['current_data']))
            output = job / 'candidate-audit.json'
            with output.open('x') as stream:
                json.dump(value, stream, indent=2, ensure_ascii=False)
                stream.write('\n')
            summary = dict(status=value['status'], measured_candidates=len(value['candidates']),
                           source_identities=len(value['sources']),
                           diagnostics=len(value['diagnostics']),
                           pending_needed_measurement=value['pending_needed_measurement'],
                           coverage_complete=value['coverage_complete'],
                           source_specific_review_required=True, chart_admitted=False)
            with (job / 'summary.json').open('x') as stream:
                json.dump(summary, stream, indent=2, ensure_ascii=False)
                stream.write('\n')
            write_state(job, 'complete-awaiting-source-review', tests_run=result.testsRun,
                        measured_candidates=summary['measured_candidates'],
                        output=str(output), chart_admitted=False)
            print(json.dumps(summary), flush=True)
            return 0
        except BaseException as error:
            traceback.print_exc()
            write_state(job, 'failed-stop', error_type=type(error).__name__,
                        error=str(error), automatic_retry=False, chart_admitted=False)
            return 1


def queue(root, current_data, job, dependency_root):
    root, current_data, job = root.resolve(), current_data.resolve(), job.resolve()
    dependency_root = dependency_root.resolve()
    metadata = dependency_root / 'pygments/__init__.py'
    if not metadata.is_file():
        raise SystemExit('Pinned dependency root has no Pygments metadata file.')
    import re
    version = re.search(r"^__version__\s*=\s*['\"]([^'\"]+)['\"]", metadata.read_text(), re.M)
    if not version:
        raise SystemExit('Pinned Pygments version cannot be read without importing it.')
    job.mkdir(parents=True, exist_ok=True)
    with (job / 'queue.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'launch.json').exists() or (job / 'snapshot').exists():
            raise SystemExit('This immutable job was already queued; it will not be launched twice.')
        snapshot = job / 'snapshot'
        snapshot.mkdir()
        source_paths = [HERE / name for name in (*MODULES,*FUTURE_MODULES,*ARTIFACTS)]
        source_paths += [root / 'overview/explorer' / name for name in EXPLORER_MODULES]
        for source in source_paths:
            destination = snapshot / source.name
            shutil.copyfile(source, destination)
            destination.chmod(0o444)
        runner = job / 'worker.py'
        shutil.copyfile(Path(__file__), runner)
        runner.chmod(0o444)
        config = dict(queued_at=now(), root=str(root), current_data=str(current_data),
                      mutex=str(LOCK), frozen_code_sources=[str(p) for p in source_paths],
                      python_executable=sys.executable, python_version=sys.version,
                      dependency_root=str(dependency_root), expected_pygments_version=version[1],
                      tests_before_audit=True, stop_on_fixture_failure=True,
                      automatic_retry=False, benchmark_processes_started=0,
                      chart_admitted=False)
        with (job / 'launch.json').open('x') as stream:
            json.dump(config, stream, indent=2)
            stream.write('\n')
        with (job / 'stdout.log').open('xb') as log:
            process = subprocess.Popen([sys.executable, '-B', '-u', str(runner),
                                        '--worker', str(job)], stdin=subprocess.DEVNULL,
                                       stdout=log, stderr=subprocess.STDOUT,
                                       start_new_session=True, env=dict(os.environ,
                                           PYTHONDONTWRITEBYTECODE='1'))
        with (job / 'process.json').open('x') as stream:
            json.dump(dict(pid=process.pid, launched_at=now(), job=str(job)), stream, indent=2)
            stream.write('\n')
        print(json.dumps(dict(job=str(job), pid=process.pid, status='queued',
                              mutex=str(LOCK), automatic_retry=False)))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--root', type=Path, default=HERE.parents[1])
    parser.add_argument('--current-data', type=Path)
    parser.add_argument('--job', type=Path, default=HERE / 'jobs/boundary-audit-01')
    parser.add_argument('--dependency-root', type=Path,
                        default=Path('/tmp/mech-clbg-compact-libs'))
    args = parser.parse_args()
    if args.worker:
        return worker(args.worker.resolve())
    current_data = args.current_data or args.root / 'overview/explorer/dist/data.json'
    queue(args.root, current_data, args.job, args.dependency_root)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
