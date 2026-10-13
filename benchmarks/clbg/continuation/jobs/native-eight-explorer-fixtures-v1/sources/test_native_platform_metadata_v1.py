"""Meaningful preservation/receipt failures for new cohort platform evidence."""
from copy import deepcopy
import unittest

from native_platform_metadata_v1 import BACKENDS, stage


def example():
    source_sha, report_sha, runtime_sha, meta_sha = (letter * 64 for letter in 'abcd')
    data = dict(languages=[dict(key=key) for key in BACKENDS], programs={}, files={source_sha:dict(sha256=source_sha)},
        provenance={}, native_language_audit=dict(languages={}), benchmarks=[dict(key='benchmark',groups=[])])
    metadata, actual, provenance = {}, {}, {}
    for language, (backend, execution) in BACKENDS.items():
        report = f'{language}/measurement.json'
        runtime = f'{language}/runtime.json'
        meta_path = f'{language}/metadata.json'
        data['provenance'][report] = report_sha
        provenance[runtime], provenance[meta_path] = runtime_sha, meta_sha
        actual[language] = dict(actual_version='Actual saved version\n')
        metadata[language] = dict(language=language, status='reviewed-actual-native-language-metadata',
            actual_compiler_version_verified=True, actual_native_architecture_verified=True, actual_memory_policy_verified=True,
            memory_strategies=['gc'], memory_label='Reviewed runtime',
            publication_metadata_evidence=dict(path=meta_path, sha256=meta_sha),
            measured_backend_template=dict(key=backend, execution=execution, label='Actual backend', verified=True,
                runtime_receipt=dict(report=runtime, report_sha256=runtime_sha, verified=True,
                    field='/actual_version', value='Actual saved version\n', value_encoding='literal-saved-string')))
        data['native_language_audit']['languages'][language] = dict(source_count=20,actual_full_native_processes=90,sample_count=3,
            artifacts=dict(metadata=dict(path=meta_path,sha256=meta_sha)))
        for index in range(20):
            key = f'benchmark:{language}-{index}'
            data['programs'][key] = dict(key=key,language=language,execution_mode=execution,required_work_validated=True,
                report=report,runtime_arguments=[f'/native/{language}',str(index)],files=[source_sha])
    return data, metadata, actual, provenance


class NativePlatformFixtures(unittest.TestCase):
    def test_complete_native_jvm_beam_stage_preserves_measurements(self):
        data, metadata, actual, provenance = example()
        before = deepcopy(data)
        changes, memory, combined = stage(data, metadata, actual, provenance)
        self.assertEqual(data, before)
        self.assertEqual(len(changes), 120)
        self.assertEqual(set(memory), set(BACKENDS))
        for key, record in changes.items():
            language = data['programs'][key]['language']
            self.assertEqual((record['key'],record['execution']), BACKENDS[language])
            self.assertEqual(record['evidence']['command'],data['programs'][key]['runtime_arguments'])
            self.assertEqual(record['evidence']['source_sha256'],['a'*64])
            self.assertEqual(combined[record['evidence']['runtime_receipt']['report']],'c'*64)

    def test_receipt_content_missing_pointer_and_encoding_reject(self):
        mutations = [
            lambda m,a,p: m['nim']['measured_backend_template']['runtime_receipt'].update(value='Guessed version'),
            lambda m,a,p: m['nim']['measured_backend_template']['runtime_receipt'].update(field='/missing'),
            lambda m,a,p: m['nim']['measured_backend_template']['runtime_receipt'].update(value_encoding='guessed'),
            lambda m,a,p: a['nim'].update(actual_version=42),
            lambda m,a,p: p.update({'nim/runtime.json':'f'*64}),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                data, metadata, actual, provenance = example()
                before = deepcopy(data)
                mutate(metadata, actual, provenance)
                with self.assertRaises(ValueError): stage(data,metadata,actual,provenance)
                self.assertEqual(data,before)

    def test_partial_cohort_wrong_backend_and_v_reject(self):
        mutations = [
            lambda d,m: d['programs'].pop('benchmark:nim-0'),
            lambda d,m: m['gleam']['measured_backend_template'].update(key='native-aot',execution='aot'),
            lambda d,m: d['languages'].append(dict(key='v')),
            lambda d,m: d['benchmarks'][0]['groups'].append(dict(language='v')),
            lambda d,m: m['pony'].update(actual_memory_policy_verified='true'),
            lambda d,m: m['clojure'].update(memory_strategies=['guessed']),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                data, metadata, actual, provenance = example()
                mutate(data,metadata)
                before = deepcopy(data)
                with self.assertRaises(ValueError): stage(data,metadata,actual,provenance)
                self.assertEqual(data,before)

    def test_late_conflict_and_serialized_receipt_preserve_input(self):
        data, metadata, actual, provenance = example()
        metadata['nim']['measured_backend_template']['runtime_receipt'].update(
            value_encoding='json-serialized-preserved-value',value='{"major": 1, "minor": 2}')
        actual['nim']['actual_version'] = dict(minor=2,major=1)
        before = deepcopy(data)
        changes, _, _ = stage(data,metadata,actual,provenance)
        self.assertEqual(len(changes),120)
        self.assertEqual(data,before)
        data['provenance']['clojure/metadata.json'] = 'f'*64
        before = deepcopy(data)
        with self.assertRaises(ValueError): stage(data,metadata,actual,provenance)
        self.assertEqual(data,before)
        data, metadata, actual, provenance = example()
        data['programs']['benchmark:clojure-19']['key'] = 'missing-target'
        before = deepcopy(data)
        with self.assertRaises(ValueError): stage(data,metadata,actual,provenance)
        self.assertEqual(data,before)
