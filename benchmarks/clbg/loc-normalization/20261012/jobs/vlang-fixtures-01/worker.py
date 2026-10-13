#!/usr/bin/env python3
"""Freeze narrowly specified proposal fixtures at the common timing boundary.

This job executes no benchmark, selects no endpoint and performs no source-pool
audit. The dependency snapshot and executed checks are independently recorded.
"""
import argparse
import fcntl
import json
import os
import re
import shutil
import subprocess
import sys
import traceback
from datetime import datetime,timezone
from pathlib import Path


HERE=Path(__file__).resolve().parent
LOCK=Path('/tmp/mech-clbg-timing.lock')
BASE=('audit.py','normalizer.py','canonical.py','lexical.py','pools.py',
      'comparisons.py','prior_array_loc.py')


def now():return datetime.now(timezone.utc).isoformat()


def state(job,status,**fields):
    value=dict(status=status,pid=os.getpid(),updated_at=now(),**fields)
    temporary=job/'state.json.tmp'
    temporary.write_text(json.dumps(value,indent=2)+'\n')
    temporary.replace(job/'state.json')


def worker(job):
    config=json.loads((job/'launch.json').read_text())
    state(job,'waiting-for-common-timing-mutex',queued_at=config['queued_at'])
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        state(job,'fixtures-running',mutex_acquired_at=now())
        try:
            import hashlib
            import importlib
            import unittest
            sys.path.insert(0,config['dependency_root'])
            pygments=importlib.import_module('pygments')
            module=Path(pygments.__file__).resolve()
            if pygments.__version__!=config['expected_pygments_version']:
                raise RuntimeError('Pinned Pygments version changed.')
            if module.parent!=Path(config['dependency_root'])/'pygments':
                raise RuntimeError('Unexpected Pygments dependency root.')
            sys.path.insert(0,str(job/'snapshot'))
            modules=[importlib.import_module(name) for name in config['suites']]
            suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(m) for m in modules)
            with (job/'fixtures.log').open('x') as stream:
                result=unittest.TextTestRunner(stream=stream,verbosity=2).run(suite)
            receipt=dict(status='passed' if result.wasSuccessful() else 'failed-fixtures',
                tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                common_timing_mutex=str(LOCK),exclusive_mutex_held=True,
                python_executable=sys.executable,python_version=sys.version,
                pygments_version=pygments.__version__,pygments_module=str(module),
                pygments_module_sha256=hashlib.sha256(module.read_bytes()).hexdigest(),
                frozen_module_pins={str(p):hashlib.sha256(p.read_bytes()).hexdigest()
                    for p in sorted((job/'snapshot').glob('*.py'))},
                suites=config['suites'],benchmark_processes_started=0,
                source_pool_audit_started=False,chart_admitted=False,
                automatic_retry=False,completed_at=now())
            with (job/'receipt.json').open('x') as stream:
                json.dump(receipt,stream,indent=2);stream.write('\n')
            state(job,receipt['status'],tests_run=result.testsRun,failures=len(result.failures),
                errors=len(result.errors),source_pool_audit_started=False,chart_admitted=False)
            print(json.dumps({k:receipt[k] for k in ('status','tests_run','failures','errors','completed_at')}),flush=True)
            return 0 if result.wasSuccessful() else 1
        except BaseException as error:
            traceback.print_exc()
            state(job,'failed-stop',error_type=type(error).__name__,error=str(error),
                automatic_retry=False,chart_admitted=False)
            return 1


def queue(job,names,suites,dependency_root):
    job=job.resolve();dependency_root=dependency_root.resolve()
    metadata=dependency_root/'pygments/__init__.py'
    version=re.search(r"^__version__\s*=\s*['\"]([^'\"]+)['\"]",metadata.read_text(),re.M)
    if not version:raise SystemExit('Cannot read pinned dependency metadata.')
    files=list(dict.fromkeys((*BASE,*names)))
    for name in files:
        if Path(name).name!=name or not name.endswith('.py') or not (HERE/name).is_file():
            raise SystemExit('Every frozen module must be a saved local Python proposal file.')
    if not suites or any(name+'.py' not in files for name in suites):
        raise SystemExit('Every suite must have a frozen local module.')
    job.mkdir(parents=True,exist_ok=True)
    with (job/'queue.lock').open('a+') as owner:
        fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
        if (job/'launch.json').exists() or (job/'snapshot').exists():
            raise SystemExit('Immutable proposal job already exists; it will not launch twice.')
        snapshot=job/'snapshot';snapshot.mkdir()
        for name in files:
            destination=snapshot/name;shutil.copyfile(HERE/name,destination);destination.chmod(0o444)
        runner=job/'worker.py';shutil.copyfile(Path(__file__),runner);runner.chmod(0o444)
        config=dict(queued_at=now(),python_executable=sys.executable,python_version=sys.version,
            dependency_root=str(dependency_root),expected_pygments_version=version[1],
            suites=suites,modules=files,mutex=str(LOCK),automatic_retry=False,
            benchmark_processes_started=0,source_pool_audit_started=False,chart_admitted=False)
        with (job/'launch.json').open('x') as stream:
            json.dump(config,stream,indent=2);stream.write('\n')
        with (job/'stdout.log').open('xb') as log:
            process=subprocess.Popen([sys.executable,'-B','-u',str(runner),'--worker',str(job)],
                stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT,start_new_session=True,
                env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'))
        with (job/'process.json').open('x') as stream:
            json.dump(dict(pid=process.pid,launched_at=now()),stream,indent=2);stream.write('\n')
        print(json.dumps(dict(status='queued',job=str(job),pid=process.pid,suites=suites,
            mutex=str(LOCK),automatic_retry=False)))


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--worker',type=Path)
    parser.add_argument('--job',type=Path)
    parser.add_argument('--module',action='append',default=[])
    parser.add_argument('--suite',action='append',default=[])
    parser.add_argument('--dependency-root',type=Path,default=Path('/tmp/mech-clbg-compact-libs'))
    args=parser.parse_args()
    if args.worker:return worker(args.worker.resolve())
    if not args.job:raise SystemExit('An explicit fresh job path is required.')
    queue(args.job,args.module,args.suite,args.dependency_root)
    return 0


if __name__=='__main__':raise SystemExit(main())
