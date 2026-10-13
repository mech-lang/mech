"""Focused producer qualification gates; no runtime or benchmark execution."""
from copy import deepcopy
import unittest
from lua_pidigits_study_data import _full_validation

REFERENCE=dict(bytes=16893,sha256='b'*64)

def fixture():
    command=['/runtime/lua','/source/pidigits-lua-6-normalized.lua','{n}']
    return dict(command=command,full_validation=dict(variant=6,kind='normalized',n=10000,
        full_output_validation=True,command=[s.replace('{n}','10000') for s in command],
        **REFERENCE,reference_match=True))

class FullValidationTests(unittest.TestCase):
    def reject(self,edit):
        row=fixture();edit(row)
        with self.assertRaises((AssertionError,KeyError,TypeError)):_full_validation(row,REFERENCE,6)
    def test_actual_producer_metadata_accepted(self):_full_validation(fixture(),REFERENCE,6)
    def test_additional_provenance_preserved(self):
        row=fixture();row['full_validation']['provenance']='independent original/copy output proof'
        previous=deepcopy(row);_full_validation(row,REFERENCE,6);self.assertEqual(row,previous)
    def test_bare_output_summary_rejected(self):self.reject(lambda row:row.update(full_validation=dict(REFERENCE,reference_match=True)))
    def test_every_required_field_is_required(self):
        for key in fixture()['full_validation']:
            with self.subTest(field=key):self.reject(lambda row:row['full_validation'].pop(key))
    def test_wrong_variant(self):self.reject(lambda row:row['full_validation'].update(variant=5))
    def test_boolean_variant(self):
        row=fixture();row['full_validation']['variant']=True
        with self.assertRaises(AssertionError):_full_validation(row,REFERENCE,1)
    def test_original_source_proof_cannot_replace_counted_copy(self):self.reject(lambda row:row['full_validation'].update(kind='original'))
    def test_small_probe_rejected(self):self.reject(lambda row:row['full_validation'].update(n=97))
    def test_noninteger_n_rejected(self):self.reject(lambda row:row['full_validation'].update(n=10000.0))
    def test_full_proof_required(self):self.reject(lambda row:row['full_validation'].update(full_output_validation=False))
    def test_boolean_full_proof_required(self):self.reject(lambda row:row['full_validation'].update(full_output_validation=1))
    def test_wrong_runtime_command(self):self.reject(lambda row:row['full_validation']['command'].__setitem__(0,'/another/lua'))
    def test_wrong_source_command(self):self.reject(lambda row:row['full_validation']['command'].__setitem__(1,'/source/original.lua'))
    def test_wrong_command_n(self):self.reject(lambda row:row['full_validation']['command'].__setitem__(2,'97'))
    def test_unexpanded_command_n(self):self.reject(lambda row:row['full_validation']['command'].__setitem__(2,'{n}'))
    def test_wrong_output_size(self):self.reject(lambda row:row['full_validation'].update(bytes=0))
    def test_wrong_output_hash(self):self.reject(lambda row:row['full_validation'].update(sha256='a'*64))
    def test_reference_match_required(self):self.reject(lambda row:row['full_validation'].update(reference_match=False))
    def test_boolean_reference_match_required(self):self.reject(lambda row:row['full_validation'].update(reference_match=1))
    def test_optional_noncanonical_flag_rejected(self):self.reject(lambda row:row['full_validation'].update(canonical_full_workload=False))
    def test_optional_probe_exclusion_flag_rejected(self):self.reject(lambda row:row['full_validation'].update(never_chart_eligible=True))

if __name__=='__main__':unittest.main()
