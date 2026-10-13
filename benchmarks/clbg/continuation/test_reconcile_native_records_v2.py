"""Synthetic six-language contract fixtures; no native program is executed."""
from copy import deepcopy
import unittest

import reconcile_native_records_v1 as original
import reconcile_native_records_v2 as subject
from test_reconcile_native_records_v1 import fixture, fingerprint


def validate(language, manifest, report, reference, evidence):
    return subject.validate_benchmark(manifest, report, 'm'*64, language=language,
        canonical_n=5500, reference=reference, fingerprint=fingerprint,
        load_evidence=lambda path: deepcopy(evidence[path]))


class ExpandedRecords(unittest.TestCase):
    def test_gleam_and_clojure_keep_exact_actual_record_contract(self):
        before = set(original.LANGUAGES)
        for language in ('gleam','clojure'):
            manifest, report, reference, evidence = fixture(language)
            inputs = deepcopy((manifest,report,evidence))
            rows = validate(language,manifest,report,reference,evidence)
            self.assertEqual(len(rows),3)
            self.assertTrue(all(row['eligible_for_chart'] is False for row in rows.values()))
            self.assertEqual((manifest,report,evidence),inputs)
        self.assertEqual(original.LANGUAGES,before)
        self.assertEqual(original.LANGUAGES,{'nim','crystal','pony','c3'})

    def test_v_cannot_be_reclassified_as_a_publication_language(self):
        manifest, report, reference, evidence = fixture('v')
        with self.assertRaises(ValueError):
            validate('v',manifest,report,reference,evidence)

    def test_new_vm_languages_cannot_use_bounded_or_changed_command_records(self):
        for language in ('gleam','clojure'):
            for field,value in (('workload_n',1000),('preparation_only',True),
                                ('command',['/synthetic-only/other-runtime','5500'])):
                manifest, report, reference, evidence = fixture(language)
                report['attempts'][0][field] = value
                with self.subTest(language=language,field=field),self.assertRaises(ValueError):
                    validate(language,manifest,report,reference,evidence)

    def test_new_vm_languages_cannot_overwrite_saved_sampler_or_output_evidence(self):
        for language in ('gleam','clojure'):
            manifest, report, reference, evidence = fixture(language)
            path = report['attempts'][0]['result_path']
            evidence[path]['seconds'] += .5
            with self.subTest(language=language),self.assertRaises(ValueError):
                validate(language,manifest,report,reference,evidence)


if __name__=='__main__':
    unittest.main(verbosity=2)
