"""Corruption fixtures only; synthetic executions are never measured/admitted."""
from copy import deepcopy
import fcntl
import hashlib
import json
from pathlib import Path
import statistics
import unittest
from datetime import datetime, timezone

import reconcile_native_records_v1 as subject


def fixture(language='nim', benchmark='spectralnorm', offset=0):
    output = b'1.274224153\n'
    reference = dict(bytes=len(output), sha256=hashlib.sha256(output).hexdigest())
    profiles = [dict(id=identity, key=benchmark + ':' + identity, benchmark=benchmark,
        language=lang, variant=variant, source='/fixture/' + identity + '.source',
        source_sha256=identity.ljust(64, '0'), binary_sha256='b' * 64,
        command=['/fixture/' + identity, '{n}'], cwd='/fixture',
        runtime_environment={}, loc=None) for identity, lang, variant in
        (('compact', language, 'compact'), ('optimized', language, 'optimized'),
         ('control', 'mech', 'control'))]
    identifiers = [row['id'] for row in profiles]
    orders = [dict(round=i+1, profiles=identifiers[i:] + identifiers[:i]) for i in range(3)]
    manifest = dict(benchmark=benchmark, samples=3, size=5500,
        canonical_full_workload=True, maximum_compute_workers=8,
        required_power_profile=subject.POWER, shared_timing_lock='/tmp/mech-clbg-timing.lock',
        maximum_rss_mib=10240, timeout_seconds=14400,
        full_output_reference=reference, profiles=profiles, orders=orders)
    summaries = {row['id']: dict(deepcopy(row), samples_seconds=[], measurement_status='measured')
                 for row in profiles}
    report = dict(status='complete', active=None, pins_unchanged_at_end=True,
        manifest_sha256='m' * 64, orders=deepcopy(orders), profiles=summaries, attempts=[])
    evidence = {}
    for order in orders:
        for identity in order['profiles']:
            row = next(row for row in profiles if row['id'] == identity)
            ordinal = offset + len(report['attempts']) + 1
            path = '/fixture/' + str(ordinal)
            command = [row['command'][0], '5500']
            raw = dict(id=identity, key=row['key'], benchmark=benchmark, language=row['language'],
                source_sha256=row['source_sha256'], cwd=row['cwd'], command=command,
                runtime_environment={}, execution_fingerprint=json.dumps(fingerprint(row)),
                exit_status=0, full_workload=True, workload_n=5500,
                qualified=True, valid_environment=True, pins_unchanged_at_end=True,
                maximum_rss_mib=10240, timeout_seconds=14400,
                peak_process_group_rss_kib=1024, seconds=float(order['round']),
                power_at_start=subject.POWER, power_at_end=subject.POWER,
                validation=dict(reference, reference_match=True),
                stdout_bytes=reference['bytes'], stdout_sha256=reference['sha256'],
                **{field: [] for field in subject.EMPTY})
            saved = dict(deepcopy(raw), attempt=ordinal, round=order['round'],
                sampler_wait_status=0, result_path=path, supervisor_result_path=path + '.supervisor',
                supervised_native_group=dict(type='native-group-started', pid=100+ordinal,
                    pgid=100+ordinal, command=command, inherited_common_mutex=True),
                qualification_checks={field: True for field in subject.CHECKS})
            evidence[path], evidence[path + '.supervisor'] = raw, saved
            report['attempts'].append(dict(deepcopy(saved), full_performance_run=True, chart_admission=False))
            summaries[identity]['samples_seconds'].append(raw['seconds'])
    for summary in summaries.values():
        summary['median_seconds'] = statistics.median(summary['samples_seconds'])
        summary['sample_stdev_seconds'] = statistics.stdev(summary['samples_seconds'])
    return manifest, report, reference, evidence


def fingerprint(row):
    return dict(source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                command=row['command'], cwd=row['cwd'], environment=row['runtime_environment'])


class Records(unittest.TestCase):
    def setUp(self):
        self.manifest, self.report, self.reference, self.evidence = fixture()

    def validate(self, language='nim', offset=0):
        return subject.validate_benchmark(self.manifest, self.report, 'm' * 64,
            language=language, canonical_n=5500, reference=self.reference,
            fingerprint=fingerprint, load_evidence=lambda path: deepcopy(self.evidence[path]),
            attempt_offset=offset)

    def reject(self):
        with self.assertRaises((ValueError, KeyError, TypeError)):
            self.validate()

    def test_complete_cohort_schemas_and_pony_global_ordinals(self):
        for language in subject.LANGUAGES:
            offset = 9 if language == 'pony' else 0
            self.manifest, self.report, self.reference, self.evidence = fixture(language, offset=offset)
            rows = self.validate(language, offset)
            self.assertEqual(len(rows), 3)
            self.assertTrue(all(row['eligible_for_chart'] is False for row in rows.values()))
            self.assertTrue(all(row['loc'] is None for row in rows.values()))
            self.assertTrue(all(row['median_seconds'] == 2 for row in rows.values()))

    def test_partial_and_failed_studies_have_no_score(self):
        for status in ('running', 'complete-with-failures', 'complete-with-source-failures'):
            self.report['status'] = status
            self.reject()
        self.report['status'] = 'complete'
        self.report['attempts'].pop()
        self.reject()

    def test_literal_gates_and_integer_native_exit_are_required(self):
        original = deepcopy(self.report)
        for field in subject.CHECKS:
            for false_value in (False, 1, None):
                self.report = deepcopy(original)
                self.report['attempts'][0]['qualification_checks'][field] = false_value
                self.reject()
        for field in ('exit_status', 'sampler_wait_status'):
            self.report = deepcopy(original)
            self.report['attempts'][0][field] = False
            self.reject()

    def test_canonical_work_source_and_command_cannot_change(self):
        original = deepcopy(self.report)
        for field, value in (('workload_n', 1000), ('preparation_only', True),
                             ('source_sha256', 'x' * 64), ('execution_fingerprint', '{}'),
                             ('command', ['/fixture/control', '1000'])):
            self.report = deepcopy(original)
            self.report['attempts'][0][field] = value
            self.reject()

    def test_process_and_full_output_claims_are_independent_gates(self):
        original = deepcopy(self.report)
        for field, value in (('inherited_common_mutex', False), ('pgid', 999), ('pgid', True)):
            self.report = deepcopy(original)
            self.report['attempts'][0]['supervised_native_group'][field] = value
            self.reject()
        self.report = deepcopy(original)
        self.report['attempts'][0]['validation']['sha256'] = 'x' * 64
        self.reject()

    def test_native_evidence_cannot_be_overwritten_by_parent(self):
        first = self.report['attempts'][0]
        self.evidence[first['result_path']]['seconds'] += 1
        self.reject()
        self.setUp()
        first = self.report['attempts'][0]
        self.evidence[first['supervisor_result_path']]['seconds'] += 1
        self.reject()
        self.setUp()
        self.report['attempts'][0]['chart_admission'] = True
        self.reject()

    def test_duplicate_slots_altered_summary_and_report_only_are_rejected(self):
        self.manifest['orders'][0]['profiles'][1] = self.manifest['orders'][0]['profiles'][0]
        self.report['orders'] = deepcopy(self.manifest['orders'])
        self.reject()
        self.setUp()
        self.report['profiles']['compact']['median_seconds'] += 1
        self.reject()
        self.setUp()
        self.manifest['never_chart_eligible'] = True
        self.reject()


if __name__ == '__main__':
    print('WAIT common mutex: new native-cohort independent record corruption fixtures', flush=True)
    with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        receipt_path = Path(__file__).with_name('native-records-v1-fixture-receipt.json')
        if receipt_path.exists():
            raise SystemExit('This fixture receipt already exists; preserve it instead of implicitly rerunning.')
        execution = unittest.main(verbosity=2, exit=False)
        result = execution.result
        receipt = dict(status='passed' if result.wasSuccessful() else 'failed-fixtures',
            tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors),
            exclusive_common_timing_mutex_held=True,
            source_pins={str(Path(path).resolve()): hashlib.sha256(Path(path).read_bytes()).hexdigest()
                         for path in (__file__, subject.__file__)},
            actual_native_processes=0, synthetic_fixture_records_never_admitted=True,
            source_pool_admission=False, chart_admission=False,
            completed_at=datetime.now(timezone.utc).isoformat())
        with receipt_path.open('x') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
        raise SystemExit(0 if result.wasSuccessful() else 1)
