"""Bounded subset-admission fixtures; synthetic measurements never publish.

The qualifying worker must hold the common timing mutex before importing this
module and throughout all four methods. These fixtures never execute a native
program, probe a runtime, or change the explorer's dataset or chart artifacts.
"""
from copy import deepcopy
from contextlib import ExitStack, contextmanager
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import native_languages_data_v2 as cohort
import native_languages_ready_v1 as ready
from test_native_languages_data_v1 import fixture, first_candidate


def selection_packet(languages=('crystal',)):
    return dict(status='reviewed-completed-native-language-subset',
        ready_languages=sorted(languages), unresolved=[],
        future_requested_languages=['elixir', 'kotlin'],
        languages={language: {kind: dict(path='/synthetic-only/'+language+'/'+kind+'.json', sha256='d'*64)
            for kind in ('suite', 'canonical_loc', 'metadata')} for language in languages})


def initial_data():
    helper_sha, control_sha = 'c'*64, 'f'*64
    programs, benchmarks = {}, []
    for benchmark in sorted(cohort.BENCHMARKS):
        key = 'old-control' if benchmark=='binarytrees' else 'old-control-'+benchmark
        # Legacy chart producers legitimately reuse an ID across workloads;
        # benchmark-qualified map keys, rather than legacy IDs, are unique.
        programs[key] = dict(key=key, id='legacy-control-shared-id', language='mech', benchmark=benchmark,
            samples=[0.1, 0.2, 0.3], seconds=0.2, loc=7, files=[control_sha],
            x=1.0, y=1.0, retained='original control')
        benchmarks.append(dict(key=benchmark, baseline_loc=7, baseline_seconds=0.2,
            groups=[dict(language='mech', programs=[key], fast=key, small=key, retained='original group')]))
    return dict(programs=programs,
        files={helper_sha: dict(name='existing helper', path='existing/helper', sha256=helper_sha,
            role='existing role', code='existing helper text', extra='preserve exactly'),
            control_sha: dict(name='existing control', path='existing/control', sha256=control_sha,
                role='application', code='synthetic unchanged control source', extra='preserve exactly')},
        provenance={'old-receipt':'unchanged'}, protocol={'existing':'unchanged',
            'language_groups':'Language filters intersect Execution, Typing, Language style and Memory management categories.'},
        languages=[dict(key='mech', label='Mech', execution='AOT', retained='original language')],
        benchmarks=benchmarks,
        reference_summary=[dict(lang='mech', label='Mech', fast=dict(x=1.0, y=1.0), small=dict(x=1.0, y=1.0))],
        language_groups=[dict(key=key, languages=['mech'] if key=='aot' else [])
            for key in ('aot', 'jit', 'interpreted', 'array', 'functional', 'dynamic')])


def pure_entry(language):
    """Use the unchanged 20-source cohort constructor and real stage contract."""
    suite, counts, meta = fixture(language)
    report = 'synthetic-only/'+language+'/measurement.json'
    runtime = 'synthetic-only/'+language+'/runtime.json'
    metadata = 'synthetic-only/'+language+'/metadata.json'
    meta.update(memory_strategies=['gc'], memory_label='Synthetic runtime GC, never published',
        publication_metadata_evidence=dict(path=metadata, sha256='d'*64),
        measured_backend_template=dict(key='native-aot', execution='aot', verified=True,
            label='Synthetic actual AOT receipt, never published',
            runtime_receipt=dict(report=runtime, report_sha256='e'*64, verified=True,
                field='/actual_version', value='Actual synthetic saved version\n', value_encoding='literal-saved-string')))
    for evidence in suite['benchmarks'].values():
        evidence.update(report='/'+report, report_sha256='b'*64)
        for row in evidence['profiles'].values():
            if row['language'] != language:
                continue
            row.update(command=['/synthetic-only/'+language+'/never-execute', row['id']],
                time_ratio_to_matched_mech=10.0)
    rows = cohort.validate_cohort(language, suite, counts, meta)
    for item in rows:
        item.update(report_name=report, counts_name='synthetic-only/'+language+'/canonical_loc.json')
        item['source_files'] = [dict(source, code='synthetic source, never execute\n',
            public_path=source['path'].lstrip('/')) for source in item['metric']['compiled_application_files']]
    entry = selection_packet((language,))['languages'][language]
    return dict(entry=entry, meta=meta, rows=rows,
        actual_runtime_report=dict(actual_version='Actual synthetic saved version\n')), {
            report:'b'*64, runtime:'e'*64, metadata:'d'*64}


def guarded_fixture(root):
    """Actual temporary byte identities exercise every guarded read boundary."""
    def sha(path):
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()

    def save(path, value):
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, indent=2)+'\n')
        return dict(path=str(path), sha256=sha(path))

    synthetic = root/'synthetic-only'
    proof = synthetic/'proof.json'
    save(proof, dict(synthetic=True, never_publish=True))
    helper = synthetic/'shared-helper.txt'
    helper.write_text('synthetic helper; never compile or publish\n')
    binary = synthetic/'not-an-executable.txt'
    binary.write_text('synthetic binary identity; never execute\n')
    runtime = synthetic/'crystal'/'runtime.json'
    save(runtime, dict(actual_version='Actual synthetic saved version\n'))
    proof_sha, helper_sha, binary_sha, runtime_sha = (sha(path) for path in (proof, helper, binary, runtime))
    suite, counts, meta = fixture('crystal')
    owner = dict(status='passed', review_identity='synthetic, never published',
        evidence_path=str(proof), evidence_sha256=proof_sha)
    counts.update(fixture_receipt=dict(path=str(proof), sha256=proof_sha, status='passed'),
        grammar_snapshot_pins={str(proof):proof_sha},
        replay_receipts=[dict(path=str(proof), sha256=proof_sha)], pins={str(proof):proof_sha})
    suite['provenance'] = {str(proof):proof_sha, str(binary):binary_sha}
    meta.update(memory_strategies=['gc'], pins={str(proof):proof_sha, str(binary):binary_sha, str(runtime):runtime_sha},
        measured_backend_template=dict(key='native-aot', execution='aot', verified=True,
            label='Synthetic actual AOT receipt, never published',
            runtime_receipt=dict(report=str(runtime), report_sha256=runtime_sha, verified=True,
                field='/actual_version', value='Actual synthetic saved version\n', value_encoding='literal-saved-string')))
    for evidence in suite['benchmarks'].values():
        evidence.update(report=str(proof), report_sha256=proof_sha)
        for row in evidence['profiles'].values():
            if row['language'] != 'crystal':
                continue
            source = synthetic/(row['id']+'.txt')
            source.write_text('synthetic '+row['id']+'; never compile or publish\n')
            source_sha = sha(source)
            row.update(source=str(source), source_sha256=source_sha, binary_sha256=binary_sha,
                command=[str(binary), row['id']], time_ratio_to_matched_mech=10.0,
                source_bundle=[dict(path=str(source), sha256=source_sha), dict(path=str(helper), sha256=helper_sha)])
            metric = counts['profiles'][row['id']]
            metric.update(source=str(source), source_sha256=source_sha, semantic_review=deepcopy(owner))
            for file, path in zip(metric['source_bundle'], (source, helper)):
                file.update(path=str(path), sha256=sha(path), ownership_review=deepcopy(owner),
                    count_receipt=dict(path=str(proof), sha256=proof_sha))
                counts['pins'][str(path)] = sha(path)
                suite['provenance'][str(path)] = sha(path)
            counts['compiler_bound_input_inventory'][row['id']] = dict(
                status='qualified-exact-authored-compiler-inputs', source_sha256=source_sha,
                binary_sha256=binary_sha, evidence_path=str(proof), evidence_sha256=proof_sha,
                files=deepcopy(row['source_bundle']))
    artifacts = dict(suite=suite, canonical_loc=counts, metadata=meta)
    packet = selection_packet()
    packet['languages']['crystal'] = {kind: save(synthetic/'crystal'/(kind+'.json'), value)
        for kind, value in artifacts.items()}
    packet_path = root/'continuation'/'synthetic-ready-never-publish.json'
    packet_pin = save(packet_path, packet)
    data = initial_data()
    data['files'].pop('c'*64)
    data['files'][helper_sha] = dict(name='existing helper', path='existing/helper', sha256=helper_sha,
        role='existing role', code=helper.read_text(), extra='preserve exactly')
    return data, packet_path, packet_pin['sha256'], packet, artifacts, helper_sha, save


@contextmanager
def guarded_dependency_sources(root):
    """Keep actual adapter bytes inside the temporary benchmark root.

    No module is loaded from these copies. Scoped path substitutions exercise
    the guarded dependency reads and are restored before leaving the fixture.
    """
    copied = root/'fixture-adapter-sources'
    copied.mkdir()
    modules = (ready.original, ready.platform, ready.groups, ready.endpoints, ready)
    with ExitStack() as stack:
        for module in modules:
            original_path = Path(module.__file__).resolve(strict=True)
            path = copied/original_path.name
            path.write_bytes(original_path.read_bytes())
            stack.enter_context(patch.object(module, '__file__', str(path)))
        yield


class NativeReadyAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # The fresh fixture worker owns this same descriptor for the full suite.
        # Opening a second descriptor would create a separate lock ownership.
        cls.lock_fd = int(os.environ['MECH_CLBG_TIMING_LOCK_FD'])
        cohort.held_mutex(cls.lock_fd)

    def test_crystal_only_keeps_unchanged_twenty_source_ninety_process_validation(self):
        packet = selection_packet()
        self.assertEqual(set(ready.validate_selection(packet)), {'crystal'})
        suite, counts, meta = fixture('crystal')
        before = deepcopy((suite, counts, meta))
        rows = cohort.validate_cohort('crystal', suite, counts, meta)
        self.assertEqual(len(rows), 20)
        self.assertEqual({item['benchmark'] for item in rows}, cohort.BENCHMARKS)
        self.assertTrue(all(item['metric']['normalized_loc']==15 and item['metric']['physical_loc']==12 for item in rows))
        self.assertEqual((suite, counts, meta), before)

    def test_partial_unknown_eighty_nine_runtime_and_unqualified_loc_reject(self):
        for language in ('v', 'red', 'elixir', 'kotlin', 'unknown'):
            with self.subTest(language=language), self.assertRaises(ValueError):
                ready.validate_selection(selection_packet((language,)))
        mutations = [
            lambda p: p.update(languages={}, ready_languages=[]),
            lambda p: p.update(ready_languages=[]),
            lambda p: p.update(ready_languages=['crystal', 'crystal']),
            lambda p: p.update(ready_languages=['nim', 'crystal']),
            lambda p: p.update(status='reviewed-eight-language-publication'),
            lambda p: p.update(unresolved=['pending actual evidence']),
            lambda p: p['languages']['crystal'].pop('canonical_loc'),
        ]
        for index, mutate in enumerate(mutations):
            packet = selection_packet()
            mutate(packet)
            with self.subTest(selection=index), self.assertRaises((ValueError, KeyError)):
                ready.validate_selection(packet)
        unsorted = selection_packet(('nim', 'crystal'))
        unsorted['ready_languages'] = ['nim', 'crystal']
        with self.assertRaises(ValueError):
            ready.validate_selection(unsorted)
        for alteration in ('missing-workload', 'missing-form', '89-processes', 'runtime-planned', 'loc-unqualified', 'omitted-helper'):
            suite, counts, meta = fixture('crystal')
            if alteration=='missing-workload': suite['benchmarks'].pop('fasta')
            elif alteration=='missing-form': suite['benchmarks']['fasta']['profiles'].pop('crystal-fasta-compact')
            elif alteration=='89-processes': suite['canonical_full_native_processes']=89
            elif alteration=='runtime-planned': meta['actual_native_architecture_verified']=False
            elif alteration=='loc-unqualified': counts['canonical_LOC_admitted']=False
            else:
                metric = counts['profiles'][first_candidate(suite)['id']]
                metric['source_bundle'].pop()
                metric.update(canonical_loc=10, physical_loc=8)
            before = deepcopy((suite, counts, meta))
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                cohort.validate_cohort('crystal', suite, counts, meta)
            self.assertEqual((suite, counts, meta), before)

    def test_pure_subset_platform_and_groups_preserve_prior_cohort_and_controls(self):
        data = initial_data()
        before = deepcopy(data)
        nim, nim_provenance = pure_entry('nim')
        admitted = ready.prepare_ready(data, {'nim':nim}, 'continuation/synthetic-nim.json', nim_provenance)
        self.assertEqual(data, before)
        prior = deepcopy(admitted)
        crystal, crystal_provenance = pure_entry('crystal')
        staged = ready.prepare_ready(admitted, {'crystal':crystal}, 'continuation/synthetic-crystal.json', crystal_provenance)
        self.assertEqual(admitted, prior)
        self.assertEqual(len(staged['programs']), 50)
        self.assertEqual(staged['programs']['old-control'], before['programs']['old-control'])
        for key, row in prior['programs'].items():
            self.assertEqual(staged['programs'][key], row)
        self.assertEqual(staged['files']['c'*64], before['files']['c'*64])
        self.assertEqual(staged['native_language_audit']['languages']['nim'], prior['native_language_audit']['languages']['nim'])
        rows = [row for row in staged['programs'].values() if row['language']=='crystal']
        self.assertEqual(len(rows), 20)
        self.assertTrue(all(row['measured_backend']['key']=='native-aot' and
            row['measured_backend']['execution']=='aot' and row['measured_backend']['verified'] is True for row in rows))
        self.assertTrue(all(row['loc']==15 and row['samples']==[1.0,2.0,3.0] and row['seconds']==2.0 for row in rows))
        memberships = {group['key']:group['languages'] for group in staged['language_groups']}
        self.assertEqual(set(memberships['aot']), {'mech', 'nim', 'crystal'})
        self.assertFalse(any('crystal' in memberships[key] for key in ('jit', 'interpreted', 'array', 'functional', 'dynamic')))
        self.assertFalse({'elixir', 'kotlin', 'v', 'red'} & {row['key'] for row in staged['languages']})
        corrupt_endpoint = deepcopy(staged)
        next(item for item in corrupt_endpoint['benchmarks'] if item['key']=='spectralnorm')['baseline_loc'] = 0
        unchanged_corrupt_endpoint = deepcopy(corrupt_endpoint)
        with self.assertRaises(ValueError): ready.finalize_ready_endpoints(corrupt_endpoint, {'crystal'})
        self.assertEqual(corrupt_endpoint, unchanged_corrupt_endpoint)
        final = deepcopy(staged)
        # An unrelated historical sample boundary is not requalified by the
        # newly completed cohort's endpoint selection.
        final['programs']['old-control']['samples'] = [0.2]
        old_control = deepcopy(final['programs']['old-control'])
        ready.finalize_ready_endpoints(final, {'crystal'})
        self.assertEqual(final['programs']['old-control'], old_control)
        self.assertEqual([(item['baseline_loc'], item['baseline_seconds']) for item in final['benchmarks']],
            [(item['baseline_loc'], item['baseline_seconds']) for item in before['benchmarks']])
        preserved_controls = deepcopy(before['programs']); preserved_controls['old-control'] = old_control
        self.assertEqual({key:row for key,row in final['programs'].items() if row['language']=='mech'}, preserved_controls)
        self.assertEqual({key:row for key,row in final['programs'].items() if row['language']=='nim'},
            {key:row for key,row in prior['programs'].items() if row['language']=='nim'})
        self.assertEqual(sum(row['language']=='crystal' for row in final['programs'].values()), 10)

    def test_guarded_late_failures_are_atomic_and_matching_admission_is_idempotent(self):
        with tempfile.TemporaryDirectory(prefix='mech-ready-subset-fixture-') as directory:
            root = Path(directory).resolve(strict=True)
            data, packet_path, packet_sha, packet, artifacts, helper_sha, save = guarded_fixture(root)
            original = deepcopy(data)
            target = root/'synthetic-only'/'crystal-spectralnorm-optimized.txt'
            read_bytes = Path.read_bytes

            def fail_late(path, *args, **kwargs):
                if path.resolve()==target:
                    raise OSError('synthetic late source read failure')
                return read_bytes(path, *args, **kwargs)

            with guarded_dependency_sources(root):
                with patch.object(Path, 'read_bytes', fail_late), self.assertRaisesRegex(OSError, 'synthetic late'):
                    ready.add_ready_native_languages(root, data, packet_path=packet_path, packet_sha256=packet_sha,
                        common_timing_mutex_fd=self.lock_fd, finalize=False)
                self.assertEqual(data, original)
                template = artifacts['metadata']['measured_backend_template']
                template['runtime_receipt']['value'] = 'Guessed runtime version'
                packet['languages']['crystal']['metadata'] = save(packet['languages']['crystal']['metadata']['path'], artifacts['metadata'])
                packet_sha = save(packet_path, packet)['sha256']
                with self.assertRaises(ValueError):
                    ready.add_ready_native_languages(root, data, packet_path=packet_path, packet_sha256=packet_sha,
                        common_timing_mutex_fd=self.lock_fd, finalize=False)
                self.assertEqual(data, original)
                template['runtime_receipt']['value'] = 'Actual synthetic saved version\n'
                packet['languages']['crystal']['metadata'] = save(packet['languages']['crystal']['metadata']['path'], artifacts['metadata'])
                packet_sha = save(packet_path, packet)['sha256']
                identities = {key:id(data[key]) for key in ('programs', 'files', 'provenance', 'protocol', 'languages', 'benchmarks', 'language_groups')}
                ready.add_ready_native_languages(root, data, packet_path=packet_path, packet_sha256=packet_sha,
                    common_timing_mutex_fd=self.lock_fd, finalize=False)
                self.assertEqual(len(data['programs']), 30)
                self.assertEqual(data['programs']['old-control'], original['programs']['old-control'])
                self.assertEqual(data['files'][helper_sha], original['files'][helper_sha])
                self.assertEqual(identities, {key:id(data[key]) for key in identities})
                admitted = deepcopy(data)
                ready.add_ready_native_languages(root, data, packet_path=packet_path, packet_sha256=packet_sha,
                    common_timing_mutex_fd=self.lock_fd, finalize=False)
                self.assertEqual(data, admitted)
                candidate = next(row for row in data['programs'].values() if row['language']=='crystal')
                candidate['seconds'] += 1.0
                conflicting = deepcopy(data)
                with self.assertRaises(ValueError):
                    ready.add_ready_native_languages(root, data, packet_path=packet_path, packet_sha256=packet_sha,
                        common_timing_mutex_fd=self.lock_fd, finalize=False)
                self.assertEqual(data, conflicting)


if __name__=='__main__':
    unittest.main(verbosity=2)
