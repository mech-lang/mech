"""Direct locked metadata qualification; no benchmark/runtime/version commands."""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
from pathlib import Path
import shutil
import sys
import sysconfig
import unittest

HERE = Path(__file__).resolve().parent
CONTINUATION = HERE.parent
CLBG = CONTINUATION.parent
DATASET = CLBG / 'overview/explorer/dist/data.json'
PROPOSAL = CONTINUATION / 'platform-filter-proposal-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
EXPECTED = tuple('test_qualify_metadata.MetadataEvidence.' + name for name in (
    'test_java_saved_jit_and_classpath_are_required',
    'test_dotnet_host_dll_cannot_be_reclassified_as_native_aot',
    'test_native_aot_requires_saved_publish_and_native_command',
    'test_native_arm64_header_rejects_x86_and_non_native_bytes',
    'test_changed_partial_nan_or_fabricated_samples_are_rejected',
    'test_exact_inventory_and_preserved_identity_are_required'))


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write('\n')


def flatten(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test.id()


class ActualStarted(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.started_ids = []
    def startTest(self, test):
        self.started_ids.append(test.id())
        super().startTest(test)


def run(output):
    print('WAIT common mutex: platform metadata qualification, 100 saved profiles', flush=True)
    with (HERE / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as mutex:
            fcntl.flock(mutex, fcntl.LOCK_EX)
            assert output.is_relative_to(CONTINUATION / 'jobs') and not output.exists()
            output.mkdir(parents=True)
            receipt = dict(status='failed', started_at=now(), exclusive_common_timing_mutex_held=True,
                expected_program_count=100, expected_fixture_count=6, required_fixture_ids=list(EXPECTED),
                actual_started_fixture_ids=[], tests_run=0, actual_native_processes=0,
                runtime_version_reruns=0, benchmark_replays=0, chart_admission=False,
                dataset_mutated=False, sources_mutated=False, timings_mutated=False, automatic_retry=False)
            pins = {}
            try:
                pins[str(Path(__file__).resolve())] = sha(__file__)
                pins[str(Path(sys.executable).resolve())] = sha(sys.executable)
                unittest_root = Path(sysconfig.get_path('stdlib')) / 'unittest'
                assert Path(unittest.__file__).resolve().parent == unittest_root.resolve()
                for source in sorted(unittest_root.rglob('*.py')):
                    pins[str(source)] = sha(source)
                snapshot = output / 'snapshot'
                snapshot.mkdir()
                frozen = {}
                sources = [HERE / 'qualify_metadata.py', HERE / 'test_qualify_metadata.py',
                    HERE / 'expected-program-ids.json', HERE / 'REVIEW.md', DATASET,
                    PROPOSAL / 'java-admitted-metadata.json', PROPOSAL / 'existing-admitted-metadata.json',
                    PROPOSAL / 'platform-evidence-plan.json', PROPOSAL / 'platform.mjs', PROPOSAL / 'REVIEW.md']
                for ordinal, source in enumerate(sources):
                    name = source.name if source.parent == HERE else str(ordinal) + '-' + source.name
                    target = snapshot / name
                    shutil.copyfile(source, target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = sha(source)
                    pins[str(target)] = sha(target)
                    frozen[str(source)] = target
                    target.chmod(0o444)
                snapshot.chmod(0o555)
                assert 'qualify_metadata' not in sys.modules and 'test_qualify_metadata' not in sys.modules
                sys.dont_write_bytecode = True
                sys.path.insert(0, str(snapshot))
                module = importlib.import_module('test_qualify_metadata')
                qualifier = importlib.import_module('qualify_metadata')
                assert Path(module.__file__).resolve() == snapshot / 'test_qualify_metadata.py'
                assert Path(qualifier.__file__).resolve() == snapshot / 'qualify_metadata.py'
                suite = unittest.defaultTestLoader.loadTestsFromModule(module)
                discovered = list(flatten(suite))
                assert suite.countTestCases() == len(discovered) == len(set(discovered)) == 6 and set(discovered) == set(EXPECTED)
                receipt['discovered_fixture_ids'] = discovered
                raw = output / 'fixtures.log'
                with raw.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=ActualStarted).run(suite)
                receipt.update(tests_run=result.testsRun, actual_started_fixture_ids=result.started_ids,
                    failures=[(test.id(), detail) for test, detail in result.failures],
                    errors=[(test.id(), detail) for test, detail in result.errors],
                    skips=[(test.id(), reason) for test, reason in result.skipped],
                    expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                    unexpected_successes=[test.id() for test in result.unexpectedSuccesses], raw_fixture_log=dict(path=str(raw), sha256=sha(raw)))
                pins[str(raw)] = sha(raw)
                assert result.wasSuccessful() and result.testsRun == len(result.started_ids) == len(set(result.started_ids)) == 6
                assert set(result.started_ids) == set(EXPECTED)
                assert not any(receipt[key] for key in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                save(output / 'fixture-receipt.json', dict(receipt, status='passed'))
                pins[str(output / 'fixture-receipt.json')] = sha(output / 'fixture-receipt.json')
                data = json.loads(frozen[str(DATASET)].read_text())
                extractions = [json.loads(frozen[str(PROPOSAL / name)].read_text())
                    for name in ('java-admitted-metadata.json', 'existing-admitted-metadata.json')]
                expected = json.loads(frozen[str(HERE / 'expected-program-ids.json')].read_text())
                annotations = qualifier.qualify(CLBG, data, extractions, expected, pins)
                assert annotations['actual_program_count'] == 100 and set(annotations['annotations']) == set(expected)
                for path, expected_digest in pins.items():
                    assert sha(path) == expected_digest, ('Qualification input changed', path)
                save(output / 'measured-backends.json', annotations)
                receipt.update(status=annotations['status'], actual_program_count=100,
                    qualified_program_count=annotations['qualified_program_count'], unresolved_programs=annotations['unresolved_programs'],
                    metadata_output=dict(path=str(output / 'measured-backends.json'), sha256=sha(output / 'measured-backends.json')),
                    source_pins=dict(pins), pins_unchanged_at_end=True)
            except BaseException as error:
                receipt.update(status='failed', error=str(error) or type(error).__name__, error_type=type(error).__name__, source_pins=dict(pins))
                raise
            finally:
                receipt['completed_at'] = now()
                path = output / 'qualification-receipt.json'
                save(path, receipt)
                save(output / 'qualification-receipt-pin.json', dict(path=str(path), sha256=sha(path),
                    exclusive_common_timing_mutex_held=True, chart_admission=False))
            print(json.dumps(dict(status=receipt['status'], qualified_programs=receipt['qualified_program_count'], receipt=str(path))), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve())
