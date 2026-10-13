"""Unexecuted named fixtures for the unhooked C++ value inventory.

Only a future worker owning the common timing mutex may import/discover/run
these fixtures and their lexer/type environment dependencies. They certify no
source LOC, native benchmark, or chart admission.
"""

import re
import unittest

from cpp_value_owners import collect_cpp_value_owners, resolve_cpp_value_owner
from c_type_environment import collect_typedef_aliases, identify_decl, _type_prefix
from lexical import mask_source


MANDELBROT_OWNERSHIP_EXCERPT = '''#include <complex>
int main (int argc, char **argv)
{
  char bit_num = 0, byte_acc = 0;
  const int iter = 50;
  const double lim = 2.0 * 2.0;
  int n = atoi(argv[1]);
  for(int y=0; y<n; ++y)
    for(int x=0; x<n; ++x)
    {
      std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);
      for(int i=0; i<iter and Z.real()*Z.real() + Z.imag()*Z.imag() <= lim; ++i)
        Z = Z*Z + C;
      if(++bit_num == 8){ emit(byte_acc); bit_num = byte_acc = 0; }
      else if(x == n-1) { byte_acc <<= (8-n%8); emit(byte_acc); }
    }
  finish(n);
}
'''


class CppSourceValueOwnerFixtures(unittest.TestCase):
    def inventory(self, source, *, macro_guard=None, language='cpp',
                  declaration_resolver=identify_decl, explicit_literals=True,
                  environment_mutator=None):
        masked = mask_source(source, 'cpp')
        environment = collect_typedef_aliases(masked['cleaned'], 'cpp',
            semantic_imports=masked['semantic_imports'],preprocessor_source=source,
            protected_literal_spans=masked['masked_literal_spans'])
        if environment_mutator is not None:
            environment_mutator(environment)
        return collect_cpp_value_owners(masked['cleaned'], language, environment,
            declaration_resolver=declaration_resolver,
            type_prefix_resolver=_type_prefix, macro_token_gates=macro_guard,
            literal_spans=masked['masked_literal_spans'] if explicit_literals else None)

    def value(self, inventory, name, source, marker, *, macro_guard=None, last=False):
        at = source.rindex(marker) if last else source.index(marker)
        token = re.search(r'(?<![A-Za-z_0-9])' + re.escape(name) + r'(?![A-Za-z_0-9])', marker)
        self.assertIsNotNone(token, (name, marker))
        at += token.start()
        return resolve_cpp_value_owner(inventory, name, at, macro_token_gates=macro_guard)

    def assert_value(self, proof, kind=None):
        self.assertIsNotNone(proof)
        self.assertEqual(proof['role'], 'value', proof)
        self.assertEqual(proof['units'], 0)
        if kind:
            self.assertEqual(proof['owner_kind'], kind)

    def assert_gated(self, proof):
        self.assertIsNotNone(proof)
        self.assertEqual(proof['role'], 'unknown', proof)
        self.assertTrue(proof.get('unresolved') or proof.get('macro'), proof)

    def test_other_language_does_not_promote_values(self):
        for language in ('c', 'c++', 'rust', 'java'):
            row = self.inventory('int n=0;', language=language)
            self.assertEqual(row['owners'], [])
            self.assertEqual(row['diagnostics'], [])

    def test_packed_ordinary_names_use_exact_offsets_and_zero_units(self):
        source = 'void f(){  int a=0, b=1, *p=0; consume(a,b,p);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        owners = {entry['name']: entry for entry in row['owners']}
        self.assertEqual(set(owners), {'a', 'b', 'p'})
        for name, owner in owners.items():
            self.assertEqual(source[owner['name_start']:owner['name_end']], name)
            self.assertEqual(owner['units'], 0)
        self.assertNotIn('events', row)

    def test_initializer_calls_and_chained_assignments_do_not_create_owners(self):
        source = 'void f(){int a=b=0;int n=arbitrary(a);consume(a,n,b);}'
        row = self.inventory(source)
        self.assertEqual({entry['name'] for entry in row['owners']}, {'a', 'n'})
        self.assert_value(self.value(row, 'n', source, 'consume(a,n,b)'))
        self.assertIsNone(self.value(row, 'b', source, 'consume(a,n,b)'))

    def test_missing_type_is_not_value_evidence(self):
        source = 'void f(){consume(mystery);}'
        row = self.inventory(source)
        self.assertIsNone(self.value(row, 'mystery', source, 'consume(mystery)'))

    def test_inner_using_alias_does_not_reuse_outer_value_proof(self):
        source = 'int T=0;void f(){using T=int;consume(T);}'
        row = self.inventory(source)
        proof = self.value(row, 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertEqual(proof['kind'], 'cpp-value-type-role-competition-review')
        self.assertTrue(proof['source_type_owners'])
        self.assertTrue(proof['source_value_owners'])

    def test_inner_source_class_does_not_reuse_outer_value_proof(self):
        source = 'int T=0;void f(){struct T{};consume(T);}'
        row = self.inventory(source)
        proof = self.value(row, 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertEqual(proof['kind'], 'cpp-value-type-role-competition-review')

    def test_type_alias_order_and_scope_expiry_preserve_outer_value(self):
        source = 'int T=0;void f(){before(T);{using T=int;inside(T);}after(T);}'
        row = self.inventory(source)
        self.assert_value(self.value(row, 'T', source, 'before(T)'))
        self.assert_gated(self.value(row, 'T', source, 'inside(T)'))
        self.assert_value(self.value(row, 'T', source, 'after(T)'))

    def test_class_order_and_scope_expiry_preserve_outer_value(self):
        source = 'int T=0;void f(){before(T);{struct T{};inside(T);}after(T);}'
        row = self.inventory(source)
        self.assert_value(self.value(row, 'T', source, 'before(T)'))
        self.assert_gated(self.value(row, 'T', source, 'inside(T)'))
        self.assert_value(self.value(row, 'T', source, 'after(T)'))

    def test_same_scope_class_and_value_do_not_get_an_arbitrary_priority(self):
        source = 'struct T{};int T=0;void f(){consume(T);}'
        proof = self.value(self.inventory(source), 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertTrue(proof['source_type_owners'])
        self.assertTrue(proof['source_value_owners'])

    def test_inner_value_and_outer_type_are_conservatively_competing(self):
        source = 'using T=int;void f(){int T=0;consume(T);}'
        proof = self.value(self.inventory(source), 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertEqual(proof['kind'], 'cpp-value-type-role-competition-review')

    def test_type_only_query_cannot_enter_unknown_bare_value_invariance(self):
        source = 'using T=int;void f(){consume(T);}'
        proof = self.value(self.inventory(source), 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertEqual(proof['source_value_owners'], [])

    def test_conditional_type_alias_does_not_prove_an_outer_value(self):
        source = 'int T=0;void f(){\n#ifdef FLAG\nusing T=int;\n#endif\nconsume(T);}'
        proof = self.value(self.inventory(source), 'T', source, 'consume(T)')
        self.assert_gated(proof)
        self.assertTrue(any(owner['conditional'] for owner in proof['source_type_owners']))

    def test_type_owner_at_end_of_source_has_valid_empty_future_visibility(self):
        row = self.inventory('using T=int;')
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(len(row['type_owners']), 1)
        owner = row['type_owners'][0]
        self.assertEqual(owner['visible_start'], owner['scope_end'])
        self.assertEqual(owner['units'], 0)

    def test_malformed_type_scope_metadata_is_not_reconstructed(self):
        source = 'int n=0;using T=int;void f(){consume(n);}'
        def damage(environment):
            environment['aliases']['T'][0]['scope_end'] = True
        row = self.inventory(source, environment_mutator=damage)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))
        self.assertIn('cpp-value-type-owner-span-review', {d['kind'] for d in row['diagnostics']})

    def test_unknown_type_owner_category_is_not_promoted(self):
        source = 'int n=0;using T=int;void f(){consume(n);}'
        def damage(environment):
            environment['aliases']['T'][0]['kind'] = 'fixture-made-up-type-owner'
        row = self.inventory(source, environment_mutator=damage)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_known_nonmacro_type_alias_does_not_block_unrelated_value(self):
        source = 'void f(){int n=0;using T=int;after(n);}'
        row = self.inventory(source)
        self.assert_value(self.value(row, 'n', source, 'after(n)'))

    def test_declaration_order_is_not_retroactive(self):
        source = 'void f(){consume(n);int n=1;consume(n);}'
        row = self.inventory(source)
        self.assertIsNone(self.value(row, 'n', source, 'consume(n)'))
        self.assert_value(self.value(row, 'n', source, 'consume(n)', last=True))

    def test_array_declaration_point_follows_written_array_suffix(self):
        source = 'void f(){int values[4];consume(values);}'
        row = self.inventory(source)
        owner = row['owners'][0]
        self.assertEqual(owner['declaration_point'], source.index(']') + 1)
        self.assert_value(self.value(row, 'values', source, 'consume(values)'))

    def test_nested_block_owner_expires_and_outer_owner_resumes(self):
        source = 'void f(){int n=1;{consume(n);int n=2;inside(n);}outside(n);}'
        row = self.inventory(source)
        before = self.value(row, 'n', source, 'consume(n)')
        inner = self.value(row, 'n', source, 'inside(n)')
        after = self.value(row, 'n', source, 'outside(n)')
        self.assert_value(before)
        self.assert_value(inner)
        self.assert_value(after)
        self.assertEqual(before['name_start'], after['name_start'])
        self.assertNotEqual(inner['name_start'], after['name_start'])

    def test_competing_same_scope_owners_are_not_reconciled_by_spelling(self):
        source = 'void f(){int n=1;int n=2;consume(n);}'
        self.assert_gated(self.value(self.inventory(source), 'n', source, 'consume(n)'))

    def test_named_function_formals_are_values_only_in_body(self):
        source = 'void f(int n,const double*values){consume(n,values);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        self.assert_value(self.value(row, 'n', source, 'consume(n,values)'),
                          'cpp-source-function-formal-value')
        self.assert_value(self.value(row, 'values', source, 'consume(n,values)'))
        self.assertIsNone(resolve_cpp_value_owner(row, 'n', source.index('int n') + 4))

    def test_function_prototype_formals_do_not_escape(self):
        source = 'void prototype(int n);void f(){consume(n);}'
        row = self.inventory(source)
        self.assertFalse(any(entry['name'] == 'n' for entry in row['owners']))
        self.assertIsNone(self.value(row, 'n', source, 'consume(n)'))

    def test_formal_defaults_reject_complete_formal_list(self):
        source = 'void f(int a,int b=1){consume(a,b);}'
        row = self.inventory(source)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'a', source, 'consume(a,b)'))
        self.assertFalse(any(entry.get('role') == 'value' for entry in row['evidence']))
        self.assertIn('cpp-value-formal-default-review', {d['kind'] for d in row['diagnostics']})

    def test_wrapped_formal_list_is_not_a_body_value_proof(self):
        source = 'void f(int a,\n int b){consume(a,b);}'
        row = self.inventory(source)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'a', source, 'consume(a,b)'))

    def test_unknown_formal_type_blocks_body_roles(self):
        source = 'int n=0;void f(Mystery value){consume(n,value);}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'n', source, 'consume(n,value)'))
        self.assert_gated(self.value(row, 'value', source, 'consume(n,value)'))

    def test_wrapped_declaration_remains_blocked(self):
        source = 'void f(){int n=\n 1;consume(n);}'
        row = self.inventory(source)
        self.assertFalse(any(entry['name'] == 'n' for entry in row['owners']))
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_callback_declaration_does_not_become_an_ordinary_value_owner(self):
        source = 'void f(){int n=[](){return 1;};consume(n);}'
        row = self.inventory(source)
        self.assertFalse(any(entry['name'] == 'n' for entry in row['owners']))
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_direct_object_or_prototype_count_profile_is_rejected(self):
        source = 'void f(){int n(0);consume(n);}'
        def false_direct(segment, environment, *, offset):
            if segment == 'int n(0)':
                return dict(declarator_start=4, type_name='int', type_evidence=dict(kind='builtin-type'),
                    declarators=[dict(name='n', name_offset=0, declarator='n(0)',
                                     direct_role='object-or-prototype', direct_initialized=True)])
            return identify_decl(segment, environment, offset=offset)
        row = self.inventory(source, declaration_resolver=false_direct)
        self.assertFalse(any(entry['name'] == 'n' for entry in row['owners']))
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))
        self.assertIn('cpp-value-declaration-profile-review', {d['kind'] for d in row['diagnostics']})

    def test_written_macro_without_callback_blocks_admission(self):
        source = '#define NAME 1\nvoid f(){int n=0;consume(n);}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))
        self.assertTrue(row['written_macro_offsets'])
        self.assertTrue(any(d.get('source_evidence') for d in row['diagnostics']))

    def test_macro_callback_is_queried_at_exact_owner_and_use_offsets(self):
        source = '#define OTHER 1\n#undef OTHER\nvoid f(){int n=0;consume(n);}'
        calls = []
        def guard(text, at, *, role):
            calls.append((text, at, role))
            self.assertEqual(source[at:at + len(text)], text)
            return []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'consume(n)', macro_guard=guard))
        self.assertTrue(any(text == 'int n=0' for text, _, _ in calls))
        self.assertTrue(any(text == 'n' and role == 'source-value-use' for text, _, role in calls))

    def test_use_requires_fresh_macro_proof_after_collection(self):
        source = '#define OTHER 1\nvoid f(){int n=0;consume(n);}'
        guard = lambda text, at, role: []
        row = self.inventory(source, macro_guard=guard)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_later_active_macro_name_is_rejected_at_use(self):
        source = 'void f(){int n=0;\n#define n 1\nconsume(n);\n#undef n\nconsume(n);}'
        define = source.index('#define n')
        undef = source.index('#undef n')
        def guard(text, at, *, role):
            if define < at < undef and re.search(r'\bn\b', text):
                return [dict(kind='fixture-active-macro-review', offset=at, macro=True)]
            return []
        row = self.inventory(source, macro_guard=guard)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)', macro_guard=guard))
        # Undef does not retroactively prove the prior unknown macro statement's
        # expansion/lifetime. Its enclosing owner remains blocked.
        self.assert_gated(self.value(row, 'n', source, 'consume(n)', last=True, macro_guard=guard))

    def test_undef_without_prior_unknown_statement_restores_use_macro_proof(self):
        source = 'void f(){int n=0;before(n);\n#define n 1\n#undef n\nafter(n);}'
        define, undef = source.index('#define n'), source.index('#undef n')
        def guard(text, at, *, role):
            return [dict(kind='fixture-active-macro-review', offset=at)] if define < at < undef and text == 'n' else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(resolve_cpp_value_owner(row, 'n', define + len('#define '), macro_token_gates=guard))
        self.assert_value(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_macro_statement_can_hide_later_binding_ownership(self):
        source = '#define DECLARE() anything\nvoid f(){int n=0;before(n);DECLARE();after(n);}'
        def guard(text, at, *, role):
            if re.search(r'\bDECLARE\b', text):
                return [dict(kind='fixture-active-macro-review', offset=at, macro=True)]
            return []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_macro_in_skipped_using_statement_blocks_later_value(self):
        source = '#define MAKE_TYPE() anything\nvoid f(){int n=0;before(n);using T=MAKE_TYPE();after(n);}'
        calls = []
        def guard(text, at, *, role):
            calls.append((text, at, role))
            return [dict(kind='fixture-owner-review', offset=at)] if 'MAKE_TYPE' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))
        self.assertTrue(any(text.startswith('using T=') and role == 'source-value-statement'
                            for text, _, role in calls))

    def test_macro_in_skipped_typedef_statement_blocks_later_value(self):
        source = '#define MAKE_TYPE() anything\nvoid f(){int n=0;before(n);typedef MAKE_TYPE() T;after(n);}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'MAKE_TYPE' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_macro_in_skipped_return_statement_blocks_later_value(self):
        source = '#define DECLARE() anything\nvoid f(){int n=0;before(n);return DECLARE();after(n);}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'DECLARE' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_missing_macro_callback_still_gates_a_skipped_return_envelope(self):
        source = '#define DECLARE() anything\nvoid f(){return DECLARE();}'
        row = self.inventory(source)
        self.assertTrue(any(issue['kind'] == 'cpp-value-statement-macro-owner-review'
                            and issue['offset'] == source.index('return')
                            for issue in row['diagnostics']))

    def test_nonmacro_return_does_not_invent_hidden_declarations(self):
        source = 'void f(){int n=0;return n;after(n);}'
        row = self.inventory(source)
        self.assert_value(self.value(row, 'n', source, 'after(n)'))

    def test_macro_in_skipped_aggregate_body_blocks_surrounding_value(self):
        source = '#define DECLARE() anything\nvoid f(){int n=0;before(n);struct Local{int field;DECLARE();};after(n);}'
        calls = []
        def guard(text, at, *, role):
            calls.append((text, at, role))
            return [dict(kind='fixture-owner-review', offset=at)] if 'DECLARE' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))
        self.assertTrue(any(text == 'DECLARE();' and at == source.index('DECLARE();')
                            for text, at, _ in calls))
        self.assertFalse(any(owner['name'] == 'field' for owner in row['owners']))

    def test_macro_control_header_does_not_leave_child_or_later_scope_usable(self):
        source = '#define TEST() anything\nvoid f(){int n=0;before(n);if(TEST()){inside(n);}after(n);}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'TEST' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_value(self.value(row, 'n', source, 'before(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'inside(n)', macro_guard=guard))
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_function_header_macro_blocks_body_lifetime_proof(self):
        source = '#define NAME anything\nvoid NAME(){int n=0;consume(n);}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-active-macro-review', offset=at)] if 'NAME' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)', macro_guard=guard))
        self.assertIn('cpp-value-function-header-review', {d['kind'] for d in row['diagnostics']})

    def test_conditional_owner_stays_blocked_after_directive_projection(self):
        source = 'void f(){\n#ifdef FLAG\nint n=0;\n#endif\nconsume(n);}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))
        self.assertTrue(any(d.get('source_evidence') for d in row['diagnostics']))

    def test_braced_for_owner_covers_header_and_body_only(self):
        source = 'void f(){for(int i=0;i<2;++i){inside(i);}outside(i);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        self.assert_value(self.value(row, 'i', source, 'i<2'), 'cpp-source-for-initializer-value')
        self.assert_value(self.value(row, 'i', source, 'inside(i)'))
        self.assertIsNone(self.value(row, 'i', source, 'outside(i)'))

    def test_unbraced_simple_for_body_has_exact_semicolon_lifetime(self):
        source = 'void f(){for(int i=0;i<2;++i)inside(i);outside(i);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        self.assert_value(self.value(row, 'i', source, 'inside(i)'))
        self.assertIsNone(self.value(row, 'i', source, 'outside(i)'))

    def test_nested_unbraced_for_lifetime_matches_actual_mandelbrot_shape(self):
        source = 'void f(){int n=8;for(int y=0;y<n;++y)for(int x=0;x<n;++x){inside(x,y,n);}outside(x,y,n);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        for name in ('x', 'y', 'n'):
            self.assert_value(self.value(row, name, source, 'inside(x,y,n)'))
        self.assertIsNone(self.value(row, 'x', source, 'outside(x,y,n)'))
        self.assertIsNone(self.value(row, 'y', source, 'outside(x,y,n)'))
        self.assert_value(self.value(row, 'n', source, 'outside(x,y,n)'))

    def test_actual_mandelbrot_compound_argument_leaves_have_source_owners(self):
        source = MANDELBROT_OWNERSHIP_EXCERPT
        row = self.inventory(source)
        for name, marker in (('x', '(double)x/n'), ('y', '(double)y/n'), ('n', 'x/n')):
            self.assert_value(self.value(row, name, source, marker))
        self.assert_value(self.value(row, 'n', source, 'finish(n)'))
        self.assertFalse(any(entry['name'] in {'Z', 'C'} for entry in row['owners']))
        self.assertTrue(row['diagnostics'])  # direct declarations and unknown if/else bodies remain gated

    def test_unknown_unbraced_loop_body_is_not_guessed(self):
        source = 'void f(){for(int i=0;i<2;++i)if(i)inside(i);outside(i);}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'i', source, 'inside(i)'))
        self.assert_gated(self.value(row, 'i', source, 'outside(i)'))
        self.assertIn('cpp-value-for-lifetime-review', {d['kind'] for d in row['diagnostics']})

    def test_loop_header_macro_is_part_of_lifetime_proof(self):
        source = '#define LIMIT anything\nvoid f(){for(int i=0;i<LIMIT;++i){inside(i);}}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'LIMIT' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_gated(self.value(row, 'i', source, 'inside(i)', macro_guard=guard))
        self.assertIn('cpp-value-for-header-owner-review', {d['kind'] for d in row['diagnostics']})

    def test_conditional_loop_body_does_not_prove_unselected_lifetime(self):
        source = 'void f(){for(int i=0;i<2;++i){\n#ifdef FLAG\ninside(i);\n#endif\n}}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'i', source, 'inside(i)'))

    def test_spaced_blocked_for_name_keeps_its_written_offset(self):
        source = '#define LIMIT anything\nvoid f(){for(  int i=0;i<LIMIT;++i){inside(i);}}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'LIMIT' in text else []
        row = self.inventory(source, macro_guard=guard)
        owner = next(entry for entry in row['blocked_owners'] if entry['name'] == 'i')
        self.assertEqual(owner['name_start'], source.index('int i') + 4)
        self.assertEqual(source[owner['name_start']:owner['name_end']], 'i')

    def test_range_for_is_explicitly_outside_classic_loop_grammar(self):
        source = 'void f(){for(auto i:items){inside(i);}outside(i);}'
        row = self.inventory(source)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'i', source, 'inside(i)'))

    def test_unknown_imperative_region_does_not_certify_outer_value_inside(self):
        source = 'void f(){int n=0;before(n);if(n){inside(n);}after(n);}'
        row = self.inventory(source)
        self.assert_value(self.value(row, 'n', source, 'before(n)'))
        self.assert_gated(self.value(row, 'n', source, 'inside(n)'))
        self.assert_value(self.value(row, 'n', source, 'after(n)'))

    def test_unknown_packed_type_cannot_reuse_outer_value_owner(self):
        source = 'void f(){int b=0;Unknown a,b;consume(b);}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'b', source, 'consume(b)'))
        self.assertIn('cpp-value-unknown-type-owner-review', {d['kind'] for d in row['diagnostics']})

    def test_unscoped_enum_exports_cannot_reuse_outer_value_owner(self):
        source = 'void f(){int n=0;{enum E{n=1};consume(n);}}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))
        self.assertIn('cpp-value-aggregate-export-owner-review', {d['kind'] for d in row['diagnostics']})

    def test_aggregate_instance_suffix_does_not_reuse_outer_value_owner(self):
        source = 'void f(){int n=0;{struct Local{} n;consume(n);}}'
        row = self.inventory(source)
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_macro_callback_gate_does_not_require_a_kind_spelling(self):
        source = '#define DECLARE() anything\nvoid f(){int n=0;DECLARE();after(n);}'
        def guard(text, at, *, role):
            return [dict(kind='fixture-owner-review', offset=at)] if 'DECLARE' in text else []
        row = self.inventory(source, macro_guard=guard)
        self.assert_gated(self.value(row, 'n', source, 'after(n)', macro_guard=guard))

    def test_real_s_identifiers_are_not_confused_with_masks(self):
        source = 'void f(){int S=1,SSS=2;consume(S,SSS);}'
        row = self.inventory(source)
        self.assertEqual(row['diagnostics'], [], row)
        self.assert_value(self.value(row, 'S', source, 'consume(S,SSS)'))
        self.assert_value(self.value(row, 'SSS', source, 'consume(S,SSS)'))

    def test_literal_overlap_uses_exact_lexer_provenance(self):
        source = 'void f(){const char* text="n";consume(text);}'
        row = self.inventory(source)
        self.assertFalse(any(entry['name'] == 'text' for entry in row['owners']))
        self.assert_gated(self.value(row, 'text', source, 'consume(text)'))

    def test_missing_literal_inventory_cannot_admit_names(self):
        source = 'void f(){int n=0;consume(n);}'
        row = self.inventory(source, explicit_literals=False)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))

    def test_query_requires_the_exact_written_name_token(self):
        source = 'void f(){int n=0;consume(n);}'
        row = self.inventory(source)
        for offset in (-1, len(source), source.index('consume'), True):
            self.assert_gated(resolve_cpp_value_owner(row, 'n', offset))

    def test_unbalanced_source_does_not_admit_partial_owners(self):
        source = 'void f(){int n=0;consume(n);'
        row = self.inventory(source)
        self.assertEqual(row['owners'], [])
        self.assert_gated(self.value(row, 'n', source, 'consume(n)'))


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
