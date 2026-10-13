"""Four exact synthetic explorer fixtures; source-only entry, no admission."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parent
EXPLORER = HERE.parents[1] / 'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON_ROOT = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python')
PYTHON = PYTHON_ROOT / 'bin/python3.12'
NEW = ('native_languages_data_v2', 'native_language_groups_v2',
       'native_platform_metadata_v2',
       'test_native_languages_prolog_v2')
RETAINED = ('native_languages_data_v1', 'native_language_groups_v1',
            'native_platform_metadata_v1', 'test_native_languages_data_v1',
            'test_native_platform_metadata_v1', 'test_native_languages_data_v2')
IDS = ['test_native_languages_prolog_v2.NativeEightAdmission.' + name for name in (
    'test_complete_original_qlf_bundle_and_six_existing_modes',
    'test_guessed_native_codegen_missing_vm_or_artifact_reject',
    'test_actual_interpreted_platform_and_group_membership',
    'test_partial_eight_wrong_vm_command_and_excluded_languages_are_transactional')]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')
        stream.flush(); os.fsync(stream.fileno())


def retain(pins, path, digest=None):
    path = Path(path).resolve(strict=True)
    actual = sha(path)
    assert digest is None or actual == digest, ('Changed byte-bound input', str(path))
    assert str(path) not in pins or pins[str(path)] == actual, ('Conflicting input pin', str(path))
    pins[str(path)] = actual
    return actual


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed exact fixture input', path)


def held(lock):
    opened, named = os.fstat(lock.fileno()), LOCK.stat()
    assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
    fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)


def snapshot(original, destination, pins):
    assert original.is_file() and not original.is_symlink()
    raw = original.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    retain(pins, original, digest)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    destination.chmod(0o444)
    retain(pins, destination, digest)
    assert destination.read_bytes() == raw
    return destination


class FrozenImports:
    """Compile owned pinned bytes directly; these modules never read a pyc."""
    def __init__(self, modules, pins, lock, loaded):
        self.modules, self.pins, self.lock, self.loaded = modules, pins, lock, loaded

    def find_spec(self, name, path=None, target=None):
        if name not in self.modules:
            return None
        return importlib.util.spec_from_loader(name, self,
            is_package=self.modules[name].name == '__init__.py')

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        held(self.lock)
        path = self.modules[module.__name__]
        raw = path.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == self.pins[str(path.resolve())]
        module.__file__ = str(path)
        if path.name == '__init__.py':
            module.__path__ = [str(path.parent)]
        record = dict(module=module.__name__, path=str(path), sha256=self.pins[str(path.resolve())], status='source-execution-started')
        self.loaded.append(record)
        try:
            exec(compile(raw, str(path), 'exec'), module.__dict__)
        except BaseException as error:
            record.update(status='source-execution-failed', error=str(error) or type(error).__name__)
            raise
        record['status'] = 'source-execution-completed'
        retain(self.pins, path, self.pins[str(path.resolve())])


def result_fields(result):
    if result is None:
        return {}
    return dict(tests_run=result.testsRun,
        failures=[(test.id(), detail) for test, detail in result.failures],
        errors=[(test.id(), detail) for test, detail in result.errors],
        skips=[(test.id(), reason) for test, reason in result.skipped],
        expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
        unexpected_successes=[test.id() for test in result.unexpectedSuccesses])


def run(job, lock):
    held(lock)
    job.mkdir(parents=True)  # Exclusive new attempt: no retry/resume/replacement.
    pins, modules, loaded = {}, {}, []
    result = finder = None
    receipt = dict(status='failed-before-discovery', expected_test_ids=IDS,
        discovered_test_ids=[], actual_test_ids=[], discovered_tests=0, tests_run=0,
        failures=[], errors=[], skips=[], expected_failures=[], unexpected_successes=[],
        loader_errors=[], execution_completed=False, inherited_common_mutex=True,
        actual_native_processes=0, full_processes_authorized=0, benchmark_replays=0,
        runtime_version_reruns=0, dataset_mutated=False, source_files_mutated=False,
        timings_mutated=False, chart_admission=False, automatic_retry=False)
    failure = None
    try:
        assert Path(sys.executable).resolve(strict=True) == PYTHON.resolve(strict=True)
        assert Path(sys.base_prefix).resolve() == PYTHON_ROOT.resolve()
        assert sys.flags.isolated == sys.flags.ignore_environment == sys.flags.no_site == 1
        assert sys.dont_write_bytecode is True
        retain(pins, PYTHON.resolve())
        retain(pins, PYTHON_ROOT / 'lib/libpython3.12.dylib')
        for path in sorted(HERE.glob('*.py')) + sorted(HERE.glob('*.md')):
            retain(pins, path)
        receipt['python'] = dict(executable=str(PYTHON.resolve()), version=sys.version,
            isolated=True, no_site=True, no_bytecode_writes=True,
            fixture_modules_executed_from_raw_source=True)
        assert not any(name in sys.modules for name in NEW + RETAINED)
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
        for name in NEW + RETAINED:
            source = EXPLORER / (name + '.py')
            if name in NEW:
                assert source.stat().st_mode & 0o222 == 0, ('Unfrozen new explorer source', str(source))
            modules[name] = snapshot(source, job / 'sources' / source.name, pins)
        review = EXPLORER / 'NATIVE_LANGUAGES_V2_REVIEW.md'
        assert review.stat().st_mode & 0o222 == 0, 'Unfrozen new explorer source review'
        snapshot(review, job/'sources'/review.name, pins)
        unittest_source = Path(importlib.util.find_spec('unittest').origin).resolve().parent
        assert unittest_source.is_relative_to(PYTHON_ROOT / 'lib/python3.12')
        for source in sorted(unittest_source.rglob('*.py')):
            relative = source.relative_to(unittest_source)
            name = 'unittest.' + '.'.join(relative.with_suffix('').parts)
            if relative.name == '__init__.py':
                name = name.removesuffix('.__init__')
            modules[name] = snapshot(source, job / 'python/unittest' / relative, pins)
        verify(pins)
        save(job / 'input-pins.json', dict(pins=pins, new_sources=list(NEW),
            review_sources=[review.name], retained_sources=list(RETAINED)))
        finder = FrozenImports(modules, pins, lock, loaded)
        sys.meta_path.insert(0, finder)
        import unittest
        import test_native_languages_prolog_v2 as tests
        suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
        def discover(part):
            if isinstance(part, unittest.TestSuite):
                for test in part: discover(test)
            else:
                receipt['discovered_test_ids'].append(part.id())
        discover(suite)
        receipt.update(discovered_tests=suite.countTestCases(), loader_errors=list(unittest.defaultTestLoader.errors))
        save(job / 'actual-discovery.json', {key: receipt[key] for key in
            ('expected_test_ids', 'discovered_test_ids', 'discovered_tests', 'loader_errors')})
        assert receipt['loader_errors'] == []
        assert len(receipt['discovered_test_ids']) == len(set(receipt['discovered_test_ids'])) == 4
        assert set(receipt['discovered_test_ids']) == set(IDS)
        verify(pins)
        with (job / 'actual-started-tests.jsonl').open('x') as starts, (job / 'fixtures.log').open('x') as log:
            class RecordedResult(unittest.TextTestResult):
                def __init__(self, *arguments):
                    nonlocal result
                    super().__init__(*arguments)
                    result = self
                def startTest(self, test):
                    identity = test.id()
                    receipt['actual_test_ids'].append(identity)
                    starts.write(json.dumps(dict(id=identity)) + '\n'); starts.flush(); os.fsync(starts.fileno())
                    super().startTest(test)
            result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=RecordedResult).run(suite)
            log.flush(); os.fsync(log.fileno())
        receipt.update(execution_completed=True, **result_fields(result))
        assert receipt['tests_run'] == receipt['discovered_tests'] == 4
        assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 4
        assert set(receipt['actual_test_ids']) == set(IDS)
        assert all(receipt[key] == [] for key in
            ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
        assert result.wasSuccessful()
        assert all(Path(sys.modules[name].__file__).resolve() == modules[name].resolve()
            for name in modules if name in sys.modules)
        verify(pins)
        receipt['status'] = 'passed-four-synthetic-explorer-fixtures-awaiting-root-review'
    except BaseException as error:
        failure = error
        receipt.update(status='failed-unqualified', error=str(error) or type(error).__name__, error_type=type(error).__name__)
        try:
            with (job / 'actual-error.log').open('x') as stream:
                stream.write(traceback.format_exc())
        except BaseException as log_error:
            receipt['error_log_write_failure'] = str(log_error) or type(log_error).__name__
    finally:
        receipt.update(result_fields(result), actual_module_execution_records=loaded)
        integrity_errors = []
        if finder is not None:
            try: sys.meta_path.remove(finder)
            except BaseException as error:
                integrity_errors.append(dict(path='fixture-import-hook', error=str(error) or type(error).__name__))
        for path in sorted(job.rglob('*')):
            if path.is_file():
                try: retain(pins, path)
                except BaseException as error:
                    integrity_errors.append(dict(path=str(path), error=str(error) or type(error).__name__))
        try:
            held(lock); verify(pins)
        except BaseException as error:
            integrity_errors.append(dict(path='input/common-integrity', error=str(error) or type(error).__name__))
        if integrity_errors:
            receipt.update(preceding_status=receipt['status'], status='failed-terminal-integrity', terminal_artifact_errors=integrity_errors)
        receipt.update(pins=pins, terminal_evidence_complete=not integrity_errors)
        save(job / 'fixture-receipt.json', receipt)
        terminal = dict(status=receipt['status'], receipt=str(job / 'fixture-receipt.json'),
            partial_evidence_pins=pins, success_requires_actual_command_exit_zero=True,
            inherited_common_mutex=True, actual_native_processes=0, full_processes_authorized=0,
            benchmark_replays=0, runtime_version_reruns=0, dataset_mutated=False,
            source_files_mutated=False, timings_mutated=False, chart_admission=False,
            automatic_retry=False)
        try: terminal['receipt_sha256'] = sha(job / 'fixture-receipt.json')
        except BaseException as error:
            terminal.update(status='failed-terminal-integrity', receipt_pin_error=str(error) or type(error).__name__)
        save(job / 'terminal-pin.json', terminal)
    if failure is not None:
        raise failure
    assert receipt['status'] == terminal['status'] == 'passed-four-synthetic-explorer-fixtures-awaiting-root-review'
    assert receipt['terminal_evidence_complete'] is True
    return receipt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--job', type=Path, required=True)
    arguments = parser.parse_args()
    job = arguments.job.resolve()
    assert job.parent == HERE.parent / 'jobs' and job.name.startswith('native-eight-explorer-fixtures-')
    print('WAIT common mutex: four synthetic explorer metadata fixtures', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        run(job, lock)


if __name__ == '__main__':
    main()
