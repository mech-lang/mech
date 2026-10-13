"""Meaningful forged-proof rejections, using preserved44 actual metadata records."""
import copy
import json
from pathlib import Path
import unittest
import proof_reuse_v7 as reuse


class ExactProofReuse(unittest.TestCase):
    def setUp(self):
        self.report = json.loads((reuse.PRIOR / 'report.json').read_text())
    def test_all_seven_exact_profiles_and_44_cases(self):
        groups = reuse.validate_records(self.report)
        self.assertEqual(set(groups), reuse.NEW_PROFILES)
        self.assertEqual(sum(len(group['cases']) for group in groups.values()), 44)
        original = (reuse.PRIOR / 'source-snapshot/support/standard_primitives/main.pony').read_text()
        corrected = (Path(__file__).resolve().parent / 'support/standard_primitives/main.pony').read_text()
        reuse.validate_primitive_correction(original, corrected)
        with self.assertRaises(AssertionError):
            reuse.validate_primitive_correction(original, corrected + '\n')
        with self.assertRaises(AssertionError):
            reuse.validate_primitive_correction(original, corrected.replace('Regex("aa")', 'Regex("aaa")'))
    def test_false_missing_nonliteral_qualification_rejected(self):
        for value in (False, None, 1, 'true'):
            altered = copy.deepcopy(self.report)
            key = next(iter(altered['attempts'][0]['qualification_checks']))
            altered['attempts'][0]['qualification_checks'][key] = value
            with self.assertRaises(AssertionError): reuse.validate_records(altered)
    def test_native_command_mismatch_rejected(self):
        self.report['attempts'][0]['supervised_native_group']['command'][0] += '-different'
        with self.assertRaises(AssertionError): reuse.validate_records(self.report)
    def test_duplicate_or_missing_case_rejected(self):
        self.report['attempts'][1]['case'] = self.report['attempts'][0]['case']
        with self.assertRaises(AssertionError): reuse.validate_records(self.report)
    def test_failed_source_attempt_not_reused(self):
        self.report['attempts'][0]['exit_status'] = 1
        with self.assertRaises(AssertionError): reuse.validate_records(self.report)
    def test_environment_or_memory_failure_not_reused(self):
        self.report['attempts'][0]['resource_violation'] = 'rss'
        with self.assertRaises(AssertionError): reuse.validate_records(self.report)


if __name__ == '__main__': unittest.main()
