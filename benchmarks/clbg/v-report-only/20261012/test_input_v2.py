"""Narrow regression checks for the fresh source selector and oracle ownership."""
import copy
import fcntl
import json
from pathlib import Path
import unittest

import full_study
import prepare
import prepare_input_v2 as revision

HERE = Path(__file__).resolve().parent


class InputRevisionChecks(unittest.TestCase):
    def setUp(self):
        self.draft = json.loads(revision.DRAFT_PATH.read_text())

    def test_selects_exact_twenty_fresh_profiles(self):
        rows = revision.select_profiles(self.draft)
        self.assertEqual(len(rows), 20)
        self.assertEqual(len({row['id'] for row in rows}), 20)
        for benchmark in full_study.BENCHMARKS:
            self.assertEqual(len(revision.select_profiles(self.draft, benchmark)), 2)

    def test_rejects_source_outside_revision(self):
        self.draft['profiles'][0]['source'] = '../outside.v'
        with self.assertRaisesRegex(AssertionError, 'Outside reviewed'):
            revision.select_profiles(self.draft)

    def test_rejects_original_source_path(self):
        self.draft['profiles'][0]['source'] = 'sources/nbody/compact.v'
        with self.assertRaisesRegex(AssertionError, 'Outside reviewed'):
            revision.select_profiles(self.draft)

    def test_rejects_duplicate_profile_key(self):
        self.draft['profiles'][1]['id'] = self.draft['profiles'][0]['id']
        with self.assertRaises(AssertionError):
            revision.select_profiles(self.draft)

    def test_rejects_missing_pair(self):
        self.draft['profiles'][0]['variant'] = 'other'
        with self.assertRaises(AssertionError):
            revision.select_profiles(self.draft)

    def test_rejects_old_profile_key(self):
        self.draft['profiles'][0]['id'] = 'historical-profile'
        with self.assertRaisesRegex(AssertionError, 'fresh profile key'):
            revision.select_profiles(self.draft)

    def test_exact_token_oracle_restores_on_success(self):
        original = prepare.references.knucleotide
        data = b'>THREEBAD\nGGGG\n>THREE actual\nACGT\n'
        with revision.exact_oracle('knucleotide'):
            output = prepare.references.knucleotide(data)
            self.assertIn(b'A 25.000\n', output)
            self.assertIn(b'C 25.000\n', output)
        self.assertIs(prepare.references.knucleotide, original)

    def test_exact_token_oracle_restores_on_failure(self):
        original = prepare.references.knucleotide
        with self.assertRaisesRegex(RuntimeError, 'injected'):
            with revision.exact_oracle('knucleotide'):
                raise RuntimeError('injected oracle-owner failure')
        self.assertIs(prepare.references.knucleotide, original)


def main():
    module = full_study.configure('nbody')
    print('WAIT common mutex: V input-v2 selector fixtures', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        destination = HERE / 'input-v2-fixtures.json'
        assert not destination.exists(), 'Preserve every attempted fixture receipt'
        files = [Path(__file__).resolve(), HERE / 'prepare_input_v2.py',
                 HERE / 'prepare_input_v2_cohort.py', HERE / 'prepare.py',
                 HERE / 'prepare_cohort.py', HERE / 'full_study.py',
                 HERE / 'dna_oracles.py', revision.DRAFT_PATH, revision.PLAN_PATH]
        draft = json.loads(revision.DRAFT_PATH.read_text())
        files += [(HERE / row['source']).resolve() for row in draft['profiles']]
        pins = {str(path): module.sha(path) for path in files}
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(InputRevisionChecks)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        module.load_harness().PinGuard(pins).verify()
        module.save(destination, dict(status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors),
            created_at=module.now(), pins=pins, pins_unchanged_at_end=True,
            full_performance_runs=0, report_only=True, never_chart_eligible=True))
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
