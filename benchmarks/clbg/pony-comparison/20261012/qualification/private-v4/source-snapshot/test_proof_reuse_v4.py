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
import guarded_prepare_v4 as new
import proof_reuse_v4 as reuse


def mock_report():
    report = dict(status='stopped-unqualified', full_performance_runs=0, chart_admission=False, attempts=[], active=None, proposal_sha256=reuse.PRIOR_SHA)
    for identifier in sorted(reuse.NEW_PROFILES):
        benchmark = identifier.split('-')[1]
        fingerprint = dict(binary_sha256='exact-old-binary', source_sha256='exact-old-source',
            command=['/old-bin/' + identifier, '{n}', '--ponymaxthreads', '8'],
            cwd='/old-cwd', environment={'DYLD_LIBRARY_PATH': '/old-library'}, support={})
        for case in sorted(reuse.CASES[benchmark]):
            n = int(case[2:])
            command = [fingerprint['command'][0], str(n), '--ponymaxthreads', '8']
            report['attempts'].append(dict(id=identifier, benchmark=benchmark, case=case,
                qualified=True, qualification_checks={key: True for key in reuse.EXPECTED_GATES},
                full_workload=False, full_performance_run=False, preparation_only=True,
                never_chart_eligible=True, exit_status=0, sampler_wait_status=0,
                pins_unchanged_at_end=True, valid_environment=True,
                validation=dict(reference_match=True, bytes=7, sha256='old-output'),
                stdout_bytes=7, stdout_sha256='old-output', execution_fingerprint=json.dumps(fingerprint),
                command=command, workload_n=n, supervised_native_group=dict(command=command,
                    inherited_common_mutex=True, pid=1234, pgid=1234), source_sha256='exact-old-source',
                runtime_environment=fingerprint['environment'], cwd=fingerprint['cwd'], seconds=0.25,
                power_at_start=dict(source='AC Power', low_power_mode=0), power_at_end=dict(source='AC Power', low_power_mode=0),
                competing_compilers=[], power_changes=[], timeout_seconds=120, maximum_rss_mib=10240,
                peak_process_group_rss_kib=0, attempt=len(report['attempts']) + 1))
    return report


class ExactOldProofChecks(unittest.TestCase):
    def setUp(self):
        self.report = mock_report()

    def reject(self, mutate):
        report = copy.deepcopy(self.report)
        mutate(report)
        with self.assertRaises(AssertionError):
            reuse.validate_records(report)

    def test_seventy_eight_mock_records_preserve_eight_fingerprints(self):
        original = copy.deepcopy(self.report)
        groups = reuse.validate_records(self.report)
        self.assertEqual(set(groups), reuse.NEW_PROFILES)
        self.assertEqual(sum(len(group['cases']) for group in groups.values()), 78)
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
        self.reject(lambda report: report.__setitem__('active', {'pid': 1}))
        self.reject(lambda report: report['attempts'][0]['supervised_native_group'].__setitem__('pgid', 999))

    def test_failed_output_or_execution_or_integrity_rejected(self):
        for field, value in (('exit_status', 1), ('sampler_wait_status', 1), ('pins_unchanged_at_end', False),
                             ('valid_environment', False), ('stdout_sha256', 'wrong'),
                             ('resource_violation', 'rss'), ('monitor_errors', ['failure']), ('seconds', float('nan')),
                             ('seconds', 0), ('power_at_end', {}), ('competing_compilers', ['compiler']),
                             ('power_changes', ['battery']), ('peak_process_group_rss_kib', 10485761),
                             ('sampler_error', 'failure'), ('registration_error', 'failure'),
                             ('supervisor_error', 'failure'), ('monitor_finish_error', 'failure'),
                             ('attempt', 999)):
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
        identifier = sorted(reuse.NEW_PROFILES)[0]
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
            self.assertEqual(code(before), code(after).replace("('empty-input', b'')", "('empty', b'')") if function == 'inputs' else code(after), function)

    def test_numeric_label_must_match_actual_input(self):
        self.reject(lambda report: report['attempts'][0].__setitem__('workload_n', 999))

    def test_four_source_copies_change_only_explicit_parentheses(self):
        replacements = {
            ('knucleotide', 'compact'): [
                ('sequence.size() - width + 1', '(sequence.size() - width) + 1'),
                ('100.0 * entry.count.f64() / total.f64()', '(100.0 * entry.count.f64()) / total.f64()')],
            ('knucleotide', 'optimized'): [
                ('100.0 * entry.count.f64() / total.f64()', '(100.0 * entry.count.f64()) / total.f64()')],
            ('binarytrees', 'compact'): [
                ('USize(1) << (_maximum - _depth + 4)', 'USize(1) << ((_maximum - _depth) + 4)')],
            ('binarytrees', 'optimized'): [
                ('USize(1) << (_maximum - _depth + 4)', 'USize(1) << ((_maximum - _depth) + 4)')],
        }
        for (benchmark, variant), changes in replacements.items():
            before = (new.HERE / 'sources' / benchmark / variant / 'main.pony').read_text()
            expected = before
            for old_text, new_text in changes:
                self.assertEqual(expected.count(old_text), 1)
                expected = expected.replace(old_text, new_text)
            after = (new.HERE / 'revisions/precedence-v4/sources' / benchmark / variant / 'main.pony').read_text()
            self.assertEqual(after, expected)

    def test_profile_revision_is_exactly_four_new_sources_and_eight_fresh_builds(self):
        original_profiles = json.loads((new.HERE / 'profiles.json').read_text())['profiles']
        current = json.loads((new.HERE / 'profiles-v4.json').read_text())['profiles']
        self.assertEqual(len(current), 20)
        changed = 0
        for before, after in zip(original_profiles, current, strict=True):
            if before['benchmark'] in ('knucleotide', 'binarytrees'):
                changed += 1
                self.assertEqual(after['prior_id'], before['id'])
                self.assertEqual(after['id'], before['id'] + '-precedence-v4')
                self.assertEqual(after['source'], 'revisions/precedence-v4/' + before['source'])
                self.assertEqual(after['revision'], 'explicit-left-associative-parentheses-v4')
                stripped = {key: value for key, value in after.items() if key not in ('prior_id', 'revision')}
                stripped.update(id=before['id'], source=before['source'])
                self.assertEqual(stripped, before)
            else:
                self.assertEqual(after, before)
        self.assertEqual(changed, 4)
        reused = {row['id'] for row in current} & reuse.PROFILES
        self.assertEqual(len(reused), 12)
        self.assertEqual(len(current) - len(reused), 8)

    def test_all_case_labels_preflight_before_any_native_work(self):
        import references
        from unittest.mock import patch
        inventory = new.preflight_inputs(references)
        self.assertEqual(len(inventory), 10)
        self.assertEqual(len(inventory['knucleotide']), 12)
        self.assertIn('empty', inventory['knucleotide'])
        self.assertIn('empty-input', inventory['knucleotide'])
        data = dict(new.inputs('knucleotide', references))
        self.assertEqual(data['empty-input'], b'')
        self.assertEqual(data['empty'], b'>THREE\n>FOUR\nGGG\n')
        for cases in ([('same', b'a'), ('same', b'b')], [('', b'a')], [('unsafe/name', b'a')], [('word', 'text')]):
            with patch.object(new, 'inputs', return_value=cases), self.assertRaises(AssertionError):
                new.preflight_inputs(references)


if __name__ == '__main__':
    unittest.main()
