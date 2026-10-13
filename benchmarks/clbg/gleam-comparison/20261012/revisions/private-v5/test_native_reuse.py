"""Three actual partial-record and narrow value-segment repair fixtures."""
import copy
import json
from pathlib import Path
import unittest

import native_reuse as reuse


class NativeReuse(unittest.TestCase):
    def test_exact_fourteen_apps_110_cases_keep_original_producer_identity(self):
        report = json.loads((reuse.PRIOR / 'report.json').read_text())
        inventory = json.loads(reuse.PRIOR_PROPOSAL.read_text())['case_inventory']
        preserved = copy.deepcopy(report)
        rows, groups = reuse.validate_records(report, inventory)
        self.assertEqual(len(rows), 14)
        self.assertEqual(sum(map(len, groups.values())), 106)
        self.assertEqual(len(rows[reuse.OLD_ID]['prior_bounded_records']), 4)
        self.assertEqual(report, preserved)
        self.assertTrue(reuse.PATCHED.isdisjoint(rows))

    def test_missing_partial_nonboolean_receipt_or_changed_runtime_cannot_reuse(self):
        actual = json.loads((reuse.PRIOR / 'report.json').read_text())
        inventory = json.loads(reuse.PRIOR_PROPOSAL.read_text())['case_inventory']
        for mutation in ('missing-profile', 'missing-case', 'duplicate', 'wait', 'ordinal', 'workload',
                         'gate', 'runtime', 'fingerprint', 'failure'):
            value = copy.deepcopy(actual)
            if mutation == 'missing-profile': value['profiles'].pop()
            elif mutation == 'missing-case': value['attempts'].pop()
            elif mutation == 'duplicate': value['attempts'][1] = copy.deepcopy(value['attempts'][0])
            elif mutation == 'wait': value['attempts'][0]['sampler_wait_status'] = False
            elif mutation == 'ordinal': value['attempts'][0]['attempt'] = True
            elif mutation == 'workload': value['attempts'][0]['workload_n'] = False
            elif mutation == 'gate': value['attempts'][0]['qualification_checks']['pins_unchanged'] = 1
            elif mutation == 'runtime': value['profiles'][1]['runtime_environment']['ERL_CRASH_DUMP'] = '/changed/dump'
            elif mutation == 'fingerprint': value['attempts'][0]['execution_fingerprint'] = '{}'
            else: value['error'] = 'different failed compiler cause'
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError): reuse.validate_records(value, inventory)
        attempt = actual['attempts'][0]
        raw = json.loads(Path(attempt['result_path']).read_text())
        supervisor = json.loads(Path(attempt['supervisor_result_path']).read_text())
        reuse.validate_receipts(raw, supervisor, attempt)
        for target, field in (('supervisor', 'sampler_wait_status'), ('supervisor', 'attempt'),
                              ('raw', 'workload_n'), ('supervisor', 'workload_n')):
            sampled, enriched = copy.deepcopy(raw), copy.deepcopy(supervisor)
            item = sampled if target == 'raw' else enriched
            item[field] = False if field == 'sampler_wait_status' else True
            with self.subTest(receipt=target, field=field), self.assertRaises(AssertionError):
                reuse.validate_receipts(sampled, enriched, attempt)
        full = copy.deepcopy(raw); full['full_workload'] = True
        with self.assertRaises(AssertionError): reuse.validate_receipts(full, supervisor, attempt)
        zero = copy.deepcopy(next(value for value in actual['attempts'] if value['stdout_bytes'] == 0))
        zero['stdout_bytes'] = zero['validation']['bytes'] = False
        with self.assertRaises(AssertionError): reuse.complete_native(zero)
        original_zero = next(value for value in actual['attempts'] if value['stdout_bytes'] == 0)
        zero_raw = json.loads(Path(original_zero['result_path']).read_text())
        zero_supervisor = json.loads(Path(original_zero['supervisor_result_path']).read_text())
        for field in ('stdout_bytes', 'validation-bytes'):
            changed = copy.deepcopy(zero_raw)
            if field == 'stdout_bytes': changed['stdout_bytes'] = False
            else: changed['validation']['bytes'] = False
            with self.subTest(raw_zero=field), self.assertRaises(AssertionError):
                reuse.validate_receipts(changed, zero_supervisor, original_zero)
        group = copy.deepcopy(attempt)
        group['supervised_native_group'].update(pid=1, pgid=True)
        with self.assertRaises(AssertionError): reuse.complete_native(group)

    def test_only_two_revcomp_value_options_change_and_failed_source_remains(self):
        reuse.validate_sources()
        old = json.loads((reuse.HERE / 'revisions/private-v4/profiles.json').read_text())['profiles']
        new = json.loads((reuse.PACKET / 'profiles.json').read_text())['profiles']
        changed = {before['id'] for before, after in zip(old, new) if before != after}
        self.assertEqual(changed, reuse.PATCHED)
        self.assertTrue(changed.isdisjoint(reuse.REUSED_IDS))
        failed = reuse.PRIOR / 'builds/gleam-revcomp-compact/compile.stderr'
        self.assertIn('only allowed in BitArray patterns', failed.read_text())
        self.assertIn('<<header:bytes, 10>>', (reuse.HERE / 'sources/revcomp/compact.gleam').read_text())


if __name__ == '__main__': unittest.main()
