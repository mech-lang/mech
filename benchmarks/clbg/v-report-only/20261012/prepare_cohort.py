"""Durable report-only build qualification after the pinned prerequisites pass.

This orchestrates preparation, never full timing or chart publication. Each
benchmark uses its own immutable revision and the common guarded native sampler.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

import full_study
from prepare import merge_pins, pins_digest

HERE = Path(__file__).resolve().parent
PREREQUISITES = ('bootstrap.json', 'oracle-fixtures.json', 'preparation-fixtures.json', 'cleanup-fixtures.json')


def interrupted(signum, frame):
    raise KeyboardInterrupt('V preparation cohort received signal ' + str(signum))


def owned_preparation(module, command, log, report, report_path, active):
    process = None
    receipt = dict(active, command=command, started_at=module.now())
    try:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        receipt['pid'] = process.pid
        report['active'] = receipt
        receipt['identity'] = module.identity(process.pid)
        module.save(report_path, report)
        return process.wait()
    finally:
        if process is not None:
            if process.poll() is None:
                # Only the owned preparation controller is signalled. Its
                # installed handlers clean its known compiler/sampler/native
                # groups while those processes retain the common descriptor.
                process.send_signal(signal.SIGINT)
                receipt['cleanup_signal'] = 'SIGINT'
                try:
                    process.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    receipt.update(cleanup_confirmed=False,
                                   cleanup_error='Owned preparation controller has not exited; inspect its native ownership before any recovery')
                    module.save(report_path, report)
                    raise RuntimeError(receipt['cleanup_error'])
            receipt.update(returncode=process.returncode, cleanup_confirmed=True,
                           finished_at=module.now())
            report.setdefault('child_receipts', []).append(dict(receipt))
            report['active'] = None
            module.save(report_path, report)


def read_prerequisites():
    result = {}
    for filename in PREREQUISITES:
        path = HERE / filename
        if not path.exists():
            return None
        result[filename] = json.loads(path.read_text())
    assert result['bootstrap.json']['status'] == 'prepared-smoke-passed'
    assert result['oracle-fixtures.json']['status'] == 'passed'
    assert result['oracle-fixtures.json']['tests_run'] == 14
    assert result['preparation-fixtures.json']['status'] == 'passed'
    assert result['preparation-fixtures.json']['tests_run'] == 5
    assert result['cleanup-fixtures.json']['status'] == 'passed'
    assert result['cleanup-fixtures.json']['tests_run'] == 8
    return result


def run(revision):
    module = full_study.configure('nbody')
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    directory = HERE / 'qualification' / revision
    directory.mkdir(parents=True, exist_ok=True)
    report_path = directory / 'cohort.json'
    with (directory / 'cohort-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not report_path.exists(), 'No duplicate cohort or implicit resume'
        report = dict(status='waiting-for-prerequisites', created_at=module.now(),
                      supervisor=module.identity(os.getpid()), revision=revision,
                      report_only=True, never_chart_eligible=True,
                      full_performance_runs=0, benchmarks={}, active=None,
                      prerequisites=list(PREREQUISITES))
        module.save(report_path, report)
        try:
            print('WAIT prerequisites:', ', '.join(PREREQUISITES), flush=True)
            while read_prerequisites() is None:
                launch = json.loads((HERE / 'bootstrap-launch.json').read_text())
                error_path = HERE / ('bootstrap-error-' + str(launch['pid']) + '.json')
                if error_path.exists():
                    raise RuntimeError('V compiler bootstrap stopped; inspect retained error receipt')
                for filename in ('oracle-fixtures.json', 'preparation-fixtures.json', 'cleanup-fixtures.json'):
                    path = HERE / filename
                    if path.exists() and json.loads(path.read_text())['status'] != 'passed':
                        raise RuntimeError('V preparation fixture prerequisite failed: ' + filename)
                time.sleep(2)
            with module.LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                prerequisites = read_prerequisites()
                assert prerequisites is not None
                pins = merge_pins(*(receipt['pins'] for receipt in prerequisites.values()))
                draft_path = HERE / 'profiles.json'
                draft = json.loads(draft_path.read_text())
                assert draft['report_only'] is True and draft['chart_admission'] is False
                rows = [row for row in draft['profiles'] if row['variant'] in ('compact', 'optimized')]
                assert len(rows) == 20
                assert {(row['benchmark'], row['variant']) for row in rows} == {
                    (benchmark, variant) for benchmark in full_study.BENCHMARKS
                    for variant in ('compact', 'optimized')
                }
                sources = [(HERE / row['source']).resolve() for row in rows]
                for path in sources:
                    assert path.is_relative_to((HERE / 'sources').resolve())
                files = sources + [draft_path, HERE / 'SOURCE_PLAN.md', Path(__file__).resolve(),
                                   HERE / 'prepare.py'] + [HERE / filename for filename in PREREQUISITES]
                pins = merge_pins(pins, {str(path): module.sha(path) for path in files})
                module.load_harness().PinGuard(pins).verify()
                report.update(status='preparing', pins=pins,
                              pins_sha256=pins_digest(pins),
                              prerequisites_passed_at=module.now())
                module.save(report_path, report)
            for benchmark in full_study.BENCHMARKS:
                with module.LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    module.load_harness().PinGuard(pins).verify()
                command = [sys.executable, '-u', str(HERE / 'prepare.py'),
                           '--benchmark', benchmark, '--revision', revision,
                           '--expected-cohort', str(report_path),
                           '--cohort-pins-sha256', pins_digest(pins)]
                log_path = directory / (benchmark + '.log')
                with log_path.open('xb') as log:
                    returncode = owned_preparation(module, command, log, report, report_path,
                                                   dict(benchmark=benchmark, log=str(log_path)))
                result_path = directory / benchmark / 'probes.json'
                result = json.loads(result_path.read_text()) if result_path.exists() else {}
                report['benchmarks'][benchmark] = dict(
                    returncode=returncode, status=result.get('status', 'no-preparation-report'),
                    probes=len(result.get('attempts', [])), result=str(result_path),
                    log=str(log_path), finished_at=module.now(),
                )
                report['active'] = None
                module.save(report_path, report)
                assert returncode == 0 and result.get('status') == 'bounded-outputs-passed', (
                    'V preparation failed; no automatic retry', benchmark, log_path,
                )
            with module.LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                module.load_harness().PinGuard(pins).verify()
            report.update(status='bounded-outputs-passed-all-ten', finished_at=module.now(),
                          pins_unchanged_at_end=True,
                          remaining_gates=['source-work traces', 'canonical V LOC',
                                           'sealed full studies', 'three full timing rounds'])
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__,
                          finished_at=module.now())
            raise
        finally:
            module.save(report_path, report)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--detach', action='store_true')
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    if not args.detach:
        run(args.revision)
        return
    directory = HERE / 'qualification' / args.revision
    directory.mkdir(parents=True, exist_ok=True)
    module = full_study.configure('nbody')
    with (directory / 'cohort-launch-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        receipt_path = directory / 'cohort-launch.json'
        assert not receipt_path.exists() and not (directory / 'cohort.json').exists()
        command = [sys.executable, '-u', str(Path(__file__).resolve()), '--revision', args.revision]
        with (directory / 'cohort.log').open('ab') as log:
            process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log,
                                       stderr=subprocess.STDOUT, start_new_session=True)
        receipt = dict(pid=process.pid, identity=module.identity(process.pid), command=command,
                       launched_at=module.now(), report_only=True, never_chart_eligible=True)
        module.save(receipt_path, receipt)
        print(json.dumps(receipt, indent=2), flush=True)


if __name__ == '__main__':
    main()
