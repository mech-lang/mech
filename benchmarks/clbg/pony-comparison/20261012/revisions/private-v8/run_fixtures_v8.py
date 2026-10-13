"""Exact actually discovered/started fixture inventory, under inherited mutex."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

PACKET = Path(__file__).resolve().parent
EXPECTED = {'directory-init': ('test_directory_init_v8.DirectoryInitialization.test_fresh_nested_build_root_reaches_owned_compile_boundary',)}


def flatten(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test.id()


class StrictResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.started_ids = []
    def startTest(self, test):
        self.started_ids.append(test.id())
        super().startTest(test)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--suite', choices=EXPECTED, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--lock-fd', type=int, required=True)
    args = parser.parse_args()
    lock_path = Path('/tmp/mech-clbg-timing.lock')
    inherited = os.fstat(args.lock_fd); actual = lock_path.stat()
    assert (inherited.st_dev, inherited.st_ino) == (actual.st_dev, actual.st_ino)
    with lock_path.open('a+') as inspection:
        try:
            fcntl.flock(inspection, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            pass
        else:
            fcntl.flock(inspection, fcntl.LOCK_UN)
            raise AssertionError('Actual common mutex is not held')
    expected = EXPECTED[args.suite]
    module = expected[0].split('.')[0]
    suite = unittest.defaultTestLoader.loadTestsFromName(module)
    discovered = list(flatten(suite))
    assert suite.countTestCases() == len(expected) and sorted(discovered) == sorted(expected)
    args.output.mkdir(parents=True, exist_ok=True)
    raw_log = args.output / (args.suite + '.log')
    receipt = args.output / (args.suite + '-fixture-receipt.json')
    assert not receipt.exists()
    with raw_log.open('x') as stream:
        result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=StrictResult).run(suite)
    passed = (result.wasSuccessful() and result.testsRun == len(expected) and
        sorted(result.started_ids) == sorted(expected) and len(result.started_ids) == len(expected)
        and not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses)
    payload = dict(status='passed' if passed else 'failed-fixtures', actual_common_mutex_held=True,
        inherited_common_mutex=True, suite=args.suite, expected_ids=list(expected), discovered_ids=discovered,
        started_ids=result.started_ids, expected_count=len(expected), tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        raw_log=str(raw_log), raw_log_sha256=hashlib.sha256(raw_log.read_bytes()).hexdigest(),
        native_probes=0, chart_admission=False)
    with receipt.open('x') as stream:
        json.dump(payload, stream, indent=2); stream.write('\n')
    print(json.dumps(dict(receipt=str(receipt), status=payload['status'], tests_run=result.testsRun)))
    return 0 if passed else 1


if __name__ == '__main__': sys.exit(main())
