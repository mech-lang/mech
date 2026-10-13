"""Mock the fresh reuse boundary and independent pi trace failures."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import guarded_prepare_v5 as preparation
import library_compat_v5 as compatibility
import proof_reuse_v5 as reuse
import source_proofs_v5 as proofs
import pi_work_v5 as pi


def mock_report():
    report = dict(status='stopped-unqualified', active=None, proposal_sha256=reuse.PRIOR_SHA,
        error_type='AssertionError', error='pure-bigint-abi', full_performance_runs=0,
        chart_admission=False, reused_bounded_cases=106, reused_cases_relabelled_as_attempts=False, attempts=[])
    import references
    for identifier in sorted(reuse.NEW_PROFILES):
        benchmark = identifier.split('-')[1]
        cases = (reuse.old.PENDING_REUSED_CASES[identifier] if identifier == 'nim-knucleotide-compact'
                 else preparation.preflight_inputs(references)[benchmark])
        fingerprint = dict(binary_sha256='binary', source_sha256='source',
            command=['/old/' + identifier, '{n}'], cwd='/old', environment={}, support={})
        for case in cases:
            n = int(case[2:]) if benchmark in preparation.NUMERIC else 10
            command = [fingerprint['command'][0], str(n)]
            report['attempts'].append(dict(attempt=len(report['attempts']) + 1, id=identifier,
                benchmark=benchmark, case=case, qualified=True, qualification_checks={k: True for k in reuse.old.EXPECTED_GATES},
                full_workload=False, full_performance_run=False, preparation_only=True, never_chart_eligible=True,
                exit_status=0, seconds=0.25, sampler_wait_status=0, pins_unchanged_at_end=True, valid_environment=True,
                validation=dict(reference_match=True, bytes=7, sha256='output'), stdout_bytes=7, stdout_sha256='output',
                execution_fingerprint=json.dumps(fingerprint), command=command, workload_n=n,
                source_sha256='source', runtime_environment={}, cwd='/old',
                supervised_native_group=dict(command=command, inherited_common_mutex=True, pid=1234, pgid=1234),
                power_at_start=dict(source='AC Power', low_power_mode=0), power_at_end=dict(source='AC Power', low_power_mode=0),
                competing_compilers=[], power_changes=[], timeout_seconds=120, maximum_rss_mib=10240, peak_process_group_rss_kib=0))
    report['attempts'].append(dict(attempt=49, id='pure-bigint-abi', case='contract', qualified=False,
        exit_status=1, stderr='initBigInt(-17) div b == initBigInt(-3)'))
    return report


class ReuseBoundary(unittest.TestCase):
    def test_all_successes_retained_and_failure_not_admitted(self):
        report = mock_report()
        original = copy.deepcopy(report)
        groups = reuse.validate_records(report, preparation)
        self.assertEqual(sum(len(g['cases']) for g in groups.values()), 48)
        self.assertNotIn('pure-bigint-abi', groups)
        self.assertEqual(report, original)

    def test_missing_duplicate_or_changed_fingerprint_rejected(self):
        for mutate in (lambda r: r['attempts'].pop(0),
                       lambda r: r['attempts'].__setitem__(1, copy.deepcopy(r['attempts'][0])),
                       lambda r: r['attempts'][0].__setitem__('source_sha256', 'changed')):
            report = mock_report()
            mutate(report)
            with self.assertRaises(AssertionError):
                reuse.validate_records(report, preparation)

    def test_direct_resource_power_status_and_group_failures_rejected(self):
        for key, value in (('seconds', 0), ('seconds', float('nan')), ('power_at_end', {}),
                           ('monitor_errors', ['bad']), ('sampler_wait_status', 1), ('qualified', False),
                           ('stdout_sha256', 'wrong'), ('workload_n', 999), ('attempt', 99)):
            report = mock_report()
            report['attempts'][0][key] = value
            with self.assertRaises(AssertionError):
                reuse.validate_records(report, preparation)
        for key, value in (('pgid', 999), ('inherited_common_mutex', False), ('command', ['wrong'])):
            report = mock_report()
            report['attempts'][0]['supervised_native_group'][key] = value
            with self.assertRaises(AssertionError):
                reuse.validate_records(report, preparation)

    def test_failed_primitive_stays_failed(self):
        for key, value in (('qualified', True), ('exit_status', 0), ('stderr', 'different error')):
            report = mock_report()
            report['attempts'][-1][key] = value
            with self.assertRaises(AssertionError):
                reuse.validate_records(report, preparation)

    def test_signed_library_contract_matches_documented_floor_and_separate_gmp_truncation(self):
        self.assertIn('initBigInt(-17) div b == initBigInt(-4)', proofs.BIGINT_ABI)
        self.assertIn('initBigInt(-17) mod b == initBigInt(3)', proofs.BIGINT_ABI)
        self.assertIn('setUi(expected, 3)', proofs.GMP_ABI)
        self.assertEqual(compatibility.UNIONS, reuse.compatibility.UNIONS)


class ActualPiTrace(unittest.TestCase):
    def trace(self, n):
        q, r, t, k, printed = 1, 0, 1, 1, 0
        lines = [f'PI_NIM1|compact|{n}']
        while printed < n:
            lines.append(f'STATE|{q}|{r}|{t}|{k}|{printed}')
            lower, upper = 3 * q + r, 4 * q + r
            d3, d4 = lower // t, upper // t
            lines.append(f'DIV|{lower}|{d3}|{upper}|{d4}')
            if d3 == d4:
                q, r, printed = q * 10, (r - d3 * t) * 10, printed + 1
            else:
                q, r, t, k = q * k, (2 * q + r) * (2 * k + 1), t * (2 * k + 1), k + 1
        return lines + [f'END_PI_NIM1|{printed}']

    def test_both_source_copies_keep_actual_divisions_and_unique_anchors(self):
        for variant in ('compact', 'optimized'):
            source = (preparation.HERE / 'sources/pidigits' / (variant + '.nim')).read_text()
            transformed = pi.copy(source, variant)
            self.assertEqual(transformed.count('"STATE|"'), 1)
            self.assertEqual(transformed.count('"DIV|"'), 1)
            if variant == 'compact':
                self.assertIn('let digit3 = numerator3 div t\n  let digit4 = numerator4 div t', transformed)
            else:
                self.assertIn('divide(digit3, numerator3, t)\n  divide(digit4, numerator4, t)', transformed)
            with self.assertRaises(AssertionError):
                pi.copy(transformed, variant)

    def test_all_candidate_operands_and_rejected_steps_reconcile(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'pi.trace'
            for n in (0, 1, 9, 10, 11, 27, 97):
                path.write_text('\n'.join(self.trace(n)) + '\n')
                result = pi.reconcile(path, n, 'compact')
                self.assertEqual(result['accepted_candidates'], n)
                if n:
                    self.assertGreater(result['rejected_candidates'], 0)

    def test_missing_wrong_or_negative_operands_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'pi.trace'
            for mutate in (lambda lines: lines.pop(2),
                           lambda lines: lines.__setitem__(2, 'DIV|3|0|4|4'),
                           lambda lines: lines.__setitem__(1, 'STATE|1|-4|1|1|0'),
                           lambda lines: lines.__setitem__(-1, 'END_PI_NIM1|999')):
                lines = self.trace(11)
                mutate(lines)
                path.write_text('\n'.join(lines) + '\n')
                with self.assertRaises(AssertionError):
                    pi.reconcile(path, 11, 'compact')


if __name__ == '__main__':
    unittest.main()
