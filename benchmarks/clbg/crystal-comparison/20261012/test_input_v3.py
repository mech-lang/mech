"""Ten proposed metadata/reuse mocks; no native compiler or benchmark."""
import ast
from copy import deepcopy
import fcntl
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import guarded_prepare_v3 as preparation
import proof_reuse_v3 as reuse

FD = None


class References:
    @staticmethod
    def dna_fixtures():
        return tuple((name, b'>THREE\nACGT\n' if name != 'empty' else b'>THREE\n>FOUR\nGGG\n')
            for name in ('ties-and-stop', 'one-base', 'empty', 'missing', 'crlf-comment-eof',
                         'embedded-header-comment', 'arbitrary-widths'))


def prior_fixture():
    report = dict(status='stopped-unqualified', active=None, error_type='FileExistsError',
        proposal_sha256=reuse.PRIOR_SHA,
        error='Existing directory /old/crystal-knucleotide-compact/empty',
        full_performance_runs=0, chart_admission=False, never_chart_eligible=True, attempts=[])
    for identity in sorted(reuse.PROFILES):
        benchmark = next(b for b in reuse.CASES if identity.startswith('crystal-' + b + '-'))
        command = ['/old/bin/' + identity, '{n}']
        environment = {'LC_ALL': 'C', 'CRYSTAL_WORKERS': '8'}
        fingerprint = dict(source_sha256='a' * 64, binary_sha256='b' * 64,
            command=command, environment=environment, cwd='/old', support={})
        for case in sorted(reuse.CASES[benchmark]):
            n = int(case[2:]) if benchmark in reuse.NUMERIC else 1000
            number = len(report['attempts']) + 1
            expanded = [command[0], str(n)]
            report['attempts'].append(dict(id=identity, attempt=number, benchmark=benchmark, case=case,
                qualified=True, qualification_checks={key: True for key in reuse.EXPECTED_GATES},
                full_workload=False, full_performance_run=False, preparation_only=True, never_chart_eligible=True,
                exit_status=0, sampler_wait_status=0, pins_unchanged_at_end=True, valid_environment=True,
                power_at_start=dict(source='AC Power', low_power_mode=0),
                power_at_end=dict(source='AC Power', low_power_mode=0),
                competing_compilers=[], power_changes=[], timeout_seconds=300, maximum_rss_mib=10240,
                peak_process_group_rss_kib=1024, validation=dict(reference_match=True, bytes=3, sha256='d' * 64),
                seconds=0.01,
                stdout_bytes=3, stdout_sha256='d' * 64, execution_fingerprint=json.dumps(fingerprint, sort_keys=True),
                command=expanded, workload_n=n, source_sha256='a' * 64,
                runtime_environment=environment, cwd='/old', supervised_native_group=dict(
                    command=expanded, inherited_common_mutex=True, pid=1000 + number, pgid=1000 + number)))
    return report


class InputReuseChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the common-lock standalone fixture entry point')

    def test_preflight_preserves_old_empty_record_and_new_zero_byte_case(self):
        cases = preparation.preflight_inputs(References)
        self.assertEqual(dict(cases['knucleotide'])['empty'], b'>THREE\n>FOUR\nGGG\n')
        self.assertEqual(dict(cases['knucleotide'])['empty-input'], b'')
        self.assertEqual(len(cases['knucleotide']), 11)
        self.assertEqual(len(cases['revcomp']), len(cases['regexredux']))

    def test_preflight_rejects_duplicate_namespace_before_any_native_launch(self):
        original = preparation.inputs
        def duplicate(benchmark, references):
            cases = original(benchmark, references)
            return cases + (cases[0],) if benchmark == 'regexredux' else cases
        with patch.object(preparation, 'inputs', side_effect=duplicate), patch.object(preparation.subprocess, 'Popen') as native:
            with self.assertRaises(AssertionError):
                preparation.preflight_inputs(References)
            native.assert_not_called()
        with patch.dict(preparation.NUMERIC, {'nbody': (0, 0)}), patch.object(preparation.subprocess, 'Popen') as native:
            with self.assertRaises(AssertionError):
                preparation.preflight_inputs(References)
            native.assert_not_called()

    def test_preflight_rejects_unsafe_names_and_nonbyte_payload(self):
        for bad in (('../escape', b'x'), ('valid', 'text')):
            with self.subTest(bad=bad), patch.object(preparation, 'inputs', return_value=(bad,)):
                with self.assertRaises(AssertionError):
                    preparation.preflight_inputs(References)

    def test_exact_fifteen_profiles_and_112_prior_cases_are_reusable(self):
        report = prior_fixture()
        before = deepcopy(report)
        groups = reuse.validate_records(report)
        self.assertEqual(set(groups), reuse.PROFILES)
        self.assertEqual(sum(len(g['cases']) for g in groups.values()), 112)
        self.assertEqual(report, before)
        self.assertEqual(set(groups['crystal-knucleotide-compact']['cases']), reuse.CASES['knucleotide'])

    def test_missing_duplicate_wrong_profile_and_size_are_rejected(self):
        for mutation in (
            lambda r: r['attempts'].pop(),
            lambda r: r['attempts'].__setitem__(-1, deepcopy(r['attempts'][-2])),
            lambda r: r['attempts'][0].update(id='crystal-unbuilt-compact'),
            lambda r: next(a for a in r['attempts'] if a['benchmark'] == 'nbody').update(workload_n=7),
        ):
            report = prior_fixture()
            mutation(report)
            with self.assertRaises(AssertionError):
                reuse.validate_records(report)

    def test_missing_invented_false_and_truthy_gate_maps_rejected(self):
        for changes in ({}, {'invented': True}, dict.fromkeys(reuse.EXPECTED_GATES, 1),
                        dict.fromkeys(reuse.EXPECTED_GATES, 'true'), dict.fromkeys(reuse.EXPECTED_GATES, False)):
            report = prior_fixture()
            report['attempts'][0]['qualification_checks'] = changes
            with self.assertRaises(AssertionError):
                reuse.validate_records(report)

    def test_native_error_scored_probe_and_changed_fingerprint_are_rejected(self):
        for mutation in (
            lambda a: a.update(error='native failure'),
            lambda a: a.update(full_workload=True),
            lambda a: a.update(full_performance_run=True),
            lambda a: a['supervised_native_group'].update(inherited_common_mutex=False),
            lambda a: a['supervised_native_group'].update(pgid=7),
            lambda a: a.update(source_sha256='c' * 64),
            lambda a: a.update(power_at_end=dict(source='Battery Power', low_power_mode=0)),
        ):
            report = prior_fixture()
            mutation(report['attempts'][0])
            with self.assertRaises(AssertionError):
                reuse.validate_records(report)
        ledger = prior_fixture()['attempts'][0]
        reuse.verify_saved_observations(deepcopy(ledger), deepcopy(ledger), ledger)
        for field, value in (('seconds', 0.02), ('power_at_end', dict(source='Battery Power', low_power_mode=0)),
                             ('peak_process_group_rss_kib', 2048), ('monitor_finish_error', 'retained raw failure')):
            with self.subTest(raw_field=field):
                raw = deepcopy(ledger)
                raw[field] = value
                with self.assertRaises(AssertionError):
                    reuse.verify_saved_observations(raw, deepcopy(ledger), ledger)
        supervisor = deepcopy(ledger)
        supervisor['supervised_native_group'] = dict(supervisor['supervised_native_group'], pid=7, pgid=7)
        with self.assertRaises(AssertionError):
            reuse.verify_saved_observations(deepcopy(ledger), supervisor, ledger)

    def test_new_source_or_binary_cannot_inherit_old_executions(self):
        fingerprint = dict(source_sha256='a' * 64, binary_sha256='b' * 64)
        reuse.require_identity(fingerprint, 'a' * 64, 'b' * 64)
        with self.assertRaises(AssertionError):
            reuse.require_identity(fingerprint, 'c' * 64, 'b' * 64)
        with self.assertRaises(AssertionError):
            reuse.require_identity(fingerprint, 'a' * 64, 'c' * 64)

    def test_five_fresh_builds_and_44_fresh_output_cases_required(self):
        full_ids = {f'crystal-{benchmark}-{variant}' for benchmark in preparation.BENCHMARKS for variant in ('compact', 'optimized')}
        fresh_ids = full_ids - reuse.PROFILES
        self.assertEqual(fresh_ids, {'crystal-knucleotide-optimized', 'crystal-regexredux-compact',
            'crystal-regexredux-optimized', 'crystal-pidigits-compact', 'crystal-pidigits-optimized'})
        cases = preparation.preflight_inputs(References)
        new_k_compact = {name for name, data in cases['knucleotide']} - reuse.CASES['knucleotide']
        self.assertEqual(new_k_compact, {'empty-input', 'exact-token-vtab', 'tail'})
        self.assertEqual(len(new_k_compact) + len(cases['knucleotide']) + 2 * len(cases['regexredux']) + 2 * len(preparation.NUMERIC['pidigits']), 44)

    def test_proved_guard_and_trace_logic_remain_exact(self):
        texts = [(preparation.HERE / name).read_text() for name in ('guarded_prepare_v2.py', 'guarded_prepare_v3.py')]
        functions = [{node.name: ast.get_source_segment(text, node) for node in ast.parse(text).body
                      if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))} for text in texts]
        for name in ('owned_command', 'probe', 'output_validation', 'build_command', 'native_images',
                     'merge', 'sha', 'load_guard', 'new_json', 'download', 'extract', 'pin_tree', 'package_layout', 'main'):
            with self.subTest(name=name):
                self.assertEqual(functions[0][name], functions[1][name])
        old = (preparation.HERE / 'source_proofs.py').read_text()
        new = (preparation.HERE / 'source_proofs_v3.py').read_text()
        self.assertEqual(old.count("[build / 'release', preparation.GMP, preparation.PCRE2]"), 2)
        self.assertEqual(new, old.replace("[build / 'release', preparation.GMP, preparation.PCRE2]",
                                        "[preparation.retained_release(), preparation.GMP, preparation.PCRE2]"))


def main():
    global FD
    print('WAIT common mutex: Crystal input-label-v3 reuse/preflight mocks', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        FD = lock.fileno()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(InputReuseChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = preparation.HERE / 'input-v3-fixtures.log'
        assert not log.exists(), 'No overwrite or automatic fixture retry'
        with log.open('x') as stream:
            stream.write(text)
        preparation.new_json(preparation.HERE / 'input-v3-fixtures.json', dict(
            status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), native_probes=0,
            actual_signals_sent=0, never_chart_eligible=True, source_revision='input-label-v3',
            prior112cases_reexecuted=False, prior24plus8fixtures_reexecuted=False,
            shared_timing_lock=str(preparation.LOCK),
            pins={str(path): preparation.sha(path) for path in (
                Path(__file__).resolve(), preparation.HERE / 'guarded_prepare_v3.py',
                preparation.HERE / 'guarded_prepare_v2.py', preparation.HERE / 'proof_reuse_v3.py',
                preparation.HERE / 'source_proofs.py', preparation.HERE / 'source_proofs_v3.py',
                preparation.HERE / 'INPUT_V3.md', log)}))
        if not result.wasSuccessful() or result.testsRun != 10:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
