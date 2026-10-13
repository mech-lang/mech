"""Unexecuted source-only fixture inventory, called under the already-held lock."""
import importlib
import json
from pathlib import Path
import shutil
import sys
import sysconfig

PROOFS = [
    'test_tree_requires_actual_fresh_leaves_and_distinct_graph_storage',
    'test_retained_graph_identity_must_survive_actual_churn',
    'test_all_seven_growing_map_inventories_and_handoff_are_required',
    'test_every_overlapping_window_including_tail_is_required',
    'test_two_actual_pi_divisions_recurrence_and_zero_extraction_are_required',
    'test_native_mutex_description_and_owned_observer_cleanup_are_required',
]
OWNERSHIP = [
    'test_restores_exact_original_execute_and_popen_after_observed_native',
    'test_timeout_retains_actual_raw_bytes_and_cleans_only_known_owned_groups',
    'test_group_reconciliation_error_is_unqualified_and_preserves_cleanup_receipt',
]
FULL = [
    'test_nine_actual_ledger_entries_three_each_and_matched_control',
    'test_unqualified_second_attempt_persists_and_stops_without_retry',
    'test_verified_source_failure_is_unscored_and_other_form_control_finish',
    'test_isolation_requires_every_literal_safety_gate_and_exact_typed_inventory',
    'test_supervisor_exception_preserves_raw_and_separate_unqualified_wrapper',
    'test_interrupt_becomes_durable_stopped_evidence_without_actual_signals',
    'test_exact_twenty_gate_names_and_literal_true_values_are_required',
]


def run_locked(snapshot, output, pins, descriptor):
    import fcntl
    import hashlib
    def sha(path):
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    def new(path, value):
        with path.open('x') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
    # Same actual inherited open descriptor, no child acquisition or lock wait.
    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
    assert not any(name in sys.modules for name in ('work_proofs', 'native_fd_observer', 'full_study',
        'test_work_proofs', 'test_native_fd_observer', 'test_full_study')), 'No cached candidate code may precede the held-lock snapshot'
    origin = Path(sysconfig.get_path('stdlib')).resolve() / 'unittest'
    unit = output / 'python-dependencies/unittest'
    for path in sorted(origin.rglob('*.py')):
        target = unit / path.relative_to(origin)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        assert sha(path) == sha(target)
        pins[str(path)] = pins[str(target)] = sha(path)
        target.chmod(0o444)
    python = Path(sys.executable).resolve()
    pins[str(python)] = sha(python)
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(unit.parent))
    sys.path.insert(0, str(snapshot))
    unittest = importlib.import_module('unittest')
    assert Path(unittest.__file__).resolve() == (unit / '__init__.py').resolve()
    class ActualResult(unittest.TextTestResult):
        def __init__(self, *args):
            super().__init__(*args)
            self.started = []
        def startTest(self, test):
            self.started.append(test.id())
            super().startTest(test)
    def flatten(suite):
        for test in suite:
            if isinstance(test, unittest.TestSuite):
                yield from flatten(test)
            else:
                yield test.id()
    receipts = {}
    for kind, name, klass, methods in (
        ('proof_parser', 'test_work_proofs', 'WorkProofChecks', PROOFS),
        ('descriptor_ownership', 'test_native_fd_observer', 'ObserverChecks', OWNERSHIP),
        ('full_pipeline', 'test_full_study', 'LedgerChecks', FULL)):
        assert name not in sys.modules
        module = importlib.import_module(name)
        assert Path(module.__file__).resolve() == (snapshot / (name + '.py')).resolve()
        dependency_names = ('work_proofs', 'native_fd_observer', 'full_study')
        candidate_identities = {key: str(Path(sys.modules[key].__file__).resolve())
            for key in (*dependency_names, name) if key in sys.modules}
        assert all(path == str((snapshot / (key + '.py')).resolve()) and path in pins
            for key, path in candidate_identities.items())
        expected = sorted(name + '.' + klass + '.' + method for method in methods)
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(getattr(module, klass))
        discovered = list(flatten(suite))
        assert sorted(discovered) == expected and len(discovered) == len(set(discovered)) == len(expected)
        log = output / (kind + '-fixtures.log')
        with log.open('x') as stream:
            result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=ActualResult).run(suite)
        pins[str(log)] = sha(log)
        passed = (result.wasSuccessful() and result.testsRun == len(result.started) == len(set(result.started)) == len(expected) and
            sorted(result.started) == expected and not (result.failures or result.errors or result.skipped or result.expectedFailures or result.unexpectedSuccesses))
        receipt = output / (kind + '-fixtures.json')
        identities = {key: str(Path(value.__file__).resolve()) for key, value in sys.modules.items()
            if key == 'unittest' or key.startswith('unittest.')}
        assert all(Path(path).is_relative_to(unit) and path in pins for path in identities.values())
        new(receipt, dict(status='passed' if passed else 'failed', expected_test_ids=expected,
            discovered_test_ids=discovered, actual_test_ids=result.started, started_test_ids=result.started,
            discovered_tests=len(discovered), tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
            expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
            actual_native_subjects=0, native_probes=0, actual_signals_sent=0,
            exclusive_mutex_held=True, shared_timing_lock='/tmp/mech-clbg-timing.lock',
            python_executable=str(python), python_executable_sha256=pins[str(python)],
            unittest_module_identities=identities, candidate_module_identities=candidate_identities, pins=dict(pins)))
        pins[str(receipt)] = sha(receipt)
        receipts[kind] = dict(path=str(receipt), sha256=pins[str(receipt)])
        assert passed, ('Actual source-only fixture inventory failed; retain exact raw/receipt', kind)
    return receipts
