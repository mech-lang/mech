"""Mocked exact-proof/library compatibility gates. No production binary or native signal runs."""
import ast
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
from types import SimpleNamespace
import unittest

import guarded_prepare_v2 as old
import guarded_prepare_v3 as new
import proof_reuse_v3 as reuse
import library_compat_v3 as compatibility


def mock_report():
    report = dict(status='stopped-unqualified', full_performance_runs=0, chart_admission=False, attempts=[])
    for identifier in sorted(reuse.PROFILES):
        benchmark = identifier.split('-')[1]
        fingerprint = dict(binary_sha256='exact-old-binary', source_sha256='exact-old-source',
            command=['/old-bin/' + identifier, '{n}'],
            cwd='/old-cwd', environment={'DYLD_LIBRARY_PATH': '/old-library'}, support={})
        for case in sorted(reuse.CASES[benchmark]):
            n = int(case[2:]) if benchmark == 'pidigits' else 30
            command = [fingerprint['command'][0], str(n)]
            report['attempts'].append(dict(id=identifier, benchmark=benchmark, case=case,
                qualified=True, qualification_checks={key: True for key in reuse.EXPECTED_GATES},
                full_workload=False, full_performance_run=False, preparation_only=True,
                never_chart_eligible=True, exit_status=0, sampler_wait_status=0,
                pins_unchanged_at_end=True, valid_environment=True,
                validation=dict(reference_match=True, bytes=7, sha256='old-output'),
                stdout_bytes=7, stdout_sha256='old-output', execution_fingerprint=json.dumps(fingerprint),
                command=command, workload_n=n, supervised_native_group=dict(command=command,
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

    def test_twenty_nine_mock_records_preserve_five_fingerprints(self):
        original = copy.deepcopy(self.report)
        groups = reuse.validate_records(self.report)
        self.assertEqual(set(groups), reuse.PROFILES)
        self.assertEqual(sum(len(group['cases']) for group in groups.values()), 29)
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

    def test_exact_command_and_inherited_lock_are_required(self):
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__(
            'inherited_common_mutex', False))
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__(
            'command', ['/unrelated-native']))
        def wrong_cap(report):
            fingerprint = json.loads(report['attempts'][0]['execution_fingerprint'])
            fingerprint['command'][-1] = '{unapproved-argument}'
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
        prior['command'] = ['/newly-recompiled-binary', '{n}']
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

    def test_build_flags_and_core_ownership_guards_identical(self):
        before = Path(old.__file__).read_text()
        after = Path(new.__file__).read_text()
        for function in ('build_command', 'owned_command', 'probe', 'inputs', 'download', 'extract', 'main'):
            def code(text):
                node = next(node for node in ast.parse(text).body if isinstance(node, ast.FunctionDef) and node.name == function)
                return ast.get_source_segment(text, node)
            self.assertEqual(code(before), code(after), function)


class UnionCompatibilityChecks(unittest.TestCase):
    def test_both_declarations_move_only_the_union_pragma(self):
        for filename, (before, after) in compatibility.UNIONS.items():
            source = 'unchanged-prefix\n' + before + '\n    mp_lc*: pointer\nunchanged-suffix\n'
            normalized = compatibility.normalize(source, filename)
            self.assertEqual(normalized, 'unchanged-prefix\n' + after + '\n    mp_lc*: pointer\nunchanged-suffix\n')
            self.assertEqual(normalized.replace(after, before), source)

    def test_missing_duplicate_or_already_patched_declaration_rejected(self):
        for filename, (before, after) in compatibility.UNIONS.items():
            for source in ('unrelated object', before + before, after):
                with self.assertRaises(AssertionError):
                    compatibility.normalize(source, filename)

    def make_wrapper(self, root):
        build = root / 'private-v3'
        wrapper = build / 'nim-gmp/public-pinned-wrapper'
        for filename, (before, after) in compatibility.UNIONS.items():
            path = wrapper / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('upstream-prefix\n' + before + '\n    mp_lc*: pointer\nupstream-suffix\n')
        return build, wrapper

    def test_fresh_private_copy_has_original_plan_and_changed_copy_receipts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            build, wrapper = self.make_wrapper(root)
            original = {filename: (wrapper / filename).read_bytes() for filename in compatibility.UNIONS}
            preparation = SimpleNamespace(sha=new.sha, new_json=new.new_json)
            output = root / 'receipts/compatibility-complete.json'
            pins = compatibility.patch(preparation, wrapper, build, output)
            receipt = json.loads(output.read_text())
            self.assertEqual(len(receipt['files']), 2)
            self.assertTrue((output.parent / 'compatibility-plan.json').exists())
            for row in receipt['files']:
                self.assertEqual(new.sha(row['path']), row['changed_sha256'])
                self.assertEqual(new.sha(row['original_copy']), row['original_sha256'])
                self.assertFalse(row['arithmetic_changed'])
                self.assertFalse(row['ffi_symbols_types_argument_order_changed'])
            self.assertEqual((output.parent / 'original-header.nim').read_bytes(), original['src/gmp/header.nim'])
            self.assertIn(str(output), pins)
            with self.assertRaises(AssertionError):
                compatibility.patch(preparation, wrapper, build, output)

    def test_second_invalid_anchor_or_old_revision_never_mutates_first(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            build, wrapper = self.make_wrapper(root)
            original = (wrapper / 'src/gmp/header.nim').read_bytes()
            (wrapper / 'src/gmp/pure.nim').write_text('unexpected public source')
            preparation = SimpleNamespace(sha=new.sha, new_json=new.new_json)
            with self.assertRaises(AssertionError):
                compatibility.patch(preparation, wrapper, build, root / 'receipts/compatibility-complete.json')
            self.assertEqual((wrapper / 'src/gmp/header.nim').read_bytes(), original)
            with self.assertRaises(AssertionError):
                compatibility.patch(preparation, wrapper, root / 'private-v2', root / 'other/compatibility-complete.json')
            self.assertEqual((wrapper / 'src/gmp/header.nim').read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
