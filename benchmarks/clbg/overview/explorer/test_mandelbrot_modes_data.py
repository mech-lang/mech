"""Admission fixtures for source identity, complete work and timing boundaries."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import statistics
import tempfile
import unittest
from unittest.mock import patch

import mandelbrot_modes_data as subject
from mandelbrot_study_data import EMPTY_FIELDS, _fingerprint


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def fixture(root):
    sources, pins, profiles = {}, {}, []
    for loc in (11, 16):
        path = root / f'canonical-{loc}.mec'
        path.write_text(f'fixture canonical source {loc}\n')
        sources[loc] = path
        pins[str(path)] = digest(path)
    for identity, (loc, mode, language) in subject.PROFILES.items():
        binary = root / (identity + '.binary')
        binary.write_text('fixture binary ' + identity)
        pins[str(binary)] = digest(binary)
        source = sources[loc]
        environment = {'RAYON_NUM_THREADS': '8'}
        command = [str(binary), '{n}', '8']
        if mode != 'aot-control':
            environment['MECH_TIMINGS'] = '1'
            command = [str(binary), str(source), mode, '{n}',
                       '--scalar' if loc == 11 else '--fp-contract-loop']
        profiles.append(dict(id=identity, key='mandelbrot:fixture:' + identity,
            benchmark='mandelbrot', language=language, loc=loc, mode=mode,
            source=str(source), source_sha256=digest(source), binary=str(binary),
            binary_sha256=digest(binary), command=command, cwd=str(root),
            runtime_environment=environment, maximum_workers=8, physical_loc=loc,
            loc_expansions=[], state_layout='fixture ' + str(loc), timing_boundary=mode))
    orders = [dict(round=index, profiles=[row['id'] for row in profiles]) for index in (1, 2, 3)]
    manifest = dict(samples=3, size=16000, workers=8, required_power_profile=subject.POWER,
        full_output_reference=subject.REFERENCE, profiles=profiles, orders=orders, pins=pins,
        maximum_rss_mib=10240, timeout_seconds=14400, timing_boundary='fixture boundaries')
    attempts = []
    for order in orders:
        for row in profiles:
            command = [part.replace('{n}', '16000') for part in row['command']]
            wall = 3. + order['round'] / 100
            prepare, kernel = .5, 2. + order['round'] / 100
            attempt = {key: row[key] for key in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd')}
            attempt.update(attempt=len(attempts) + 1, round=order['round'], command=command,
                runtime_environment=row['runtime_environment'], execution_fingerprint=json.dumps(_fingerprint(row)),
                qualified=True, valid_source_timing=True,
                qualification_checks={key: True for key in subject.CHECKS},
                sampler_wait_status=0, exit_status=0, full_workload=True, workload_n=16000,
                canonical_full_workload=True, never_chart_eligible=False,
                valid_environment=True, pins_unchanged_at_end=True, maximum_rss_mib=10240,
                peak_process_group_rss_kib=1024, timeout_seconds=14400,
                power_at_start=subject.POWER, power_at_end=subject.POWER,
                validation=dict(subject.REFERENCE, reference_match=True),
                stdout_bytes=subject.REFERENCE['bytes'], stdout_sha256=subject.REFERENCE['sha256'],
                supervised_native_group=dict(pid=100 + len(attempts), pgid=100 + len(attempts), command=command),
                wall_seconds=wall, seconds=wall, stderr='', **{key: [] for key in EMPTY_FIELDS})
            if row['mode'] != 'aot-control':
                score = kernel if row['mode'] == 'resident' else wall
                attempt.update(seconds=score, prepare_seconds=prepare, kernel_seconds=kernel,
                    stderr='TIMING ' + json.dumps(dict(prepare_seconds=prepare, kernel_seconds=kernel)) + '\n',
                    timing=dict(wall_seconds=wall, prepare_seconds=prepare, kernel_seconds=kernel, reported_seconds=score))
            attempts.append(attempt)
    report = dict(status='complete', active=None, pins_unchanged_at_end=True,
        manifest=str(root / subject.MANIFEST), manifest_sha256='reviewed-fixture',
        orders=orders, attempts=attempts, profiles={})
    for row in profiles:
        samples = [attempt for attempt in attempts if attempt['id'] == row['id']]
        seconds = [attempt['seconds'] for attempt in samples]
        report['profiles'][row['id']] = dict(loc=row['loc'], mode=row['mode'],
            state_layout=row['state_layout'], timing_boundary=row['timing_boundary'], samples_seconds=seconds,
            sample_details=[{key: attempt[key] for key in ('seconds', 'wall_seconds', 'prepare_seconds',
                'kernel_seconds', 'validation', 'supervised_native_group') if key in attempt} for attempt in samples],
            median_seconds=statistics.median(seconds), sample_stdev_seconds=statistics.stdev(seconds))
    return manifest, report, {loc: digest(path) for loc, path in sources.items()}


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.manifest, self.report, source_digests = fixture(self.root)
        self.patch = patch.dict(subject.SOURCES, source_digests, clear=True)
        self.patch.start()

    def tearDown(self):
        self.patch.stop()
        self.directory.cleanup()

    def check(self, report=None, manifest=None):
        return subject.validate_modes_study(manifest or self.manifest, report or self.report, 'reviewed-fixture')

    def test_scores_reconcile_with_mode_boundary_and_complete_rounds(self):
        rows = self.check()
        self.assertEqual(rows['scalar11-jit']['samples_seconds'], [3.01, 3.02, 3.03])
        self.assertEqual(rows['scalar11-resident']['samples_seconds'], [2.01, 2.02, 2.03])
        self.assertEqual(rows['row16-jit']['loc'], 16)

    def test_loaded_timer_cannot_replace_fresh_jit_elapsed(self):
        report = deepcopy(self.report)
        attempt = report['attempts'][0]
        attempt['seconds'] = attempt['kernel_seconds']
        attempt['timing']['reported_seconds'] = attempt['kernel_seconds']
        with self.assertRaises(AssertionError):
            self.check(report)

    def test_resident_preparation_cannot_be_added_to_score(self):
        report = deepcopy(self.report)
        attempt = report['attempts'][1]
        attempt['seconds'] = attempt['wall_seconds']
        attempt['timing']['reported_seconds'] = attempt['wall_seconds']
        with self.assertRaises(AssertionError):
            self.check(report)

    def test_missing_duplicate_forged_and_impossible_timing_records_fail(self):
        for stderr in ('', self.report['attempts'][0]['stderr'] * 2,
                       'TIMING {"prepare_seconds":9,"kernel_seconds":9}\n',
                       'TIMING {"prepare_seconds":0,"kernel_seconds":NaN}\n'):
            report = deepcopy(self.report)
            report['attempts'][0]['stderr'] = stderr
            with self.assertRaises(AssertionError):
                self.check(report)

    def test_resource_output_status_and_cleanup_flags_do_not_admit_failure(self):
        mutations = {'exit_status': 1, 'sampler_wait_status': 256,
            'stdout_bytes': 100, 'source_sha256': 'f' * 64,
            'cleanup_error': 'survivor', 'monitor_errors': ['probe failure'],
            'peak_process_group_rss_kib': 20 * 1024 * 1024,
            'never_chart_eligible': True, 'canonical_full_workload': False}
        for field, value in mutations.items():
            with self.subTest(field=field):
                report = deepcopy(self.report)
                report['attempts'][0][field] = value
                with self.assertRaises(AssertionError):
                    self.check(report)

    def test_work_limit_and_old_source_cannot_be_consistently_relabelled(self):
        manifest = deepcopy(self.manifest)
        manifest['profiles'][0]['command'][-1] = '--scalar-eight-pixels'
        with self.assertRaises(AssertionError):
            self.check(manifest=manifest)
        manifest = deepcopy(self.manifest)
        manifest['profiles'][0]['source_sha256'] = 'f' * 64
        with self.assertRaises(AssertionError):
            self.check(manifest=manifest)
        manifest = deepcopy(self.manifest)
        manifest['profiles'][0]['runtime_environment']['RAYON_NUM_THREADS'] = '9'
        with self.assertRaises(AssertionError):
            self.check(manifest=manifest)

    def test_missing_duplicate_and_forged_summary_are_rejected(self):
        report = deepcopy(self.report)
        report['attempts'].pop()
        with self.assertRaises(AssertionError):
            self.check(report)
        report = deepcopy(self.report)
        report['attempts'][6] = deepcopy(report['attempts'][0])
        with self.assertRaises(AssertionError):
            self.check(report)
        report = deepcopy(self.report)
        report['profiles']['row16-resident']['median_seconds'] = .001
        with self.assertRaises(AssertionError):
            self.check(report)

    def test_running_study_contributes_no_provenance_or_samples(self):
        path = self.root / subject.REPORT
        path.parent.mkdir(parents=True)
        path.write_text('{"status":"running"}')
        subject.add_mandelbrot_modes(self.root, {}, lambda _: self.fail('loaded running report'),
            lambda *args, **kwargs: self.fail('added partial score'),
            lambda *args: self.fail('loaded partial source'), digest)

    def test_import_preserves_old_source_samples_and_selects_only_new_profiles(self):
        manifest_path, report_path = self.root / subject.MANIFEST, self.root / subject.REPORT
        manifest_path.parent.mkdir(parents=True)
        manifest_path.write_text(json.dumps(self.manifest))
        reviewed = digest(manifest_path)
        self.report['manifest_sha256'] = reviewed
        report_path.write_text(json.dumps(self.report))
        data = dict(programs={}, files={}, provenance={}, protocol={},
                    benchmarks=[dict(key='mandelbrot', groups=[])])
        for language in ('mech-jit', 'mech-resident'):
            key, source_sha = language + ':old', language + '-old-source'
            data['programs'][key] = dict(key=key, loc=29, samples=[.4, .5, .6], files=[source_sha])
            data['files'][source_sha] = dict(code='old ' + language, sha256=source_sha)
            data['benchmarks'][0]['groups'].append(dict(language=language, programs=[key]))
        old = deepcopy(data)

        def load(relative):
            path = self.root / relative
            data['provenance'][relative] = digest(path)
            return json.loads(path.read_text())

        def add_file(path, expected):
            self.assertEqual(digest(path), expected)
            data['files'][expected] = dict(code=Path(path).read_text(), sha256=expected)
            return [expected]

        def measured(key, benchmark, language, loc, samples, source, source_sha, report, source_report, **extra):
            data['programs'][key] = dict(key=key, benchmark=benchmark, language=language,
                loc=loc, samples=samples, seconds=statistics.median(samples),
                files=add_file(source, source_sha), report=report, source_report=source_report, **extra)

        with patch.object(subject, 'REVIEWED_MANIFEST_SHA256', reviewed):
            subject.add_mandelbrot_modes(self.root, data, load, measured, add_file, digest)
        audit = data['mandelbrot_modes_audit']
        self.assertEqual(set(audit['profiles']), set(subject.PROFILES))
        for group in data['benchmarks'][0]['groups']:
            language = group['language']
            previous = audit['previous_endpoints'][language]
            self.assertEqual(previous['programs'], [old['programs'][language + ':old']])
            self.assertEqual(previous['files'], {language + '-old-source': old['files'][language + '-old-source']})
            self.assertTrue(all(data['programs'][key]['loc'] == 11 for key in group['programs']))
            self.assertTrue(all(data['programs'][key]['language'] == language for key in group['programs']))
        self.assertEqual(len(audit['study_sources']), 2)


if __name__ == '__main__':
    unittest.main()
