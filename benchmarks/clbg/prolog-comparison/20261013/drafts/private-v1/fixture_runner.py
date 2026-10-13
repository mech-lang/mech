"""Same-PID local fixture runner. Caller already holds the actual common mutex."""
import importlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys

IDS = [
    'test_contracts.PreparationContracts.test_twenty_profiles_and_explicit_same_source_aliases',
    'test_contracts.PreparationContracts.test_compiler_inventory_requires_each_helper_and_no_foreign_authored_root',
    'test_contracts.PreparationContracts.test_actual_runtime_flags_reject_bounded_or_x86_or_disabled_gc',
    'test_contracts.PreparationContracts.test_native_counter_and_workload_boolean_substitution_is_rejected',
    'test_contracts.PreparationContracts.test_isolation_requires_exact_twenty_boolean_gates_and_clean_ownership',
    'test_contracts.PreparationContracts.test_full_admission_requires_actual_scored_allocation_not_source_assertion',
]


def run(output, pins, sha, verify, save):
    result = None
    path_inserted = False
    receipt = dict(status='failed', expected_test_ids=IDS, actual_test_ids=[],
        discovered_test_ids=[], discovered_tests=0, tests_run=0,
        execution_completed=False, inherited_common_mutex=True)
    try:
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
        assert 'test_contracts' not in sys.modules
        original = Path(importlib.util.find_spec('unittest').origin).resolve().parent
        frozen = output / 'python' / 'unittest'
        frozen.mkdir(parents=True)
        for source in sorted(original.rglob('*.py')):
            target = frozen / source.relative_to(original)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            digest = sha(source)
            assert str(source) not in pins or pins[str(source)] == digest
            pins[str(source)] = pins[str(target)] = digest
        pins[str(Path(sys.executable).resolve())] = sha(Path(sys.executable).resolve())
        actual_test = output / 'fixture-sources' / 'test_contracts.py'
        actual_test.parent.mkdir()
        for name in ('test_contracts.py', 'profiles.json'):
            source = Path(__file__).with_name(name)
            target = actual_test.parent / name
            shutil.copyfile(source, target)
            assert sha(target) == pins[str(source.resolve())]
            pins[str(target)] = sha(target)
        verify(pins)
        sys.path.insert(0, str(frozen.parent))
        path_inserted = True
        unittest = importlib.import_module('unittest')
        assert Path(unittest.__file__).resolve() == frozen / '__init__.py'
        def check_unittest_modules():
            for name, loaded in list(sys.modules.items()):
                if name == 'unittest' or name.startswith('unittest.'):
                    actual = Path(loaded.__file__).resolve()
                    assert actual.is_relative_to(frozen) and actual.suffix == '.py'
                    assert sha(actual) == pins[str(actual)]
        check_unittest_modules()
        class RecordedResult(unittest.TextTestResult):
            def __init__(self, *args):
                nonlocal result
                super().__init__(*args)
                result = self
            def startTest(self, test):
                receipt['actual_test_ids'].append(test.id())
                super().startTest(test)
        sys.path.insert(0, str(actual_test.parent))
        try:
            suite = unittest.defaultTestLoader.loadTestsFromName('test_contracts')
        finally:
            sys.path.pop(0)
        receipt['discovered_tests'] = suite.countTestCases()
        def discover_tests(part):
            if isinstance(part, unittest.TestSuite):
                for child in part:
                    discover_tests(child)
            else:
                receipt['discovered_test_ids'].append(part.id())
        discover_tests(suite)
        assert len(receipt['discovered_test_ids']) == len(set(receipt['discovered_test_ids'])) == 6
        assert set(receipt['discovered_test_ids']) == set(IDS)
        check_unittest_modules()
        assert Path(sys.modules['test_contracts'].__file__).resolve() == actual_test
        verify(pins)
        with (output / 'fixtures.log').open('x') as stream:
            result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=RecordedResult).run(suite)
        receipt['execution_completed'] = True
        receipt.update(tests_run=result.testsRun, failures=[(t.id(), why) for t, why in result.failures],
            errors=[(t.id(), why) for t, why in result.errors], skips=[(t.id(), why) for t, why in result.skipped],
            expected_failures=[(t.id(), why) for t, why in result.expectedFailures],
            unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
        assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 6
        assert set(receipt['actual_test_ids']) == set(IDS)
        assert receipt['discovered_tests'] == receipt['tests_run'] == 6
        assert all(receipt[key] == [] for key in ('failures','errors','skips','expected_failures','unexpected_successes'))
        assert result.wasSuccessful()
        check_unittest_modules()
        verify(pins)
        receipt['status'] = 'passed'
    except BaseException as error:
        receipt.update(status='failed', error=str(error) or type(error).__name__, error_type=type(error).__name__)
        raise
    finally:
        if result is not None:
            receipt.update(tests_run=result.testsRun,
                failures=[(t.id(), why) for t, why in result.failures],
                errors=[(t.id(), why) for t, why in result.errors],
                skips=[(t.id(), why) for t, why in result.skipped],
                expected_failures=[(t.id(), why) for t, why in result.expectedFailures],
                unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
        save(output / 'fixture-receipt.json', receipt)
        if path_inserted:
            sys.path.pop(0)
    return receipt
