"""Mocked exact-proof/alias gates. No production binary or native signal runs."""
import ast
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
from types import SimpleNamespace
import unittest

import guarded_prepare as old
import guarded_prepare_v3 as new
import proof_reuse_v2 as reuse


def mock_report():
    report = dict(status='stopped-unqualified', full_performance_runs=0, chart_admission=False, attempts=[])
    for identifier in sorted(reuse.PROFILES):
        benchmark = identifier.split('-')[1]
        fingerprint = dict(binary_sha256='exact-old-binary', source_sha256='exact-old-source',
            command=['/old-bin/' + identifier, '{n}', '--ponymaxthreads', '8'],
            cwd='/old-cwd', environment={'DYLD_LIBRARY_PATH': '/old-library'}, support={})
        for case in sorted(reuse.CASES[benchmark]):
            command = [fingerprint['command'][0], '30', '--ponymaxthreads', '8']
            report['attempts'].append(dict(id=identifier, benchmark=benchmark, case=case,
                qualified=True, qualification_checks={key: True for key in reuse.EXPECTED_GATES},
                full_workload=False, full_performance_run=False, preparation_only=True,
                never_chart_eligible=True, exit_status=0, sampler_wait_status=0,
                pins_unchanged_at_end=True, valid_environment=True,
                validation=dict(reference_match=True, bytes=7, sha256='old-output'),
                stdout_bytes=7, stdout_sha256='old-output', execution_fingerprint=json.dumps(fingerprint),
                command=command, workload_n=30, supervised_native_group=dict(command=command,
                    inherited_common_mutex=True), source_sha256='exact-old-source',
                runtime_environment=fingerprint['environment'], cwd=fingerprint['cwd']))
    return report


class ExactOldProofChecks(unittest.TestCase):
    def setUp(self):
        self.report = mock_report()

    def reject(self, mutate):
        report = copy.deepcopy(self.report)
        mutate(report)
        with self.assertRaises(AssertionError):
            reuse.validate_records(report)

    def test_twenty_mock_records_preserve_four_fingerprints(self):
        original = copy.deepcopy(self.report)
        groups = reuse.validate_records(self.report)
        self.assertEqual(set(groups), reuse.PROFILES)
        self.assertEqual(sum(len(group['cases']) for group in groups.values()), 20)
        self.assertEqual(self.report, original)

    def test_missing_or_duplicate_case_rejected(self):
        self.reject(lambda report: report['attempts'].pop())
        self.reject(lambda report: report['attempts'].__setitem__(1, copy.deepcopy(report['attempts'][0])))

    def test_changed_fingerprint_or_runtime_rejected(self):
        for field in ('source_sha256', 'cwd', 'runtime_environment', 'command'):
            with self.subTest(field=field):
                self.reject(lambda report: report['attempts'][0].__setitem__(field, 'changed'))
        def changed_binary(report):
            fingerprint = json.loads(report['attempts'][0]['execution_fingerprint'])
            fingerprint['binary_sha256'] = 'fresh-recompiled-binary'
            report['attempts'][0]['execution_fingerprint'] = json.dumps(fingerprint)
        self.reject(changed_binary)

    def test_native_cap_and_inherited_lock_are_required(self):
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__(
            'inherited_common_mutex', False))
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__(
            'command', ['/unrelated-native']))
        def wrong_cap(report):
            fingerprint = json.loads(report['attempts'][0]['execution_fingerprint'])
            fingerprint['command'][-1] = '9'
            report['attempts'][0]['execution_fingerprint'] = json.dumps(fingerprint)
        self.reject(wrong_cap)

    def test_failed_output_or_execution_or_integrity_rejected(self):
        for field, value in (('exit_status', 1), ('sampler_wait_status', 1), ('pins_unchanged_at_end', False),
                             ('valid_environment', False), ('stdout_sha256', 'wrong'),
                             ('resource_violation', 'rss'), ('monitor_errors', ['failure'])):
            with self.subTest(field=field):
                self.reject(lambda report: report['attempts'][0].__setitem__(field, value))

    def test_full_scored_or_unqualified_record_cannot_be_reused(self):
        for field in ('full_workload', 'full_performance_run', 'qualified', 'preparation_only', 'never_chart_eligible'):
            with self.subTest(field=field):
                self.reject(lambda report: report['attempts'][0].__setitem__(field,
                    not report['attempts'][0][field]))
        for checks in ({}, {'invented_gate': True}, {key: True for key in reuse.EXPECTED_GATES if key != 'pins_unchanged'},
                       dict.fromkeys(reuse.EXPECTED_GATES, 1),
                       dict.fromkeys(reuse.EXPECTED_GATES, 'true'), dict.fromkeys(reuse.EXPECTED_GATES, False)):
            with self.subTest(checks=checks):
                self.reject(lambda report: report['attempts'][0].__setitem__('qualification_checks', checks))

    def test_changed_source_or_binary_identity_rejected(self):
        fingerprint = dict(source_sha256='old-source', binary_sha256='old-binary')
        reuse.require_identity(fingerprint, 'old-source', 'old-binary')
        for source, binary in (('changed-source', 'old-binary'), ('old-source', 'new-binary')):
            with self.assertRaises(AssertionError):
                reuse.require_identity(fingerprint, source, binary)

    def test_reused_row_keeps_exact_old_execution_and_provenance(self):
        identifier = sorted(reuse.PROFILES)[0]
        group = reuse.validate_records(self.report)[identifier]
        fingerprint = group['fingerprint']
        prior = dict(declaration=dict(id=identifier, benchmark=group['benchmark']), source='/old-source',
            binary=fingerprint['command'][0], source_sha256=fingerprint['source_sha256'],
            binary_sha256=fingerprint['binary_sha256'], command=fingerprint['command'], cwd=fingerprint['cwd'],
            runtime_environment=fingerprint['environment'], execution_fingerprint=json.dumps(fingerprint),
            qualification_origin='exact-unchanged-v1-binary-and-records', prior_records=['old-record'])
        row = reuse.profile_row(prior)
        self.assertEqual(row['command'], fingerprint['command'])
        self.assertEqual(row['runtime_environment'], fingerprint['environment'])
        self.assertEqual(row['source'], '/old-source')
        self.assertEqual(row['new_native_cases'], 0)
        self.assertEqual(row['prior_bounded_records'], ['old-record'])
        self.assertNotIn('qualified', row)
        self.assertNotIn('seconds', row)
        prior['command'] = ['/newly-recompiled-binary', '{n}', '--ponymaxthreads', '8']
        with self.assertRaises(AssertionError):
            reuse.profile_row(prior)

    def test_retained_archive_must_match_extracted_tool_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / 'tool.tar'
            directory = root / 'extracted'
            directory.mkdir()
            target = directory / 'ponyc'
            target.write_bytes(b'exact-old-compiler')
            with tarfile.open(archive, 'w') as bundle:
                member = tarfile.TarInfo('ponyc')
                member.size = len(target.read_bytes())
                bundle.addfile(member, io.BytesIO(target.read_bytes()))
            digest = new.sha(target)
            preparation = SimpleNamespace(sha=lambda path: digest)
            self.assertEqual(reuse.archive_correspondence(preparation, archive, directory), {str(target.resolve()): digest})
            with self.assertRaises(AssertionError):
                reuse.archive_correspondence(SimpleNamespace(sha=lambda path: 'changed-tool'), archive, directory)

    def test_alias_is_only_build_change_and_core_ownership_guards_identical(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'source-snapshot'
            (root / 'support-v2/ponytest').mkdir(parents=True)
            (root / 'support-v2/ponytest/aliases.pony').write_text('fixture alias')
            source = root / 'sources/pidigits/compact/main.pony'
            old_command = old.build_command(Path('/compiler'), Path('/sdk'), Path('/regex'), Path('/gmp'),
                                            source, Path('/destination'), 'fixture')
            new_command = new.build_command(Path('/compiler'), Path('/sdk'), Path('/regex'), Path('/gmp'),
                                            source, Path('/destination'), 'fixture')
            offset = new_command.index(str(root / 'support-v2'))
            self.assertEqual(new_command[offset-1:offset+1], ['--path', str(root / 'support-v2')])
            self.assertEqual(new_command[:offset-1] + new_command[offset+1:], old_command)
        before = Path(old.__file__).read_text()
        after = Path(new.__file__).read_text()
        for function in ('owned_command', 'native_images', 'probe', 'inputs', 'download', 'extract', 'main'):
            def code(text):
                node = next(node for node in ast.parse(text).body if isinstance(node, ast.FunctionDef) and node.name == function)
                return ast.get_source_segment(text, node)
            self.assertEqual(code(before), code(after), function)


if __name__ == '__main__':
    unittest.main()
