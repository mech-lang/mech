"""Same-PID local fixture runner. Caller already holds the actual common mutex."""
import importlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys

IDS = [
    'test_image_closure_v2.RuntimeClosure.test_scoped_arm64_header_and_load_command_inventory',
    'test_image_closure_v2.RuntimeClosure.test_dylib_install_id_is_not_a_dependency',
    'test_image_closure_v2.RuntimeClosure.test_executable_and_loader_anchors_differ_for_a_library',
    'test_image_closure_v2.RuntimeClosure.test_rpaths_require_actual_unique_private_images',
    'test_image_closure_v2.RuntimeClosure.test_fresh_import_rejects_cache_or_changed_source_before_execution',
    'test_image_closure_v2.RuntimeClosure.test_partial_attach_preserves_actual_device_entities_without_detach_authority',
]


def run(output, pins, sha, verify, save):
    result = None
    path_inserted = False
    receipt = dict(status='failed', expected_test_ids=IDS, actual_test_ids=[],
        discovered_test_ids=[], discovered_tests=0, tests_run=0,
        execution_completed=False, inherited_common_mutex=True)
    try:
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
        assert all(name not in sys.modules for name in (
            'test_image_closure_v2', 'image_closure_v2', 'bootstrap_runtime_v2'))
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
        actual_test = output / 'fixture-sources' / 'test_image_closure_v2.py'
        actual_test.parent.mkdir()
        for name in ('test_image_closure_v2.py', 'image_closure_v2.py', 'bootstrap_runtime_v2.py'):
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
            suite = unittest.defaultTestLoader.loadTestsFromName('test_image_closure_v2')
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
        for name in ('test_image_closure_v2', 'image_closure_v2', 'bootstrap_runtime_v2'):
            actual = Path(sys.modules[name].__file__).resolve()
            assert actual == actual_test.parent / (name + '.py') and sha(actual) == pins[str(actual)]
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
        for name in ('test_image_closure_v2', 'image_closure_v2', 'bootstrap_runtime_v2'):
            actual = Path(sys.modules[name].__file__).resolve()
            assert actual == actual_test.parent / (name + '.py') and sha(actual) == pins[str(actual)]
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
