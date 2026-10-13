"""Same-PID local fixture runner. Caller already holds the actual common mutex."""
import importlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys

IDS = [
    'test_work_proofs.SourceHooks.test_exact_reverse_projection_preserves_all_original_source_bytes',
    'test_work_proofs.TreeWork.test_complete_source_iteration_ranges_and_retained_graphs',
    'test_work_proofs.TreeWork.test_shared_leaf_or_missing_second_real_child_is_rejected',
    'test_work_proofs.MapWork.test_all_seven_actual_maps_capacity_and_overlapping_tail_totals',
    'test_work_proofs.MapWork.test_empty_maps_are_real_distinct_default_maps_and_zero_is_not_false',
    'test_work_proofs.PiWork.test_actual_both_divisions_zero_tail_and_independent_golden_digits',
]


def run(output, pins, sha, verify, save, original_sources):
    result = None
    path_inserted = False
    receipt = dict(status='failed', expected_test_ids=IDS, actual_test_ids=[],
        discovered_test_ids=[], discovered_tests=0, tests_run=0,
        execution_completed=False, inherited_common_mutex=True)
    try:
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
        assert all(name not in sys.modules for name in ('test_work_proofs', 'instrument_sources', 'work_proofs'))
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
        actual_test = output / 'fixture-sources' / 'test_work_proofs.py'
        actual_test.parent.mkdir()
        for name in ('test_work_proofs.py', 'instrument_sources.py', 'work_proofs.py', 'observer.pl'):
            source = Path(__file__).with_name(name)
            target = actual_test.parent / name
            shutil.copyfile(source, target)
            assert sha(target) == pins[str(source.resolve())]
            pins[str(target)] = sha(target)
        for bench in ('binarytrees', 'pidigits', 'knucleotide'):
            for form in ('compact', 'optimized'):
                source = Path(original_sources[(bench, form)])
                target = actual_test.parent / 'original-sources/sources' / bench / (form + '.pl')
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
                assert sha(source) == sha(target) == pins[str(source)]
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
            suite = unittest.defaultTestLoader.loadTestsFromName('test_work_proofs')
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
        for name in ('test_work_proofs', 'instrument_sources', 'work_proofs'):
            path = Path(sys.modules[name].__file__).resolve()
            assert path == actual_test.parent / (name + '.py') and sha(path) == pins[str(path)]
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
        for name in ('test_work_proofs', 'instrument_sources', 'work_proofs'):
            path = Path(sys.modules[name].__file__).resolve()
            assert path == actual_test.parent / (name + '.py') and sha(path) == pins[str(path)]
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
