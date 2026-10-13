"""Synthetic admission corruption checks; no native benchmark is executed.

Run these only under the caller-held common timing mutex. Synthetic seconds,
sources and proof shapes exercise the importer and never enter a dataset.
"""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import statistics
import tempfile
import unittest
from unittest.mock import patch

import native_languages_data_v1 as importer


def fixture(language='c3'):
    counts = dict(language=language, status='qualified-exact-source-LOC', profile_count=20,
        canonical_LOC_admitted=True, chart_admission=False,
        complete_authored_source_bundles=True, unresolved=[], profiles={}, compiler_bound_input_inventory={},
        fixture_receipt=dict(path='/synthetic-only/fixture', sha256='f'*64, status='passed'),
        grammar_snapshot_pins={'/synthetic-only/grammar':'e'*64},
        replay_receipts=[dict(path='/synthetic-only/replay',sha256='d'*64)],
        pins={'/synthetic-only/fixture':'f'*64, '/synthetic-only/replay':'d'*64,
              '/synthetic-only/review':'9'*64, '/synthetic-only/grammar':'e'*64})
    ownership = dict(status='passed',review_identity='synthetic',evidence_path='/synthetic-only/review',evidence_sha256='9'*64)
    suite = dict(language=language,
        status='ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication',
        canonical_full_native_processes=90, candidate_sources=20, actual_samples_per_source=3,
        source_timing_relabels=0, source_or_report_mutations=0, chart_admitted=False, benchmarks={})
    meta = dict(language=language, label='Synthetic fixture, never published',
        status='reviewed-actual-native-language-metadata', actual_compiler_version_verified=True,
        actual_native_architecture_verified=True, actual_memory_policy_verified=True,
        release_year=2019, release_source='https://example.invalid/synthetic-fixture',
        release_note='Synthetic test input', backend_label='Synthetic test input', memory_label='Synthetic test input',
        execution='JIT' if language in {'gleam','clojure'} else 'AOT', color='#123456', color_source='documented display fallback',
        runtime_support_by_benchmark={b:'Synthetic test input' for b in importer.BENCHMARKS},
        timing_boundary='Synthetic test input', source_provenance='Synthetic test input')
    for benchmark in sorted(importer.BENCHMARKS):
        profiles = {}
        for variant in ('compact', 'optimized'):
            identity = language + '-' + benchmark + '-' + variant
            source = '/synthetic-only/' + identity
            files = [dict(path=source, sha256='a'*64, normalized_loc=10, physical_loc=8,
                          role='application', ownership_review=deepcopy(ownership), unresolved_diagnostics=[],
                          count_receipt=dict(path='/synthetic-only/replay',sha256='d'*64)),
                     dict(path=source+'-helper', sha256='c'*64, normalized_loc=5, physical_loc=4,
                          role='application-helper', ownership_review=deepcopy(ownership), unresolved_diagnostics=[],
                          count_receipt=dict(path='/synthetic-only/replay',sha256='d'*64))]
            row = dict(id=identity, key=benchmark+':'+identity, benchmark=benchmark, language=language,
                variant=variant, source=source, source_sha256='a'*64, binary_sha256='b'*64,
                source_bundle=[dict(path=f['path'], sha256=f['sha256']) for f in files],
                eligible_for_chart=False, publication_status='records-reconciled-awaiting-qualified-canonical-LOC',
                samples_seconds=[1.0,2.0,3.0], median_seconds=2.0,
                sample_stdev_seconds=statistics.stdev([1.0,2.0,3.0]),
                sample_validation=[dict(reference_match=True) for _ in range(3)],
                sample_execution_metadata=[dict(synthetic=True) for _ in range(3)])
            profiles[identity] = row
            counts['profiles'][row['id']] = dict(benchmark=benchmark,variant=variant,
                source=source, source_sha256='a'*64, semantic_review=deepcopy(ownership),
                source_bundle=files, canonical_loc=15, physical_loc=12, expansions=[])
            counts['compiler_bound_input_inventory'][row['id']] = dict(status='qualified-exact-authored-compiler-inputs',
                source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                evidence_path='/synthetic-only/review', evidence_sha256='9'*64,
                files=deepcopy(row['source_bundle']))
        profiles['mech'] = dict(language='mech', samples_seconds=[0.1,0.2,0.3], median_seconds=0.2)
        suite['benchmarks'][benchmark] = dict(profiles=profiles)
    return suite, counts, meta


def first_candidate(suite):
    return next(row for row in suite['benchmarks']['binarytrees']['profiles'].values() if row['language']!='mech')


class NativeAdmission(unittest.TestCase):
    def test_complete_shape_preserves_all_source_counts_and_inputs(self):
        for language in importer.LANGUAGES:
            suite, counts, meta = fixture(language)
            original = deepcopy((suite, counts, meta))
            rows = importer.validate_cohort(language, suite, counts, meta)
            self.assertEqual(len(rows),20)
            self.assertEqual({row['benchmark'] for row in rows}, importer.BENCHMARKS)
            self.assertTrue(all(row['metric']['normalized_loc']==15 for row in rows))
            self.assertEqual((suite, counts, meta), original)

    def test_v_and_partial_suite_are_rejected(self):
        suite, counts, meta = fixture()
        with self.assertRaises(ValueError):
            importer.validate_cohort('v', suite, counts, meta)
        for alteration in ('missing-workload', 'missing-form', 'bounded-process-count'):
            suite, counts, meta = fixture()
            if alteration=='missing-workload':suite['benchmarks'].pop('fasta')
            elif alteration=='missing-form':suite['benchmarks']['fasta']['profiles'].pop('c3-fasta-compact')
            else:suite['canonical_full_native_processes']=89
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                importer.validate_cohort('c3', suite, counts, meta)

    def test_changed_binary_or_omitted_authored_helper_cannot_use_old_count(self):
        for alteration in ('changed-source', 'omitted-helper', 'wrong-total', 'boolean-count'):
            suite, counts, meta = fixture()
            row = first_candidate(suite)
            metric = counts['profiles'][row['id']]
            if alteration=='changed-source':metric['source_sha256']='d'*64
            elif alteration=='omitted-helper':metric['source_bundle'].pop()
            elif alteration=='wrong-total':metric['canonical_loc']=10
            else:metric['source_bundle'][0]['normalized_loc']=True
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                importer.validate_cohort('c3', suite, counts, meta)

    def test_altered_median_output_or_unresolved_parser_evidence_is_rejected(self):
        for alteration in ('median', 'output', 'diagnostic', 'unreviewed', 'unbound-grammar'):
            suite, counts, meta = fixture()
            row = first_candidate(suite)
            metric = counts['profiles'][row['id']]
            if alteration=='median':row['median_seconds']=1.0
            elif alteration=='output':row['sample_validation'][1]['reference_match']=False
            elif alteration=='diagnostic':metric['source_bundle'][0]['unresolved_diagnostics']=['unresolved owner']
            elif alteration=='unreviewed':counts['status']='source-proposals-awaiting-semantic-review'
            else:counts['grammar_snapshot_pins']['/synthetic-only/grammar']='0'*64
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                importer.validate_cohort('c3', suite, counts, meta)

    def test_planned_backend_or_missing_library_provenance_cannot_be_published(self):
        for alteration in ('planned-backend', 'planned-architecture', 'planned-memory', 'library-provenance'):
            suite, counts, meta = fixture()
            if alteration=='planned-backend':meta['actual_compiler_version_verified']=False
            elif alteration=='planned-architecture':meta['actual_native_architecture_verified']=False
            elif alteration=='planned-memory':meta['actual_memory_policy_verified']=False
            else:meta['runtime_support_by_benchmark'].pop('regexredux')
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                importer.validate_cohort('c3', suite, counts, meta)

    def test_missing_compiler_inventory_cannot_hide_a_helper_from_reused_rows(self):
        for language in ('nim','pony','gleam','clojure'):
            for alteration in ('missing-inventory','empty-inventory','main-only-count'):
                suite, counts, meta = fixture(language)
                row = first_candidate(suite)
                row.pop('source_bundle')
                if alteration=='missing-inventory':
                    counts['compiler_bound_input_inventory'].pop(row['id'])
                elif alteration=='empty-inventory':
                    counts['compiler_bound_input_inventory'][row['id']]['files'] = []
                else:
                    metric = counts['profiles'][row['id']]
                    metric['source_bundle'].pop()
                    metric.update(canonical_loc=10,physical_loc=8)
                with self.subTest(language=language,alteration=alteration),self.assertRaises((ValueError,KeyError)):
                    importer.validate_cohort(language,suite,counts,meta)

    def test_different_id_cannot_overwrite_an_existing_candidate_key(self):
        suite, counts, meta = fixture()
        rows = [row for row in suite['benchmarks']['binarytrees']['profiles'].values() if row['language']=='c3']
        rows[1]['key'] = rows[0]['key']
        with self.assertRaisesRegex(ValueError,'source identity'):
            importer.validate_cohort('c3',suite,counts,meta)


def publication_fixture(root):
    """Real temporary files for transaction checks; every measurement is synthetic."""
    def sha(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()
    def save(path, value):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, indent=2) + '\n')
        return dict(path=str(path), sha256=sha(path))
    proof = root / 'synthetic-only' / 'proof.json'
    save(proof, dict(synthetic=True, never_publish=True))
    proof_pin = {str(proof):sha(proof)}
    helper = root / 'synthetic-only' / 'shared-helper.txt'
    helper.write_text('synthetic helper; never compile or publish\n')
    helper_sha = sha(helper)
    binary = root / 'synthetic-only' / 'not-an-executable.txt'
    binary.write_text('synthetic binary identity; never execute\n')
    packet = dict(status='reviewed-six-language-publication', languages={}, unresolved=[])
    artifacts = {}
    for language in sorted(importer.LANGUAGES):
        suite, counts, meta = fixture(language)
        counts['fixture_receipt'] = dict(path=str(proof),sha256=sha(proof),status='passed')
        counts['grammar_snapshot_pins'] = dict(proof_pin)
        counts['replay_receipts'] = [dict(path=str(proof),sha256=sha(proof))]
        counts['pins'] = dict(proof_pin)
        meta['provenance'] = dict(proof_pin)
        suite['provenance'] = dict(proof_pin)
        owner = dict(status='passed', review_identity='synthetic, never published',
                     evidence_path=str(proof), evidence_sha256=sha(proof))
        for benchmark, evidence in suite['benchmarks'].items():
            evidence.update(report=str(proof),report_sha256=sha(proof))
            for row in evidence['profiles'].values():
                if row['language']=='mech':
                    continue
                source = root / 'synthetic-only' / (row['id']+'.txt')
                source.write_text('synthetic '+row['id']+'; never compile or publish\n')
                row.update(source=str(source),source_sha256=sha(source),binary_sha256=sha(binary),
                           command=[str(binary)],time_ratio_to_matched_mech=10.0,
                           source_bundle=[dict(path=str(source),sha256=sha(source)),
                                          dict(path=str(helper),sha256=helper_sha)])
                metric = counts['profiles'][row['id']]
                metric.update(source=str(source),source_sha256=sha(source),semantic_review=deepcopy(owner))
                for file, path in zip(metric['source_bundle'], (source,helper)):
                    file.update(path=str(path),sha256=sha(path),ownership_review=deepcopy(owner),
                                count_receipt=dict(path=str(proof),sha256=sha(proof)))
                    counts['pins'][str(path)] = sha(path)
                    suite['provenance'][str(path)] = sha(path)
                counts['compiler_bound_input_inventory'][row['id']] = dict(
                    status='qualified-exact-authored-compiler-inputs',source_sha256=sha(source),
                    binary_sha256=sha(binary),evidence_path=str(proof),evidence_sha256=sha(proof),
                    files=deepcopy(row['source_bundle']))
        suite['provenance'][str(binary)] = sha(binary)
        artifacts[language] = dict(suite=suite,canonical_loc=counts,metadata=meta)
        packet['languages'][language] = {}
        for kind, value in artifacts[language].items():
            packet['languages'][language][kind] = save(root/'synthetic-only'/language/(kind+'.json'),value)
    packet_path = root/'continuation'/'synthetic-never-publish.json'
    save(packet_path,packet)
    old_helper = dict(name='existing helper',path='existing/path',sha256=helper_sha,
                      role='existing role',loc_note='Existing note',code=helper.read_text(),extra='preserve')
    data = dict(programs={'old-control':dict(language='mech',samples=[0.1,0.2,0.3])},
                files={helper_sha:old_helper},provenance={'old-receipt':'unchanged'},protocol={'existing':'unchanged'},
                languages=[dict(key='mech',label='Mech')],
                benchmarks=[dict(key=b,groups=[dict(language='mech',programs=['old-control'])])
                            for b in sorted(importer.BENCHMARKS)])
    return data, packet_path, packet, artifacts, helper_sha, save


class NativePublicationTransaction(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # The qualifying runner holds this descriptor throughout the fixture
        # suite. Opening a second lock here would deadlock with that owner.
        cls.lock_fd = int(os.environ['MECH_CLBG_TIMING_LOCK_FD'])
        importer.held_mutex(cls.lock_fd)

    def test_complete_import_preserves_existing_records_metadata_and_map_identities(self):
        with tempfile.TemporaryDirectory(prefix='mech-native-import-fixture-') as directory:
            root = Path(directory)
            data, packet_path, packet, artifacts, helper_sha, save = publication_fixture(root)
            original = deepcopy(data)
            identities = {key:id(data[key]) for key in ('programs','files','provenance','protocol','languages','benchmarks')}
            result = importer.add_native_languages(root,data,packet_path=packet_path,
                                                   common_timing_mutex_fd=self.lock_fd)
            self.assertEqual(set(result),importer.LANGUAGES)
            self.assertEqual(len(data['programs']),121)
            self.assertEqual(data['programs']['old-control'],original['programs']['old-control'])
            self.assertEqual(data['files'][helper_sha],original['files'][helper_sha])
            self.assertEqual(data['languages'][0],original['languages'][0])
            self.assertEqual(identities,{key:id(data[key]) for key in identities})
            new_rows = [row for row in data['programs'].values() if row['language']!='mech']
            self.assertTrue(all(row['file_roles'][helper_sha]['normalized_loc']==5 for row in new_rows))
            self.assertFalse(any(row['language']=='v' for row in new_rows))

    def test_late_changed_report_leaves_entire_input_unchanged(self):
        with tempfile.TemporaryDirectory(prefix='mech-native-import-fixture-') as directory:
            root = Path(directory)
            data, packet_path, packet, artifacts, helper_sha, save = publication_fixture(root)
            suite = artifacts['pony']['suite']
            suite['benchmarks']['regexredux']['report_sha256'] = '0'*64
            packet['languages']['pony']['suite'] = save(Path(packet['languages']['pony']['suite']['path']),suite)
            save(packet_path,packet)
            original = deepcopy(data)
            with self.assertRaisesRegex(ValueError,'Actual measured report changed'):
                importer.add_native_languages(root,data,packet_path=packet_path,
                                              common_timing_mutex_fd=self.lock_fd)
            self.assertEqual(data,original)

    def test_late_source_read_failure_leaves_entire_input_unchanged(self):
        with tempfile.TemporaryDirectory(prefix='mech-native-import-fixture-') as directory:
            root = Path(directory)
            data, packet_path, packet, artifacts, helper_sha, save = publication_fixture(root)
            target = root/'synthetic-only'/'pony-spectralnorm-optimized.txt'
            original = deepcopy(data)
            real_read = Path.read_text
            def guarded_read(path,*args,**kwargs):
                if path==target:
                    raise OSError('synthetic late source read failure')
                return real_read(path,*args,**kwargs)
            with patch.object(Path,'read_text',guarded_read),self.assertRaisesRegex(OSError,'synthetic late'):
                importer.add_native_languages(root,data,packet_path=packet_path,
                                              common_timing_mutex_fd=self.lock_fd)
            self.assertEqual(data,original)

    def test_cross_cohort_identity_collision_leaves_entire_input_unchanged(self):
        with tempfile.TemporaryDirectory(prefix='mech-native-import-fixture-') as directory:
            root = Path(directory)
            data, packet_path, packet, artifacts, helper_sha, save = publication_fixture(root)
            source_id = 'pony-binarytrees-compact'
            duplicate_id = 'c3-binarytrees-compact'
            suite, counts = artifacts['pony']['suite'], artifacts['pony']['canonical_loc']
            row = suite['benchmarks']['binarytrees']['profiles'].pop(source_id)
            row.update(id=duplicate_id,key='binarytrees:'+duplicate_id)
            suite['benchmarks']['binarytrees']['profiles'][duplicate_id] = row
            counts['profiles'][duplicate_id] = counts['profiles'].pop(source_id)
            counts['compiler_bound_input_inventory'][duplicate_id] = counts['compiler_bound_input_inventory'].pop(source_id)
            for kind in ('suite','canonical_loc'):
                packet['languages']['pony'][kind] = save(Path(packet['languages']['pony'][kind]['path']),artifacts['pony'][kind])
            save(packet_path,packet)
            original = deepcopy(data)
            with self.assertRaisesRegex(ValueError,'across publication cohorts'):
                importer.add_native_languages(root,data,packet_path=packet_path,
                                              common_timing_mutex_fd=self.lock_fd)
            self.assertEqual(data,original)

    def test_unrelated_descriptor_is_rejected_before_any_import(self):
        with tempfile.TemporaryDirectory(prefix='mech-native-import-fixture-') as directory:
            root = Path(directory)
            data, packet_path, packet, artifacts, helper_sha, save = publication_fixture(root)
            original = deepcopy(data)
            with (root/'wrong-lock').open('a+') as wrong_lock,self.assertRaisesRegex(ValueError,'Different timing descriptor'):
                importer.add_native_languages(root,data,packet_path=packet_path,
                                              common_timing_mutex_fd=wrong_lock.fileno())
            self.assertEqual(data,original)


if __name__=='__main__':
    unittest.main(verbosity=2)
