"""Inherited common-lock actual three-method inventory; source mocks only."""
import argparse
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import shutil
import sys
import sysconfig

EXPECTED = sorted('test_controller.RecoveryChecks.' + name for name in (
    'test_exact_typed_failure_isolation_rejects_each_safety_and_inventory_corruption',
    'test_failed_tree_and_all75_original_records_remain_unchanged_in_additive_seed',
    'test_already_measured_or_failed_selected_source_and_alias_duplication_cannot_recover'))


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--receipt', type=Path, required=True); parser.add_argument('--lock-fd', type=int, required=True)
    args = parser.parse_args()
    named = Path('/tmp/mech-clbg-timing.lock').stat(); actual = os.fstat(args.lock_fd)
    assert args.lock_fd >= 64 and (named.st_dev, named.st_ino) == (actual.st_dev, actual.st_ino)
    fcntl.flock(args.lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    assert 'unittest' not in sys.modules
    root = Path(__file__).resolve().parent
    runtime = Path(sys.executable).resolve()
    unittest_root = Path(sysconfig.get_path('stdlib')).resolve() / 'unittest'
    snapshot = args.receipt.parent / 'python-dependencies/unittest'
    dependencies = {}
    for source in sorted(unittest_root.rglob('*.py')):
        target = snapshot / source.relative_to(unittest_root)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        assert sha(source) == sha(target)
        dependencies[str(source)] = dependencies[str(target)] = sha(source)
        target.chmod(0o444)
    dependencies[str(runtime)] = sha(runtime)
    assert not list(snapshot.rglob('*.pyc'))
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(snapshot.parent))
    unittest = importlib.import_module('unittest')
    assert Path(unittest.__file__).resolve() == (snapshot / '__init__.py').resolve()
    tests = importlib.import_module('test_controller'); controller = importlib.import_module('controller')
    assert Path(tests.__file__).resolve() == root / 'test_controller.py' and Path(controller.__file__).resolve() == root / 'controller.py'
    class Recorded(unittest.TextTestResult):
        def __init__(self, *args): super().__init__(*args); self.started = []
        def startTest(self, test): self.started.append(test.id()); super().startTest(test)
    def ids(suite):
        for item in suite:
            if isinstance(item, unittest.TestSuite): yield from ids(item)
            else: yield item.id()
    receipt = dict(status='failed', expected_test_ids=EXPECTED, discovered_test_ids=[], started_test_ids=[], tests_run=0,
        actual_native_subjects=0, actual_signals_sent=0, exclusive_common_mutex_held=True, inherited_common_fd=args.lock_fd,
        python_executable=str(runtime), python_dependency_pins=dependencies)
    try:
        suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
        receipt['discovered_test_ids'] = list(ids(suite))
        receipt['discovered_tests'] = suite.countTestCases()
        assert sorted(receipt['discovered_test_ids']) == EXPECTED
        result = unittest.TextTestRunner(verbosity=2, resultclass=Recorded).run(suite)
        receipt.update(started_test_ids=result.started, actual_test_ids=result.started, tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
            expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses))
        assert receipt['tests_run'] == receipt['discovered_tests'] == len(EXPECTED) and sorted(result.started) == EXPECTED and len(set(result.started)) == 3
        assert result.wasSuccessful() and all(receipt[key] == 0 for key in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
        for name, module in sys.modules.items():
            if name == 'unittest' or name.startswith('unittest.'):
                path = Path(module.__file__).resolve(); assert path.is_relative_to(snapshot) and str(path) in dependencies
        assert all(sha(path) == digest for path, digest in dependencies.items())
        receipt['status'] = 'passed'
    finally:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        with args.receipt.open('x') as stream: json.dump(receipt, stream, indent=2); stream.write('\n')
    assert receipt['status'] == 'passed'


if __name__ == '__main__': main()
