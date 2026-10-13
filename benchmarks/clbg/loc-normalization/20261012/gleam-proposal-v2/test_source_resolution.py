import copy
import hashlib
from pathlib import Path
import unittest

from gleam_units import propose_units

IO = '''@external(erlang, "math", "sqrt") pub fn sqrt(value: Float) -> Float
@external(erlang, "binary", "at") pub fn byte(value: BitArray, index: Int) -> Int
@external(erlang, "clbg_io_ffi", "read_all") pub fn read_all() -> BitArray
'''
CLBG = next(path for path in Path(__file__).resolve().parents if path.name == 'clbg')
VENDOR = CLBG / 'gleam-comparison/20261012/.build/private-v1/stdlib/src'
CODE = '''fn mapping(value: Int) -> Int { case value {
65 | 97 -> 0 67 | 99 -> 1 71 | 103 -> 2 84 | 116 -> 3 _ -> panic as "Invalid nucleotide"
} }
'''
COMPARE = '''fn compare(a: #(Int, Int), b: #(Int, Int)) -> order.Order {
case a.1 > b.1 { True -> Lt False -> Eq }
}
'''
PREFIX = '''import gleam/list
import gleam/dict
import gleam/order.{Eq, Gt, Lt}
'''


def context(source, helper=IO):
    texts = {'caller':(source,'benchmark','application'), 'io':(helper,'clbg_io','application-helper')}
    for module in ('list','dict','order'):
        texts[module] = ((VENDOR / ('gleam/' + module + '.gleam')).read_text(),'gleam/'+module,'vendor')
    return dict(status='synthetic-fixture-context-never-semantic-admission', vendor_version='1.0.5', sources={
        path:dict(text=text,module=module,role=role,sha256=hashlib.sha256(text.encode()).hexdigest())
        for path,(text,module,role) in texts.items()})


class SourceBoundComposition(unittest.TestCase):
    def metric(self, source, helper=IO, binding=None):
        return propose_units(source,'caller',context(source,helper) if binding is None else binding)

    def test_float_sqrt_grouping_has_exact_ffi_and_unchanged_operator_ast(self):
        source = 'import clbg_io as io\nfn run(n: Float) { let squared = n *. n let value = 0.01 /. { squared *. io.sqrt(squared) } value }'
        result = self.metric(source)
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(result['normalized_loc'],6)
        self.assertEqual(result['normalized_loc'],self.metric(source.replace('{ squared *. io.sqrt(squared) }','squared *. io.sqrt(squared)'))['normalized_loc'])
        certificate = result['source_bound_composition_proposals'][0]
        self.assertEqual(certificate['original_owner_ast']['children'][0]['children'][0]['operator'],'*.')
        self.assertEqual(certificate['resolved_declarations'][0]['external_annotations'][0]['module'],'math')
        self.assertEqual(propose_units(IO)['normalized_loc'],6)
        self.assertIs(certificate['semantic_admission'],False)
        for helper in (IO.replace('"sqrt"','"sin"'),IO.replace('sqrt(value: Float) -> Float','sqrt(value: Float) -> Int')):
            self.assertTrue(self.metric(source,helper)['diagnostics'])
        self.assertTrue(self.metric(source.replace('n: Float','n: Unknown'))['diagnostics'])
        self.assertTrue(self.metric(source.replace('io.sqrt(squared)','io.sqrt(0xdead)'))['diagnostics'])

    def test_byte_boolean_group_preserves_short_circuit_and_rejects_lexical_shadows(self):
        expression = '{ width == 6 || io.byte(line, 6) == 32 || io.byte(line, 6) == 9 }'
        source = 'import clbg_io as io\nfn run(line: BitArray, width: Int) { let selected = width >= 6 && '+expression+' selected }'
        result = self.metric(source)
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(result['normalized_loc'],5)
        original = result['source_bound_composition_proposals'][0]['original_owner_ast']
        self.assertEqual(original['children'][0]['children'][0]['operator'],'||')
        self.assertEqual(len(result['source_bound_composition_proposals'][0]['resolved_declarations']),2)
        for changed in (source.replace('width: Int','width: Int, io: Int'),
                        source.replace('fn run','const io = 1\nfn run'),
                        source.replace('let selected','let io = 1 let selected'),
                        source.replace('let selected = width >= 6 && '+expression+' selected', 'consume(fn(io) { '+expression+' })'),
                        source.replace('let selected = width >= 6 && '+expression+' selected', 'case width { io -> '+expression+' }'),
                        source.replace('line: BitArray','line: Unknown')):
            metric = self.metric(changed)
            self.assertTrue(metric['diagnostics'],metric)
            self.assertFalse(metric['source_bound_composition_proposals'])

    def test_local_integer_case_and_panic_body_remain_independently_counted(self):
        source = 'import clbg_io as io\n'+CODE+'fn run(sequence: BitArray, index: Int, key: Int, modulus: Int) { let value = { key * 4 + mapping(io.byte(sequence, index)) } % modulus value }'
        result = self.metric(source)
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(result['normalized_loc'],self.metric(source.replace('{ key * 4 + mapping(io.byte(sequence, index)) }','key * 4 + mapping(io.byte(sequence, index))'))['normalized_loc'])
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind']=='case-arm-header']),5)
        self.assertTrue(any('panic' in ref['exact_authored_or_vendor_source'] for ref in result['source_bound_composition_proposals'][0]['resolved_declarations']))
        for changed in (source.replace('65 | 97 -> 0','65 | 97 -> mutate()'),
                        source.replace('fn mapping','const mapping = 1\nfn mapping'),
                        source.replace('mapping(value: Int) -> Int','mapping(value: Unknown) -> Int'),
                        source.replace('mapping(io.byte(sequence, index))','unknown(io.byte(sequence, index))')):
            self.assertTrue(self.metric(changed)['diagnostics'])

    def test_collection_prefix_keeps_comparator_and_every_callback_owner(self):
        source = PREFIX+COMPARE+'fn run(counts: dict.Dict(Int, Int)) { list.each(list.sort(dict.to_list(counts), compare), fn(entry) { emit(entry) }) }'
        result = self.metric(source)
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(result['normalized_loc'],16)
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind']=='anonymous-function-header']),1)
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind']=='case-arm-header']),2)
        proofs = result['source_bound_composition_proposals']
        self.assertEqual(len(proofs),2)
        self.assertTrue(all(proof['first_direct_callback_ast']['kind']=='anonymous-function' for proof in proofs))
        changed = source.replace('emit(entry) })','emit(entry) }, fn(other) { emit(other) })')
        metric = self.metric(changed)
        self.assertEqual(len([event for event in metric['canonical_events'] if event['kind']=='anonymous-function-header']),2)
        self.assertEqual(metric['normalized_loc'],19)
        for bad in (source.replace('counts: dict.Dict(Int, Int)','counts: dict.Dict(Int, Int), compare: Int'),
                    source.replace('fn compare','const compare = 1\nfn compare'),
                    source.replace('list.each','let compare = unknown list.each'),
                    source.replace('True -> Lt','True -> { emit(a) Lt }'),
                    source.replace('dict.to_list(counts)','mutate(counts)')):
            self.assertTrue(self.metric(bad)['diagnostics'])

    def test_changed_source_helper_vendor_signature_or_bytes_never_reuses_context(self):
        source = 'import clbg_io as io\nfn run(n: Float) { let value = { n *. io.sqrt(n) } value }'
        bound = context(source)
        changed = copy.deepcopy(bound); changed['sources']['io']['text'] += '\n// changed byte'
        self.assertTrue(self.metric(source,binding=changed)['diagnostics'])
        self.assertTrue(self.metric(source+'\n// changed caller',binding=bound)['diagnostics'])
        prefix = PREFIX+COMPARE+'fn run(counts: dict.Dict(Int, Int)) { list.each(list.sort(dict.to_list(counts), compare), fn(entry) { emit(entry) }) }'
        for vendor in ('list','dict','order'):
            bound = context(prefix); bound['sources'][vendor]['text'] += '\n// changed vendor byte'
            self.assertTrue(self.metric(prefix,binding=bound)['diagnostics'])
        bound = context(prefix); bound['vendor_version'] = 'current-unverified'
        self.assertTrue(self.metric(prefix,binding=bound)['diagnostics'])
        bound = context(prefix); text = bound['sources']['dict']['text'].replace('to_list(dict: Dict(k, v))','to_list(dict: Dict(k, v), extra: Int)')
        bound['sources']['dict'].update(text=text,sha256=hashlib.sha256(text.encode()).hexdigest())
        self.assertTrue(self.metric(prefix,binding=bound)['diagnostics'])

    def test_nested_work_hidden_calls_and_actual_bits_form_never_gain_scalar_certificate(self):
        source = 'import clbg_io as io\nfn run() { let value = { io.byte(<<io.read_all():bits>>, 0) } value }'
        self.assertTrue(self.metric(source)['diagnostics'])
        self.assertFalse(self.metric(source)['source_bound_composition_proposals'])
        source = 'import clbg_io as io\nfn run(n: Float) { let value = { let inner = io.sqrt(n) emit(inner) inner } value }'
        result = self.metric(source)
        self.assertFalse(result['source_bound_composition_proposals'])
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind']=='binding']),2)
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind']=='written-close']),2)
        bits = propose_units('fn run(header: BitArray) { <<header:bits, 10>> }')
        self.assertEqual(bits['diagnostics'],[])
        self.assertEqual(bits['normalized_loc'],3)


if __name__ == '__main__': unittest.main()
