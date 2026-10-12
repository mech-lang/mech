"""Qualification and provenance fixtures; no benchmark processes are launched."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import statistics
import tempfile
import unittest
from unittest.mock import patch

import mandelbrot_study_data as subject


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def fixture(name, root):
    spec = subject.STUDIES[name]
    profiles, pins = [], {}
    times = {'scalar11': 2.1, 'five16-loop-fma': .55, 'five16-rows-loop-fma': .42,
             'frozen20': 2.3, 'frozen29-audited': .44, 'whole11': 55.,
             'active14': 225., 'whole16-parallel': 13., 'active19-parallel': 47., 'mech29-control': .44}
    for identity, loc in spec['locs'].items():
        language = 'mech' if name == 'mech' or identity == 'mech29-control' else 'uiua'
        source = root / (identity + ('.mec' if language == 'mech' else '.ua'))
        source.write_text(identity + '\n')
        binary = root / (identity + '.binary')
        binary.write_text('fixture binary ' + identity)
        row = dict(id=identity, key='mandelbrot:fixture:' + identity, benchmark='mandelbrot',
                   language=language, source=str(source), source_sha256=sha(source), loc=loc,
                   cwd=str(root), runtime_environment={})
        if language == 'mech':
            row.update(binary_sha256=sha(binary), command=[str(binary), '{n}', '8'],
                       native_object_sha256='a' * 64, emission=dict(command=['compiler', str(source), 'emit']))
        else:
            row.update(runtime_sha256=sha(binary), command=[str(binary), 'run', str(source), '{n}'],
                       runtime=dict(path=str(binary), sha256=sha(binary)),
                       maximum_workers=8 if identity.endswith('parallel') else 1)
        pins[str(source)], pins[str(binary)] = sha(source), sha(binary)
        profiles.append(row)
    orders = [dict(round=index, profiles=list(spec['locs'])) for index in (1, 2, 3)]
    manifest = dict(samples=3, size=16000, workers=8, required_power_profile=subject.POWER,
                    full_output_reference=subject.REFERENCE, profiles=profiles, pins=pins,
                    orders=orders, maximum_rss_mib=10240, timeout_seconds=600,
                    timing_boundary='Fixture: full fresh process and exact PBM')
    attempts = []
    for order in orders:
        for row in profiles:
            command = [part.replace('{n}', '16000') for part in row['command']]
            attempt = {key: row[key] for key in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd')}
            attempt.update(attempt=len(attempts) + 1, round=order['round'], command=command,
                runtime_environment={}, execution_fingerprint=json.dumps(subject._fingerprint(row)),
                qualified=True, sampler_wait_status=0, exit_status=0,
                full_workload=True, workload_n=16000, valid_environment=True, pins_unchanged_at_end=True,
                maximum_rss_mib=10240, peak_process_group_rss_kib=1024, timeout_seconds=600,
                power_at_start=subject.POWER, power_at_end=subject.POWER,
                validation=dict(subject.REFERENCE, reference_match=True),
                stdout_bytes=subject.REFERENCE['bytes'], stdout_sha256=subject.REFERENCE['sha256'],
                supervised_native_group=dict(pid=100 + len(attempts), pgid=100 + len(attempts),
                                             command=command, inherited_common_mutex=True),
                seconds=times[row['id']] + .001 * order['round'],
                **{field: [] for field in subject.EMPTY_FIELDS})
            if name == 'mech':
                attempt['qualification_checks'] = {field: True for field in subject.REQUIRED_CHECKS}
            else:
                attempt['fatal_qualification_fields'] = []
            attempts.append(attempt)
    report = dict(status='complete', active=None, pins_unchanged_at_end=True,
                  manifest=str(root / spec['manifest']), manifest_sha256=spec['digest'],
                  orders=orders, attempts=attempts, profiles={})
    for row in profiles:
        samples = [a['seconds'] for a in attempts if a['id'] == row['id']]
        report['profiles'][row['id']] = dict(loc=row['loc'], samples_seconds=samples,
            median_seconds=statistics.median(samples), sample_stdev_seconds=statistics.stdev(samples))
    return manifest, report


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.manifest, self.report = fixture('mech', self.root)
        self.spec = subject.STUDIES['mech']

    def tearDown(self):
        self.directory.cleanup()

    def check(self, report=None, manifest=None):
        return subject.validate_study(manifest or self.manifest, report or self.report,
                                      self.spec['digest'], self.spec['locs'])

    def test_complete_rounds_reconcile_raw_samples(self):
        rows = self.check()
        for sample, expected in zip(rows['scalar11']['samples_seconds'], [2.101, 2.102, 2.103]):
            self.assertAlmostEqual(sample, expected)
        self.assertEqual(rows['five16-rows-loop-fma']['loc'], 16)

    def test_worker_limit_cannot_be_changed_with_consistent_relabelled_records(self):
        manifest, report = deepcopy(self.manifest), deepcopy(self.report)
        manifest['profiles'][0]['command'][-1] = '9'
        for attempt in report['attempts']:
            if attempt['id'] == 'scalar11':
                attempt['command'][-1] = '9'
                attempt['supervised_native_group']['command'][-1] = '9'
                attempt['execution_fingerprint'] = json.dumps(subject._fingerprint(manifest['profiles'][0]))
        with self.assertRaises(AssertionError):
            self.check(report, manifest)

    def test_flag_cannot_hide_fatal_execution_or_late_monitor_errors(self):
        mutations = {
            'sampler exit': ('sampler_wait_status', 256), 'native exit': ('exit_status', 1),
            'late resource violation': ('resource_violation', {'rss_kib': 99999999}),
            'cleanup error': ('cleanup_error', 'group survived'),
            'monitor error': ('monitor_errors', ['power probe failed']),
            'environment mismatch': ('valid_environment', False),
            'pins changed': ('pins_unchanged_at_end', False),
            'incomplete output': ('stdout_bytes', 100),
            'RSS limit exceeded despite flag': ('peak_process_group_rss_kib', 20 * 1024 * 1024),
        }
        for label, (field, value) in mutations.items():
            with self.subTest(label=label):
                report = deepcopy(self.report)
                report['attempts'][0][field] = value
                with self.assertRaises(AssertionError):
                    self.check(report)

    def test_full_reference_and_authoritative_native_receipt_are_required(self):
        for field, value in [('validation', dict(subject.REFERENCE, sha256='f' * 64, reference_match=True)),
                             ('supervised_native_group', dict(pid=101, pgid=102, command=['other']))]:
            report = deepcopy(self.report)
            report['attempts'][0][field] = value
            with self.assertRaises(AssertionError):
                self.check(report)
        report = deepcopy(self.report)
        del report['attempts'][0]['supervised_native_group']
        with self.assertRaises(KeyError):
            self.check(report)

    def test_source_command_and_fingerprint_cannot_be_relabelled(self):
        for field, value in [('source_sha256', 'f' * 64), ('command', ['binary', '16000', '9']),
                             ('execution_fingerprint', '{}')]:
            report = deepcopy(self.report)
            report['attempts'][0][field] = value
            with self.assertRaises(AssertionError):
                self.check(report)

    def test_duplicate_missing_or_forged_summary_samples_are_rejected(self):
        report = deepcopy(self.report)
        report['attempts'][5] = deepcopy(report['attempts'][0])
        with self.assertRaises(AssertionError):
            self.check(report)
        report = deepcopy(self.report)
        report['attempts'].pop()
        with self.assertRaises(AssertionError):
            self.check(report)
        report = deepcopy(self.report)
        report['profiles']['scalar11']['median_seconds'] = .01
        with self.assertRaises(AssertionError):
            self.check(report)

    def test_running_study_and_wrong_manifest_are_not_complete_evidence(self):
        for field, value in [('status', 'running'), ('manifest_sha256', 'f' * 64), ('active', {'pid': 101})]:
            report = deepcopy(self.report)
            report[field] = value
            with self.assertRaises(AssertionError):
                self.check(report)

    def test_uiua_fatal_fields_and_inherited_lock_are_required(self):
        manifest, report = fixture('uiua', self.root)
        spec = subject.STUDIES['uiua']
        subject.validate_study(manifest, report, spec['digest'], spec['locs'])
        report['attempts'][0]['supervised_native_group']['inherited_common_mutex'] = False
        with self.assertRaises(AssertionError):
            subject.validate_study(manifest, report, spec['digest'], spec['locs'])


class ImportTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.specs = deepcopy(subject.STUDIES)
        for name, spec in self.specs.items():
            manifest, report = fixture(name, self.root)
            path = self.root / spec['manifest']
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(manifest))
            spec['digest'] = sha(path)
            report['manifest_sha256'] = spec['digest']
            report_path = self.root / spec['report']
            report_path.parent.mkdir(parents=True, exist_ok=True)
            report_path.write_text(json.dumps(report))
        self.data = dict(programs={}, files={}, benchmarks=[dict(key='mandelbrot', groups=[])],
                         provenance={}, protocol={})
        for language in ('mech', 'uiua'):
            key, digest = language + ':old', language + '-old-digest'
            self.data['programs'][key] = dict(key=key, loc=29 if language == 'mech' else 11,
                                            samples=[.44, .45, .46], files=[digest])
            self.data['files'][digest] = dict(code='old ' + language + ' source bytes', sha256=digest)
            self.data['benchmarks'][0]['groups'].append(dict(language=language, programs=[key]))

    def tearDown(self):
        self.directory.cleanup()

    def add_file(self, path, expected):
        self.assertEqual(sha(path), expected)
        self.data['files'][expected] = dict(code=Path(path).read_text(), sha256=expected)
        return [expected]

    def measured(self, key, benchmark, language, loc, samples, source, digest, report, source_report, **extra):
        self.data['programs'][key] = dict(key=key, language=language, loc=loc, samples=samples,
            files=self.add_file(source, digest), seconds=statistics.median(samples), **extra)

    def run_import(self):
        def load(relative):
            self.data['provenance'][relative] = sha(self.root / relative)
            return json.loads((self.root / relative).read_text())
        with patch.dict(subject.STUDIES, self.specs, clear=True):
            subject.add_mandelbrot_studies(self.root, self.data, load, self.measured, self.add_file, sha)

    def test_endpoints_preserve_previous_source_samples_and_paired_control_identity(self):
        previous = deepcopy(self.data)
        self.run_import()
        audit = self.data['mandelbrot_study_audit']
        for language in ('mech', 'uiua'):
            group = next(g for g in self.data['benchmarks'][0]['groups'] if g['language'] == language)
            self.assertEqual(len(group['programs']), 2)
            self.assertNotIn(language + ':old', group['programs'])
            archived = audit['previous_endpoints'][language]
            self.assertEqual(archived['programs'][0], previous['programs'][language + ':old'])
            self.assertEqual(archived['files'], {language + '-old-digest': previous['files'][language + '-old-digest']})
        self.assertTrue(audit['selected']['mech']['fast'].endswith('five16-rows-loop-fma'))
        fast = self.data['programs'][audit['selected']['uiua']['fast']]
        control = fast['mech_comparison']
        self.assertEqual(control['scope'], 'paired-control')
        self.assertEqual(control['control_loc'], 29)
        self.assertTrue(control['source'].endswith('mech29-control.mec'))
        self.assertEqual(control['samples_seconds'], [.441, .442, .443])
        self.assertNotIn('fastest_loc', control)
        self.assertEqual(control['control_loc_x_mech'], 16 / 29)

    def test_running_study_admits_nothing_and_registers_no_live_report_pin(self):
        path = self.root / self.specs['uiua']['report']
        report = json.loads(path.read_text())
        report['status'] = 'running'
        path.write_text(json.dumps(report))
        previous = deepcopy(self.data)
        self.run_import()
        self.assertEqual(self.data, previous)

    def test_changed_artifact_is_rejected_before_endpoint_mutation(self):
        report = json.loads((self.root / self.specs['mech']['manifest']).read_text())
        Path(report['profiles'][0]['source']).write_text('changed source')
        previous = deepcopy(self.data['programs'])
        with self.assertRaises(AssertionError):
            self.run_import()
        self.assertEqual(self.data['programs'], previous)


if __name__ == '__main__':
    unittest.main()
