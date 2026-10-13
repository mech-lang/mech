"""Unexecuted fixtures for the unhooked single-row direct counter seam.

Only a future frozen worker holding the common benchmark timing mutex may
import, discover or run these fixtures and their lexer/parser dependencies.
Assertions are proposed contracts, not executed pass receipts or chart LOC.
"""

import copy
import unittest

from cpp_direct_counter_adapter import analyze_cpp_direct_counter_segment
from cpp_value_owners import collect_cpp_value_owners
from c_preprocessor_type_macros import type_macro_token_gates
from c_type_environment import (collect_typedef_aliases, identify_decl, _type_prefix,
                                _resolve)
from lexical import mask_source
from normalizer import _scope_map


class CppDirectCounterAdapterFixtures(unittest.TestCase):
    def prepare(self, source, marker, *, environment_mutator=None):
        masked = mask_source(source, 'cpp')
        cleaned = masked['cleaned']
        environment = collect_typedef_aliases(cleaned, 'cpp',
            semantic_imports=masked['semantic_imports'], preprocessor_source=source,
            protected_literal_spans=masked['masked_literal_spans'])
        if environment_mutator is not None:
            environment_mutator(environment)
        macros = environment['type_macro_environment']
        guard = lambda text, at, *, role: type_macro_token_gates(macros, text, at, role=role)
        inventory = collect_cpp_value_owners(cleaned, 'cpp', environment,
            declaration_resolver=identify_decl, type_prefix_resolver=_type_prefix,
            macro_token_gates=guard, literal_spans=masked['masked_literal_spans'])
        contexts, _ = _scope_map(cleaned, 'cpp')
        start = source.index(marker)
        end = start + len(marker)
        self.assertEqual(source[end - 1], ';', marker)
        # Fixture spans are explicitly the complete written marker. These
        # fixture helpers are not a proposed production ownership collector.
        previous = start - 1
        while previous >= 0 and cleaned[previous].isspace():
            previous -= 1
        protected = []
        if previous < 0:
            boundary = dict(kind='source-start', offset=0)
        elif cleaned[previous] in ';{}':
            kind = {';': 'statement-terminator', '{': 'executable-opener',
                    '}': 'executable-closer'}[cleaned[previous]]
            boundary = dict(kind=kind, offset=previous)
        else:
            line_start = cleaned.rfind('\n', 0, previous) + 1
            self.assertTrue(cleaned[line_start:previous + 1].lstrip().startswith('#'),
                            'Fixture must supply a separately owned prior boundary.')
            span = dict(start=line_start, end=previous + 1,
                        kind='fixture-explicit-preprocessor-owner')
            protected.append(span)
            boundary = dict(kind='owned-header-or-directive-boundary',
                            offset=span['end'], owner=span)
        statement = dict(kind='independent-semicolon-statement', start=start, end=end,
                         terminator_offset=end - 1, ownership_proven=True,
                         start_boundary=boundary)
        return dict(segment=cleaned[start:end], offset=start, source_mask=cleaned,
                    contexts=contexts, statement_owner=statement,
                    type_environment=environment, value_inventory=inventory,
                    literal_spans=masked['masked_literal_spans'], protected_spans=protected)

    def analyze(self, source, marker, **changes):
        arguments = self.prepare(source, marker)
        arguments.update(changes)
        segment = arguments.pop('segment')
        return analyze_cpp_direct_counter_segment(segment, **arguments)

    def assert_count(self, result, bindings, uninitialized=None):
        self.assertEqual(result['diagnostics'], [], result)
        self.assertIsNotNone(result['declaration'], result)
        self.assertEqual(result['counter_terms'], dict(declaration_bindings=bindings,
            uninitialized=bindings if uninitialized is None else uninitialized), result)

    def assert_gated(self, result, kind=None):
        self.assertIsNone(result['declaration'], result)
        self.assertIsNone(result['counter_terms'], result)
        self.assertFalse(result['fallback_allowed'], result)
        self.assertFalse(result['ordinary_fallback_used'], result)
        self.assertTrue(result['diagnostics'], result)
        if kind is not None:
            self.assertIn(kind, {row['kind'] for row in result['diagnostics']}, result)

    def test_exact_whitespace_offset_and_packed_numeric_direct_names(self):
        source = 'void f(){   int  a(0), b(1);}'
        result = self.analyze(source, '   int  a(0), b(1);')
        self.assert_count(result, 2)
        self.assertTrue(result['matched'])
        for row in result['declaration']['declarators']:
            at = source.index('   int') + result['declaration']['declarator_start'] + row['name_offset']
            self.assertEqual(source[at:at + len(row['name'])], row['name'])

    def test_segment_without_final_terminator_retains_exact_whole_owner(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        segment = arguments.pop('segment').removesuffix(';')
        self.assert_count(analyze_cpp_direct_counter_segment(segment, **arguments), 1)

    def test_spectralnorm_complete_template_and_source_value_roles(self):
        source = '#include <vector>\nusing namespace std;\nvoid f(int N){vector<double> u(N), v(N);}'
        result = self.analyze(source, 'vector<double> u(N), v(N);')
        self.assert_count(result, 2)
        self.assertTrue(all(row['direct_role'] == 'object' for row in result['declaration']['declarators']))

    def test_source_written_nested_loop_values_for_mandelbrot(self):
        marker = 'std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);'
        source = ('#include <complex>\nint main(){int n=100;'
                  'for(int y=0;y<n;++y) for(int x=0;x<n;++x){' + marker + '}}')
        self.assert_count(self.analyze(source, marker), 2)

    def test_callee_agnostic_composition_does_not_invent_value_owners(self):
        source = 'void f(){int a(arbitrary(factory()));}'
        result = self.analyze(source, 'int a(arbitrary(factory()));')
        self.assert_count(result, 1)
        self.assertTrue(any(row['kind'] == 'callee-agnostic-call-composition'
                            for row in result['evidence']))

    def test_actual_absent_bare_role_preserves_equal_count_profile(self):
        result = self.analyze('void f(){int a(mystery);}', 'int a(mystery);')
        self.assert_count(result, 1)
        self.assertEqual(result['declaration']['declarators'][0]['direct_role'], 'object-or-prototype')

    def test_absent_compound_leaf_remains_a_role_gate(self):
        result = self.analyze('void f(){int a(mystery+1),b(0);}', 'int a(mystery+1),b(0);')
        self.assert_gated(result, 'cpp-direct-argument-role-review')

    def test_explicit_blocked_role_cannot_use_bare_absence_profile(self):
        arguments = self.prepare('void f(){int a(mystery);}', 'int a(mystery);')
        arguments['value_inventory']['blocked_regions'].append(dict(
            start=arguments['offset'], end=arguments['statement_owner']['end'],
            kind='fixture-explicit-unknown-owner', units=0))
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-argument-role-review')

    def test_source_type_only_argument_is_a_prototype_formal(self):
        result = self.analyze('using U=int;void f(){int a(U);}', 'int a(U);')
        self.assert_count(result, 1)
        self.assertEqual(result['declaration']['declarators'][0]['direct_role'], 'prototype')

    def test_type_value_competition_in_argument_is_not_arbitrarily_resolved(self):
        source = 'struct U{};int U=0;void f(){int a(U);}'
        self.assert_gated(self.analyze(source, 'int a(U);'), 'cpp-direct-argument-role-review')

    def test_conditional_type_ownership_cannot_use_absent_bare_profile(self):
        source = '#if FLAG\nusing U=int;\n#endif\nvoid f(){int a(U);}'
        self.assert_gated(self.analyze(source, 'int a(U);'), 'cpp-direct-argument-role-review')

    def test_qualified_argument_role_is_explicitly_reviewed(self):
        source = 'void f(){int a(ns::mystery);}'
        self.assert_gated(self.analyze(source, 'int a(ns::mystery);'), 'cpp-direct-argument-role-review')

    def test_prototype_pointer_reference_type_id_suffixes_are_structural(self):
        source = 'using U=int;void f(){int prototype(U const* p, U&& r);}'
        self.assert_count(self.analyze(source, 'int prototype(U const* p, U&& r);'), 1)

    def test_explicit_assignment_cast_type_id_suffix_is_fully_consumed(self):
        source = 'void f(){int p=0;int a(static_cast<int*>(p));}'
        self.assert_count(self.analyze(source, 'int a(static_cast<int*>(p));'), 1)

    def test_source_identifier_S_is_not_treated_as_a_literal_mask(self):
        source = 'void f(){int S=1;int a(S+1);}'
        result = self.analyze(source, 'int a(S+1);')
        self.assert_count(result, 1)

    def test_actual_checked_literal_overlap_stays_gated(self):
        source = 'void f(){int a("text");}'
        result = self.analyze(source, 'int a("text");')
        self.assert_gated(result, 'cpp-direct-literal-data-owner-review')

    def test_omitted_literal_inventory_does_not_admit_mask_letters(self):
        result = self.analyze('void f(){int a("text");}', 'int a("text");', literal_spans=None)
        self.assert_gated(result, 'cpp-direct-literal-provenance-review')

    def test_fake_literal_source_correspondence_is_rejected(self):
        arguments = self.prepare('void f(){int a("text");}', 'int a("text");')
        arguments['literal_spans'] = copy.deepcopy(arguments['literal_spans'])
        arguments['literal_spans'][0]['source_text'] = 'X' * len(arguments['literal_spans'][0]['source_text'])
        segment = arguments.pop('segment')
        # Same-width nonspace text still masks identically, so token provenance
        # itself cannot prove reconstructed original contents. Corrupt width
        # as well; source_text must be original lexical evidence, never guessed.
        arguments['literal_spans'][0]['source_text'] += 'X'
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-literal-provenance-review')

    def test_overlapping_actual_literal_owners_are_rejected(self):
        arguments = self.prepare('void f(){int a("text");}', 'int a("text");')
        arguments['literal_spans'] = arguments['literal_spans'] * 2
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-literal-provenance-review')

    def test_active_macro_callee_is_checked_outside_role_callback(self):
        source = '#define factory() 0\nvoid f(){int a(factory());}'
        self.assert_gated(self.analyze(source, 'int a(factory());'),
                          'c-type-preprocessor-macro-owner-review')

    def test_active_macro_builtin_cast_is_checked_outside_role_callback(self):
        source = '#define double int\nvoid f(){int n=1;int a((double)n);}'
        self.assert_gated(self.analyze(source, 'int a((double)n);'),
                          'c-type-preprocessor-macro-owner-review')

    def test_macro_undef_restores_inactive_callee_token(self):
        source = '#define factory() 0\n#undef factory\nvoid f(){int a(factory());}'
        self.assert_count(self.analyze(source, 'int a(factory());'), 1)

    def test_unrelated_macro_does_not_promote_or_block_actual_tokens(self):
        source = '#define OTHER 0\nvoid f(){int a(factory());}'
        self.assert_count(self.analyze(source, 'int a(factory());'), 1)

    def test_missing_macro_event_inventory_stays_gated(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        arguments['type_environment'] = dict(arguments['type_environment'])
        arguments['type_environment'].pop('type_macro_environment')
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-counter-macro-inventory-review')

    def test_wrapped_declarators_never_reach_ordinary_fallback(self):
        marker = 'int a(0),\n b(1);'
        self.assert_gated(self.analyze('void f(){' + marker + '}', marker),
                          'cpp-direct-wrapped-statement-owner-review')

    def test_wrapped_template_prefix_is_not_reduced_to_a_row(self):
        marker = 'std::vector<\n int\n> a(0),b(1);'
        source = '#include <vector>\nvoid f(){' + marker + '}'
        self.assert_gated(self.analyze(source, marker), 'cpp-direct-wrapped-statement-owner-review')

    def test_only_a_tail_row_of_complete_statement_cannot_get_fallback(self):
        marker = 'int a(0),\n b(1);'
        arguments = self.prepare('void f(){' + marker + '}', marker)
        arguments['offset'] += marker.index('b')
        segment = 'b(1);'
        arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-statement-span-review')

    def test_written_semicolon_without_independent_owner_is_insufficient(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        arguments['statement_owner'].pop('ownership_proven')
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-statement-owner-review')

    def test_structural_for_header_is_not_a_declaration_statement(self):
        source = 'void f(){for(int a(0);a<1;++a){}}'
        self.assert_gated(self.analyze(source, 'int a(0);'),
                          'cpp-direct-structural-statement-owner-review')

    def test_protected_existing_header_or_field_owner_is_excluded(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        arguments['protected_spans'].append(dict(start=arguments['offset'],
            end=arguments['statement_owner']['end'], kind='fixture-existing-field-owner'))
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-protected-statement-owner-review')

    def test_multiple_independent_statements_are_not_one_owner(self):
        marker = 'int a(0);int b(1);'
        self.assert_gated(self.analyze('void f(){' + marker + '}', marker),
                          'cpp-direct-multiple-statement-owner-review')

    def test_mismatched_source_inventory_is_not_rebased_or_repaired(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        arguments['value_inventory'] = dict(arguments['value_inventory'], source_mask='different')
        segment = arguments.pop('segment')
        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),
                          'cpp-direct-counter-input-owner-review')

    def test_direct_delimiter_diagnostic_before_match_prevents_fallback(self):
        calls = []
        def direct(*args, **kwargs):
            return dict(declaration=None, matched=False, evidence=[], diagnostics=[dict(
                kind='fixture-before-match-diagnostic', offset=kwargs['offset'])])
        def ordinary(*args, **kwargs):
            calls.append((args, kwargs))
            return dict(declaration_bindings=99, uninitialized=99)
        result = self.analyze('void f(){int a(0);}', 'int a(0);',
                              direct_analyzer=direct, ordinary_resolver=ordinary)
        self.assert_gated(result, 'fixture-before-match-diagnostic')
        self.assertEqual(calls, [])

    def test_matched_diagnostic_discards_partial_rows_and_prevents_fallback(self):
        calls = []
        def direct(*args, **kwargs):
            return dict(declaration=dict(declaration_bindings=1, uninitialized=1),
                matched=True, evidence=[], diagnostics=[dict(kind='fixture-partial-gate', offset=kwargs['offset'])])
        def ordinary(*args, **kwargs):
            calls.append(True)
        result = self.analyze('void f(){int a(0);}', 'int a(0);',
                              direct_analyzer=direct, ordinary_resolver=ordinary)
        self.assert_gated(result, 'fixture-partial-gate')
        self.assertTrue(result['matched'])
        self.assertEqual(calls, [])

    def test_matched_without_complete_declaration_is_a_contract_gate(self):
        direct = lambda *args, **kwargs: dict(declaration=None, matched=True, evidence=[], diagnostics=[])
        result = self.analyze('void f(){int a(0);}', 'int a(0);', direct_analyzer=direct)
        self.assert_gated(result, 'cpp-direct-counter-helper-contract-review')

    def test_unmatched_helper_cannot_smuggle_declaration_units(self):
        direct = lambda *args, **kwargs: dict(declaration=dict(declaration_bindings=1, uninitialized=1),
            matched=False, evidence=[], diagnostics=[])
        result = self.analyze('void f(){int a(0);}', 'int a(0);', direct_analyzer=direct)
        self.assert_gated(result, 'cpp-direct-counter-helper-contract-review')

    def test_ordinary_packed_declarations_use_unchanged_fallback(self):
        result = self.analyze('void f(){int a,b;}', 'int a,b;')
        self.assert_count(result, 2)
        self.assertFalse(result['matched'])
        self.assertTrue(result['fallback_allowed'])
        self.assertTrue(result['ordinary_fallback_used'])

    def test_late_ordinary_diagnostic_clears_both_fallback_branches(self):
        for marker in ('int a,b;', 'Unknown a,b;'):
            with self.subTest(marker=marker):
                arguments = self.prepare('void f(){' + marker + '}', marker)
                original_environment = copy.deepcopy(arguments['type_environment'])
                calls = []
                def ordinary(segment, environment, *, offset):
                    calls.append((segment, offset))
                    environment['diagnostics'].append(dict(
                        kind='fixture-late-ordinary-owner-review', offset=offset + 2,
                        reason='The resolver found countable names but could not prove their complete owner.'))
                    declarator_start = segment.index('a')
                    return dict(count=2, declaration_bindings=2, uninitialized=2,
                        declarator_start=declarator_start, reason='fixture-provisional-ordinary-count',
                        declarators=[dict(name='a', name_offset=0, initialized=False),
                                     dict(name='b', name_offset=2, initialized=False)])
                segment = arguments.pop('segment')
                result = analyze_cpp_direct_counter_segment(segment, **arguments,
                                                            ordinary_resolver=ordinary)
                self.assertEqual(calls, [(segment, arguments['offset'])])
                self.assertFalse(result['matched'])
                self.assertIsNone(result['declaration'])
                self.assertIsNone(result['counter_terms'])
                self.assertFalse(result['fallback_allowed'])
                # This historical flag records the one invocation that emitted
                # the gate; it never grants another fallback or count.
                self.assertTrue(result['ordinary_fallback_used'])
                self.assertEqual(result['diagnostics'], [dict(
                    kind='fixture-late-ordinary-owner-review', offset=arguments['offset'] + 2,
                    reason='The resolver found countable names but could not prove their complete owner.')])
                self.assertEqual(arguments['type_environment'], original_environment)

    def test_ordinary_chained_assignment_units_remain_the_callers(self):
        result = self.analyze('void f(){int a=0,b=c=0;}', 'int a=0,b=c=0;')
        self.assert_count(result, 2, 0)
        self.assertEqual(result['counter_terms']['uninitialized'] + 3, 3)
        self.assertNotIn('assignment_events', result)

    def test_mixed_direct_and_assignment_bindings_keep_legacy_additive_terms(self):
        marker = 'int a(0),b=c=0;'
        result = self.analyze('void f(){' + marker + '}', marker)
        self.assert_count(result, 2, 1)
        rows = result['declaration']['declarators']
        self.assertFalse(rows[0]['initialized'])
        self.assertTrue(rows[0]['direct_initialized'])
        self.assertTrue(rows[1]['initialized'])
        self.assertEqual(result['counter_terms']['uninitialized'] + 2, 3)

    def test_independent_constructor_member_resolver_is_not_broadened(self):
        arguments = self.prepare('void f(){int m();}', 'int m();')
        self.assertIsNone(identify_decl(arguments['segment'], arguments['type_environment'],
                                       offset=arguments['offset']))
        segment = arguments.pop('segment')
        result = analyze_cpp_direct_counter_segment(segment, **arguments)
        self.assert_count(result, 1)
        self.assertEqual(result['declaration']['declarators'][0]['direct_role'], 'prototype')
        self.assertIsNone(identify_decl(segment, arguments['type_environment'], offset=arguments['offset']))

    def test_successful_direct_count_does_not_mutate_type_or_value_inventories(self):
        arguments = self.prepare('void f(){int a(0);}', 'int a(0);')
        before = copy.deepcopy((arguments['type_environment'], arguments['value_inventory']))
        segment = arguments.pop('segment')
        self.assert_count(analyze_cpp_direct_counter_segment(segment, **arguments), 1)
        self.assertEqual(before, (arguments['type_environment'], arguments['value_inventory']))

    def test_single_direct_name_imported_type_shadow_is_a_collection_obligation(self):
        source = '#include <vector>\nusing namespace std;\nint vector(0); vector<int> a,b;'
        arguments = self.prepare(source, 'int vector(0);')
        aliases = copy.deepcopy(arguments['type_environment']['aliases'])
        shadows = copy.deepcopy(arguments['type_environment']['shadows'])
        segment = arguments.pop('segment')
        result = analyze_cpp_direct_counter_segment(segment, **arguments)
        self.assert_gated(result, 'cpp-direct-type-shadow-collection-review')
        self.assertEqual(len(result['shadow_obligations']), 1)
        self.assertEqual(arguments['type_environment']['aliases'], aliases)
        self.assertEqual(arguments['type_environment']['shadows'], shadows)
        later = self.analyze(source, 'vector<int> a,b;')
        self.assertIsNone(later['declaration'], later)

    def test_packed_direct_imported_type_shadow_is_not_repaired_by_count(self):
        source = '#include <vector>\nusing namespace std;\nint vector(0),other(1); vector<int> a,b;'
        result = self.analyze(source, 'int vector(0),other(1);')
        self.assert_gated(result, 'cpp-direct-type-shadow-collection-review')
        self.assertEqual([row['name'] for row in result['shadow_obligations']], ['vector'])
        later = self.analyze(source, 'vector<int> a,b;')
        self.assertIsNone(later['declaration'], later)

    def test_omitted_type_shadow_record_cannot_override_independent_value_blocker(self):
        source = '#include <vector>\nusing namespace std;\nint vector(0); vector<int> a,b;'
        def omit(environment):
            environment['shadows'] = [row for row in environment['shadows'] if row['name'] != 'vector']
        arguments = self.prepare(source, 'vector<int> a,b;', environment_mutator=omit)
        segment = arguments.pop('segment')
        result = analyze_cpp_direct_counter_segment(segment, **arguments)
        self.assert_gated(result, 'cpp-direct-type-value-prefix-owner-review')

    def test_absent_positive_type_prefix_does_not_promote_uppercase_spelling(self):
        result = self.analyze('void f(){Mystery a(0),b(1);}', 'Mystery a(0),b(1);')
        self.assertIsNone(result['declaration'])
        self.assertIsNone(result['counter_terms'])
        self.assertFalse(result['matched'])
        self.assertTrue(result['fallback_allowed'])

    def test_bad_supplied_prefix_contract_is_gated_before_fallback(self):
        def prefix(*args, **kwargs):
            return (1, 'int', dict(kind='fixture-explicit-unknown-type', unresolved=True))
        result = self.analyze('void f(){int a(0);}', 'int a(0);', type_prefix_resolver=prefix)
        self.assert_gated(result, 'cpp-direct-counter-type-prefix-contract-review')

    def test_argument_assignment_discards_all_names_without_fallback(self):
        marker = 'int a(0),b(value=1);'
        result = self.analyze('void f(){int value=0;' + marker + '}', marker)
        self.assert_gated(result, 'cpp-direct-argument-assignment-review')
        self.assertTrue(result['matched'])

    def test_executable_argument_body_keeps_its_separate_owner_gate(self):
        marker = 'int a([](){return 1;}),b(0);'
        result = self.analyze('void f(){' + marker + '}', marker)
        self.assert_gated(result, 'cpp-direct-argument-executable-review')


if __name__ == '__main__':
    unittest.main()
