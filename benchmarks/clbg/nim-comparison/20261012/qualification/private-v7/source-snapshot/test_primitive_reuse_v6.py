"""Meaningful corruption fixtures for the three successful primitive boundary."""
import copy
import json
import unittest

import primitive_reuse_v6 as reuse


def mock_report():
    report = dict(status='stopped-unqualified',active=None,proposal_sha256=reuse.PRIOR_SHA,
        error_type='AssertionError',error="('Owned command failed', 'build', 1)",full_performance_runs=0,
        chart_admission=False,never_chart_eligible=True,no_automatic_retry=True,
        reused_bounded_cases=154,reused_cases_relabelled_as_attempts=False,attempts=[])
    for number,identifier in enumerate(reuse.IDS,1):
        fingerprint = dict(binary_sha256='binary',source_sha256='source',command=[str(reuse.BUILD / 'bin' / identifier),'{n}'],
            cwd=str(reuse.HERE),environment={},support={})
        command = [fingerprint['command'][0],'0']
        report['attempts'].append(dict(id=identifier,key='pidigits:'+identifier,attempt=number,case='contract',
            benchmark='pidigits',language='nim',workload_n=0,full_workload=False,full_performance_run=False,
            preparation_only=True,never_chart_eligible=True,qualified=True,sampler_wait_status=0,
            exit_status=0,seconds=0.25,power_at_start=dict(source='AC Power',low_power_mode=0),
            power_at_end=dict(source='AC Power',low_power_mode=0),competing_compilers=[],power_changes=[],
            timeout_seconds=120,maximum_rss_mib=10240,peak_process_group_rss_kib=0,valid_environment=True,pins_unchanged_at_end=True,
            qualification_checks={key:True for key in reuse.production.old.EXPECTED_GATES},
            source_sha256='source',execution_fingerprint=json.dumps(fingerprint,sort_keys=True),runtime_environment={},
            cwd=str(reuse.HERE),command=command,supervised_native_group=dict(type='native-group-started',command=command,
                inherited_common_mutex=True,pid=1234,pgid=1234),stdout_bytes=7,stdout_sha256='output',
            validation=dict(reference_match=True,bytes=7,sha256='output')))
    return report


class PrimitiveReuse(unittest.TestCase):
    def test_exact_three_prior_records_retained_without_relabelling(self):
        report=mock_report();original=copy.deepcopy(report)
        records=reuse.validate_records(report)
        self.assertEqual([r['id'] for r in records],reuse.IDS)
        self.assertEqual(report,original)
        records[0]['attempt']=99
        self.assertEqual(report,original)

    def test_missing_duplicate_changed_identity_or_nonboolean_gate_rejected(self):
        def nonbool(r): r['attempts'][0]['qualification_checks'][next(iter(reuse.production.old.EXPECTED_GATES))]=1
        for mutate in (lambda r:r['attempts'].pop(),lambda r:r['attempts'].__setitem__(1,copy.deepcopy(r['attempts'][0])),
            lambda r:r['attempts'][0].__setitem__('source_sha256','changed'),nonbool,
            lambda r:r['attempts'][0]['qualification_checks'].pop(next(iter(reuse.production.old.EXPECTED_GATES)))):
            report=mock_report();mutate(report)
            with self.assertRaises(AssertionError): reuse.validate_records(report)

    def test_direct_native_ownership_environment_and_observation_corruptions_rejected(self):
        for key,value in (('seconds',0),('seconds',float('nan')),('qualified',False),('sampler_wait_status',1),
            ('stdout_sha256','changed'),('workload_n',1),('power_at_end',{}),('monitor_errors',['bad']),('attempt',99)):
            report=mock_report();report['attempts'][0][key]=value
            with self.assertRaises(AssertionError): reuse.validate_records(report)
        for key,value in (('pgid',999),('inherited_common_mutex',False),('command',['wrong'])):
            report=mock_report();report['attempts'][0]['supervised_native_group'][key]=value
            with self.assertRaises(AssertionError): reuse.validate_records(report)

    def test_only_unscored_reserved_diagnostic_identifier_changes(self):
        reuse.diagnostic_correction()
        self.assertNotIn('distinct',reuse.corrected.ORC_OWNERSHIP)
        self.assertEqual(reuse.corrected.ORC_OWNERSHIP.count('threadIds'),3)
        self.assertEqual(reuse.original.BIGINT_ABI,reuse.corrected.BIGINT_ABI)


if __name__=='__main__': unittest.main(verbosity=2)
