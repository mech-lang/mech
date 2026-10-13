"""Qualify four receipt/preservation fixtures under the shared benchmark lock."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import shutil
import sys
import traceback

HERE = Path(__file__).resolve().parent
EXPLORER = HERE.parent / 'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PRIOR = HERE / 'jobs/publication-fixtures-v2/fixture-receipt.json'
UNITTEST_PRIOR = HERE / 'jobs/platform-metadata-data-v2/fixture-receipt.json'
OUTPUT = HERE / 'jobs/native-platform-v1-fixtures'
NAMES = ('native_languages_data_v1', 'native_platform_metadata_v1', 'test_native_platform_metadata_v1')
EXPECTED = sorted('test_native_platform_metadata_v1.NativePlatformFixtures.' + name for name in (
    'test_complete_native_jvm_beam_stage_preserves_measurements',
    'test_receipt_content_missing_pointer_and_encoding_reject',
    'test_partial_cohort_wrong_backend_and_v_reject',
    'test_late_conflict_and_serialized_receipt_preserve_input'))


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex: new-cohort platform receipt fixtures', flush=True)
    with (HERE / 'native-platform-v1-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            a, b = os.fstat(lock.fileno()), LOCK.stat()
            assert (a.st_dev, a.st_ino) == (b.st_dev, b.st_ino)
            assert not OUTPUT.exists()
            OUTPUT.mkdir()
            pins = {}
            receipt = dict(status='failed', required_fixture_ids=EXPECTED, discovered_fixture_ids=[],
                actual_started_fixture_ids=[], tests_run=0, expected_fixture_count=4,
                common_timing_mutex=str(LOCK), exclusive_common_timing_mutex_held=True,
                actual_native_processes=0, dataset_mutated=False, chart_admission=False,
                automatic_retry=False, started_at=datetime.now(timezone.utc).isoformat())
            try:
                prior = json.loads(PRIOR.read_text())
                assert prior['status'] == 'passed' and prior['tests_run'] == prior['expected_fixture_count'] == 16
                assert prior['exclusive_common_timing_mutex_held'] is True
                assert sorted(prior['actual_started_fixture_ids']) == sorted(prior['required_fixture_ids'])
                assert len(set(prior['actual_started_fixture_ids'])) == 16
                assert all(prior[k] == 0 for k in ('failures','errors','skips','expected_failures','unexpected_successes'))
                pins.update(prior['source_pins'])
                for path, value in pins.items(): assert sha(path) == value
                pins[str(PRIOR)] = sha(PRIOR)
                tested_dependencies = json.loads(UNITTEST_PRIOR.read_text())
                assert tested_dependencies['status'] == 'passed' and tested_dependencies['tests_run'] == tested_dependencies['expected_fixture_count'] == 4
                assert tested_dependencies['exclusive_common_timing_mutex_held'] is tested_dependencies['pins_unchanged_at_end'] is True
                assert sorted(tested_dependencies['actual_started_fixture_ids']) == sorted(tested_dependencies['required_fixture_ids'])
                assert len(set(tested_dependencies['actual_started_fixture_ids'])) == 4
                assert all(tested_dependencies[k] == [] for k in ('failures','errors','skips','expected_failures','unexpected_successes'))
                for path, value in tested_dependencies['source_pins'].items():
                    assert path not in pins or pins[path] == value
                    assert sha(path) == value
                    pins[path] = value
                pins[str(UNITTEST_PRIOR)] = sha(UNITTEST_PRIOR)
                pins[str(Path(__file__).resolve())] = sha(__file__)
                pins[str(Path(sys.executable).resolve())] = sha(Path(sys.executable).resolve())
                snapshot = OUTPUT / 'snapshot'
                snapshot.mkdir()
                for name in NAMES:
                    source, target = EXPLORER / (name + '.py'), snapshot / (name + '.py')
                    value = sha(source)
                    assert str(source) not in pins or pins[str(source)] == value
                    shutil.copyfile(source, target)
                    assert sha(target) == value
                    pins[str(source)] = pins[str(target)] = value
                    target.chmod(0o444)
                assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
                # Reuse the already qualified immutable unittest source snapshot.
                unittest_path = Path(tested_dependencies['unittest_module_identities']['unittest']).resolve()
                assert unittest_path.name == '__init__.py' and str(unittest_path) in pins
                assert unittest_path.is_relative_to(UNITTEST_PRIOR.parent)
                sys.dont_write_bytecode = True
                sys.path.insert(0, str(unittest_path.parent.parent))
                unittest = importlib.import_module('unittest')
                assert Path(unittest.__file__).resolve() == unittest_path
                assert all(name not in sys.modules for name in NAMES)
                sys.path.insert(0, str(snapshot))
                tests = importlib.import_module(NAMES[-1])
                for name in NAMES:
                    assert Path(sys.modules[name].__file__).resolve() == snapshot / (name + '.py')
                for path, value in pins.items(): assert sha(path) == value
                class Recorded(unittest.TextTestResult):
                    def __init__(self, *args, **kwargs):
                        super().__init__(*args, **kwargs)
                        self.started = []
                    def startTest(self, test):
                        self.started.append(test.id())
                        super().startTest(test)
                suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
                def flatten(items):
                    for item in items:
                        if isinstance(item, unittest.TestSuite): yield from flatten(item)
                        else: yield item.id()
                discovered = list(flatten(suite))
                assert sorted(discovered) == EXPECTED and len(discovered) == len(set(discovered)) == 4
                for path, value in pins.items(): assert sha(path) == value
                log = OUTPUT / 'fixtures.log'
                with log.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream,verbosity=2,resultclass=Recorded).run(suite)
                receipt.update(discovered_fixture_ids=discovered, actual_started_fixture_ids=result.started,
                    tests_run=result.testsRun, failures=[(t.id(),v) for t,v in result.failures],
                    errors=[(t.id(),v) for t,v in result.errors], skips=[(t.id(),v) for t,v in result.skipped],
                    expected_failures=[(t.id(),v) for t,v in result.expectedFailures],
                    unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
                pins[str(log)] = sha(log)
                assert result.wasSuccessful() and sorted(result.started) == EXPECTED and result.testsRun == 4
                assert all(not receipt[k] for k in ('failures','errors','skips','expected_failures','unexpected_successes'))
                for name, module in list(sys.modules.items()):
                    if name == 'unittest' or name.startswith('unittest.'):
                        path = Path(module.__file__).resolve()
                        assert path.is_relative_to(unittest_path.parent) and str(path) in pins
                for path, value in pins.items(): assert sha(path) == value
                receipt.update(status='passed', pins_unchanged_at_end=True)
            except BaseException as error:
                receipt.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                with (OUTPUT / 'failure.log').open('x') as stream: stream.write(traceback.format_exc())
                raise
            finally:
                receipt.update(source_pins=pins,completed_at=datetime.now(timezone.utc).isoformat())
                path = OUTPUT / 'fixture-receipt.json'
                save(path, receipt)
                terminal_pins = {}
                try:
                    for item in sorted(OUTPUT.rglob('*')):
                        if item.is_file(): terminal_pins[str(item)] = sha(item)
                    receipt_sha = terminal_pins[str(path)]
                except BaseException as error:
                    save(OUTPUT / 'terminal-pins.json', dict(terminal_status='failed-terminal-integrity',
                        preceding_fixture_status=receipt['status'],pins=terminal_pins,
                        error=str(error) or type(error).__name__,error_type=type(error).__name__,
                        available_bytes_not_fully_pinned=True,exclusive_common_timing_mutex_held=True,
                        chart_admission=False,automatic_retry=False))
                    raise
                save(OUTPUT / 'terminal-pins.json', dict(receipt_sha256=receipt_sha,terminal_status=receipt['status'],
                    pins=terminal_pins,exclusive_common_timing_mutex_held=True,chart_admission=False,automatic_retry=False))
            print(json.dumps(dict(status=receipt['status'],tests_run=receipt['tests_run'],receipt=str(path))),flush=True)


if __name__ == '__main__':
    main()
