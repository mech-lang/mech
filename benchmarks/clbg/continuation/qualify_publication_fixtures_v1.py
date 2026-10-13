"""Qualify additive record/importer corruption fixtures under the common mutex.

Only synthetic records and temporary text files are used. This controller
executes no compiler or native benchmark and does not publish a dataset.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import io
import json
import os
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
EXPLORER = ROOT / 'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
EXPECTED = {
    'test_reconcile_native_records_v2.ExpandedRecords.'+name for name in (
        'test_gleam_and_clojure_keep_exact_actual_record_contract',
        'test_v_cannot_be_reclassified_as_a_publication_language',
        'test_new_vm_languages_cannot_use_bounded_or_changed_command_records',
        'test_new_vm_languages_cannot_overwrite_saved_sampler_or_output_evidence')
} | {
    'test_native_languages_data_v1.NativeAdmission.'+name for name in (
        'test_complete_shape_preserves_all_source_counts_and_inputs',
        'test_v_and_partial_suite_are_rejected',
        'test_changed_binary_or_omitted_authored_helper_cannot_use_old_count',
        'test_altered_median_output_or_unresolved_parser_evidence_is_rejected',
        'test_planned_backend_or_missing_library_provenance_cannot_be_published',
        'test_missing_compiler_inventory_cannot_hide_a_helper_from_reused_rows',
        'test_different_id_cannot_overwrite_an_existing_candidate_key')
} | {
    'test_native_languages_data_v1.NativePublicationTransaction.'+name for name in (
        'test_complete_import_preserves_existing_records_metadata_and_map_identities',
        'test_late_changed_report_leaves_entire_input_unchanged',
        'test_late_source_read_failure_leaves_entire_input_unchanged',
        'test_cross_cohort_identity_collision_leaves_entire_input_unchanged',
        'test_unrelated_descriptor_is_rejected_before_any_import')
}
INPUTS = [Path(__file__).resolve(), HERE/'reconcile_native_records_v1.py',
          HERE/'test_reconcile_native_records_v1.py',HERE/'reconcile_native_records_v2.py',
          HERE/'test_reconcile_native_records_v2.py',EXPLORER/'native_languages_data_v1.py',
          EXPLORER/'test_native_languages_data_v1.py',Path(sys.executable).resolve()]


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def flatten(suite):
    for test in suite:
        if isinstance(test,unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test.id()


def run(job):
    job = job.resolve()
    job.mkdir(parents=True,exist_ok=True)
    with (job/'owner.lock').open('a+') as owner:
        fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
        if (job/'fixture-receipt.json').exists():
            raise RuntimeError('Terminal fixture receipt already exists; no implicit overwrite')
        print('WAIT LOCK: publication contract fixtures only; zero native benchmarks',flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            started = datetime.now(timezone.utc).isoformat()
            pins = {str(path):sha(path) for path in INPUTS}
            os.environ['MECH_CLBG_TIMING_LOCK_FD'] = str(lock.fileno())
            sys.path[:0] = [str(HERE),str(EXPLORER)]
            log = io.StringIO()
            started_ids,discovered = [],[]
            receipt = dict(status='failed',started_at=started,source_pins=pins,
                required_fixture_ids=sorted(EXPECTED),expected_fixture_count=len(EXPECTED),
                exclusive_common_timing_mutex_held=True,actual_native_processes=0,
                synthetic_fixture_records_never_admitted=True,source_pool_admission=False,chart_admission=False)
            class ObservedResult(unittest.TextTestResult):
                def startTest(self,test):
                    started_ids.append(test.id())
                    super().startTest(test)
            try:
                modules = [importlib.import_module(name) for name in
                           ('test_reconcile_native_records_v2','test_native_languages_data_v1')]
                suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(module) for module in modules)
                discovered = list(flatten(suite))
                assert len(discovered)==len(EXPECTED) and set(discovered)==EXPECTED,'Discovered fixture inventory differs'
                result = unittest.TextTestRunner(stream=log,verbosity=2,resultclass=ObservedResult).run(suite)
                receipt.update(tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                    skips=len(result.skipped),expected_failures=len(result.expectedFailures),
                    unexpected_successes=len(result.unexpectedSuccesses))
                assert all(sha(Path(path))==digest for path,digest in pins.items()),'Fixture input changed during run'
                assert result.testsRun==len(EXPECTED) and len(started_ids)==len(EXPECTED) and set(started_ids)==EXPECTED,'Actual fixture execution differs'
                assert result.wasSuccessful() and not any((result.failures,result.errors,result.skipped,
                    result.expectedFailures,result.unexpectedSuccesses)),'Fixture suite did not pass cleanly'
                receipt['status'] = 'passed'
            except BaseException as error:
                receipt.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            finally:
                receipt.update(discovered_fixture_ids=discovered,actual_started_fixture_ids=started_ids,
                               completed_at=datetime.now(timezone.utc).isoformat())
                with (job/'fixtures.log').open('x') as stream:
                    stream.write(log.getvalue())
                receipt['raw_log'] = dict(path=str(job/'fixtures.log'),sha256=sha(job/'fixtures.log'))
                with (job/'fixture-receipt.json').open('x') as stream:
                    json.dump(receipt,stream,indent=2)
                    stream.write('\n')
                print(log.getvalue(),end='',flush=True)
                print(json.dumps(dict(status=receipt['status'],receipt=str(job/'fixture-receipt.json'),
                                      error=receipt.get('error'),actual_native_processes=0)),flush=True)
            if receipt['status']!='passed':
                raise SystemExit(1)


if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--job',type=Path,required=True)
    run(parser.parse_args().job)
