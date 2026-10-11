"""Small file fixtures for the R/APL/Uiua importer; no benchmark processes run."""
import ast
import copy
import hashlib
import json
import os
import statistics
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from r_apl_uiua_data import COHORT, _LegacyEvidence, _execution_fingerprint, _loc_metadata, add_r_apl_uiua_languages


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Fixture:
    def __init__(self, root, candidate_samples=3, control_samples=3, alias=True):
        self.root = Path(root)
        self.directory = self.root / COHORT
        (self.directory / 'results').mkdir(parents=True)
        self.audit_path = self.directory / 'source-audit-r.json'
        self.report_path = self.directory / 'results/timings-r.json'
        self.summary_path = self.directory / 'results/summary.json'
        runtime = self.file('runtime', 'general interpreter')
        support = self.file('support', 'general library')
        source = self.file('compact.R', 'application algorithm\n')
        fast_source = self.file('fast.R', 'application algorithm\n')
        control_source = self.file('control.mec', 'unchanged Mech application\n')
        binary = self.file('control', 'Mech AOT executable')
        algorithm = dict(strict=True, native_benchmark_kernel=False)
        candidate = dict(key='regexredux:r-compact', id='r-compact', benchmark='regexredux',
            language='r', source=str(source), source_sha256=sha(source),
            command=[str(runtime), str(source), '{n}'], cwd=str(self.root),
            runtime_environment={'R_ENABLE_JIT': '3'},
            runtime=dict(path=str(runtime), sha256=sha(runtime)),
            libraries={str(support): sha(support)}, loc=10, physical_loc=9, loc_expansions=1,
            small_validation=dict(reference_match=True), algorithm=algorithm,
            eligible_for_chart=False, measurement_status='not-measured')
        candidates = [candidate]
        if alias:
            optimized = copy.deepcopy(candidate)
            optimized.update(key='regexredux:r-fast', id='r-fast', source=str(fast_source),
                             command=[str(runtime), str(fast_source), '{n}'],
                             timing_alias_of=candidate['key'])
            candidates.append(optimized)
        control = dict(key='regexredux:mech-control-for-r', id='mech-control-for-r',
            benchmark='regexredux', language='mech', source=str(control_source),
            source_sha256=sha(control_source), binary=str(binary), binary_sha256=sha(binary),
            command=[str(binary), '{n}'], cwd=str(self.root), runtime_environment={},
            loc=20, algorithm=algorithm)
        baseline = dict(fastest_loc=20, smallest_loc=12, fastest_key='regexredux:old-mech-fast',
                        smallest_key='regexredux:old-mech-small', control_key=control['key'])
        self.audit = dict(status='reviewed', language='r', programs=candidates, controls=[control],
            workloads={'regexredux': [1, 10]}, baseline_endpoints={'regexredux': baseline},
            pins={str(path): sha(path) for path in (runtime, support, source, fast_source, control_source, binary)},
            power_required=dict(source='AC Power', low_power_mode=0),
            timing_boundary='Full fresh process, including runtime startup and output.')
        self.report = dict(status='partial', language='r', sample_count=max(3, candidate_samples, control_samples),
            source_audit=str(self.audit_path), source_audit_sha256='',
            workloads=copy.deepcopy(self.audit['workloads']),
            baseline_endpoints=copy.deepcopy(self.audit['baseline_endpoints']), programs={}, attempts=[])
        for frozen in candidates + [control]:
            row = copy.deepcopy(frozen)
            row.update(samples_seconds=[], sample_validation=[], attempts=[], measurement_status='pending')
            self.report['programs'][row['key']] = row
            if row.get('timing_alias_of'):
                owner = self.report['programs'][row['timing_alias_of']]
                for field in ('samples_seconds', 'sample_validation', 'median_seconds', 'sample_stdev_seconds',
                              'measurement_status'):
                    if field in owner:
                        row[field] = copy.deepcopy(owner[field])
                row['aliased_sample_source'] = owner['key']
                continue
            count = control_samples if row['language'] == 'mech' else candidate_samples
            for sample in range(count):
                number = len(self.report['attempts']) + 1
                seconds = (2 if row['language'] == 'mech' else 4) + sample * .1
                validation = dict(reference_match=True, bytes=12, sha256='a' * 64)
                attempt = {field: copy.deepcopy(row[field]) for field in
                           ('key', 'id', 'benchmark', 'language', 'source_sha256', 'cwd', 'runtime_environment')}
                attempt.update(attempt_number=number, sample=sample + 1,
                    execution_fingerprint=_execution_fingerprint(row),
                    command=[argument.replace('{n}', '10') for argument in row['command']],
                    workload_n=10, full_workload=True, seconds=seconds, exit_status=0,
                    valid_environment=True, power_at_start=copy.deepcopy(self.audit['power_required']),
                    power_at_end=copy.deepcopy(self.audit['power_required']), competing_compilers=[],
                    power_changes=[], monitor_errors=[], resource_monitor_errors=[], resource_violation=None,
                    validation=validation, stdout_bytes=12, stdout_sha256='a' * 64)
                self.report['attempts'].append(attempt)
                row['attempts'].append(number)
                row['samples_seconds'].append(seconds)
                row['sample_validation'].append(copy.deepcopy(validation))
            if count:
                row['median_seconds'] = statistics.median(row['samples_seconds'])
                row['measurement_status'] = 'measured' if count >= 3 else 'partial'
            if count >= 2:
                row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
        self.data = dict(languages=[dict(key='mech', label='Mech')],
            benchmarks=[dict(key='regexredux', groups=[dict(language='mech',
                programs=['regexredux:old-mech-fast', 'regexredux:old-mech-small'])])],
            programs={'regexredux:old-mech-fast': dict(seconds=8, samples=[7, 8, 9], loc=20),
                      'regexredux:old-mech-small': dict(seconds=16, samples=[15, 16, 17], loc=12)},
            exclusions=[], protocol={'existing': 'unchanged'}, provenance={'existing': 'unchanged'})
        self.previous = copy.deepcopy(self.data)

    def file(self, name, content):
        path = self.directory / name
        path.write_text(content)
        return path

    def candidate(self):
        return self.report['programs']['regexredux:r-compact']

    def control(self):
        return self.report['programs']['regexredux:mech-control-for-r']

    def summary(self):
        rows = list(self.report['programs'].values())
        candidates = [row for row in rows if row['language'] == 'r']
        valid = lambda row: len(row['samples_seconds']) >= 3 and not row.get('full_failure')
        admitted = [row for row in candidates if valid(row)]
        control = self.control()
        baseline = self.audit['baseline_endpoints']['regexredux']
        failed = [dict(key=row['key'], failure=row['full_failure']) for row in candidates if row.get('full_failure')]
        pending = [dict(key=row['key'], valid_samples=len(row['samples_seconds']), status=row['measurement_status'])
                   for row in candidates if not valid(row) and not row.get('full_failure')]
        comparison = dict(language='r', benchmark='regexredux', prepared_candidates=len(candidates),
            admitted_candidates=len(admitted), pending_candidates=pending, failed_candidates=failed,
            control_admitted=valid(control), coverage='pending', fastest=None, smallest=None)
        ready = bool(admitted and valid(control))
        if ready:
            for role in ('fastest', 'smallest'):
                order = (lambda row: (statistics.median(row['samples_seconds']), row['loc'], row['key'])) if role == 'fastest' else (
                    lambda row: (row['loc'], statistics.median(row['samples_seconds']), row['key']))
                row = min(admitted, key=order)
                seconds = statistics.median(row['samples_seconds'])
                mech_seconds = statistics.median(control['samples_seconds'])
                mech_loc = baseline['fastest_loc' if role == 'fastest' else 'smallest_loc']
                endpoint = {field: copy.deepcopy(row[field]) for field in
                            ('key', 'id', 'source', 'source_sha256', 'command', 'loc', 'samples_seconds', 'sample_validation')}
                endpoint.update(seconds=seconds, stdev_seconds=statistics.stdev(row['samples_seconds']),
                    time_x_mech=seconds / mech_seconds, loc_x_mech=row['loc'] / mech_loc,
                    mech_seconds=mech_seconds, mech_samples_seconds=copy.deepcopy(control['samples_seconds']),
                    mech_loc=mech_loc, control_key=control['key'], timing_alias_of=row.get('timing_alias_of'))
                comparison[role] = endpoint
            comparison['coverage'] = 'complete-prepared-candidates' if len(admitted) == len(candidates) else 'partial-prepared-candidates'
        return dict(status='complete' if ready and not pending else 'partial', comparisons=[comparison],
            coverage={'r': dict(status=self.report['status'], prepared_benchmarks=1,
                admitted_benchmarks=int(ready), failures=failed, pending=pending)},
            reports={str(self.report_path): sha(self.report_path)},
            admission_rule='At least three candidate and fresh-control samples.',
            units='Seconds; ratios use fresh Mech median and frozen endpoint LOC.')

    def commit(self):
        self.audit_path.write_text(json.dumps(self.audit))
        self.report['source_audit_sha256'] = sha(self.audit_path)
        self.report_path.write_text(json.dumps(self.report))
        self.summary_path.write_text(json.dumps(self.summary()))

    def legacy_migrations(self):
        """Two pinned migrations using the actual reviewed original runner bytes."""
        common = Path(os.environ['CLBG_COMMON_PREPARE'])
        original = common.parent / 'evidence/harness-v1-partial'
        archive = self.directory / 'evidence/harness-v1-partial'
        staging = self.directory / 'evidence/harness-repair-staging'
        archive.mkdir(parents=True)
        staging.mkdir()
        for candidate in self.audit['programs']:
            candidate['maximum_workers'] = 1
            self.report['programs'][candidate['key']]['maximum_workers'] = 1
        old_audit = copy.deepcopy(self.audit)
        for name in ('run.py', 'prepare.py'):
            (archive / name).write_bytes((original / name).read_bytes())
            (self.directory / name).write_bytes((common.parent / name).read_bytes())
            old_audit['pins'][str(self.directory / name)] = sha(archive / name)
            self.audit['pins'][str(self.directory / name)] = sha(self.directory / name)
        for name in ('compact/run.py', 'array-comparison/all-20261009/run.py'):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((common.parent.parents[1] / name).read_bytes())
            old_audit['pins'][str(path)] = self.audit['pins'][str(path)] = sha(path)
        old_report = copy.deepcopy(self.report)
        for attempt in old_report['attempts']:
            del attempt['runtime_environment']
            del attempt['execution_fingerprint']
        old_audit_path = archive / 'source-audit-r.json'
        old_report_path = archive / 'timings-r.json'
        old_audit_path.write_text(json.dumps(old_audit))
        old_report['source_audit_sha256'] = sha(old_audit_path)
        old_report_path.write_text(json.dumps(old_report))
        first = dict(prior_report=str(old_report_path), prior_report_sha256=sha(old_report_path),
            prior_source_audit=str(old_audit_path), prior_source_audit_sha256=sha(old_audit_path),
            preserved_keys=list(self.report['programs']), retired_keys=[], new_keys=[],
            reason='Explicit approved guard repair; identical executions preserved.')
        migrated = copy.deepcopy(old_report)
        for attempt in migrated['attempts']:
            attempt['prior_guard_revision_attempt_number'] = attempt['attempt_number']
        migrated['guard_revisions'] = [copy.deepcopy(first)]
        staging_audit_path = staging / 'source-audit-r.json'
        staging_report_path = staging / 'timings-r.json'
        staging_audit_path.write_text(json.dumps(self.audit))
        migrated['source_audit_sha256'] = sha(staging_audit_path)
        staging_report_path.write_text(json.dumps(migrated))
        second = dict(prior_report=str(staging_report_path), prior_report_sha256=sha(staging_report_path),
            prior_source_audit=str(staging_audit_path), prior_source_audit_sha256=sha(staging_audit_path),
            preserved_keys=list(self.report['programs']), retired_keys=[], new_keys=[],
            reason='Second approved guard repair; identical executions preserved.')
        self.report['attempts'] = migrated['attempts']
        self.report['guard_revisions'] = [first, second]
        self.report['historical_attempts'] = [dict(key='retired:old-source', never_chart_eligible=True)]
        self.report['retired_candidates'] = {'retired:old-source': dict(source_sha256='b' * 64)}
        return archive

    def import_data(self):
        def load(name):
            self.data['provenance'][name] = sha(self.root / name)
            return json.loads((self.root / name).read_text())

        def add_file(path, expected):
            assert sha(path) == expected
            return [expected]

        def measured(key, benchmark, language, loc, samples, source, source_sha, report, source_report, **extra):
            self.data['programs'][key] = dict(key=key, benchmark=benchmark, language=language,
                loc=loc, seconds=statistics.median(samples), samples=samples,
                files=add_file(source, source_sha), report=report, source_report=source_report, **extra)

        add_r_apl_uiua_languages(self.root, self.data, load, measured, add_file, sha)
        return self.data


class ImporterTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)

    def fixture(self, **options):
        return Fixture(self.temporary.name, **options)

    def test_valid_samples_alias_ratios_and_existing_cohorts(self):
        fixture = self.fixture()
        fixture.commit()
        data = fixture.import_data()
        for key, value in fixture.previous['programs'].items():
            self.assertEqual(data['programs'][key], value)
        self.assertEqual(data['benchmarks'][0]['groups'][0], fixture.previous['benchmarks'][0]['groups'][0])
        self.assertEqual(data['languages'][0], fixture.previous['languages'][0])
        self.assertEqual(data['protocol']['existing'], 'unchanged')
        point = data['programs']['regexredux:r-fast']
        self.assertEqual(point['timing_alias_of'], 'regexredux:r-compact')
        self.assertEqual(point['aliased_sample_source'], 'regexredux:r-compact')
        self.assertEqual(point['samples'], fixture.candidate()['samples_seconds'])
        self.assertEqual(point['mech_comparison']['seconds'], 2.1)
        self.assertAlmostEqual(point['mech_comparison']['time_x_mech'], 4.1 / 2.1)
        self.assertEqual(point['mech_comparison']['fastest_loc_x_mech'], .5)
        self.assertAlmostEqual(point['mech_comparison']['smallest_loc_x_mech'], 10 / 12)
        self.assertEqual(data['r_apl_uiua_audit']['ratio_policy'], data['r_apl_uiua_audit']['units'])

    def test_more_than_three_valid_samples_are_preserved(self):
        fixture = self.fixture(candidate_samples=5, control_samples=5)
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(len(data['programs']['regexredux:r-compact']['samples']), 5)

    def test_frozen_empty_measurement_slots_use_reconciled_attempts(self):
        fixture = self.fixture()
        for row in fixture.audit['programs'] + fixture.audit['controls']:
            row.update(samples_seconds=[], sample_validation=[], attempts=[])
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(data['programs']['regexredux:r-compact']['samples'], [4, 4.1, 4.2])
        fixture.candidate()['samples_seconds'][0] += 1
        fixture.commit()
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_frozen_measurement_slots_cannot_contain_timings(self):
        fixture = self.fixture()
        fixture.audit['programs'][0]['samples_seconds'] = [4]
        fixture.commit()
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_original_apl_loc_schema_is_verified_from_source(self):
        fixture = self.fixture()
        source = fixture.file('count.apl', "⍝ comment\n∇ Z←F X\n Z←'←⋄⍝' ⍝ comment\n∇\nF 1\n")
        row = dict(language='apl', key='apl:count', source=str(source), loc=3,
            loc_rule='One independent statement per line; comments/blanks and standalone closing ∇ excluded; function headers, I/O and termination count. No diamonds or combined assignments.')
        self.assertEqual(_loc_metadata(row), (3, []))
        for code in ("A←1 ⋄ B←2\n", "A←B←1\n"):
            source.write_text(code)
            row['loc'] = 1
            with self.assertRaises(AssertionError):
                _loc_metadata(row)

    @unittest.skipUnless(os.environ.get('CLBG_COMMON_PREPARE'), 'Set CLBG_COMMON_PREPARE for pinned legacy runner fixtures')
    def test_exact_two_step_legacy_lineage_is_labelled_and_history_preserved(self):
        fixture = self.fixture()
        archive = fixture.legacy_migrations()
        fixture.commit()
        original_hash = sha(archive / 'timings-r.json')
        data = fixture.import_data()
        point = data['programs']['regexredux:r-fast']
        metadata = point['sample_execution_metadata']
        self.assertEqual(len(metadata), 3)
        self.assertTrue(all(row['mode'] == 'reconciled-from-pinned-old-row-and-runner' for row in metadata))
        self.assertTrue(all(len(row['lineage']) == 2 for row in metadata))
        self.assertIn('ambient environment was not recorded', metadata[0]['limitation'])
        self.assertEqual(sha(archive / 'timings-r.json'), original_hash)
        self.assertNotIn('runtime_environment', fixture.report['attempts'][0])
        history = data['r_apl_uiua_audit']['report_history']['r']
        for name in ('guard_revisions', 'historical_attempts', 'retired_candidates'):
            self.assertEqual(history[name], fixture.report[name])
        self.assertEqual(point['mech_comparison']['sample_execution_metadata'][0]['mode'], metadata[0]['mode'])

    @unittest.skipUnless(os.environ.get('CLBG_COMMON_PREPARE'), 'Set CLBG_COMMON_PREPARE for pinned legacy runner fixtures')
    def test_bad_legacy_attempt_lineage_or_runner_is_rejected(self):
        for change in ('attempt', 'snapshot', 'runner', 'retrofill'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                archive = fixture.legacy_migrations()
                if change == 'attempt':
                    fixture.report['attempts'][0]['seconds'] += .01
                elif change == 'retrofill':
                    fixture.report['attempts'][0].update(runtime_environment=fixture.candidate()['runtime_environment'],
                                                        execution_fingerprint=_execution_fingerprint(fixture.candidate()))
                fixture.commit()
                if change == 'snapshot':
                    (archive / 'timings-r.json').write_text((archive / 'timings-r.json').read_text() + '\n')
                elif change == 'runner':
                    (archive / 'run.py').write_text('unapproved runner\n')
                with self.assertRaises(AssertionError):
                    fixture.import_data()

    @unittest.skipUnless(os.environ.get('CLBG_COMMON_PREPARE'), 'Set CLBG_COMMON_PREPARE for the pinned interruption correction')
    def test_excluded_interruption_correction_requires_exact_original_fields(self):
        cohort = Path(os.environ['CLBG_COMMON_PREPARE']).parent
        root = cohort.parents[1]
        report = json.loads((cohort / 'results/timings-r.json').read_text())
        audit = json.loads((cohort / 'source-audit-r.json').read_text())
        provenance = {}
        legacy = _LegacyEvidence(root, report, audit,
            lambda name: (provenance.update({name: sha(root / name)}) or json.loads((root / name).read_text())),
            sha, provenance)
        staging = json.loads((cohort / 'evidence/harness-repair-staging/timings-r.json').read_text())
        revision = staging['guard_revisions'][0]
        original_report = json.loads(Path(revision['prior_report']).read_text())
        original = original_report['attempts'][5]
        corrected = staging['attempts'][5]
        legacy.exact_attempt(corrected, original, revision)
        self.assertTrue(legacy.corrections[0]['never_chart_eligible'])
        self.assertTrue(any(name.endswith('interrupt-classification-correction.json') for name in provenance))
        for field, value in (('original_error', 'different error'), ('error', '')):
            bad = copy.deepcopy(corrected)
            bad[field] = value
            with self.subTest(field=field), self.assertRaises(AssertionError):
                legacy.exact_attempt(bad, original, revision)

    @unittest.skipUnless(os.environ.get('CLBG_COMMON_PREPARE'), 'Set CLBG_COMMON_PREPARE to test the real harness contract')
    def test_logical_symlink_pins_match_real_prepare_fingerprint(self):
        fixture = self.fixture()
        prepare_path = Path(os.environ['CLBG_COMMON_PREPARE'])
        parsed = ast.parse(prepare_path.read_text(), filename=str(prepare_path))
        # Execute only the real pure helpers; importing prepare.py loads other runners.
        helpers = [node for node in parsed.body if isinstance(node, ast.FunctionDef)
                   and node.name in ('support_pins', 'execution_fingerprint')]
        self.assertEqual({node.name for node in helpers}, {'support_pins', 'execution_fingerprint'})
        namespace = dict(json=json, Path=Path, compact=SimpleNamespace(ROOT=fixture.root))
        exec(compile(ast.Module(body=helpers, type_ignores=[]), str(prepare_path), 'exec'), namespace)
        real_fingerprint = namespace['execution_fingerprint']
        physical = fixture.directory / 'support'
        logical = fixture.directory / 'opt-support'
        logical.symlink_to(physical)
        digest = sha(physical)
        fixture.audit['pins'][str(logical)] = digest
        for frozen in fixture.audit['programs']:
            row = fixture.report['programs'][frozen['key']]
            for candidate in (frozen, row):
                candidate['libraries'] = {str(logical): digest}
                candidate['support_pins'] = [dict(pin_files={str(logical): digest}),
                                             dict(path=str(logical), sha256=digest)]
            expected = real_fingerprint(row)
            self.assertEqual(_execution_fingerprint(row), expected)
            support = json.loads(expected)['support']
            self.assertEqual(support[str(logical)], digest)
            self.assertEqual(support[str(physical.resolve())], digest)
            for index in row['attempts']:
                fixture.report['attempts'][index - 1]['execution_fingerprint'] = expected
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(data['programs']['regexredux:r-fast']['support_pins'][str(logical)], digest)
        self.assertEqual(data['programs']['regexredux:r-fast']['support_pins'][str(physical.resolve())], digest)

    def test_stdin_workload_command_may_omit_n_argument(self):
        fixture = self.fixture()
        for frozen in fixture.audit['programs']:
            frozen['command'].pop()
            row = fixture.report['programs'][frozen['key']]
            row['command'].pop()
            for index in row['attempts']:
                attempt = fixture.report['attempts'][index - 1]
                attempt['command'].pop()
                attempt['execution_fingerprint'] = _execution_fingerprint(row)
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(len(data['programs']['regexredux:r-compact']['runtime_arguments']), 2)

    def test_partial_candidate_coverage_imports_only_verified_source(self):
        fixture = self.fixture()
        frozen = fixture.audit['programs'][1]
        pending = fixture.report['programs'][frozen['key']]
        for row in (frozen, pending):
            del row['timing_alias_of']
        for field in ('aliased_sample_source', 'median_seconds', 'sample_stdev_seconds'):
            pending.pop(field, None)
        pending.update(samples_seconds=[], sample_validation=[], measurement_status='pending')
        fixture.commit()
        data = fixture.import_data()
        self.assertIn('regexredux:r-compact', data['programs'])
        self.assertNotIn('regexredux:r-fast', data['programs'])
        audit = data['r_apl_uiua_audit']
        self.assertEqual(audit['status'], 'partial')
        self.assertEqual(audit['comparisons'][0]['coverage'], 'partial-prepared-candidates')
        self.assertEqual(audit['coverage']['r']['pending'][0]['key'], 'regexredux:r-fast')

    def test_discarded_environment_attempt_does_not_contribute_a_sample(self):
        fixture = self.fixture(candidate_samples=4)
        attempt = fixture.report['attempts'][0]
        attempt.update(valid_environment=False, power_at_end=dict(source='Battery Power', low_power_mode=0))
        for key in ('regexredux:r-compact', 'regexredux:r-fast'):
            row = fixture.report['programs'][key]
            row['samples_seconds'].pop(0)
            row['sample_validation'].pop(0)
            row['median_seconds'] = statistics.median(row['samples_seconds'])
            row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(data['programs']['regexredux:r-compact']['samples'], [4.1, 4.2, 4.3])

    def test_candidate_waits_for_three_fresh_control_samples(self):
        fixture = self.fixture(control_samples=2)
        fixture.commit()
        data = fixture.import_data()
        self.assertNotIn('regexredux:r-compact', data['programs'])
        self.assertEqual(data['r_apl_uiua_audit']['admitted']['r'], [])
        self.assertEqual(data['r_apl_uiua_audit']['coverage']['r']['admitted_benchmarks'], 0)
        self.assertEqual(data['languages'], fixture.previous['languages'])

    def test_control_failure_is_explicit_and_prevents_plotting(self):
        fixture = self.fixture(control_samples=0)
        fixture.control().update(full_failure=dict(error='Control timeout'), measurement_status='failed')
        fixture.commit()
        data = fixture.import_data()
        self.assertNotIn('regexredux:r-compact', data['programs'])
        self.assertEqual(data['r_apl_uiua_audit']['control_failures'][0]['failure']['error'], 'Control timeout')

    def test_failed_only_language_remains_in_coverage_and_exclusions(self):
        fixture = self.fixture(candidate_samples=0, alias=False)
        fixture.candidate().update(full_failure=dict(error='RSS ceiling', error_type='RuntimeError'),
                                   measurement_status='failed')
        fixture.commit()
        data = fixture.import_data()
        self.assertEqual(len(data['r_apl_uiua_audit']['failures']), 1)
        self.assertEqual(data['exclusions'][0]['failure']['error'], 'RSS ceiling')
        self.assertNotIn('regexredux:r-compact', data['programs'])

    def test_foreign_or_changed_attempts_are_rejected(self):
        for field, value in (('key', 'regexredux:mech-control-for-r'), ('source_sha256', 'b' * 64),
                             ('command', ['different-runtime', '10']), ('cwd', '/different'),
                             ('workload_n', 1), ('full_workload', False),
                             ('execution_fingerprint', 'different execution'),
                             ('runtime_environment', {'R_ENABLE_JIT': '0'})):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.report['attempts'][0][field] = value
                fixture.commit()
                with self.assertRaises(AssertionError):
                    fixture.import_data()

    def test_frozen_row_execution_cannot_drift(self):
        fixture = self.fixture()
        fixture.candidate()['runtime_environment']['R_ENABLE_JIT'] = '0'
        fixture.commit()
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_attempt_requires_recorded_environment_and_execution_fingerprint(self):
        for field in ('runtime_environment', 'execution_fingerprint'):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                del fixture.report['attempts'][0][field]
                fixture.commit()
                with self.assertRaises(AssertionError):
                    fixture.import_data()

    def test_alias_requires_identical_frozen_execution(self):
        fixture = self.fixture()
        fixture.audit['programs'][1]['runtime_environment']['R_ENABLE_JIT'] = '0'
        fixture.report['programs']['regexredux:r-fast']['runtime_environment']['R_ENABLE_JIT'] = '0'
        fixture.commit()
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_false_environment_claim_and_output_evidence_are_rejected(self):
        for field, value in (('power_at_end', dict(source='Battery Power', low_power_mode=0)),
                             ('stdout_sha256', 'b' * 64), ('competing_compilers', ['compiler']),
                             ('resource_violation', {'rss_kib': 999999}), ('exit_status', 1)):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.report['attempts'][0][field] = value
                fixture.commit()
                with self.assertRaises(AssertionError):
                    fixture.import_data()

    def test_duplicate_attempt_reference_is_rejected(self):
        fixture = self.fixture()
        fixture.candidate()['attempts'][1] = fixture.candidate()['attempts'][0]
        fixture.commit()
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_changed_summary_report_hash_is_rejected(self):
        fixture = self.fixture()
        fixture.commit()
        fixture.report_path.write_text(fixture.report_path.read_text() + '\n')
        with self.assertRaises(AssertionError):
            fixture.import_data()

    def test_summary_ratios_and_coverage_must_match_admission(self):
        for field in ('ratio', 'coverage', 'control'):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.commit()
                summary = json.loads(fixture.summary_path.read_text())
                if field == 'ratio':
                    summary['comparisons'][0]['fastest']['time_x_mech'] = 999
                elif field == 'coverage':
                    summary['coverage']['r']['admitted_benchmarks'] = 0
                else:
                    summary['comparisons'][0]['control_admitted'] = False
                fixture.summary_path.write_text(json.dumps(summary))
                with self.assertRaises(AssertionError):
                    fixture.import_data()

    def test_current_source_and_support_pins_are_required(self):
        for filename in ('compact.R', 'support'):
            with self.subTest(filename=filename), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.commit()
                (fixture.directory / filename).write_text('changed\n')
                with self.assertRaises(AssertionError):
                    fixture.import_data()


if __name__ == '__main__':
    unittest.main()
