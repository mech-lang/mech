"""Unexecuted strict six-process adapter fixtures; no native commands."""
from copy import deepcopy
import json
import statistics
import unittest
from unittest.mock import patch

import pending_minima_records as records


BASE = '/fixture/benchmarks/clbg'


def fixture(benchmark='mandelbrot'):
    study = records.STUDIES[benchmark]
    source = BASE + '/loc-normalization/20261012/pending-minima/' + benchmark + '/qualification-v1/candidate'
    runtime = '/fixture/runtime/' + study['language']
    original = BASE + '/' + study['source_suffix']
    control_source, control_binary = BASE + '/mech/current.mec', BASE + '/mech/current-aot'
    foreign = dict(id=study['identity'], key=benchmark + ':' + study['identity'], benchmark=benchmark,
        language=study['language'], source=source, source_sha256=study['source_sha256'],
        original_source=original, runtime=dict(path=runtime, sha256='c' * 64, version='fixture'),
        command=[runtime] + study['flags'] + [source, '{n}'], cwd=BASE,
        runtime_environment={'LC_ALL': 'C'}, maximum_workers=1, loc=None,
        role='Listed compact candidate', algorithm=dict(strict=True, native_benchmark_kernel=False),
        required_work_review=dict(status='passed', source_sha256=study['source_sha256']),
        small_validation=dict(reference_match=True), adversarial_validation=[dict(qualified=True, full_workload=False)])
    control = dict(id='mech-aot-control', key=benchmark + ':loc-minimum-study:mech-control',
        benchmark=benchmark, language='mech', source=control_source, source_sha256='a' * 64,
        binary=control_binary, binary_sha256='b' * 64, command=[control_binary, control_source, '{n}', '8'],
        cwd=BASE, runtime_environment={}, maximum_workers=8, loc=None, role='Fresh exact-source Mech control')
    rows = [foreign, control]
    orders = [dict(round=i + 1, profiles=[r['id'] for r in rows][i % 2:] + [r['id'] for r in rows][:i % 2]) for i in range(3)]
    manifest_path = BASE + '/' + records.COHORT + '/' + benchmark + '/full-study/manifest.json'
    pins = {source: study['source_sha256'], original: study['source_sha256'], runtime: 'c' * 64,
            control_source: 'a' * 64, control_binary: 'b' * 64}
    digests = dict(pins, **{manifest_path: study['manifest_sha256']})
    manifest = dict(status='prepared-awaiting-root-review', benchmark=benchmark, samples=3,
        size=study['size'], required_power_profile=deepcopy(records.POWER), shared_timing_lock=records.LOCK,
        maximum_compute_workers=8, maximum_rss_mib=8192, timeout_seconds=14400,
        environment_retry_limit=3, chart_admitted=False, profiles=rows, orders=orders, pins=pins)
    report = dict(status='complete', active=None, manifest=manifest_path,
        manifest_sha256=study['manifest_sha256'], pins_unchanged_at_end=True, orders=deepcopy(orders),
        profiles={r['id']: dict(loc=None, role=r['role'], samples_seconds=[]) for r in rows}, attempts=[])
    by_id = {r['id']: r for r in rows}
    for ordered in orders:
        for identity in ordered['profiles']:
            row, number = by_id[identity], len(report['attempts']) + 1
            command = [part.replace('{n}', str(study['size'])) for part in row['command']]
            # Fixture identity is independently assembled from the actual
            # frozen producer schema, without the adapter's fingerprint helper.
            support = {runtime: 'c' * 64} if row is foreign else {}
            fingerprint = dict(source_sha256=row['source_sha256'], binary_sha256=row.get('binary_sha256'),
                command=[part.replace(source if row is foreign else control_source, '{source}') for part in row['command']],
                cwd=BASE, environment=row['runtime_environment'], support=support)
            attempt = {field: row[field] for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd')}
            attempt.update(attempt=number, round=ordered['round'], command=command,
                runtime_environment=deepcopy(row['runtime_environment']), execution_fingerprint=json.dumps(fingerprint, sort_keys=True),
                full_workload=True, workload_n=study['size'], sampler_wait_status=0, exit_status=0,
                pins_unchanged_at_end=True, fatal_qualification_fields=[], remaining_group_members=[],
                monitor_errors=[], resource_monitor_errors=[], supervisor_remaining_members=[],
                competing_compilers=[], power_changes=[], maximum_rss_mib=8192, timeout_seconds=14400,
                peak_process_group_rss_kib=1024, validation=records.reference(benchmark),
                stdout_bytes=records.reference(benchmark)['bytes'], stdout_sha256=records.reference(benchmark)['sha256'],
                supervised_native_group=dict(type='native-group-started', pid=1000 + number, pgid=1000 + number,
                    command=command, inherited_common_mutex=True), seconds=float(number),
                result_path=manifest_path.rsplit('/', 1)[0] + f'/attempts/{number:03d}-{identity}.json',
                environment_attempt=1, qualified=True, valid_environment=True, retryable_environment_only=False,
                power_at_start=deepcopy(records.POWER), power_at_end=deepcopy(records.POWER))
            report['attempts'].append(attempt)
            report['profiles'][identity]['samples_seconds'].append(attempt['seconds'])
    for summary in report['profiles'].values():
        samples = summary['samples_seconds']
        summary.update(median_seconds=statistics.median(samples), sample_stdev_seconds=statistics.stdev(samples))
    return manifest, report, digests


def raw_records(report):
    result = {}
    for attempt in report['attempts']:
        raw = deepcopy(attempt)
        del raw['environment_attempt']
        result[attempt['result_path']] = raw
    return result


class PendingMinimumRecords(unittest.TestCase):
    def validate(self, manifest, report, digests, raw=None):
        raw = raw_records(report) if raw is None else raw
        with patch.object(records, '_read_raw', side_effect=lambda path: deepcopy(raw[path])):
            return records.validate_study(manifest, report, manifest_sha256=records.STUDIES[manifest['benchmark']]['manifest_sha256'],
                benchmark=manifest['benchmark'], digest=lambda path: digests[str(path)])

    def rejected(self, mutate, benchmark='mandelbrot'):
        manifest, report, digests = fixture(benchmark)
        mutate(manifest, report, digests)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            self.validate(manifest, report, digests)

    def test_accept_all_four_exact_whole_output_oracles_without_mutation(self):
        for benchmark in records.STUDIES:
            with self.subTest(benchmark=benchmark):
                manifest, report, digests = fixture(benchmark)
                original = deepcopy((manifest, report))
                qualified, discarded = self.validate(manifest, report, digests)
                self.assertEqual(set(qualified), {records.STUDIES[benchmark]['identity'], 'mech-aot-control'})
                self.assertEqual(discarded, [])
                self.assertEqual((manifest, report), original)
                for row in qualified.values():
                    self.assertIsNone(row['loc'])
                    self.assertEqual(len(row['sample_execution_metadata']), 3)
                    self.assertEqual(row['median_seconds'], statistics.median(row['samples_seconds']))

    def test_reject_incomplete_or_active_or_terminal_failure(self):
        for change in ({'status': 'running'}, {'active': {'id': 'native'}}, {'error': 'saved failure'}):
            with self.subTest(change=change):
                self.rejected(lambda m, r, d: r.update(change))

    def test_reject_manifest_digest_and_changed_source_pin(self):
        self.rejected(lambda m, r, d: r.update(manifest_sha256='f' * 64))
        self.rejected(lambda m, r, d: d.update({m['profiles'][0]['source']: 'f' * 64}))

    def test_reject_missing_or_duplicate_profile_and_order(self):
        self.rejected(lambda m, r, d: m['profiles'].append(deepcopy(m['profiles'][0])))
        self.rejected(lambda m, r, d: r['orders'][0]['profiles'].reverse())
        self.rejected(lambda m, r, d: r['profiles'].pop('mech-aot-control'))

    def test_reject_scaled_size_and_probe_flags(self):
        self.rejected(lambda m, r, d: m.update(size=200))
        self.rejected(lambda m, r, d: r['attempts'][0].update(full_workload=False))
        self.rejected(lambda m, r, d: r['attempts'][0].update(workload_n=200))
        self.rejected(lambda m, r, d: r['attempts'][0].update(canonical_full_workload=False))
        self.rejected(lambda m, r, d: r['attempts'][0].update(never_chart_eligible=True))
        self.rejected(lambda m, r, d: r['attempts'][0].update(preparation_only=True))

    def test_reject_command_runtime_flags_and_fingerprint(self):
        self.rejected(lambda m, r, d: r['attempts'][0]['command'].append('--shortcut'))
        self.rejected(lambda m, r, d: r['attempts'][0].update(execution_fingerprint='{}'))
        self.rejected(lambda m, r, d: m['profiles'][0]['command'].remove('--yjit'), 'binarytrees')
        self.rejected(lambda m, r, d: m['profiles'][0]['command'].remove('opcache.jit=tracing'), 'fannkuchredux')

    def test_reject_incomplete_wrong_or_self_declared_output(self):
        self.rejected(lambda m, r, d: r['attempts'][0].update(stdout_bytes=1))
        self.rejected(lambda m, r, d: r['attempts'][0]['validation'].update(sha256='f' * 64))
        self.rejected(lambda m, r, d: m.update(full_output_reference={'bytes': 1, 'sha256': 'f' * 64}))

    def test_reject_native_registration_mutex_and_remaining_children(self):
        for update in ({'pid': 0}, {'pgid': 3}, {'inherited_common_mutex': False}, {'command': ['wrong']}):
            with self.subTest(update=update):
                self.rejected(lambda m, r, d: r['attempts'][0]['supervised_native_group'].update(update))
        self.rejected(lambda m, r, d: r['attempts'][0].update(supervisor_remaining_members=[{'pid': 3}]))

    def test_reject_sampler_exit_and_missing_true_raw_gates(self):
        self.rejected(lambda m, r, d: r['attempts'][0].update(sampler_wait_status=256))
        self.rejected(lambda m, r, d: r['attempts'][0].update(exit_status=1))
        self.rejected(lambda m, r, d: r['attempts'][0].pop('fatal_qualification_fields'))
        self.rejected(lambda m, r, d: r['attempts'][0].update(pins_unchanged_at_end=1))

    def test_reject_resource_monitor_and_late_wrapper_failure(self):
        self.rejected(lambda m, r, d: r['attempts'][0].update(peak_process_group_rss_kib=8192 * 1024 + 1))
        self.rejected(lambda m, r, d: r['attempts'][0].update(resource_monitor_errors=['lost process']))
        self.rejected(lambda m, r, d: r['attempts'][0].update(supervisor_failure=True))

    def test_reject_nan_zero_boolean_and_infinite_seconds(self):
        for seconds in (float('nan'), float('inf'), 0, -1, True):
            with self.subTest(seconds=seconds):
                self.rejected(lambda m, r, d: r['attempts'][0].update(seconds=seconds))

    def test_reject_missing_extra_and_falsified_summary_samples(self):
        self.rejected(lambda m, r, d: r['attempts'].pop())
        self.rejected(lambda m, r, d: r['attempts'].append(deepcopy(r['attempts'][-1])))
        self.rejected(lambda m, r, d: r['profiles']['mech-aot-control'].update(median_seconds=0.01))
        self.rejected(lambda m, r, d: r['profiles']['mech-aot-control'].update(sample_stdev_seconds=0.0))

    def test_reject_changed_original_raw_execution_record(self):
        manifest, report, digests = fixture()
        raw = raw_records(report)
        raw[report['attempts'][0]['result_path']]['seconds'] = 0.01
        with self.assertRaises(ValueError):
            self.validate(manifest, report, digests, raw)

    def test_archive_real_environment_exclusion_without_relabel_or_score(self):
        manifest, report, digests = fixture()
        excluded = deepcopy(report['attempts'][0])
        excluded.update(qualified=False, valid_environment=False, retryable_environment_only=True,
            power_at_end=dict(source='Battery Power', low_power_mode=0))
        report['attempts'].insert(0, excluded)
        report['attempts'][1]['environment_attempt'] = 2
        for number, attempt in enumerate(report['attempts'], 1):
            attempt['attempt'] = number
            attempt['result_path'] = report['manifest'].rsplit('/', 1)[0] + f'/attempts/{number:03d}-{attempt["id"]}.json'
        original = deepcopy(report)
        qualified, discarded = self.validate(manifest, report, digests)
        self.assertEqual(discarded, [excluded])
        self.assertEqual(report, original)
        self.assertEqual(qualified[manifest['profiles'][0]['id']]['samples_seconds'], [1.0, 4.0, 5.0])

    def test_reject_fake_environment_retry_and_nonenvironment_failure(self):
        self.rejected(lambda m, r, d: r['attempts'][0].update(qualified=False, valid_environment=False, retryable_environment_only=True))
        self.rejected(lambda m, r, d: r['attempts'][0].update(error='output failure', qualified=False,
            valid_environment=False, retryable_environment_only=True, power_changes=[{'source': 'battery'}]))

    def test_reject_third_environment_failure_and_changed_producer_loc(self):
        manifest, report, digests = fixture()
        failures = []
        for retry in range(1, 4):
            attempt = deepcopy(report['attempts'][0])
            attempt.update(qualified=False, valid_environment=False, retryable_environment_only=True,
                environment_attempt=retry, power_changes=[{'source': 'Battery Power'}])
            failures.append(attempt)
        report['attempts'] = failures + report['attempts']
        for number, attempt in enumerate(report['attempts'], 1):
            attempt['attempt'] = number
            attempt['result_path'] = report['manifest'].rsplit('/', 1)[0] + f'/attempts/{number:03d}-{attempt["id"]}.json'
        with self.assertRaises(ValueError):
            self.validate(manifest, report, digests)
        self.rejected(lambda m, r, d: m['profiles'][0].update(loc=40))


if __name__ == '__main__':
    unittest.main()
