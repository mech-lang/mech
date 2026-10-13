"""Unhooked declaration fixtures; standalone execution takes the timing mutex."""

import unittest

from audit import timing_boundary
from lexical import mask_source
from normalizer import _scope_map
from record_fields import classify_declaration_fields, SOURCE_EVIDENCE


class RecordFieldFixtures(unittest.TestCase):
    def classify(self, source, language):
        masked = mask_source(source, language)
        self.assertEqual(masked['diagnostics'], [], masked)
        contexts, diagnostics = _scope_map(masked['cleaned'], language)
        self.assertEqual(diagnostics, [], diagnostics)
        return classify_declaration_fields(masked['cleaned'], language, contexts)

    def fields(self, source, language, names, units=None):
        result = self.classify(source, language)
        self.assertEqual(result['diagnostics'], [], result)
        self.assertEqual([event['name'] for event in result['events']], names, result)
        self.assertEqual(sum(event['units'] for event in result['events']),
                         len(names) if units is None else units, result)
        for ownership in result['owned_spans']:
            self.assertTrue(ownership['exclude_internal_boundaries'])
            self.assertEqual(ownership['units'], 0)
            self.assertEqual(source[ownership['start'] - 1], '{')
        for event in result['events']:
            self.assertEqual(event['kind'], 'declaration-field')
            self.assertNotIn(source[event['start']], '{}')
        return result

    def test_packed_and_expanded_named_rust_fields(self):
        for source in ('struct V{x:f64,y:f64,z:f64}',
                       'struct V {\n x:f64,\n y:f64,\n z:f64,\n}'):
            self.fields(source, 'rust', ['x', 'y', 'z'])

    def test_actual_nbody_packed_member_grammar(self):
        self.fields('struct Planet {\n x:f64,y:f64,z:f64,\n'
                    ' vx:f64,vy:f64,vz:f64,\n mass:f64,\n}',
                    'rust', ['x', 'y', 'z', 'vx', 'vy', 'vz', 'mass'])

    def test_generic_type_commas_remain_structural(self):
        self.fields("struct Tree<'a> {children:Option<(&'a Tree<'a>, &'a Tree<'a>)>,"
                    " map:HashMap<K,Vec<(A,B)>>, buffer:[u8;16],}",
                    'rust', ['children', 'map', 'buffer'])

    def test_multiline_type_definition_is_one_field(self):
        result = self.fields('struct T {\n map:HashMap<\n K,\n Vec<(A,B)>\n >,\n count:u32,\n}',
                             'rust', ['map', 'count'])
        self.assertEqual(result['events'][0]['owned_rows'], [2, 3, 4, 5])

    def test_public_reference_and_pointer_fields(self):
        self.fields("pub struct T {pub x:usize,pub(crate) y:&'static str,"
                    ' z:*mut std::ffi::c_void,}', 'rust', ['x', 'y', 'z'])

    def test_tuple_struct_positional_types_do_not_expand(self):
        result = self.classify('struct V(pub f64,pub f64,pub f64);', 'rust')
        self.assertEqual(result['events'], [], result)
        self.assertEqual(result['owned_spans'], [], result)
        self.assertEqual(result['diagnostics'], [], result)

    def test_literals_constructor_arguments_and_parameters_do_not_expand(self):
        for source in ('let value=Tree{left:a,right:b};',
                       'fn f(x:(A,B),y:HashMap<K,V>){consume(x,y);}',
                       'let data=[Node{left:a,right:b},Node{left:c,right:d}];'):
            result = self.classify(source, 'rust')
            self.assertEqual(result['events'], [], result)
            self.assertEqual(result['diagnostics'], [], result)

    def test_actual_pidigits_packed_enum_names(self):
        self.fields('pub enum c_void {__variant1,__variant2}',
                    'rust', ['__variant1', '__variant2'])

    def test_unit_and_tuple_enum_variants(self):
        for source in ("enum Item {Empty,Freq(usize),Occ(&'static str),Pair(A,B),}",
                       "enum Item {\n Empty,\n Freq(usize),\n Occ(&'static str),\n Pair(A,B),\n}"):
            self.fields(source, 'rust', ['Empty', 'Freq', 'Occ', 'Pair'])

    def test_generic_types_within_enum_tuple_variants(self):
        self.fields('enum T {Data(HashMap<K,V>,Option<(A,B)>),Empty}',
                    'rust', ['Data', 'Empty'])

    def test_rust_discriminants_count_constant_once(self):
        self.fields('enum Flag {A=1,B=1<<2,C=A|B}', 'rust', ['A', 'B', 'C'])

    def test_c_and_cpp_simple_enum_constants(self):
        for language in ('c', 'cpp'):
            for source in ('enum E {A,B=4,C=B+1};',
                           'enum E {\n A,\n B=4,\n C=B+1\n};'):
                self.fields(source, language, ['A', 'B', 'C'])

    def test_cpp_scoped_enum_owner_is_not_silently_reclassified(self):
        result = self.classify('enum class E : unsigned {A,B};', 'cpp')
        self.assertEqual(result['events'], [], result)
        self.assertTrue(any(item['reason'] == 'type-owner-not-proven-executable'
                            for item in result['diagnostics']), result)

    def test_anonymous_c_enum_constants(self):
        self.fields('enum {A=0,B=1};', 'c', ['A', 'B'])

    def test_java_enum_constructor_arguments_are_structural(self):
        self.fields('enum E {A(1,2),B(3),C}', 'java', ['A', 'B', 'C'])

    def test_java_enum_members_after_semicolon_stay_unowned(self):
        source = 'enum E {A,B; private int x,y; void f(){x=1;y=2;} }'
        result = self.fields(source, 'java', ['A', 'B'])
        end = result['owned_spans'][0]['end']
        self.assertEqual(source[end:], ' private int x,y; void f(){x=1;y=2;} }')

    def test_java_nested_type_after_constant_section_stays_unowned(self):
        source = 'enum E {A,B; class Inner{int x,y;} }'
        result = self.fields(source, 'java', ['A', 'B'])
        self.assertEqual(source[result['owned_spans'][0]['end']:], ' class Inner{int x,y;} }')

    def test_direct_initializer_alias_bindings_add_independent_units(self):
        self.fields('enum E {A=x=1,B=2};', 'c', ['A', 'B'], units=3)

    def test_empty_types_have_zero_member_units(self):
        for language in ('rust', 'c', 'cpp', 'java'):
            self.fields('enum E {}', language, [])
        self.fields('struct T {}', 'rust', [])

    def test_unknown_field_types_gate_whole_owner(self):
        for source in ('struct T{x:TypeMacro!(),y:u32}',
                       'struct T{x:<T as Trait>::Assoc,y:u32}',
                       'struct T{x:fn(A,B)->C,y:u32}',
                       'struct T{x:u32,#[cfg(flag)]y:u32}',
                       'struct T{x:Vec<{N+1}>,y:u32}'):
            result = self.classify(source, 'rust')
            self.assertEqual(result['events'], [], result)
            self.assertEqual(result['owned_spans'], [], result)
            self.assertTrue(result['diagnostics'], result)

    def test_named_enum_payload_and_anonymous_java_body_stay_gated(self):
        for source, language in (('enum E {Node{left:T,right:T},Empty}', 'rust'),
                                 ('enum E {A{void f(){}},B}', 'java')):
            result = self.classify(source, language)
            self.assertEqual(result['events'], [], result)
            self.assertTrue(result['diagnostics'], result)

    def test_multiline_literal_argument_wrapping_stays_gated(self):
        result = self.classify('enum E {A(1,\n2),B}', 'java')
        self.assertEqual(result['events'], [], result)
        self.assertTrue(result['diagnostics'], result)

    def test_nested_initializer_assignment_stays_reviewed(self):
        result = self.classify('enum E {A=(x=1),B};', 'c')
        self.assertEqual(result['events'], [], result)
        self.assertTrue(any(item['reason'] == 'nested-enum-initializer-binding-review'
                            for item in result['diagnostics']), result)

    def test_empty_interior_comma_groups_stay_gated(self):
        for source in ('enum E {,A}', 'enum E {A,,B}', 'struct T{,x:u32}'):
            result = self.classify(source, 'rust')
            self.assertEqual(result['events'], [], result)
            self.assertTrue(any(item['reason'] == 'empty-interior-member-group'
                                for item in result['diagnostics']), result)

    def test_comments_and_string_payloads_are_already_masked(self):
        self.fields('struct T{x:u32,/* comma, colon: and } */y:u32}', 'rust', ['x', 'y'])
        self.fields('enum E {A("x,y"),B}', 'java', ['A', 'B'])

    def test_separate_owners_produce_separate_ownership(self):
        result = self.fields('struct A{x:u32,y:u32}\nstruct B{z:u32}',
                             'rust', ['x', 'y', 'z'])
        self.assertEqual(len(result['owned_spans']), 2)
        self.assertLessEqual(result['owned_spans'][0]['end'], result['owned_spans'][1]['start'])

    def test_source_evidence_does_not_claim_global_count_or_admission(self):
        self.assertEqual(len(SOURCE_EVIDENCE), 4)
        for item in SOURCE_EVIDENCE:
            self.assertIn('scope', item)
            self.assertNotIn('normalized_loc', item)
            self.assertNotIn('admitted', item)

    def test_bad_context_length_and_unsupported_language(self):
        with self.assertRaises(ValueError):
            classify_declaration_fields('x', 'rust', [])
        result = classify_declaration_fields('x', 'python', [()])
        self.assertEqual(result['status'], 'unsupported-language')
        self.assertEqual(result['events'], [])


if __name__ == '__main__':
    with timing_boundary():
        unittest.main()
