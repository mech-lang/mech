"""Unexecuted macro proof-gate fixtures for a future reviewed mutex job."""

import unittest

from c_preprocessor_type_macros import (collect_type_macro_events,
    type_macro_name_gates, type_macro_token_gates, type_macro_owner_gates,
    with_type_macro_owner_gate, without_type_macro_directives)


class CPreprocessorTypeMacroFixtures(unittest.TestCase):
    def collect(self, source, cleaned=None):
        cleaned = source if cleaned is None else cleaned
        return collect_type_macro_events(cleaned, 'cpp', original_source=source)

    def test_object_definition_exact_name_and_visibility_offsets(self):
        source = '#define IUB 4\nstd::vector<IUB> a,b;'
        environment = self.collect(source)
        event = environment['events'][0]
        self.assertEqual((event['name_start'], event['name_end']), (8, 11))
        self.assertEqual(event['visible_start'], source.index('std::'))
        gates = type_macro_token_gates(environment, 'std::vector<IUB>', source.index('std::'))
        self.assertEqual([row['token_name'] for row in gates], ['IUB'])
        self.assertEqual(gates[0]['offset'], source.index('IUB>'))
        self.assertIsNone(gates[0]['units'])
        self.assertEqual(environment['diagnostics'], [])

    def test_unconditional_undef_resets_future_token_ownership(self):
        source = '#define T 4\nT first;\n#undef T\nT later;'
        environment = self.collect(source)
        self.assertTrue(type_macro_name_gates(environment, 'T', source.index('T first')))
        self.assertEqual(type_macro_name_gates(environment, 'T', source.index('T later')), [])
        self.assertEqual([row['directive'] for row in environment['events']], ['define', 'undef'])

    def test_conditional_definition_is_uncertain_after_endif(self):
        source = '#if FEATURE\n#define T 4\n#endif\nT first;\n#undef T\nT later;'
        environment = self.collect(source)
        first = type_macro_name_gates(environment, 'T', source.index('T first'))
        self.assertEqual(first[0]['macro_state'], 'uncertain')
        self.assertEqual(type_macro_name_gates(environment, 'T', source.index('T later')), [])

    def test_conditional_undef_cannot_reset_active_macro(self):
        source = '#define T 4\n#if FEATURE\n#undef T\n#else\n#endif\nT first;\n#undef T\nT later;'
        environment = self.collect(source)
        self.assertEqual(type_macro_name_gates(environment, 'T', source.index('T first'))[0]['macro_state'],
                         'uncertain')
        self.assertEqual(type_macro_name_gates(environment, 'T', source.index('T later')), [])

    def test_nested_and_alternative_branches_are_not_evaluated(self):
        source = '#if 1\n#if 0\n#define T int\n#elifdef OTHER\n#undef T\n#endif\n#endif\nT value;'
        environment = self.collect(source)
        self.assertTrue(all(row['conditional'] for row in environment['events']))
        self.assertEqual(type_macro_name_gates(environment, 'T', source.index('T value'))[0]['macro_state'],
                         'uncertain')

    def test_macro_lifetime_ignores_source_brace_scopes(self):
        source = 'void f(){\n#define T int\n}\nT value;'
        environment = self.collect(source)
        self.assertTrue(type_macro_name_gates(environment, 'T', source.index('T value')))

    def test_each_qualified_identifier_component_uses_its_actual_offset(self):
        source = '#define std foreign\n#define vector Other\nstd::vector<int> a,b;'
        environment = self.collect(source)
        gates = type_macro_token_gates(environment, 'std::vector<int>', source.index('std::'))
        self.assertEqual([row['token_name'] for row in gates], ['std', 'vector'])
        self.assertEqual([row['offset'] for row in gates],
                         [source.index('std::'), source.index('vector<int>')])

    def test_nested_proven_type_span_queries_argument_identifiers(self):
        source = '#define IUB 4\nstd::vector<std::array<IUB,4>> a,b;'
        environment = self.collect(source)
        gates = type_macro_token_gates(environment, 'std::vector<std::array<IUB,4>>', source.index('std::'))
        self.assertEqual([row['token_name'] for row in gates], ['IUB'])
        self.assertEqual(gates[0]['offset'], source.index('IUB,4'))

    def test_header_contract_exposure_does_not_claim_filename_expansion(self):
        source = '#define vector Other\n#include <vector>\n#undef vector\nstd::vector<int> a,b;'
        cleaned = source.replace('<vector>', 'S' * 8)
        environment = self.collect(source, cleaned)
        entry = dict(name='std::vector', kind='explicit-header-class-template-contract',
                     definition_start=source.index('#include'),
                     definition_end=source.index('<vector>') + 8,
                     source_text=cleaned[source.index('#include'):source.index('<vector>') + 8],
                     template_contract=dict(name='std::vector'))
        gates = type_macro_owner_gates(environment, entry)
        self.assertEqual(gates[0]['role'], 'imported-type-contract-macro-exposure')
        self.assertFalse(gates[0]['source_token'])
        self.assertEqual(gates[0]['offset'], entry['definition_end'])
        self.assertEqual(type_macro_token_gates(environment, 'std::vector<int>', source.index('std::')), [])
        self.assertTrue(with_type_macro_owner_gate(environment, entry)['macro'])

    def test_protected_filename_alone_is_not_queried_as_type_identifier(self):
        source = '#define vector Other\n#include <vector>\n'
        cleaned = source.replace('<vector>', 'S' * 8)
        environment = self.collect(source, cleaned)
        entry = dict(name='std::complex', kind='explicit-header-class-template-contract',
                     definition_start=source.index('#include'),
                     definition_end=source.index('<vector>') + 8,
                     template_contract=dict(name='std::complex'))
        self.assertEqual(type_macro_owner_gates(environment, entry), [])

    def test_clean_repeated_include_does_not_repair_exposed_header_declaration(self):
        source = '#define vector Other\n#include <vector>\n#undef vector\n#include <vector>\n'
        environment = self.collect(source, source.replace('<vector>', 'S' * 8))
        first, later = source.index('#include'), source.rindex('#include')
        def entry(start):
            return dict(name='std::vector', kind='explicit-header-class-template-contract',
                        definition_start=start, definition_end=start + len('#include <vector>'),
                        template_contract=dict(name='std::vector'))
        original = with_type_macro_owner_gate(environment, entry(first))
        repeated = with_type_macro_owner_gate(environment, entry(later), prior_contract_owners=[original])
        self.assertTrue(repeated['macro'])
        self.assertEqual(repeated['macro_type_gates'][0]['role'],
                         'prior-imported-type-contract-macro-exposure')

    def test_macro_affected_source_class_remains_gated_after_undef(self):
        source = '#define IUB Other\nstruct IUB{};\n#undef IUB\nstd::vector<IUB> a,b;'
        environment = self.collect(source)
        start = source.index('struct IUB')
        entry = dict(name='IUB', kind='source-local-cpp-named-type',
                     definition_start=start, definition_end=start + len('struct IUB{'),
                     source_text='struct IUB{')
        copy = with_type_macro_owner_gate(environment, entry)
        self.assertTrue(copy['macro'])
        self.assertNotIn('macro', entry)
        self.assertEqual(type_macro_name_gates(environment, 'IUB', source.index('IUB>')), [])

    def test_clean_later_source_declaration_has_independent_ownership(self):
        source = '#define IUB Other\nstruct IUB{};\n#undef IUB\nstruct IUB{};'
        environment = self.collect(source)
        first = source.index('struct IUB')
        later = source.rindex('struct IUB')
        def entry(start):
            return dict(name='IUB', kind='source-local-cpp-named-type',
                        definition_start=start, definition_end=start + len('struct IUB{'),
                        source_text='struct IUB{')
        self.assertTrue(with_type_macro_owner_gate(environment, entry(first))['macro'])
        self.assertNotIn('macro', with_type_macro_owner_gate(environment, entry(later)))

    def test_source_alias_name_and_rhs_have_declaration_point_gates(self):
        source = '#define Alias Other\n#define IUB 4\nusing Alias=std::vector<IUB>;\n#undef Alias\n#undef IUB\nAlias a,b;'
        environment = self.collect(source)
        text = 'using Alias=std::vector<IUB>;'
        entry = dict(name='Alias', kind='source-local-using', definition_start=source.index(text),
                     definition_end=source.index(text) + len(text), source_text=text)
        gates = type_macro_owner_gates(environment, entry)
        self.assertEqual([row['token_name'] for row in gates], ['Alias', 'IUB'])
        self.assertTrue(with_type_macro_owner_gate(environment, entry)['macro'])

    def test_namespace_import_queries_written_std_without_freezing_vector_macro(self):
        source = '#define vector Other\nusing namespace std;\n#undef vector\nvector<int> a,b;'
        environment = self.collect(source)
        text = 'using namespace std;'
        entry = dict(name='vector', kind='explicit-standard-template-namespace-import',
                     definition_start=source.index(text), definition_end=source.index(text) + len(text),
                     source_text=text)
        self.assertEqual(type_macro_owner_gates(environment, entry), [])
        self.assertEqual(type_macro_token_gates(environment, 'vector<int>', source.index('vector<int>')), [])

    def test_function_like_macros_remain_conservative_without_expansion(self):
        source = '#define IUB(x) replacement(x)\nstd::vector<IUB> a,b;'
        environment = self.collect(source)
        self.assertTrue(environment['events'][0]['function_like'])
        gates = type_macro_name_gates(environment, 'IUB', source.index('IUB>'))
        self.assertTrue(gates[0]['function_like'])

    def test_continued_directive_spans_never_own_source_declarations(self):
        source = '#define T \\\nstruct IUB{};\nstd::vector<IUB> a,b;'
        environment = self.collect(source)
        self.assertTrue(environment['events'][0]['continued'])
        at = source.index('struct IUB')
        gates = type_macro_token_gates(environment, 'struct IUB{', at, role='type-declaration-point')
        self.assertEqual(gates[-1]['kind'], 'c-type-macro-directive-token-owner-review')
        scratch = without_type_macro_directives(source, environment)
        self.assertEqual(len(scratch), len(source))
        self.assertEqual(scratch.count('\n'), source.count('\n'))
        self.assertNotIn('struct IUB', scratch)
        self.assertIn('std::vector<IUB>', scratch)

    def test_original_source_proves_splice_hidden_by_literal_mask(self):
        source = '#define TEXT "a\\\nb"\nstruct IUB{};'
        chars = list(source)
        start, end = source.index('"'), source.rindex('"') + 1
        for offset in range(start, end):
            if not chars[offset].isspace():
                chars[offset] = 'S'
        cleaned = ''.join(chars)
        environment = self.collect(source, cleaned)
        self.assertEqual(environment['events'][0]['end'], source.index('struct IUB'))
        self.assertTrue(environment['events'][0]['continued'])

    def test_comments_and_literals_do_not_introduce_directive_owners(self):
        source = '/*\n#define T int\n*/\n"#define U int";\nstruct IUB{};'
        cleaned = ''.join(char if char.isspace() else ' ' for char in source[:source.index('struct IUB')]) + 'struct IUB{};'
        environment = self.collect(source, cleaned)
        self.assertEqual(environment['events'], [])

    def test_missing_original_extent_and_mismatched_source_remain_gated(self):
        source = '#define T int\nstruct IUB{};'
        for original in (None, source + ' '):
            environment = collect_type_macro_events(source, 'cpp', original_source=original)
            gates = type_macro_name_gates(environment, 'IUB', source.index('IUB'))
            self.assertTrue(gates)
            self.assertEqual(gates[0]['macro_state'], 'uncertain')

    def test_malformed_undef_and_unbalanced_conditionals_remain_gated(self):
        for source in ('#define T int\n#undef T extra\nT value;',
                       '#if FEATURE\n#define T int\nT value;',
                       '#endif\nT value;'):
            environment = self.collect(source)
            self.assertTrue(environment['diagnostics'])
            self.assertTrue(type_macro_name_gates(environment, 'T', source.index('T value')))

    def test_unrelated_macros_and_external_flag_text_are_not_inferred(self):
        source = '#define CONSTANT 4\nstruct IUB{};\nstd::vector<int> a,b;'
        environment = self.collect(source)
        environment['external_compiler_flags'] = ['-Dvector=Other']
        self.assertEqual(type_macro_token_gates(environment, 'std::vector<int>', source.index('std::')), [])
        self.assertEqual(type_macro_owner_gates(environment, dict(name='IUB',
            kind='source-local-cpp-named-type', source_text='struct IUB{',
            definition_start=source.index('struct IUB'))), [])

    def test_missing_use_offset_never_guesses_macro_lifetime(self):
        source = '#define T int\n#undef T\nT value;'
        environment = self.collect(source)
        self.assertTrue(type_macro_name_gates(environment, 'T', None))

    def test_literal_mask_identifiers_are_protected_from_macro_queries(self):
        source = '#define S Other\ntypedef int Alias[sizeof("X S X")];'
        start, end = source.index('"'), source.rindex('"') + 1
        cleaned = source[:start] + ''.join(char if char.isspace() else 'S'
                                         for char in source[start:end]) + source[end:]
        environment = collect_type_macro_events(cleaned, 'cpp', original_source=source,
            protected_literal_spans=[dict(start=start, end=end)])
        entry = dict(name='Alias', kind='source-local-typedef',
                     definition_start=source.index('typedef'), definition_end=len(source),
                     source_text=cleaned[source.index('typedef'):])
        self.assertEqual(type_macro_owner_gates(environment, entry), [])

    def test_spliced_line_comment_cannot_certify_apparent_include_or_later_undef(self):
        source = '// comment\\\n#include <vector>\n#undef vector\nstd::vector<int> a,b;'
        first_end = source.index('#include')
        cleaned = ''.join(char if char.isspace() else ' ' for char in source[:first_end]) + source[first_end:]
        environment = self.collect(source, cleaned)
        entry = dict(name='std::vector', kind='explicit-header-class-template-contract',
                     definition_start=source.index('#include'),
                     definition_end=source.index('<vector>') + 8,
                     template_contract=dict(name='std::vector'))
        self.assertTrue(type_macro_owner_gates(environment, entry))
        self.assertTrue(type_macro_token_gates(environment, 'std::vector<int>', source.index('std::')))
        self.assertTrue(any(row['kind'] == 'c-type-preprocessor-directive-ownership-review'
                            for row in environment['diagnostics']))

    def test_split_macro_name_and_alternative_introducer_remain_uncertain(self):
        for source in ('#define IU\\\nB 4\n#undef IUB\nIUB value;',
                       '%:define IUB 4\nIUB value;'):
            environment = self.collect(source)
            self.assertTrue(environment['diagnostics'])
            self.assertTrue(type_macro_name_gates(environment, 'IUB', source.index('IUB value')))

    def test_spliced_comment_source_owner_is_gated_but_later_clean_owner_is_independent(self):
        source = '// comment\\\nstruct IUB{};\nstruct IUB{};\nIUB a,b;'
        first = source.index('struct IUB')
        later = source.rindex('struct IUB')
        cleaned = ''.join(char if char.isspace() else ' ' for char in source[:first]) + source[first:]
        environment = self.collect(source, cleaned)
        def entry(start):
            return dict(name='IUB', kind='source-local-cpp-named-type',
                        definition_start=start, definition_end=start + len('struct IUB{'),
                        source_text='struct IUB{')
        self.assertTrue(with_type_macro_owner_gate(environment, entry(first))['macro'])
        self.assertNotIn('macro', with_type_macro_owner_gate(environment, entry(later)))
        self.assertEqual(type_macro_name_gates(environment, 'IUB', source.index('IUB a,b')), [])
        self.assertTrue(any(row['kind'] == 'c-type-preprocessor-source-ownership-review'
                            for row in environment['diagnostics']))
