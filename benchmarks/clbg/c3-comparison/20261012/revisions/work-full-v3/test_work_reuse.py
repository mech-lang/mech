import copy
import json
from types import SimpleNamespace
import unittest

import qualify_work
import work_reuse


class PreservedWorkChecks(unittest.TestCase):
    def setUp(self):
        self.report = json.loads((work_reuse.PRIOR / 'report.json').read_text())
        self.qualification = json.loads((work_reuse.HERE / 'qualification/private-v3/report.json').read_text())
        self.cases = [name for name, _ in qualify_work.preparation.inputs('knucleotide', qualify_work.references)]

    def test_exact_38_catalog_preserves_failed_allocation_and_original_attempts(self):
        self.assertEqual(len(work_reuse.validate_catalog(self.report, self.qualification, self.cases)), 38)
        before = copy.deepcopy(self.report)
        corruptions = []
        for field, value in (('status','source-work-allocation-passed-awaiting-full-review-and-LOC'),
                             ('error','Final original creator lacks real heap allocation'),
                             ('pins',{}), ('full_performance_runs',1)):
            bad = copy.deepcopy(self.report); bad[field] = value; corruptions.append(bad)
        bad = copy.deepcopy(self.report); bad['proofs'].pop(); corruptions.append(bad)
        bad = copy.deepcopy(self.report); bad['proofs'][-1] = copy.deepcopy(bad['proofs'][0]); corruptions.append(bad)
        bad = copy.deepcopy(self.report); bad['profiles'][0]['binary_sha256'] = '0'*64; corruptions.append(bad)
        for bad in corruptions:
            with self.assertRaises(AssertionError):
                work_reuse.validate_catalog(bad, self.qualification, self.cases)
        self.assertEqual(self.report, before)

    def test_qualified_record_binds_source_runtime_raw_types_and_proof(self):
        attempt, proof = copy.deepcopy(self.report['attempts'][0]), copy.deepcopy(self.report['proofs'][0])
        fingerprint = json.loads(attempt['execution_fingerprint'])
        observed = dict(source_sha256=fingerprint['source_sha256'], binary_sha256=fingerprint['binary_sha256'],
            command=fingerprint['command'], cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'],
            support=fingerprint['support'], key=attempt['key'], benchmark=attempt['benchmark'], language='c3')
        def fingerprint_of(row):
            return json.dumps(dict(source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                command=row['command'], cwd=row['cwd'], environment=row['runtime_environment'], support=row['support']), sort_keys=True)
        harness = SimpleNamespace(execution_fingerprint=fingerprint_of)
        def validate(record, saved_proof=proof, row=observed):
            return work_reuse.validate_record(record, saved_proof, row, harness, 1, qualify_work.native_reuse)
        validate(attempt)
        changes = [('attempt',True), ('sampler_wait_status',False), ('workload_n',False),
                   ('full_workload',True), ('stdout_bytes',False), ('source_sha256','0'*64)]
        for name, value in changes:
            bad = copy.deepcopy(attempt); bad[name] = value
            with self.subTest(field=name), self.assertRaises(AssertionError):
                validate(bad)
        bad = copy.deepcopy(attempt); bad['validation']['bytes'] = False
        with self.assertRaises(AssertionError): validate(bad)
        bad = copy.deepcopy(attempt); bad['supervised_native_group']['pgid'] = True
        with self.assertRaises(AssertionError): validate(bad)
        bad = copy.deepcopy(attempt); del bad['qualification_checks']['pins_unchanged']
        with self.assertRaises(AssertionError): validate(bad)
        bad = copy.deepcopy(attempt); bad['qualification_checks']['pins_unchanged'] = 1
        with self.assertRaises(AssertionError): validate(bad)
        bad = copy.deepcopy(observed); bad['runtime_environment']['CLBG_TRACE'] = '/new/trace'
        with self.assertRaises(AssertionError): validate(attempt, row=bad)
        bad = copy.deepcopy(observed); bad['support'][next(iter(bad['support']))] = '0'*64
        with self.assertRaises(AssertionError): validate(attempt, row=bad)
        bad = copy.deepcopy(proof); bad['reconciliation']['accepted_candidates'] = 1
        with self.assertRaises(AssertionError): validate(attempt, saved_proof=bad)


if __name__ == '__main__':
    unittest.main()
