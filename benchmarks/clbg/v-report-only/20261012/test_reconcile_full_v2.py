"""Exercise real complete records and reject corrupted or incomplete claims."""
from copy import deepcopy
import fcntl
import json
from pathlib import Path
import unittest

import full_study
import reconcile_full_v2 as subject


class Reconciliation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        module = full_study.configure('regexredux')
        cls.manifest = json.loads(module.MANIFEST.read_text())
        cls.report = json.loads(module.REPORT.read_text())
        cls.digest = module.sha(module.MANIFEST)
        cls.harness = module.load_harness()
        cls.reference = subject.expected_output(cls.harness, 'regexredux')
        cls.raw = {a['result_path']: json.loads(Path(a['result_path']).read_text()) for a in cls.report['attempts']}

    def validate(self, report=None, manifest=None, raw=None):
        return subject.validate_data(manifest or deepcopy(self.manifest), report or deepcopy(self.report),
            self.digest, self.harness, self.reference, lambda p: deepcopy((raw or self.raw)[p]))

    def altered_attempt(self, mutate):
        report, raw = deepcopy(self.report), deepcopy(self.raw)
        mutate(report['attempts'][0])
        value = deepcopy(report['attempts'][0])
        value.pop('environment_attempt')
        raw[report['attempts'][0]['result_path']] = value
        with self.assertRaises(ValueError):
            self.validate(report=report, raw=raw)

    def test_complete_actual_records_reconcile(self):
        rows, exclusions = self.validate()
        self.assertEqual(len(rows), 3)
        self.assertEqual(exclusions, [])
        self.assertTrue(all(len(r['samples_seconds']) == 3 for r in rows.values()))

    def test_missing_run_cannot_be_complete(self):
        report = deepcopy(self.report)
        report['attempts'].pop()
        with self.assertRaises(ValueError):
            self.validate(report=report)

    def test_duplicate_profile_in_round_rejected(self):
        manifest = deepcopy(self.manifest)
        manifest['orders'][0]['profiles'][1] = manifest['orders'][0]['profiles'][0]
        report = deepcopy(self.report)
        report['orders'] = deepcopy(manifest['orders'])
        with self.assertRaises(ValueError):
            self.validate(report=report, manifest=manifest)

    def test_bounded_input_and_preparation_markers_rejected(self):
        self.altered_attempt(lambda a: a.update(workload_n=1000))
        self.altered_attempt(lambda a: a.update(preparation_only=True))

    def test_source_identity_or_fingerprint_change_rejected(self):
        self.altered_attempt(lambda a: a.update(source_sha256='0' * 64))
        self.altered_attempt(lambda a: a.update(execution_fingerprint='{}'))

    def test_whole_output_and_native_ownership_rejected(self):
        self.altered_attempt(lambda a: a['validation'].update(sha256='0' * 64))
        self.altered_attempt(lambda a: a['supervised_native_group'].update(inherited_common_mutex=False))

    def test_errors_cannot_be_environment_exclusions(self):
        self.altered_attempt(lambda a: a.update(error='failed worker'))
        self.altered_attempt(lambda a: a.update(qualified=False, valid_environment=False,
                                               retryable_environment_only=True))

    def test_summary_and_raw_record_changes_rejected(self):
        report = deepcopy(self.report)
        next(iter(report['profiles'].values()))['median_seconds'] += 1
        with self.assertRaises(ValueError):
            self.validate(report=report)
        raw = deepcopy(self.raw)
        next(iter(raw.values()))['seconds'] += 1
        with self.assertRaises(ValueError):
            self.validate(raw=raw)


    def test_contradictory_retry_metadata_in_raw_rejected(self):
        raw = deepcopy(self.raw)
        next(iter(raw.values()))['environment_attempt'] = 999
        with self.assertRaises(ValueError):
            self.validate(raw=raw)

    def test_supervisor_error_fields_without_primary_error_rejected(self):
        for field in ('supervisor_error', 'sampler_error', 'registration_error', 'supervisor_cleanup_error'):
            self.altered_attempt(lambda a, field=field: a.update(**{field: 'retained failure'}))


if __name__ == '__main__':
    with full_study.configure('regexredux').LOCK.open('a+') as lock:
        print('WAIT common mutex: V full-record reconciliation corruption fixtures', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        unittest.main(verbosity=2)
