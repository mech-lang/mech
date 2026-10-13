"""One fresh report-only input-v2 cohort using the proven owned-child guard."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import sys
import time

import full_study
import prepare_cohort as base
from prepare import merge_pins, pins_digest
from prepare_input_v2 import (DRAFT_PATH, PLAN_PATH, SOURCE_DIRECTORY,
                              ADDITIVE_PREREQUISITES, read_additive_prerequisites, select_profiles)

HERE = Path(__file__).resolve().parent
PREREQUISITES = base.PREREQUISITES + tuple(ADDITIVE_PREREQUISITES)
interrupted = base.interrupted
owned_preparation = base.owned_preparation


def read_prerequisites():
    original = base.read_prerequisites()
    if original is None or any(not (HERE / name).exists() for name in ADDITIVE_PREREQUISITES):
        return None
    return dict(original, **read_additive_prerequisites())


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
                draft_path = DRAFT_PATH
                draft = json.loads(draft_path.read_text())
                assert draft['report_only'] is True and draft['chart_admission'] is False
                rows = select_profiles(draft)
                assert len(rows) == 20
                assert {(row['benchmark'], row['variant']) for row in rows} == {
                    (benchmark, variant) for benchmark in full_study.BENCHMARKS
                    for variant in ('compact', 'optimized')
                }
                sources = [(HERE / row['source']).resolve() for row in rows]
                for path in sources:
                    assert path.is_relative_to(SOURCE_DIRECTORY.resolve())
                files = sources + [draft_path, PLAN_PATH, Path(__file__).resolve(),
                                   HERE / 'prepare_input_v2.py', HERE / 'prepare.py',
                                   HERE / 'prepare_cohort.py', HERE / 'dna_oracles.py',
                                   HERE / 'test_input_v2.py'] + [HERE / filename for filename in PREREQUISITES]
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
                command = [sys.executable, '-u', str(HERE / 'prepare_input_v2.py'),
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
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    run(args.revision)


if __name__ == '__main__':
    main()
