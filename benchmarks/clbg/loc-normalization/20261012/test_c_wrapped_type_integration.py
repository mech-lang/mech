"""Proposed whole-normalizer checks; execute only in the frozen mutex job."""
import unittest

from normalizer import normalize


class CWrappedTypeIntegrationFixtures(unittest.TestCase):
    def reviews(self, row):
        return row['wrapped_declaration_review']['diagnostics']

    def kinds(self, row):
        return {item['kind'] for item in self.reviews(row)}

    def test_wrapped_proven_template_body_retains_exact_source_review(self):
        source = '#include <vector>\nvoid f(){std::vector<\nint\n> a,b;}'
        row = normalize(source, 'cpp')
        self.assertEqual(row['review_status'], 'needs-review')
        kinds = self.kinds(row)
        self.assertIn('cpp-wrapped-proven-type-declaration-owner-review', kinds)
        self.assertIn('cpp-wrapped-independent-declarators-owner-review', kinds)
        for item in self.reviews(row):
            self.assertEqual(item['offset'], source.index('std::vector'))
            self.assertIsNone(item['units'])

    def test_inline_proven_template_bindings_keep_existing_count(self):
        packed = '#include <vector>\nvoid f(){std::vector<int> a,b;}'
        expanded = '#include <vector>\nvoid f(){std::vector<int> a;\nstd::vector<int> b;}'
        for source in (packed, expanded):
            row = normalize(source, 'cpp')
            self.assertEqual(row['diagnostics'], [], row)
            self.assertEqual(row['normalized_loc'], 5)

    def test_inline_builtin_type_with_wrapped_names_cannot_pass_as_normalized(self):
        source = 'void f(){int a,\n b,c;}'
        row = normalize(source, 'cpp')
        self.assertIn('cpp-wrapped-independent-declarators-owner-review', self.kinds(row))
        item = next(item for item in self.reviews(row)
                    if item['kind'] == 'cpp-wrapped-independent-declarators-owner-review')
        self.assertEqual(item['declaration_names'], ['a', 'b', 'c'])
        self.assertEqual(item['declaration_name_rows'], [1, 2, 2])
        self.assertEqual(item['offset'], source.index('int a'))

    def test_type_row_followed_by_all_names_retains_declarator_review(self):
        source = 'void f(){int\n a,b,c;}'
        row = normalize(source, 'c')
        self.assertIn('cpp-wrapped-independent-declarators-owner-review', self.kinds(row))
        self.assertEqual(row['review_status'], 'needs-review')

    def test_wrapped_builtin_modifiers_keep_type_and_name_reviews(self):
        source = 'void f(){const\nint a,b;}'
        row = normalize(source, 'c')
        self.assertIn('cpp-wrapped-proven-type-declaration-owner-review', self.kinds(row))
        self.assertIn('cpp-wrapped-independent-declarators-owner-review', self.kinds(row))

    def test_inline_names_with_wrapped_data_preserve_literal_rows(self):
        source = 'void f(){\nint values[][2] = {\n{1,2},\n{3,4}\n};\n}'
        row = normalize(source, 'cpp')
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(self.reviews(row), [])
        self.assertEqual(row['normalized_loc'], 6)

    def test_complete_written_alias_owner_is_not_a_body_declaration(self):
        for source in ('typedef unsigned long T;\nT a,b;',
                       'typedef unsigned\nlong T;\nT a,b;'):
            row = normalize(source, 'c')
            self.assertEqual(self.reviews(row), [], row)
            self.assertEqual(row['diagnostics'], [], row)
            self.assertEqual(row['normalized_loc'], 3)

    def test_typedef_aggregate_does_not_hide_wrapped_fields(self):
        source = '#include <vector>\ntypedef struct {std::vector<\nint\n> a,b;} Pair;'
        row = normalize(source, 'cpp')
        self.assertIn('cpp-wrapped-proven-type-declaration-owner-review', self.kinds(row))
        self.assertEqual(row['review_status'], 'needs-review')

    def test_for_formal_type_wrapping_does_not_create_body_declaration_review(self):
        source = '#include <vector>\nvoid f(){for(std::vector<\nint>*a=0,*b=0;again();step()){} }'
        row = normalize(source, 'cpp')
        self.assertEqual(self.reviews(row), [], row)

    def test_actual_nested_callback_body_remains_eligible(self):
        source = '#include <vector>\nvoid f(){consume([](){std::vector<\nint> a,b;});}'
        row = normalize(source, 'cpp')
        self.assertIn('cpp-wrapped-proven-type-declaration-owner-review', self.kinds(row))

    def test_macro_type_use_diagnostics_remain_in_final_result(self):
        source = '#define int Hidden\nint a,b;'
        row = normalize(source, 'c')
        self.assertTrue(any('macro' in item['kind'] for item in row['diagnostics']), row)
        self.assertEqual(row['review_status'], 'needs-review')

    def test_non_c_language_keeps_existing_body_accounting(self):
        row = normalize('x=1; y=2;', 'python')
        self.assertEqual(row['normalized_loc'], 2)
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(self.reviews(row), [])
