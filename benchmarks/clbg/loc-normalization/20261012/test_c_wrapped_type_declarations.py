"""Unexecuted wrapped-type source-window fixtures for a future locked job."""

import unittest

from c_wrapped_type_declarations import classify_wrapped_type_declaration_gates
from c_type_environment import collect_typedef_aliases, identify_decl, _type_prefix
from lexical import mask_source
from normalizer import _scope_map


class CWrappedTypeDeclarationFixtures(unittest.TestCase):
    def fixture(self, source, *, events=(), protected=(), before_mask=True):
        masked = mask_source(source, 'cpp')
        checked = masked['cleaned']
        contexts, _ = _scope_map(checked, 'cpp')
        environment = collect_typedef_aliases(checked, 'cpp',
            semantic_imports=masked.get('semantic_imports', ()), preprocessor_source=source,
            protected_literal_spans=masked.get('masked_literal_spans', ()))
        chars = list(checked)
        for event in events:
            for index in range(event['start'], event['end']):
                if not chars[index].isspace():
                    chars[index] = ' '
        body = ''.join(chars)
        result = classify_wrapped_type_declaration_gates(body, 'cpp', contexts, environment,
            type_prefix=_type_prefix, declaration_probe=identify_decl,
            type_source=checked if before_mask else None, owned_boundary_events=list(events),
            protected_spans=list(protected),
            preprocessor_directive_spans=environment['type_macro_environment']['directive_spans'])
        return checked, body, contexts, environment, result

    def wrapped(self, result):
        return [row for row in result['diagnostics']
                if row['kind'] == 'cpp-wrapped-proven-type-declaration-owner-review']

    def function_events(self, source):
        start, opening, closing = source.index('void f'), source.index('{'), source.rindex('}')
        return [dict(kind='header', scope='braced', start=start, end=opening),
                dict(kind='open', scope='braced', start=opening, end=opening + 1),
                dict(kind='close', scope='braced', start=closing, end=closing + 1)]

    def test_plain_packed_wrapped_type_has_exact_statement_and_type_spans(self):
        declaration = 'std::vector<\n int\n> a,b;'
        source = '#include <vector>\n' + declaration
        _, _, _, _, result = self.fixture(source)
        rows = self.wrapped(result)
        self.assertEqual(len(rows), 1)
        row = rows[0]
        at = source.index(declaration)
        self.assertEqual((row['offset'], row['type_start'], row['type_end']),
                         (at, at, source.index('> a') + 1))
        self.assertEqual(row['statement_end'], len(source))
        self.assertEqual(row['terminator_offset'], len(source) - 1)
        self.assertEqual(row['declaration_names'], ['a', 'b'])
        self.assertIsNone(row['units'])
        self.assertTrue(row['declaration_owner_proven'])

    def test_expanded_written_declarations_each_retain_cross_row_gate(self):
        source = '#include <vector>\nstd::vector<\nint> a;\nstd::vector<\nint> b;'
        _, _, _, _, result = self.fixture(source)
        rows = self.wrapped(result)
        self.assertEqual([row['declaration_names'] for row in rows], [['a'], ['b']])
        self.assertTrue(all(row['units'] is None for row in rows))

    def test_nested_proven_templates_remain_structural_inside_type_span(self):
        source = '#include <vector>\n#include <complex>\nstd::vector<\nstd::complex<double>\n> a,b;'
        _, _, _, _, result = self.fixture(source)
        row = self.wrapped(result)[0]
        self.assertEqual(row['type_evidence']['units'], 0)
        self.assertEqual(row['type_evidence']['arguments'][0]['units'], 0)
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_wrapped_modifier_of_proven_builtin_type_is_also_reviewed(self):
        source = 'const\nint a,b;'
        _, _, _, _, result = self.fixture(source)
        row = self.wrapped(result)[0]
        self.assertEqual(row['type_name'], 'int')
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_inline_type_with_packed_wrapped_literal_initializers_does_not_gate(self):
        source = ('#include <vector>\nstd::vector<const char*> a = {\n"X"\n},'
                  ' b = {\n"Y"\n};')
        first, other = source.index('{'), source.rindex('{')
        events = [dict(kind='value-expression', scope='composed-value', start=start,
                       end=source.index('}', start) + 1) for start in (first, other)]
        protected = [dict(start=source.index('std::vector'), end=len(source),
                          role='field', owner_proven=True)]
        checked, body, _, _, result = self.fixture(source, events=events, protected=protected)
        self.assertEqual(result['diagnostics'], [])
        self.assertEqual(len(body), len(source))
        self.assertEqual(body.count('\n'), source.count('\n'))
        self.assertIn('std::vector<const char*>', checked)
        self.assertNotIn('events', result)
        self.assertNotIn('cleaned', result)
        self.assertNotIn('normalized_loc', result)

    def test_inline_type_with_all_names_on_one_later_row_retains_declarator_gate(self):
        source = '#include <vector>\nstd::vector<int>\n a,b;'
        _, _, _, _, result = self.fixture(source)
        self.assertEqual([row['kind'] for row in result['diagnostics']],
                         ['cpp-wrapped-independent-declarators-owner-review'])
        self.assertEqual(result['diagnostics'][0]['declaration_name_rows'], [3, 3])
        self.assertEqual(result['diagnostics'][0]['type_start_row'], 2)

    def test_function_body_reset_exposes_its_wrapped_local_declaration(self):
        source = '#include <vector>\nvoid f(){std::vector<\nint> a,b;}'
        _, _, _, _, result = self.fixture(source, events=self.function_events(source))
        row = self.wrapped(result)[0]
        self.assertEqual(row['statement_start'], source.index('{') + 1)
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_nested_callback_body_declaration_survives_outer_call_context(self):
        source = '#include <vector>\nconsume([](){std::vector<\nint> a,b;});'
        opening, closing = source.index('{'), source.index('}')
        events = [dict(kind='header', scope='braced', start=source.index('[]'), end=opening),
                  dict(kind='open', scope='braced', start=opening, end=opening + 1),
                  dict(kind='close', scope='braced', start=closing, end=closing + 1)]
        _, _, _, _, result = self.fixture(source, events=events)
        row = self.wrapped(result)[0]
        self.assertEqual(row['offset'], source.index('std::vector'))
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_for_header_semicolons_do_not_expose_formal_fragments(self):
        source = '#include <vector>\nfor(std::vector<\nint>*a=0,*b=0;check();step()){}'
        _, _, _, _, result = self.fixture(source)
        self.assertEqual(result['diagnostics'], [])

    def test_grouped_operand_semicolons_never_reset_a_statement_window(self):
        source = '#include <vector>\nsink((first;std::vector<\nint> a,b;));'
        _, _, _, _, result = self.fixture(source)
        self.assertEqual(result['diagnostics'], [])

    def test_wrapped_constructor_temporaries_have_no_declaration_gate(self):
        for tail in (' (a,b);', ' {a,b};'):
            source = '#include <vector>\nvoid f(){int a=2,b=3;std::vector<\nint>' + tail + '}'
            _, _, _, _, result = self.fixture(source, events=self.function_events(source))
            self.assertEqual(result['diagnostics'], [])

    def test_unowned_wrapped_prototype_retains_unknown_tail_review(self):
        source = '#include <vector>\nstd::vector<\nint> f();'
        _, _, _, _, result = self.fixture(source)
        row = self.wrapped(result)[0]
        self.assertFalse(row['declaration_owner_proven'])
        self.assertTrue(row['tail_review_required'])
        self.assertEqual(row['declaration_names'], [])

    def test_exact_full_prototype_owner_suppresses_the_wrapped_body_gate(self):
        source = '#include <vector>\nstd::vector<\nint> f();'
        protected = [dict(start=source.index('std::vector'), end=len(source),
                          role='prototype', owner_proven=True)]
        _, _, _, _, result = self.fixture(source, protected=protected)
        self.assertEqual(result['diagnostics'], [])

    def test_partial_header_cannot_erase_wrapped_return_type_proof(self):
        source = '#include <vector>\nstd::vector<\nint> f(){return {};}'
        opening, closing = source.index('{'), source.rindex('}')
        events = [dict(kind='header', scope='braced', start=source.index('> f'), end=opening),
                  dict(kind='open', scope='braced', start=opening, end=opening + 1),
                  dict(kind='close', scope='braced', start=closing, end=closing + 1)]
        _, _, _, _, result = self.fixture(source, events=events)
        row = self.wrapped(result)[0]
        self.assertEqual(row['type_end'], source.index('> f') + 1)
        self.assertEqual(row['boundary_role'], 'canonical-header')
        self.assertTrue(row['tail_review_required'])

    def test_partial_header_without_preblank_source_retains_unknown_profile_review(self):
        source = '#include <vector>\nstd::vector<\nint> f(){return {};}'
        opening, closing = source.index('{'), source.rindex('}')
        events = [dict(kind='header', scope='braced', start=source.index('> f'), end=opening),
                  dict(kind='open', scope='braced', start=opening, end=opening + 1),
                  dict(kind='close', scope='braced', start=closing, end=closing + 1)]
        _, _, _, _, result = self.fixture(source, events=events, before_mask=False)
        self.assertTrue(any(row['kind'] == 'cpp-wrapped-header-owner-review'
                            and row['type_proof_available'] is False for row in result['diagnostics']))
        self.assertEqual(self.wrapped(result), [])

    def test_exact_full_function_profile_protects_a_partial_canonical_header(self):
        source = '#include <vector>\nstd::vector<\nint> f(){return {};}'
        opening, closing = source.index('{'), source.rindex('}')
        events = [dict(kind='header', scope='braced', start=source.index('> f'), end=opening),
                  dict(kind='open', scope='braced', start=opening, end=opening + 1),
                  dict(kind='close', scope='braced', start=closing, end=closing + 1)]
        protected = [dict(start=source.index('std::vector'), end=opening + 1,
                          role='function-profile', owner_proven=True)]
        _, _, _, _, result = self.fixture(source, events=events, protected=protected)
        self.assertEqual(result['diagnostics'], [])

    def test_exact_whole_alias_owner_is_a_boundary_and_not_a_body_declaration(self):
        source = '#include <vector>\nusing Vec=std::vector<\nint>;std::vector<\nint> a,b;'
        start, end = source.index('using Vec'), source.index(';') + 1
        events = [dict(kind='type-alias-binding', scope='source-local-type-alias', start=start, end=end)]
        _, _, _, _, result = self.fixture(source, events=events)
        row = self.wrapped(result)[0]
        self.assertEqual(row['statement_start'], end)
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_aggregate_alias_protection_keeps_unowned_field_body_visible(self):
        source = '#include <vector>\ntypedef struct Bag{std::vector<\nint>*a,*b;} Bag;'
        protected = [dict(start=source.index('typedef'), end=len(source),
                          role='whole-alias', owner_proven=True)]
        _, _, _, _, result = self.fixture(source, protected=protected)
        row = self.wrapped(result)[0]
        self.assertEqual(row['statement_start'], source.index('{') + 1)
        self.assertEqual(row['declaration_names'], ['a', 'b'])

    def test_already_owned_exact_field_is_excluded(self):
        source = '#include <vector>\nstd::vector<\nint> a,b;'
        protected = [dict(start=source.index('std::vector'), end=len(source),
                          role='field', owner_proven=True)]
        _, _, _, _, result = self.fixture(source, protected=protected)
        self.assertEqual(result['diagnostics'], [])

    def test_annotation_event_endpoint_never_replaces_statement_start(self):
        source = '#include <vector>\n[[tag]]\nstd::vector<\nint> a,b;'
        start, end = source.index('[['), source.index(']]') + 2
        events = [dict(kind='compiler-attribute', scope='annotation', start=start, end=end)]
        _, _, _, _, result = self.fixture(source, events=events)
        row = self.wrapped(result)[0]
        self.assertEqual(row['statement_start'], source.index('[[tag]]'))
        self.assertEqual(row['type_start'], source.index('std::vector'))

    def test_operand_event_endpoint_never_replaces_independent_semicolon(self):
        source = '#include <vector>\nconsume({0});\nstd::vector<\nint> a,b;'
        start, end = source.index('{0}'), source.index('{0}') + len('{0}')
        events = [dict(kind='value-expression', scope='composed-value', start=start, end=end)]
        _, _, _, _, result = self.fixture(source, events=events)
        row = self.wrapped(result)[0]
        self.assertEqual(row['statement_start'], source.index(';') + 1)
        self.assertNotEqual(row['statement_start'], end)

    def test_unknown_quoted_header_retains_existing_gate_without_type_promotion(self):
        source = '#include "vector"\nstd::vector<\nint> a,b;'
        _, _, _, environment, result = self.fixture(source)
        self.assertTrue(environment['diagnostics'])
        self.assertEqual(self.wrapped(result), [])

    def test_missing_or_malformed_environment_and_context_extent_are_diagnostics(self):
        source = 'const\nint a,b;'
        checked, body, contexts, environment, _ = self.fixture(source)
        cases = [(None, contexts), ({'source_length': len(source), 'aliases': []}, contexts),
                 (environment, contexts[:-1]), (environment, None)]
        for env, frames in cases:
            row = classify_wrapped_type_declaration_gates(body, 'cpp', frames, env,
                type_prefix=_type_prefix, declaration_probe=identify_decl, type_source=checked)
            self.assertEqual(row['diagnostics'][0]['kind'], 'cpp-wrapped-type-source-provenance-review')
            self.assertIsNone(row['diagnostics'][0]['units'])

    def test_source_projection_mismatch_and_uncertain_directive_never_certify_boundary(self):
        source = '#include <vector>\nstd::vector<\nint> a,b;'
        checked, body, contexts, environment, _ = self.fixture(source)
        directive = dict(environment['type_macro_environment']['directive_spans'][0], extent_proven=False)
        row = classify_wrapped_type_declaration_gates(body, 'cpp', contexts, environment,
            type_prefix=_type_prefix, type_source=checked,
            preprocessor_directive_spans=[directive])
        self.assertEqual(row['diagnostics'][0]['role'], 'unproved-preprocessor-boundary-span')
        row = classify_wrapped_type_declaration_gates(body, 'cpp', contexts, environment,
            type_prefix=_type_prefix, type_source='X' + checked[1:])
        self.assertEqual(row['diagnostics'][0]['role'], 'source-projection-token-or-newline-mismatch')

    def test_inline_type_with_named_declarators_on_different_rows_has_distinct_gate(self):
        source = 'int a,\n b,c;'
        _, _, _, _, result = self.fixture(source)
        rows = [row for row in result['diagnostics']
                if row['kind'] == 'cpp-wrapped-independent-declarators-owner-review']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['declaration_names'], ['a', 'b', 'c'])
        self.assertEqual(rows[0]['declaration_name_offsets'],
                         [source.index('a,'), source.index('b,'), source.index('c;')])
        self.assertEqual(rows[0]['declaration_name_rows'], [1, 2, 2])
        self.assertIsNone(rows[0]['units'])
        self.assertEqual(self.wrapped(result), [])

    def test_inline_type_with_names_before_wrapped_literal_data_has_no_name_row_gate(self):
        source = '#include <vector>\nstd::vector<const char*> a,b={\n"X"\n};'
        _, _, _, _, result = self.fixture(source)
        self.assertEqual(result['diagnostics'], [])

    def test_wrapped_type_and_wrapped_names_keep_their_two_reviews_distinct(self):
        source = '#include <vector>\nstd::vector<\nint> a,\n b;'
        _, _, _, _, result = self.fixture(source)
        self.assertEqual({row['kind'] for row in result['diagnostics']},
                         {'cpp-wrapped-proven-type-declaration-owner-review',
                          'cpp-wrapped-independent-declarators-owner-review'})

    def test_empty_enum_interior_point_event_is_not_a_reset_or_provenance_failure(self):
        source = '#include <vector>\nenum E{};std::vector<\nint> a,b;'
        point = source.index('{') + 1
        events = [dict(kind='declaration-interior-ownership', scope='enum',
                       start=point, end=point, units=0)]
        _, _, _, _, result = self.fixture(source, events=events)
        self.assertFalse(any(row['kind'] == 'cpp-wrapped-type-source-provenance-review'
                             for row in result['diagnostics']))
        self.assertEqual(self.wrapped(result)[0]['declaration_names'], ['a', 'b'])
