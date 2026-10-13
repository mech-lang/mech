"""Metadata fixtures for the new Crystal timing ledger; never launch native work."""
import copy
import fcntl
import hashlib
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import full_study
import seal_full_studies


class LedgerChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='crystal-full-ledger-mock-')
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.output = self.base / 'spectralnorm'
        self.output.mkdir()
        self.report = self.output / 'report.json'
        self.calls = []
        self.guard = SimpleNamespace(OUT=self.output, REPORT=self.report,
            now=lambda: '2026-10-12T00:00:00+00:00', identity=lambda pid: dict(pid=pid),
            verify=lambda pins: None, benchmark_failure=lambda attempt: attempt.get('verified-source-failure') is True)
        def save(path, value):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value))
        self.guard.save = save
        self.harness = SimpleNamespace(install_interrupt_handlers=lambda: None, REQUIRED={},
            compact=SimpleNamespace(wait_for_environment=lambda required: None))
        self.rows = [dict(id='compact', key='spectralnorm:compact', language='crystal', source_sha256='c'),
            dict(id='optimized', key='spectralnorm:optimized', language='crystal', source_sha256='o'),
            dict(id='control', key='spectralnorm:control', language='mech', source_sha256='m')]
        ids = [row['id'] for row in self.rows]
        self.manifest = dict(size=5500, pins={}, profiles=self.rows,
            orders=[dict(round=i+1, profiles=ids[i:]+ids[:i]) for i in range(3)])
        self.patches = [patch.object(full_study, 'LOCK', self.base / 'mock.lock'),
            patch.object(full_study, 'configure', return_value=self.guard),
            patch.object(full_study, 'review', return_value=(self.manifest, self.harness))]
        for replacement in self.patches:
            replacement.start()
            self.addCleanup(replacement.stop)

    def attempt(self, harness, row, manifest, report, round_index, lock_fd):
        self.calls.append((row['id'], round_index))
        return dict(attempt=len(self.calls), qualified=True, seconds=2.0 if row['language'] == 'crystal' else 1.0,
                    validation=dict(reference_match=True), full_workload=True)

    def test_nine_actual_ledger_entries_three_each_and_matched_control(self):
        self.guard.one_attempt = self.attempt
        full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        report = json.loads(self.report.read_text())
        self.assertEqual(report['status'], 'complete')
        self.assertEqual(len(report['attempts']), 9)
        self.assertEqual(self.calls, [(identifier, order['round']) for order in self.manifest['orders'] for identifier in order['profiles']])
        for row in report['profiles'].values():
            self.assertEqual(len(row['samples_seconds']), 3)
            self.assertEqual(row['matched_mech_source_sha256'], 'm')
            self.assertEqual(row['time_x_mech'], 2.0 if row['language'] == 'crystal' else 1.0)
        with self.assertRaises(AssertionError):
            full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        self.assertEqual(len(self.calls), 9, 'Existing report must prevent another actual sample')

    def test_unqualified_second_attempt_persists_and_stops_without_retry(self):
        def attempt(*args):
            value = self.attempt(*args)
            if len(self.calls) == 2:
                value.update(qualified=False, error='fixture output mismatch',
                    qualification_checks={key: True for key in full_study.GATES},
                    **{'verified-source-failure': True})
                value['qualification_checks']['native_inherited_common_mutex'] = False
            return value
        self.guard.one_attempt = attempt
        with self.assertRaises(AssertionError):
            full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        report = json.loads(self.report.read_text())
        self.assertEqual(report['status'], 'stopped-unqualified')
        self.assertEqual(len(self.calls), len(report['attempts']))
        self.assertEqual(len(self.calls), 2)
        self.assertFalse(report['attempts'][-1]['qualified'])
        self.assertIsNone(report['active'])

    def test_verified_source_failure_is_unscored_and_other_form_control_finish(self):
        def attempt(*args):
            value = self.attempt(*args)
            if args[1]['id'] == 'compact':
                value.update(qualified=False, error='fixture real output mismatch', error_type='OutputValidationError',
                    qualification_checks={key: True for key in full_study.GATES},
                    **{'verified-source-failure': True})
                value['qualification_checks']['no_execution_error'] = False
                value['qualification_checks']['complete_output_reference_match'] = False
            return value
        self.guard.one_attempt = attempt
        full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        report = json.loads(self.report.read_text())
        self.assertEqual(report['status'], 'complete-with-source-failures')
        self.assertEqual(len(report['attempts']), 7)
        failed = report['profiles']['compact']
        self.assertEqual(failed['samples_seconds'], [])
        self.assertEqual(failed['measurement_status'], 'failed-full-source-unscored')
        self.assertIsNone(failed['time_x_mech'])
        self.assertEqual(len(report['profiles']['optimized']['samples_seconds']), 3)
        self.assertEqual(len(report['profiles']['control']['samples_seconds']), 3)
        self.assertEqual(report['profiles']['optimized']['time_x_mech'], 2.0)
        self.assertEqual(sum(identifier == 'compact' for identifier, _ in self.calls), 1)

    def test_isolation_requires_every_literal_safety_gate_and_exact_typed_inventory(self):
        value = dict(qualification_checks={key: True for key in full_study.GATES},
                     **{'verified-source-failure': True})
        value['qualification_checks']['no_execution_error'] = False
        self.assertTrue(full_study.isolated_profile_failure(self.guard, value))
        for key in full_study.SOURCE_FAILURE_SAFETY:
            for changed_value in (False, 1, None):
                with self.subTest(key=key, value=changed_value):
                    changed = copy.deepcopy(value)
                    if changed_value is None:
                        del changed['qualification_checks'][key]
                    else:
                        changed['qualification_checks'][key] = changed_value
                    self.assertFalse(full_study.isolated_profile_failure(self.guard, changed))
        changed = copy.deepcopy(value)
        changed['qualification_checks']['unknown'] = True
        self.assertFalse(full_study.isolated_profile_failure(self.guard, changed))
        changed = copy.deepcopy(value)
        changed['qualification_checks']['no_execution_error'] = 0
        self.assertFalse(full_study.isolated_profile_failure(self.guard, changed))

    def test_supervisor_exception_preserves_raw_and_separate_unqualified_wrapper(self):
        raw = self.output / 'attempts/001-compact.json'
        evidence = dict(qualified=True, seconds=2.0, validation=dict(reference_match=True))
        self.guard.save(raw, evidence)
        def attempt(*args):
            self.calls.append(('compact', 1))
            raise RuntimeError('fixture supervisor cleanup failure')
        self.guard.one_attempt = attempt
        with self.assertRaises(RuntimeError):
            full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        self.assertEqual(json.loads(raw.read_text()), evidence)
        wrapped = json.loads(raw.with_suffix('.wrapper.json').read_text())
        self.assertFalse(wrapped['qualified'])
        self.assertTrue(wrapped['wrapper_supervisor_failure'])
        report = json.loads(self.report.read_text())
        self.assertEqual(report['status'], 'stopped-unqualified')
        self.assertEqual(report['attempts'], [wrapped])
        self.assertEqual(len(self.calls), 1)

    def test_interrupt_becomes_durable_stopped_evidence_without_actual_signals(self):
        def attempt(*args):
            raise KeyboardInterrupt('fixture interruption')
        self.guard.one_attempt = attempt
        with self.assertRaises(KeyboardInterrupt):
            full_study.run('spectralnorm', 'mock-v1', 'reviewed-fixture')
        report = json.loads(self.report.read_text())
        self.assertEqual(report['status'], 'stopped-unqualified')
        self.assertEqual(report['error_type'], 'KeyboardInterrupt')
        self.assertEqual(len(report['attempts']), 1)
        self.assertFalse(report['attempts'][0]['qualified'])
        self.assertEqual(report['profiles']['compact']['samples_seconds'], [])

    def test_exact_twenty_gate_names_and_literal_true_values_are_required(self):
        value = dict(qualification_checks={key: True for key in seal_full_studies.GATES},
            qualified=True, full_workload=False, exit_status=0, sampler_wait_status=0,
            validation=dict(reference_match=True), pins_unchanged_at_end=True, full_performance_run=False)
        seal_full_studies.exact_true_checks(value)
        for changes in ({next(iter(seal_full_studies.GATES)): 1}, {'unknown': True}):
            changed = copy.deepcopy(value)
            changed['qualification_checks'].update(changes)
            with self.assertRaises(AssertionError):
                seal_full_studies.exact_true_checks(changed)
        changed = copy.deepcopy(value)
        removed = next(iter(seal_full_studies.GATES))
        del changed['qualification_checks'][removed]
        changed['qualification_checks']['unknown'] = True
        with self.assertRaises(AssertionError):
            seal_full_studies.exact_true_checks(changed)


def main():
    print('WAIT common mutex: Crystal full-study metadata ledger fixtures', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(LedgerChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = Path(__file__).with_name('full-study-fixtures.log')
        receipt = Path(__file__).with_name('full-study-fixtures.json')
        with log.open('x') as stream:
            stream.write(text)
        paths = [Path(__file__).resolve(), Path(full_study.__file__).resolve(),
                 Path(seal_full_studies.__file__).resolve(), log]
        with receipt.open('x') as stream:
            json.dump(dict(status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
                failures=len(result.failures), errors=len(result.errors), native_probes=0, actual_signals_sent=0,
                shared_timing_lock=str(full_study.LOCK),
                pins={str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}), stream, indent=2)
            stream.write('\n')
        if not result.wasSuccessful() or result.testsRun != 7:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
