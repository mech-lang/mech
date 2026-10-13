"""Unexecuted synthetic metadata/transaction fixtures; no native subjects."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import platform_metadata_data_v2 as adapter

# The later direct fixture runner injects its actual held common descriptor.
# Reopening/relocking the common path here could deadlock against that owner.
MUTEX_FD = None


def fixture_mutex():
    adapter.held_mutex(MUTEX_FD)
    return os.fdopen(os.dup(MUTEX_FD), 'a+')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n')
    return sha(path)


def package(root, unresolved=False, group_prose=None):
    root = Path(root).resolve()
    source = root / 'sources/app.txt'
    source.parent.mkdir()
    source.write_text('Synthetic authored source, never compiled or measured.\n')
    source_digest = sha(source)
    runtime_path = root / 'reports/runtime.json'
    runtime_digest = write(runtime_path, {'engine': 'synthetic saved runtime receipt'})
    provenance, programs, annotations = {}, {}, {}
    for language, count in adapter.COUNTS.items():
        relative = 'reports/' + language + '.json'
        report_digest = write(root / relative, {'language': language, 'scope': 'synthetic fixture only'})
        provenance[relative] = report_digest
        for ordinal in range(count):
            identity = language + '-' + str(ordinal)
            key = 'work:' + identity
            backend, execution = adapter.BACKENDS[language]
            programs[key] = dict(key=key, id=identity, language=language, benchmark='work',
                files=[source_digest], report=relative, loc=17, seconds=2.0, samples=[1.0, 2.0, 3.0],
                runtime_policy='legacy GraalVM NativeAOT display' if language == 'java' else 'previous policy',
                execution_mode='aot' if language == 'java' else None,
                timing_detail={'retained': True}, source_roles={source_digest: 'previous counted source'})
            annotations[key] = dict(key=backend, execution=execution, verified=True,
                label='OpenJDK JVM JIT' if language == 'java' else 'Synthetic ' + backend,
                evidence=dict(verified=True, report=relative, report_sha256=report_digest,
                    source_sha256=[source_digest], command=['/saved/never-executed-runtime', identity],
                    runtime_receipt=dict(verified=True, report='reports/runtime.json', report_sha256=runtime_digest,
                        field='/engine', value='synthetic saved runtime receipt', value_encoding='literal-saved-string')))
    files = {source_digest: dict(path='sources/app.txt', sha256=source_digest, code=source.read_text(),
        name='app.txt', normalized_loc=17, physical_loc=17, role='preserved metadata')}
    data = dict(programs=programs, files=files, provenance=provenance,
        languages=[dict(key=language, release_year=1995 if language == 'java' else 2000, label=language)
                   for language in adapter.COUNTS], benchmarks=[dict(key='work', baseline_loc=17, baseline_seconds=2.0)],
        language_groups=[dict(key='aot', languages=['c', 'java', 'csharp-aot', 'fsharp-aot']),
                         dict(key='jit', languages=['erlang', 'csharp', 'fsharp']),
                         dict(key='interpreted', languages=['python']),
                         dict(key='functional', languages=['fsharp', 'fsharp-aot'])],
        protocol=dict(language_groups=group_prose or 'Groups overlap. Java uses GraalVM native AOT builds in this cohort.', metric='preserved'),
        reference_summary=[dict(lang='java', actual='preserved')])
    dataset = root / 'overview/explorer/dist/data.json'
    inventory = root / 'continuation/platform-metadata-qualification-v1/expected-program-ids.json'
    pins = {str(dataset): write(dataset, data), str(inventory): write(inventory, sorted(programs)),
            str(source): source_digest, str(runtime_path): runtime_digest}
    pins.update({str(root / path): value for path, value in provenance.items()})
    output = root / 'continuation/jobs/platform-metadata-qualification-v1'
    output.mkdir(parents=True)
    baseline = output / 'snapshot/4-data.json'
    baseline.parent.mkdir()
    baseline.write_bytes(dataset.read_bytes())
    pins[str(baseline)] = sha(baseline)
    raw = output / 'fixtures.log'
    raw.write_text('Synthetic passed six-ID fixture receipt; no actual benchmarks.\n')
    pins[str(raw)] = sha(raw)
    unresolved_rows = []
    if unresolved:
        for key in ('work:java-15', 'work:java-16'):
            backend = annotations[key]
            backend['verified'] = backend['evidence']['verified'] = False
            backend.pop('label')
            backend['evidence'].pop('runtime_receipt')
            backend['unresolved_reason'] = 'Historical runtime artifact unavailable in this synthetic fixture'
            unresolved_rows.append(dict(key=key, reason=backend['unresolved_reason']))
    status = 'reconciled-with-unresolved-platform-metadata' if unresolved else 'qualified-preserved-platform-metadata'
    qualification = dict(status=status, expected_program_count=100, actual_program_count=100,
        qualified_program_count=100-len(unresolved_rows), unresolved_programs=unresolved_rows, annotations=annotations,
        provenance=dict(provenance, **{'reports/runtime.json': runtime_digest}), pins=pins,
        dataset_mutated=False, sources_mutated=False, timings_mutated=False, chart_admission=False,
        native_processes=0, runtime_version_reruns=0)
    qualification_path = output / 'measured-backends.json'
    qualification_digest = write(qualification_path, qualification)
    receipt = dict(status=status, expected_program_count=100, actual_program_count=100,
        qualified_program_count=100-len(unresolved_rows), unresolved_programs=unresolved_rows,
        exclusive_common_timing_mutex_held=True, pins_unchanged_at_end=True,
        expected_fixture_count=6, tests_run=6,
        required_fixture_ids=sorted(adapter.FIXTURE_IDS), discovered_fixture_ids=sorted(adapter.FIXTURE_IDS),
        actual_started_fixture_ids=sorted(adapter.FIXTURE_IDS),
        failures=[], errors=[], skips=[], expected_failures=[], unexpected_successes=[],
        actual_native_processes=0, runtime_version_reruns=0, benchmark_replays=0,
        chart_admission=False, dataset_mutated=False, sources_mutated=False, timings_mutated=False,
        source_pins=pins, raw_fixture_log=dict(path=str(raw), sha256=sha(raw)),
        metadata_output=dict(path=str(qualification_path), sha256=qualification_digest))
    receipt_path = output / 'qualification-receipt.json'
    receipt_digest = write(receipt_path, receipt)
    return dict(root=root, data=data, qualification=qualification, receipt=receipt,
        qualification_path=qualification_path, qualification_sha256=qualification_digest,
        receipt_path=receipt_path, receipt_sha256=receipt_digest, source=source, runtime_path=runtime_path,
        baseline=baseline, dataset=dataset)


def apply(value, descriptor):
    return adapter.add_platform_metadata(value['root'], value['data'],
        qualification_path=value['qualification_path'], qualification_sha256=value['qualification_sha256'],
        receipt_path=value['receipt_path'], receipt_sha256=value['receipt_sha256'], common_timing_mutex_fd=descriptor)


class PlatformMetadataTransaction(unittest.TestCase):
    def test_complete_actual_overlay_preserves_records_and_object_identities(self):
        for prose in ('Groups overlap. Java uses GraalVM native AOT builds in this cohort.',
                      'Language filters intersect Execution, Typing, Language style and Memory management categories. '
                      'Choices within a category are alternatives; empty categories include all. '
                      'Execution describes the measured runtime: Java uses GraalVM native AOT, Ruby uses YJIT and PHP uses tracing JIT.'):
            with self.subTest(prose=prose), tempfile.TemporaryDirectory() as directory, fixture_mutex() as mutex:
                value = package(Path(directory), group_prose=prose)
                data, old = value['data'], deepcopy(value['data'])
                maps = {name: data[name] for name in ('programs', 'files', 'provenance', 'protocol', 'languages', 'language_groups')}
                rows = dict(data['programs'])
                sample_objects = {key: row['samples'] for key, row in rows.items()}
                file_objects = {key: row['files'] for key, row in rows.items()}
                groups = [group['languages'] for group in data['language_groups']]
                outcome = apply(value, mutex.fileno())
                self.assertEqual(outcome['qualified_programs'], 100)
                self.assertIs(outcome['java_execution_repaired'], True)
                self.assertIs(outcome['java_backend_verification_complete'], True)
                self.assertEqual((outcome['java_verified_profiles'], outcome['java_unclassified_profiles']), (17, 0))
                for name, original in maps.items(): self.assertIs(data[name], original)
                for index, group in enumerate(data['language_groups']): self.assertIs(group['languages'], groups[index])
                for key, row in data['programs'].items():
                    self.assertIs(row, rows[key])
                    self.assertIs(row['samples'], sample_objects[key])
                    self.assertIs(row['files'], file_objects[key])
                    for name, original in old['programs'][key].items():
                        if name not in ('runtime_policy', 'execution_mode'): self.assertEqual(row[name], original)
                for name in ('files', 'languages', 'benchmarks', 'reference_summary'):
                    self.assertEqual(data[name], old[name])
                self.assertNotIn('java', data['language_groups'][0]['languages'])
                self.assertIn('java', data['language_groups'][1]['languages'])
                self.assertNotIn('Java uses GraalVM native AOT', data['protocol']['language_groups'])
                self.assertEqual(data['protocol']['metric'], 'preserved')

    def test_mixed_java_verification_clears_legacy_modes_and_preserves_provenance(self):
        for prose in ('Groups overlap. Java uses GraalVM native AOT builds in this cohort.',
                      'Execution describes the measured runtime: Java uses GraalVM native AOT, Ruby uses YJIT.'):
            with self.subTest(prose=prose), tempfile.TemporaryDirectory() as directory, fixture_mutex() as mutex:
                value = package(Path(directory), unresolved=True, group_prose=prose)
                old = deepcopy(value['data'])
                outcome = apply(value, mutex.fileno())
                for key in ('work:java-15', 'work:java-16'):
                    row = value['data']['programs'][key]
                    self.assertIs(row['measured_backend']['verified'], False)
                    self.assertTrue(row['measured_backend']['unresolved_reason'])
                    self.assertEqual(row['runtime_policy'], 'Unclassified')
                    self.assertEqual(row['execution_mode'], 'unknown')
                    prior = row['prior_runtime_metadata']
                    self.assertEqual(prior['fields'], {name: old['programs'][key][name] for name in ('runtime_policy', 'execution_mode')})
                    self.assertEqual(prior['dataset'], dict(path=value['baseline'].relative_to(value['root']).as_posix(),
                        sha256=value['qualification']['pins'][str(value['baseline'])],
                        origin_path='overview/explorer/dist/data.json',
                        origin_sha256=value['qualification']['pins'][str(value['dataset'])]))
                    self.assertEqual(prior['scope'], 'preserved-display-fields-not-runtime-verification')
                    for name in ('seconds', 'samples', 'files', 'report', 'loc'):
                        self.assertEqual(row[name], old['programs'][key][name])
                self.assertEqual(outcome['qualified_programs'], 98)
                self.assertIs(outcome['java_execution_repaired'], True)
                self.assertIs(outcome['java_backend_verification_complete'], False)
                self.assertEqual((outcome['java_verified_profiles'], outcome['java_unclassified_profiles']), (15, 2))
                self.assertTrue(all('java' not in group['languages'] for group in value['data']['language_groups']
                                    if group['key'] in ('aot', 'jit', 'interpreted')))
                self.assertIn('15 of 17', value['data']['protocol']['java'])
                self.assertIn('2 remain Unclassified', value['data']['protocol']['java'])
                self.assertNotIn('Java uses GraalVM native AOT', value['data']['protocol']['language_groups'])
                self.assertEqual(value['data']['files'], old['files'])
                # A subsequent producer run may have rewritten its generated
                # output and added independent new-language records. Only the
                # immutable qualified baseline is historical evidence.
                data = value['data']
                unrelated = dict(key='other:rust-new', id='rust-new', benchmark='other', language='rust',
                                 seconds=3.0, samples=[2.0, 3.0, 4.0], loc=42, files=[], report='other.json')
                data['programs'][unrelated['key']] = unrelated
                data['languages'].append(dict(key='rust', label='Rust', release_year=2010))
                generated = write(value['dataset'], data)
                self.assertNotEqual(generated, value['qualification']['pins'][str(value['dataset'])])
                before_rebuild = deepcopy(data)
                rebuilt = apply(value, mutex.fileno())
                self.assertEqual(rebuilt['qualified_programs'], 98)
                self.assertEqual(data, before_rebuild)
                self.assertIs(data['programs'][unrelated['key']], unrelated)

    def test_late_changed_source_report_receipt_or_provenance_leaves_input_unchanged(self):
        for mutation in ('source', 'report', 'receipt', 'provenance', 'baseline', 'late-read'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory, fixture_mutex() as mutex:
                value = package(Path(directory))
                if mutation == 'source': value['source'].write_text('changed source\n')
                elif mutation == 'report': (value['root'] / 'reports/java.json').write_text('changed report\n')
                elif mutation == 'receipt': value['receipt_path'].write_text('changed receipt\n')
                elif mutation == 'provenance': value['data']['provenance'][value['qualification_path'].relative_to(value['root']).as_posix()] = 'f' * 64
                elif mutation == 'baseline': value['baseline'].write_text('changed immutable baseline\n')
                before = deepcopy(value['data'])
                if mutation == 'late-read':
                    phase = dict(staged=False, injected=0)
                    original_stage, original_read = adapter.stage, Path.read_bytes
                    target = value['runtime_path'].resolve()
                    def staged(*args, **kwargs):
                        pending = original_stage(*args, **kwargs)
                        phase['staged'] = True
                        return pending
                    def failing_read(path):
                        if phase['staged'] and path.resolve() == target:
                            phase['injected'] += 1
                            raise OSError('Synthetic read failure after all hundred rows were staged')
                        return original_read(path)
                    with mock.patch.object(adapter, 'stage', staged), mock.patch.object(Path, 'read_bytes', failing_read):
                        with self.assertRaises(OSError): apply(value, mutex.fileno())
                    self.assertTrue(phase['staged'])
                    self.assertEqual(phase['injected'], 1)
                else:
                    with self.assertRaises((ValueError, FileNotFoundError)):
                        apply(value, mutex.fileno())
                self.assertEqual(value['data'], before)

    def test_partial_typed_or_foreign_descriptor_evidence_is_transactional(self):
        for mutation in ('missing-id', 'string-verified', 'sample-change', 'skipped', 'foreign-descriptor', 'unknown-aot-prose', 'prior-metadata-collision'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory, fixture_mutex() as mutex:
                value = package(Path(directory), unresolved=mutation == 'prior-metadata-collision')
                qualification, receipt = value['qualification'], value['receipt']
                if mutation == 'missing-id': qualification['annotations'].pop('work:java-16')
                elif mutation == 'string-verified': qualification['annotations']['work:java-16']['verified'] = 'true'
                elif mutation == 'sample-change': value['data']['programs']['work:java-16']['samples'][0] = 99.0
                elif mutation == 'skipped': receipt['skips'] = [['fixture', 'skipped']]
                elif mutation == 'unknown-aot-prose': value['data']['protocol']['language_groups'] = 'Java is measured using local GraalVM NativeAOT.'
                elif mutation == 'prior-metadata-collision': value['data']['programs']['work:java-16']['prior_runtime_metadata'] = {'fields': 'different prior fields'}
                if mutation in ('missing-id', 'string-verified'):
                    value['qualification_sha256'] = write(value['qualification_path'], qualification)
                    receipt['metadata_output']['sha256'] = value['qualification_sha256']
                if mutation in ('missing-id', 'string-verified', 'skipped'):
                    value['receipt_sha256'] = write(value['receipt_path'], receipt)
                before = deepcopy(value['data'])
                if mutation == 'foreign-descriptor':
                    other = value['root'] / 'unrelated.lock'
                    with other.open('a+') as foreign, self.assertRaises(ValueError): apply(value, foreign.fileno())
                else:
                    with self.assertRaises(ValueError): apply(value, mutex.fileno())
                self.assertEqual(value['data'], before)
