"""Execute only the fixed endpoint fixtures, under the common timing mutex."""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
from pathlib import Path
import shutil
import sys
import unittest

HERE = Path(__file__).resolve().parent
EXPLORER = HERE.parent / 'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PREREQUISITE = HERE / 'jobs/publication-fixtures-v2/fixture-receipt.json'
EXPECTED = tuple('test_publication_endpoints_v1.PublicationEndpoints.' + name for name in (
    'test_added_forms_choose_actual_fast_and_small_and_preserve_observations',
    'test_source_only_small_remains_without_invented_time_or_complete_suite',
    'test_one_source_can_own_both_endpoints_and_stable_ties_remain_stable',
    'test_late_invalid_group_is_transactional'))


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def now():
    return datetime.now(timezone.utc).isoformat()


def new_json(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def flatten(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test.id()


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.started_ids = []
    def startTest(self, test):
        self.started_ids.append(test.id())
        super().startTest(test)


def run(output):
    print('WAIT common mutex: four publication endpoint fixtures', flush=True)
    with (HERE / 'publication-endpoints-v1-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert output.is_relative_to(HERE / 'jobs') and not output.exists()
            output.mkdir(parents=True)
            receipt = dict(status='failed', started_at=now(), required_fixture_ids=list(EXPECTED),
                discovered_fixture_ids=[], actual_started_fixture_ids=[], tests_run=0,
                expected_fixture_count=4, actual_native_processes=0, chart_admission=False,
                source_pool_admission=False, exclusive_common_timing_mutex_held=True,
                automatic_retry=False, prerequisite=str(PREREQUISITE))
            try:
                previous = json.loads(PREREQUISITE.read_text())
                assert previous['status'] == 'passed' and previous['tests_run'] == previous['expected_fixture_count'] == 16
                assert previous['exclusive_common_timing_mutex_held'] is True
                assert previous['failures'] == previous['errors'] == previous['skips'] == 0
                assert previous['expected_failures'] == previous['unexpected_successes'] == 0
                assert len(previous['actual_started_fixture_ids']) == len(set(previous['actual_started_fixture_ids'])) == 16
                assert set(previous['actual_started_fixture_ids']) == set(previous['required_fixture_ids']) == set(previous['discovered_fixture_ids'])
                pins = dict(previous['source_pins'])
                for path, digest in pins.items():
                    assert sha(path) == digest, ('Qualified publication dependency changed', path)
                pins[str(PREREQUISITE)] = sha(PREREQUISITE)
                pins[str(Path(__file__).resolve())] = sha(__file__)
                pins[str(Path(sys.executable).resolve())] = sha(sys.executable)
                snapshot = output / 'snapshot'
                snapshot.mkdir()
                for name in ('publication_endpoints_v1.py', 'test_publication_endpoints_v1.py'):
                    source, target = EXPLORER / name, snapshot / name
                    shutil.copyfile(source, target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = sha(source)
                    pins[str(target)] = sha(target)
                    target.chmod(0o444)
                snapshot.chmod(0o555)
                assert 'publication_endpoints_v1' not in sys.modules and 'test_publication_endpoints_v1' not in sys.modules
                sys.dont_write_bytecode = True
                sys.path.insert(0, str(snapshot))
                module = importlib.import_module('test_publication_endpoints_v1')
                for name in ('publication_endpoints_v1', 'test_publication_endpoints_v1'):
                    assert Path(sys.modules[name].__file__).resolve() == snapshot / (name + '.py')
                suite = unittest.defaultTestLoader.loadTestsFromModule(module)
                discovered = list(flatten(suite))
                assert suite.countTestCases() == len(discovered) == 4 and set(discovered) == set(EXPECTED)
                raw_log = output / 'fixtures.log'
                with raw_log.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=RecordedResult).run(suite)
                receipt.update(discovered_fixture_ids=discovered, actual_started_fixture_ids=result.started_ids,
                    tests_run=result.testsRun, failures=[(test.id(), detail) for test, detail in result.failures],
                    errors=[(test.id(), detail) for test, detail in result.errors],
                    skips=[(test.id(), reason) for test, reason in result.skipped],
                    expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                    unexpected_successes=[test.id() for test in result.unexpectedSuccesses],
                    raw_log=dict(path=str(raw_log), sha256=sha(raw_log)))
                pins[str(raw_log)] = sha(raw_log)
                assert result.wasSuccessful() and result.testsRun == len(result.started_ids) == len(set(result.started_ids)) == 4
                assert set(result.started_ids) == set(EXPECTED)
                assert not any(receipt[key] for key in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                for path, digest in pins.items():
                    assert sha(path) == digest, ('Endpoint fixture dependency changed', path)
                receipt.update(status='passed', source_pins=pins, pins_unchanged_at_end=True)
            except BaseException as error:
                receipt.update(status='failed', error=str(error) or type(error).__name__, error_type=type(error).__name__)
                raise
            finally:
                receipt['completed_at'] = now()
                path = output / 'fixture-receipt.json'
                new_json(path, receipt)
                new_json(output / 'fixture-receipt-pin.json', dict(path=str(path), sha256=sha(path),
                    exclusive_common_timing_mutex_held=True, chart_admission=False))
            print(json.dumps(dict(status=receipt['status'], tests_run=receipt['tests_run'], receipt=str(path))), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve())


if __name__ == '__main__':
    main()
