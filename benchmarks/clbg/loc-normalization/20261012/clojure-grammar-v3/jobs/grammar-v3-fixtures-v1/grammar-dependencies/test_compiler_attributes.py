"""Approved written GNU attribute units; common-mutex fixtures only."""
import unittest
from compiler_attributes import classify_gnu_attributes
from normalizer import normalize
from c_type_environment import collect_typedef_aliases,identify_decl


class GNUAttributeFixtures(unittest.TestCase):
    def classify(self,source):return classify_gnu_attributes(source,'c')

    def test_known_settings_one_unit_per_written_group(self):
        result=self.classify('typedef double lane __attribute__((vector_size(16),aligned(16)));')
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(len(result['events']),1)
        self.assertEqual(result['events'][0]['units'],1)
        self.assertEqual(result['events'][0]['settings'],['vector_size','aligned'])

    def test_double_underscore_settings_and_keyword_alias(self):
        for source in ('int x __attribute__((__unused__));','int x __attribute((unused));'):
            result=self.classify(source)
            self.assertEqual(result['diagnostics'],[])
            self.assertEqual(len(result['events']),1)

    def test_mask_preserves_offsets_and_wrapped_line_positions(self):
        source='typedef double lane\r\n__attribute__((\r\n vector_size(16),\r\n aligned(32)\r\n));'
        result=self.classify(source)
        self.assertEqual(len(result['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(source) if c in '\r\n'],
            [i for i,c in enumerate(result['cleaned']) if c in '\r\n'])
        self.assertIn('typedef double lane',result['cleaned'])
        self.assertNotIn('vector_size',result['cleaned'])

    def test_arguments_nested_data_and_commas_are_structural(self):
        result=self.classify('void f() __attribute__((format(printf,1,2),aligned(sizeof(int)*4)));')
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(len(result['events']),1)

    def test_unknown_or_malformed_group_retains_source_review_gate(self):
        for source,kind in (('int x __attribute__((GENERATE_CODE(3)));','gnu-attribute-setting-owner-review'),
            ('int x __attribute__((unknown));','gnu-attribute-setting-owner-review'),
            ('int x __attribute__(packed);','gnu-attribute-group-owner-review'),
            ('int x __attribute__((packed);','gnu-attribute-group-owner-review')):
            result=self.classify(source)
            self.assertEqual(result['events'],[])
            self.assertEqual(result['cleaned'],source)
            self.assertIn(kind,{d['kind'] for d in result['diagnostics']})

    def test_known_setting_cannot_hide_executable_callback_or_independent_bindings(self):
        for source in ('int x __attribute__((aligned(([]{int n=4;return n;}()))));',
                       'int x __attribute__((aligned(value=16)));'):
            result=self.classify(source)
            self.assertEqual(result['events'],[])
            self.assertEqual(result['cleaned'],source)
            self.assertIn('gnu-attribute-setting-owner-review',{d['kind'] for d in result['diagnostics']})

    def test_macro_and_continued_macro_do_not_impute_generated_attributes(self):
        for source in ('#define ALIGN __attribute__((aligned(32)))\nint x ALIGN;',
                       '#define ALIGN ' + '\\' + '\n  __attribute__((aligned(32)))\nint x ALIGN;'):
            result=self.classify(source)
            self.assertEqual(result['events'],[])
            self.assertIn('gnu-attribute-macro-owner-review',{d['kind'] for d in result['diagnostics']})

    def test_nearby_identifiers_and_other_languages_are_never_attributes(self):
        source='int x; call_attribute(unused); int __attribute_helper;'
        self.assertEqual(self.classify(source)['events'],[])
        self.assertEqual(classify_gnu_attributes('__attribute__((unused))','rust')['events'],[])

    def test_typedef_attribute_mask_establishes_actual_alias_scope(self):
        source='typedef double lane __attribute__((vector_size(16))); lane a,b;'
        env=collect_typedef_aliases(source,'c')
        self.assertEqual(env['diagnostics'],[])
        self.assertEqual(len(env['compiler_attribute_evidence']),1)
        declaration=identify_decl('lane a,b;',env,offset=source.index('lane a,b'))
        self.assertEqual(declaration['count'],2)

    def test_two_groups_remain_independent_meaningful_annotations(self):
        result=self.classify('int x __attribute__((unused)) __attribute__((aligned(32)));')
        self.assertEqual(result['diagnostics'],[])
        self.assertEqual(sum(event['units'] for event in result['events']),2)


class GNUAttributeIntegrationFixtures(unittest.TestCase):
    def count(self,source,expected,language='c'):
        result=normalize(source,language)
        self.assertEqual(result['diagnostics'],[],result)
        self.assertEqual(result['normalized_loc'],expected,result)
        return result

    def test_typedef_alias_plus_attribute_plus_independent_names_packed_expanded(self):
        for source in ('typedef double lane __attribute__((vector_size(16))); lane a,b;',
            'typedef double lane\n__attribute__((\nvector_size(16)\n));\nlane a;\nlane b;'):
            self.count(source,4)

    def test_named_function_modifier_header_and_attribute_are_distinct(self):
        for source in ('__attribute__((always_inline)) static inline int f(int x){int y=x;return y;}',
            '__attribute__((always_inline))\nstatic inline int f(int x)\n{\nint y=x;\nreturn y;\n}'):
            self.count(source,5)

    def test_attribute_on_independent_binding_does_not_hide_declaration_commas(self):
        self.count('int a __attribute__((unused)),b;',3)
        self.count('int a __attribute__((unused)) __attribute__((aligned(16)));',3)

    def test_raw_physical_lines_and_string_comment_literals_are_preserved(self):
        source='typedef double lane\n__attribute__((\nvector_size(16)\n));'
        result=self.count(source,2)
        self.assertEqual(result['physical_loc'],4)
        self.count('char *text="__attribute__((unused));"; // __attribute__((packed))',1)

    def test_unknown_macro_annotations_require_source_review(self):
        result=normalize('typedef double lane __attribute__((HOST_MACRO(16))); lane a,b;','c')
        self.assertEqual(result['review_status'],'needs-review')
        self.assertTrue(any(d['kind']=='gnu-attribute-setting-owner-review' for d in result['diagnostics']))


if __name__=='__main__':unittest.main()
