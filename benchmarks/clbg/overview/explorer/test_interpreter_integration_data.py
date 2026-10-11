"""Read-only import/resume fixtures; synthetic completions never leave memory.

Run with ``python3 -m unittest test_interpreter_integration_data.py``. An
optional CLBG_ROOT selects the benchmark snapshot. From overview/explorer in
a mirrored repository, the containing clbg directory is detected automatically.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import statistics
import unittest


HERE = Path(__file__).resolve().parent
COHORT = 'interpreter-integration/20261011'
DEFAULT_CLBG = Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg')


def clbg_root():
    if os.environ.get('CLBG_ROOT'):
        return Path(os.environ['CLBG_ROOT']).expanduser().resolve()
    for ancestor in HERE.parents:
        if ancestor.name == 'clbg' and (ancestor / COHORT / 'baseline.json').is_file():
            return ancestor
    return DEFAULT_CLBG


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def real_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


class InterpreterIntegrationFixtures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = clbg_root()
        cls.importer = load_module(HERE / 'interpreter_integration_data.py',
                                   'interpreter_importer_fixture')
        cls.resume = load_module(cls.root / COHORT / 'run.py',
                                 'interpreter_resume_fixture')
        cls.baseline = json.loads((cls.root / COHORT / 'baseline.json').read_text())
        cls.actual = json.loads((cls.root / COHORT / 'timings.json').read_text())
        # Pin callbacks model immutable fixture files without repeatedly hashing
        # measured executables. Unregistered helper/reference files use real SHA.
        cls.pins = {str(cls.root.parents[1] / path): digest
                    for path, digest in cls.baseline['source_files'].items()}
        cls.pins.update({source['source']: source['source_sha256']
                         for source in cls.baseline['programs']})
        cls.pins.update({cls.baseline['binary']: cls.baseline['binary_sha256'],
                         cls.baseline['after_binary']: cls.baseline['after_binary_sha256']})

    @classmethod
    def fixture_sha(cls, path):
        return cls.pins.get(str(path)) or real_sha(path)

    def completion(self, sample_count=3):
        report = copy.deepcopy(self.actual)
        report.update(status='complete', sample_count=sample_count,
                      completed_benchmarks=list(self.importer.WORKLOADS), changes={})
        report.pop('active_run', None)
        for row in report['programs'].values():
            self.assertTrue(row['samples_seconds'], 'Fixture needs a validated sample per phase')
            # Repeated values exist only in this synthetic admission fixture.
            # This creates no claimed measurements and writes no report files.
            while len(row['samples_seconds']) < sample_count:
                for field in ('samples_seconds', 'sample_validation', 'sample_details'):
                    row[field].append(copy.deepcopy(row[field][-1]))
            for field in ('samples_seconds', 'sample_validation', 'sample_details'):
                row[field] = row[field][:sample_count]
            row['median_seconds'] = statistics.median(row['samples_seconds'])
            row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
            row['full_validation'] = row['sample_validation'][-1]
        for source in self.baseline['programs']:
            key = source['source_key']
            before, after = (report['programs'][key + ':' + phase]
                             for phase in ('before', 'after'))
            report['changes'][key] = dict(benchmark=source['benchmark'], loc=source['loc'],
                before_seconds=before['median_seconds'], after_seconds=after['median_seconds'],
                after_over_before=after['median_seconds'] / before['median_seconds'],
                before_stdev_seconds=before['sample_stdev_seconds'],
                after_stdev_seconds=after['sample_stdev_seconds'])
        return report

    def validate(self, report):
        return self.importer.validate_complete_report(self.root, report, self.baseline,
                                                      self.fixture_sha)

    def payload(self):
        programs = {
            'old-aot': dict(key='old-aot', benchmark='nbody', language='mech', seconds=1, loc=10),
            'old-jit': dict(key='old-jit', benchmark='nbody', language='mech-jit', seconds=2, loc=10),
        }
        files = {}
        for source in self.baseline['programs']:
            key, digest = source['key'], source['source_sha256']
            files[digest] = dict(sha256=digest, code=Path(source['source']).read_text(),
                                 path=source['source'])
            programs[key] = dict(key=key, id=key.split(':', 1)[1], benchmark=source['benchmark'],
                language=source['language'], loc=source['loc'], physical_loc=source['physical_loc'],
                loc_expansions=copy.deepcopy(source['loc_expansions']), files=[digest],
                algorithm=dict(strict=True, reason='Existing reviewed source'),
                seconds=source['median_seconds'], samples=copy.deepcopy(source['samples_seconds']),
                timing_sd=source['sample_stdev_seconds'],
                sample_details=copy.deepcopy(source['sample_details']),
                report='execution-modes/20261010-final/timings.json',
                source_report='execution-modes/20261010/builds.json',
                binary_sha256=source['binary_sha256'], compiler_sha256=source['binary_sha256'],
                timing_boundary=source['timing_boundary'], execution_mode='resident',
                required_work_validated=True, measurement_status='measured',
                runtime_policy='Interpreted', source_provenance='Original source provenance')
        benchmarks = [dict(key=benchmark, groups=[dict(language='mech-resident',
            programs=[source['key'] for source in self.baseline['programs']
                      if source['benchmark'] == benchmark])])
                      for benchmark in self.importer.WORKLOADS]
        benchmarks[0]['groups'].extend([dict(language='mech', programs=['old-aot']),
                                       dict(language='mech-jit', programs=['old-jit'])])
        return dict(programs=programs, files=files, benchmarks=benchmarks,
                    languages=[dict(key=key) for key in ('mech', 'mech-jit', 'mech-resident')],
                    protocol={}, provenance={})

    def export(self, report, data):
        def load(name):
            if name == COHORT + '/timings.json':
                return report
            if name == COHORT + '/baseline.json':
                return self.baseline
            return json.loads((self.root / name).read_text())
        self.importer.add_interpreter_integration_audit(self.root, data, load, self.fixture_sha)

    def test_valid_three_sample_completion(self):
        sources = len(self.baseline['programs'])
        self.assertEqual(self.validate(self.completion()), (3, sources, 6 * sources))

    def test_valid_five_sample_completion_derives_totals(self):
        report, data = self.completion(5), self.payload()
        self.export(report, data)
        audit = data['interpreter_integration_audit']
        self.assertEqual(audit['sample_count'], 5)
        self.assertEqual(audit['sources'], len(self.baseline['programs']))
        self.assertEqual(audit['completed_samples'], 10 * len(self.baseline['programs']))
        self.assertIn('not pinned at execution time',
                      audit['verification_time_helpers']['scope'])

    def test_immutable_phase_row_corruptions_rejected(self):
        mutations = [('command', ['wrong']), ('runtime_environment', {'MECH_FAKE': '1'}),
                     ('phase', 'wrong'), ('mode', 'jit'), ('loc', 999),
                     ('binary_sha256', '0' * 64), ('source_sha256', '0' * 64),
                     ('timing_boundary', 'wrong')]
        for field, value in mutations:
            with self.subTest(field=field):
                report = self.completion()
                row = next(iter(report['programs'].values()))
                row[field] = value
                with self.assertRaises(AssertionError):
                    self.validate(report)

    def test_noncanonical_workload_and_duplicate_completion_rejected(self):
        report = self.completion()
        report['workloads']['nbody'][1] = 1000
        with self.assertRaises(AssertionError):
            self.validate(report)
        report = self.completion()
        report['completed_benchmarks'][0] = report['completed_benchmarks'][1]
        with self.assertRaises(AssertionError):
            self.validate(report)

    def test_nonfinite_or_missing_sample_evidence_rejected(self):
        for value in (0, float('nan'), float('inf'), True):
            with self.subTest(seconds=value):
                report = self.completion()
                next(iter(report['programs'].values()))['samples_seconds'][0] = value
                with self.assertRaises(AssertionError):
                    self.validate(report)
        report = self.completion()
        next(iter(report['programs'].values()))['sample_details'].pop()
        with self.assertRaises(AssertionError):
            self.validate(report)

    def test_invalid_output_records_and_statistics_rejected(self):
        for field, value in (('reference_match', 1), ('sha256', '0' * 64)):
            with self.subTest(validation=field):
                report = self.completion()
                row = next(row for row in report['programs'].values()
                           if row['benchmark'] == 'knucleotide')
                row['sample_validation'][0][field] = value
                with self.assertRaises(AssertionError):
                    self.validate(report)
        for field in ('median_seconds', 'sample_stdev_seconds'):
            with self.subTest(statistic=field):
                report = self.completion()
                next(iter(report['programs'].values()))[field] = 123456
                with self.assertRaises(AssertionError):
                    self.validate(report)
        report = self.completion()
        next(iter(report['changes'].values()))['after_over_before'] = 123456
        with self.assertRaises(AssertionError):
            self.validate(report)

    def test_running_and_partial_reports_never_admitted(self):
        for status in ('running', 'partial'):
            with self.subTest(status=status):
                report, data = self.completion(), self.payload()
                report['status'] = status
                before = copy.deepcopy(data)
                self.export(report, data)
                self.assertEqual(data, before)

    def test_completed_audit_replaces_all_candidates_and_preserves_identity_and_history(self):
        data = self.payload()
        before = copy.deepcopy(data)
        report = self.completion()
        self.export(report, data)
        for field in ('benchmarks', 'languages', 'files'):
            self.assertEqual(data[field], before[field])
        self.assertEqual(set(data['programs']), set(before['programs']))
        for key in ('old-aot', 'old-jit'):
            self.assertEqual(data['programs'][key], before['programs'][key])
        for source in self.baseline['programs']:
            key = source['source_key'] + ':resident'
            point, previous = data['programs'][key], before['programs'][key]
            after = report['programs'][source['source_key'] + ':after']
            for field in ('key', 'id', 'benchmark', 'language', 'loc', 'physical_loc',
                          'loc_expansions', 'files', 'algorithm', 'source_provenance'):
                self.assertEqual(point[field], previous[field], (key, field))
            for field, row_field in (('seconds', 'median_seconds'), ('samples', 'samples_seconds'),
                ('timing_sd', 'sample_stdev_seconds'), ('sample_details', 'sample_details'),
                ('full_output_validation', 'sample_validation'), ('full_validation', 'full_validation'),
                ('binary_sha256', 'binary_sha256'), ('runtime_environment', 'runtime_environment'),
                ('runtime_arguments', 'command')):
                self.assertEqual(point[field], after[row_field], (key, field))
            self.assertEqual(point['report'], COHORT + '/timings.json')
            self.assertEqual(point['source_report'], COHORT + '/baseline.json')
            self.assertEqual(point['measurement_phase'], 'after')
            archived = copy.deepcopy(point['previous_measurement'])
            self.assertEqual(archived.pop('execution_record'), source)
            self.assertEqual(archived, previous)
        audit = data['interpreter_integration_audit']
        self.assertEqual(audit['selected_phase'], 'after')
        self.assertEqual(audit['selected_samples'], 45)
        self.assertEqual(audit['completed_samples'], 90)
        self.assertEqual(len(audit['selected_programs']), 15)
        self.assertIn('LLVM codegen', audit['result_policy'])
        self.assertIn('+5.93%', audit['result_policy'])
        self.assertIn('cause of the observed timing change is not proven', audit['result_policy'])

    def test_candidate_identity_and_all_fifteen_required_before_any_mutation(self):
        key = self.baseline['programs'][0]['key']
        mutations = [lambda data: data['programs'].pop(key),
            lambda data: data['programs'][key].update(key=key + ':wrong'),
            lambda data: data['programs'][key].update(loc=999),
            lambda data: data['programs'][key].update(files=['0' * 64]),
            lambda data: data['files'][data['programs'][key]['files'][0]].update(sha256='0' * 64),
            lambda data: data['files'][data['programs'][key]['files'][0]].update(code='wrong source'),
            lambda data: data['programs'][key].update(samples=[1, 2, 3]),
            lambda data: data['programs'][key].update(binary_sha256='0' * 64)]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                data = self.payload()
                mutation(data)
                original = copy.deepcopy(data)
                with self.assertRaises(AssertionError):
                    self.export(self.completion(), data)
                self.assertEqual(data, original)

    def test_canonical_after_selected_even_when_before_or_confirmation_is_faster(self):
        report, data = self.completion(), self.payload()
        self.export(report, data)
        source_key = self.importer.DEFAULT_FANNKUCH
        point = data['programs'][source_key + ':resident']
        before = report['programs'][source_key + ':before']
        after = report['programs'][source_key + ':after']
        self.assertGreater(after['median_seconds'], before['median_seconds'])
        self.assertEqual(point['samples'], after['samples_seconds'])
        confirmation = data['interpreter_integration_audit']['confirmation']
        if confirmation and confirmation['status'] == 'complete':
            self.assertIs(confirmation['used_for_chart'], False)
            self.assertNotEqual(point['samples'], confirmation['programs']['after']['samples_seconds'])

    def test_archived_and_new_measurements_are_independent_copies(self):
        report, data = self.completion(), self.payload()
        self.export(report, data)
        key = self.baseline['programs'][0]['key']
        point = data['programs'][key]
        archived = copy.deepcopy(point['previous_measurement'])
        point['samples'][0] = 12345
        point['sample_details'][0]['seconds'] = 12345
        self.assertEqual(point['previous_measurement'], archived)
        self.assertNotEqual(report['programs'][key.rsplit(':', 1)[0] + ':after']['samples_seconds'][0], 12345)

    def test_actual_complete_report_and_optional_evidence_with_real_file_pins(self):
        self.assertEqual(self.importer.validate_complete_report(
            self.root, self.actual, self.baseline, real_sha), (3, 15, 90))
        data = self.payload()
        def load(name):
            data['provenance'][name] = real_sha(self.root / name)
            return json.loads((self.root / name).read_text())
        self.importer.add_interpreter_integration_audit(self.root, data, load, real_sha)
        audit = data['interpreter_integration_audit']
        self.assertEqual(audit['report_sha256'], real_sha(self.root / COHORT / 'timings.json'))
        self.assertEqual(audit['baseline_sha256'], real_sha(self.root / COHORT / 'baseline.json'))
        if audit['confirmation'] and audit['confirmation']['status'] == 'complete':
            self.assertEqual(audit['confirmation']['report_sha256'],
                             data['provenance'][COHORT + '/fannkuch-confirmation.json'])
        if audit['codegen_inspection']:
            self.assertIs(audit['codegen_inspection']['llvm_generated_code_changed'], True)
            self.assertIs(audit['codegen_inspection']['causal_attribution_proven'], False)
            for name, digest in audit['codegen_inspection']['disassembly_pins'].items():
                self.assertEqual(data['provenance'][name], real_sha(self.root / name))

    def test_confirmation_drift_or_invalid_sample_never_replaces_canonical_points(self):
        name = COHORT + '/fannkuch-confirmation.json'
        if not (self.root / name).is_file():
            self.skipTest('Optional independent confirmation not present')
        original_confirmation = json.loads((self.root / name).read_text())
        if original_confirmation['status'] != 'complete':
            self.skipTest('Optional independent confirmation still running')
        mutations = [
            lambda confirmation: confirmation['programs']['after'].update(command=['wrong']),
            lambda confirmation: confirmation['pins'].update({str(self.root / COHORT / 'timings.json'): '0' * 64}),
            lambda confirmation: confirmation['comparison'].update(after_x_before=0.5),
            lambda confirmation: confirmation['programs']['after']['sample_validation'][0].update(reference_match=False),
            lambda confirmation: confirmation['programs']['after']['sample_details'][0].update(competing_compilers=['rustc']),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                confirmation = copy.deepcopy(original_confirmation)
                mutation(confirmation)
                data, report = self.payload(), self.completion()
                original = copy.deepcopy(data)
                def load(relative):
                    if relative == COHORT + '/timings.json':
                        return report
                    if relative == COHORT + '/baseline.json':
                        return self.baseline
                    if relative == name:
                        return confirmation
                    return json.loads((self.root / relative).read_text())
                with self.assertRaises(AssertionError):
                    self.importer.add_interpreter_integration_audit(
                        self.root, data, load, self.fixture_sha)
                self.assertEqual(data, original)

    def test_codegen_binary_and_disassembly_pin_drift_rejected(self):
        name = COHORT + '/codegen-inspection.json'
        if not (self.root / name).is_file():
            self.skipTest('Optional codegen inspection not present')
        inspection = json.loads((self.root / name).read_text())
        for field in ('binary_sha256', 'disassembly_sha256'):
            with self.subTest(field=field):
                changed = copy.deepcopy(inspection)
                changed['rows']['after'][field] = '0' * 64
                def load(relative):
                    return changed if relative == name else json.loads((self.root / relative).read_text())
                with self.assertRaises(AssertionError):
                    self.importer._codegen_evidence(self.root, self.actual, load, self.fixture_sha)

    def expected_resume_rows(self):
        rows = {}
        for source in self.baseline['programs']:
            for phase in ('before', 'after'):
                binary = self.baseline['binary' if phase == 'before' else 'after_binary']
                key = source['source_key'] + ':' + phase
                rows[key] = {field: source[field] for field in self.importer.ROW_FIELDS}
                rows[key].update(key=key, phase=phase, binary=binary,
                    binary_sha256=self.baseline['binary_sha256' if phase == 'before'
                                                else 'after_binary_sha256'],
                    command=[binary] + source['command'][1:])
        return rows

    def test_resume_accepts_saved_progress_and_rejects_command_or_environment_drift(self):
        rows = self.expected_resume_rows()
        self.resume.validate_resume(self.actual, rows, self.baseline,
                                    self.actual['sample_count'])
        for field, value in (('command', ['wrong']),
                             ('runtime_environment', {'MECH_FAKE': '1'})):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.actual)
                next(iter(changed['programs'].values()))[field] = value
                with self.assertRaises(AssertionError):
                    self.resume.validate_resume(changed, rows, self.baseline,
                                                changed['sample_count'])


if __name__ == '__main__':
    unittest.main()
