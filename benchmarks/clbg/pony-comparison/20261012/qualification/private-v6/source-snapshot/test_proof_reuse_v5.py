"""Mock fixtures for exact v4 compact-K proof and lossless source lineage."""
import ast
import copy
import json
from pathlib import Path
import unittest

import guarded_prepare_v4 as old
import guarded_prepare_v5 as new
import proof_reuse_v5 as reuse
import test_proof_reuse_v4 as prior_tests


def mock_report():
    report = prior_tests.mock_report()
    report['proposal_sha256'] = reuse.PRIOR_SHA
    sample = copy.deepcopy(report['attempts'][0])
    report['attempts'] = []
    for case in sorted(reuse.CASES['knucleotide']):
        row = copy.deepcopy(sample)
        row.update(id='pony-knucleotide-compact-precedence-v4', benchmark='knucleotide',
                   case=case, workload_n=1000, attempt=len(report['attempts']) + 1)
        fingerprint = json.loads(row['execution_fingerprint'])
        fingerprint['command'][0] = '/old-bin/pony-knucleotide-compact-precedence-v4'
        row['execution_fingerprint'] = json.dumps(fingerprint)
        row['command'] = [fingerprint['command'][0], '1000', '--ponymaxthreads', '8']
        row['supervised_native_group']['command'] = row['command']
        report['attempts'].append(row)
    return report


class V5ProofChecks(unittest.TestCase):
    def setUp(self):
        self.report = mock_report()

    def reject(self, change):
        report = copy.deepcopy(self.report)
        change(report)
        with self.assertRaises(AssertionError):
            reuse.validate_records(report)

    def test_twelve_exact_records_are_preserved(self):
        before = copy.deepcopy(self.report)
        groups = reuse.validate_records(self.report)
        self.assertEqual(set(groups), reuse.NEW_PROFILES)
        self.assertEqual(sum(len(group['cases']) for group in groups.values()), 12)
        self.assertEqual(self.report, before)

    def test_missing_duplicate_wrong_workload_case_rejected(self):
        self.reject(lambda report: report['attempts'].pop())
        self.reject(lambda report: report['attempts'].__setitem__(1, copy.deepcopy(report['attempts'][0])))
        self.reject(lambda report: report['attempts'][0].__setitem__('workload_n', 1))
        self.reject(lambda report: report['attempts'][0].__setitem__('case', 'invented-case'))

    def test_changed_source_binary_fingerprint_rejected(self):
        for field in ('source_sha256', 'command', 'runtime_environment', 'cwd'):
            self.reject(lambda report: report['attempts'][0].__setitem__(field, 'changed'))
        def mutation(report):
            fingerprint = json.loads(report['attempts'][0]['execution_fingerprint'])
            fingerprint['binary_sha256'] = 'recompiled'
            report['attempts'][0]['execution_fingerprint'] = json.dumps(fingerprint)
        self.reject(mutation)

    def test_environment_cleanup_resource_and_output_gates(self):
        for field, value in (('exit_status', 1), ('sampler_wait_status', 1), ('pins_unchanged_at_end', False),
                             ('valid_environment', False), ('seconds', float('nan')), ('seconds', 0),
                             ('monitor_errors', ['failure']), ('resource_violation', 'rss'),
                             ('cleanup_error', 'failed'), ('peak_process_group_rss_kib', 10485761),
                             ('power_at_end', {}), ('competing_compilers', ['cc']),
                             ('stdout_sha256', 'wrong'), ('supervisor_cleanup_error', 'failure')):
            self.reject(lambda report: report['attempts'][0].__setitem__(field, value))
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__('inherited_common_mutex', False))
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__('pgid', 999))

    def test_unqualified_full_or_invented_checks_rejected(self):
        for field in ('qualified', 'full_workload', 'full_performance_run', 'preparation_only', 'never_chart_eligible'):
            self.reject(lambda report: report['attempts'][0].__setitem__(field, not report['attempts'][0][field]))
        self.reject(lambda report: report['attempts'][0].__setitem__('qualification_checks', {'invented': True}))
        self.reject(lambda report: report.__setitem__('active', {'pid': 1}))

    def test_lossless_shift_casts_are_only_source_changes(self):
        before = (new.HERE / 'revisions/precedence-v4/sources/knucleotide/optimized/main.pony').read_text()
        after = (new.HERE / 'revisions/shift-types-v5/sources/knucleotide/optimized/main.pony').read_text()
        expected = before.replace('key >> shift)', 'key >> shift.u64())').replace('<< (2 * _width))', '<< (2 * _width).u64())')
        self.assertEqual(after, expected)
        self.assertEqual(before.count('key >> shift)'), 1)
        self.assertEqual(before.count('<< (2 * _width))'), 1)

    def test_profile_lineage_is_one_new_profile_seven_fresh(self):
        previous = json.loads((new.HERE / 'profiles-v4.json').read_text())['profiles']
        current = json.loads((new.HERE / 'profiles-v5.json').read_text())['profiles']
        differences = [(a, b) for a, b in zip(previous, current, strict=True) if a != b]
        self.assertEqual(len(differences), 1)
        a, b = differences[0]
        self.assertEqual(b['id'], 'pony-knucleotide-optimized-shift-types-v5')
        self.assertEqual(b['parent_id'], a['id'])
        self.assertEqual(b['parent_source'], a['source'])
        self.assertEqual(len(set(row['id'] for row in current) & reuse.PROFILES), 13)
        self.assertEqual(len(current) - len(reuse.PROFILES), 7)

    def test_build_native_owner_probe_and_inventory_guards_unchanged(self):
        before, after = Path(old.__file__).read_text(), Path(new.__file__).read_text()
        for name in ('owned_command', 'native_images', 'build_command', 'probe', 'inputs', 'preflight_inputs', 'download', 'extract', 'main'):
            def code(text):
                node = next(node for node in ast.parse(text).body if isinstance(node, ast.FunctionDef) and node.name == name)
                return ast.get_source_segment(text, node)
            self.assertEqual(code(before), code(after), name)

    def test_profile_row_keeps_old_binary_command(self):
        sample = self.report['attempts'][0]
        fingerprint = json.loads(sample['execution_fingerprint'])
        prior = dict(declaration=dict(id=sample['id'], benchmark='knucleotide'), source='/old-source',
            binary=fingerprint['command'][0], source_sha256=fingerprint['source_sha256'],
            binary_sha256=fingerprint['binary_sha256'], command=fingerprint['command'], cwd=fingerprint['cwd'],
            runtime_environment=fingerprint['environment'], qualification_origin='exact-unchanged-v4-binary-and-records',
            prior_records=['retained-record'])
        row = reuse.profile_row(prior)
        self.assertEqual(row['command'], fingerprint['command'])
        self.assertEqual(row['new_native_cases'], 0)
        self.assertNotIn('seconds', row)


if __name__ == '__main__':
    unittest.main()
