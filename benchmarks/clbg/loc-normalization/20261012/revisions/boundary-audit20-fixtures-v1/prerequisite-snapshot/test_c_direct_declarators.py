"""Unexecuted direct-declarator proposal fixtures for a future locked worker.

A future caller may supply analyze_direct_decl, identify_direct_decl and the
unchanged existing _declarators under the common timing boundary, then call
exercise_direct_declarator_fixtures. The unittest wrapper imports proposal
dependencies only when a future worker loads this source under the common
timing boundary. No import, fixture, or discovery has run during preparation.
The file does not collect types, read frozen inputs, lex, or normalize/count LOC.
The assertions describe proposed declaration counts, never measured LOC totals.
"""


SPECTRAL_PACKED = 'vector<double> u(N), v(N);'
SPECTRAL_EXPANDED = ('vector<double> u(N);', 'vector<double> v(N);')
SPECTRAL_MEMBER = 'vector<double> v(u.size());'
MANDELBROT_PACKED = 'std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);'
MANDELBROT_EXPANDED = (
    'std::complex<double> Z(0,0);',
    'std::complex<double> C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);',
)

ACCEPTED_FIXTURES = (
    dict(name='spectralnorm-packed', text=SPECTRAL_PACKED, type='vector<double>', count=2),
    dict(name='spectralnorm-member-call', text=SPECTRAL_MEMBER, type='vector<double>', count=1),
    dict(name='mandelbrot-packed-casts', text=MANDELBROT_PACKED, type='std::complex<double>', count=2),
    dict(name='callee-agnostic-nested-calls', text='T a(arbitrary(factory(value),obj.method(i))),b(0);', type='T', count=2),
    dict(name='member-index-and-operators', text='T a(obj.field[i]+value*2),b(value ? a : b);', type='T', count=2),
    dict(name='pointer-reference-direct-heads', text='T *a(0),&b(value),&&c(value);', type='T', count=3),
    dict(name='prototype-structural-formals', text='T f(const double* values,int sizes[2]),g();', type='T', count=2),
    dict(name='prototype-object-ambiguity', text='T a(U()),b((U())),c(U(value));', type='T', count=3),
    dict(name='unknown-bare-role-count-invariance', text='T a(mystery),b(0);', type='T', count=2),
    dict(name='explicit-cpp-cast', text='T a(static_cast<double>(value)),b((double)value);', type='T', count=2),
    dict(name='comparisons-are-composition', text='T a(value<2 && i>=0),b(value==i);', type='T', count=2),
    dict(name='separate-declaration-rows', text='T a(0),\n  b(1);', type='T', count=2),
)

GATED_FIXTURES = (
    dict(name='argument-assignment', text='T a(value=0),b(1);', kind='cpp-direct-argument-assignment-review'),
    dict(name='formal-default', text='T f(int n=0),g();', kind='cpp-direct-argument-assignment-review'),
    dict(name='argument-compound-assignment', text='T a(value+=1),b(0);', kind='cpp-direct-argument-assignment-review'),
    dict(name='argument-alternative-assignment', text='T a(value and_eq 1),b(0);', kind='cpp-direct-argument-assignment-review'),
    dict(name='argument-update', text='T a(++value),b(0);', kind='cpp-direct-argument-update-review'),
    dict(name='nested-call-update', text='T a(factory(value++)),b(0);', kind='cpp-direct-argument-update-review'),
    dict(name='grouped-comma-sequence', text='T a((value,i)),b(0);', kind='cpp-direct-argument-grammar-review'),
    dict(name='index-comma-sequence', text='T a(obj[value,i]),b(0);', kind='cpp-direct-argument-grammar-review'),
    dict(name='ternary-comma-sequence', text='T a(value ? i,value : 0),b(0);', kind='cpp-direct-argument-grammar-review'),
    dict(name='semicolon-sequence', text='T a((value;i)),b(0);', kind='cpp-direct-argument-executable-review'),
    dict(name='lambda-callback', text='T a([](){return 1;}),b(0);', kind='cpp-direct-argument-executable-review'),
    dict(name='braced-data-argument', text='T a({1,2}),b(0);', kind='cpp-direct-argument-executable-review'),
    dict(name='control-in-argument', text='T a(if(value)),b(0);', kind='cpp-direct-argument-control-review'),
    dict(name='new-operand-role', text='T a(new U),b(0);', kind='cpp-direct-argument-control-review'),
    dict(name='unknown-expression-leaf', text='T a(mystery+1),b(0);', kind='cpp-direct-argument-role-review'),
    dict(name='unknown-multiword-formal', text='T f(Mystery item),g();', kind='cpp-direct-argument-role-review'),
    dict(name='mixed-object-formal-list', text='T a(int,1),b(0);', kind='cpp-direct-mixed-argument-role-review'),
    dict(name='unsupported-formal-function-pointer', text='T f(int (*cb)(int)),g();', kind='cpp-direct-argument-role-review'),
    dict(name='unsupported-named-function-formal', text='T f(int callback()),g();', kind='cpp-direct-argument-role-review'),
    dict(name='reserved-declarator-name', text='T double(0),b(0);', kind='cpp-direct-reserved-declarator-name-review'),
    dict(name='reserved-callee', text='T a(decltype(value)),b(0);', kind='cpp-direct-argument-control-review'),
    dict(name='malformed-hex-float', text='T a(0x1.0),b(0);', kind='cpp-direct-argument-grammar-review'),
    dict(name='malformed-float-suffix', text='T a(1.0u),b(0);', kind='cpp-direct-argument-grammar-review'),
    dict(name='template-call-comma-role', text='T a(make<value,i>(0)),b(0);', kind='cpp-direct-template-argument-role-review'),
    dict(name='template-call-shift-comma-role', text='T a(make<x<<y,i>(0)),b(0);', kind='cpp-direct-template-argument-role-review'),
    dict(name='template-call-right-shift-comma-role', text='T a(make<x>>y,i>(0)),b(0);', kind='cpp-direct-template-argument-role-review'),
    dict(name='trailing-noexcept', text='T f() noexcept,g();', kind='cpp-direct-declarator-tail-review'),
    dict(name='trailing-array', text='T a(0)[2],b(0);', kind='cpp-direct-declarator-tail-review'),
    dict(name='trailing-function', text='T a(0)(1),b(0);', kind='cpp-direct-declarator-tail-review'),
    dict(name='multiline-scalar-arguments', text='T a(\n  value\n),b(0);', kind='cpp-direct-wrapped-arguments-review'),
    dict(name='multiline-formals', text='T f(int value,\n double i),g();', kind='cpp-direct-wrapped-arguments-review'),
)


def _source_roles(name, absolute_offset):
    values = {'N', 'x', 'y', 'n', 'u', 'a', 'b', 'c', 'value', 'i', 'obj', 'make'}
    types = {'T', 'U', 'Item', 'Scalar'}
    role = 'value' if name in values else 'type' if name in types else None
    if role is None:
        return None
    return dict(role=role, kind='fixture-explicit-source-role', name=name,
                start=absolute_offset, end=absolute_offset + len(name),
                owner='fixture-supplied-exact-offset-role')


def _prefix(spelling):
    # Fixture metadata is supplied explicitly; no type collector is invoked.
    return len(spelling), spelling, dict(kind='fixture-supplied-proven-prefix', type_name=spelling)


def exercise_direct_declarator_fixtures(analyze, identify, legacy_declarators):
    """Future guarded fixture entry point; caller owns all execution/imports."""
    checked = []
    for fixture in ACCEPTED_FIXTURES:
        text = fixture['text']
        before = text
        proposal = analyze(text, offset=1000, type_prefix=_prefix(fixture['type']),
                           role_resolver=_source_roles, legacy_declarators=legacy_declarators)
        declaration = proposal['declaration']
        assert declaration is not None, (fixture['name'], proposal)
        assert not proposal['diagnostics'], (fixture['name'], proposal)
        assert (declaration['count'], declaration['declaration_bindings'], declaration['uninitialized']) == (
            fixture['count'], fixture['count'], fixture['count']), (fixture['name'], proposal)
        assert all(row['argument_units'] == 0 and row['units'] == 1
                   and not row['initialized'] for row in declaration['declarators'])
        assert text == before
        assert identify(text, offset=1000, type_prefix=_prefix(fixture['type']),
                        role_resolver=_source_roles, legacy_declarators=legacy_declarators) == declaration
        checked.append(fixture['name'])
    for fixture in GATED_FIXTURES:
        proposal = analyze(fixture['text'], offset=1000, type_prefix=_prefix('T'),
                           role_resolver=_source_roles, legacy_declarators=legacy_declarators)
        assert proposal['declaration'] is None, (fixture['name'], proposal)
        assert fixture['kind'] in [issue['kind'] for issue in proposal['diagnostics']], (fixture['name'], proposal)
        assert identify(fixture['text'], offset=1000, type_prefix=_prefix('T'),
                        role_resolver=_source_roles, legacy_declarators=legacy_declarators) is None
        checked.append(fixture['name'])

    # Packed and expanded spelling must propose the same independent names.
    for packed, expanded, spelling in (
            (SPECTRAL_PACKED, SPECTRAL_EXPANDED, 'vector<double>'),
            (MANDELBROT_PACKED, MANDELBROT_EXPANDED, 'std::complex<double>')):
        together = analyze(packed, offset=0, type_prefix=_prefix(spelling), role_resolver=_source_roles)
        separate = [analyze(text, offset=0, type_prefix=_prefix(spelling), role_resolver=_source_roles)
                    for text in expanded]
        assert together['declaration']['count'] == sum(item['declaration']['count'] for item in separate) == 2
        assert [row['name'] for row in together['declaration']['declarators']] == [
            item['declaration']['declarators'][0]['name'] for item in separate]
    checked.append('packed-expanded-source-declarations')

    # Preserve the original scalar/pointer/reference/array/= delegate intact.
    text = 'T a(0),*p,&r=value,arr[2],b=c=0;'
    proposal = analyze(text, offset=100, type_prefix=_prefix('T'),
                       role_resolver=_source_roles, legacy_declarators=legacy_declarators)
    declaration = proposal['declaration']
    assert declaration is not None, proposal
    assert (declaration['count'], declaration['uninitialized']) == (5, 3), proposal
    assert declaration['declarators'][-1]['declarator'] == 'b=c=0'
    assert not declaration['declarators'][-1]['direct_initialized']
    assert declaration['declarators'][0]['direct_initialized']
    assert [row['name'] for row in declaration['declarators']] == ['a', 'p', 'r', 'arr', 'b']
    # The existing caller's three written '=' owners remain additive: 3+3=6.
    assert declaration['uninitialized'] + 3 == 6
    checked.append('legacy-additive-counter-and-byte-preservation')

    # Absolute name/group offsets include all original leading/part whitespace.
    text = '  T  a(0),\n   b(value);  '
    proposal = analyze(text, offset=500, type_prefix=_prefix('T'), role_resolver=_source_roles)
    declaration = proposal['declaration']
    assert declaration is not None, proposal
    for row in declaration['declarators']:
        assert text[declaration['declarator_start'] + row['name_offset']:].startswith(row['name'])
        assert text[row['argument_opening'] - 500] == '('
        assert text[row['argument_end'] - 501] == ')'
    value_evidence = [row for row in proposal['evidence'] if row['kind'] == 'resolved-value-operand']
    assert value_evidence[0]['offset'] == 500 + text.index('value')
    checked.append('exact-original-offsets')

    # Unknown bare names record invariant grammar roles, never become types.
    proposal = analyze('T a(mystery);', offset=0, type_prefix=_prefix('T'))
    assert proposal['declaration']['count'] == 1
    assert proposal['declaration']['declarators'][0]['direct_role'] == 'object-or-prototype'
    assert any(row['kind'] == 'bare-name-object-or-formal-ambiguity' for row in proposal['evidence'])
    checked.append('bare-name-ambiguity-evidence')

    # Callees have no closed spelling allowlist; nested argument leaves do.
    proposal = analyze('T a(unlisted_member.callee()),b(any_namespace::build(0));',
                       offset=0, type_prefix=_prefix('T'))
    assert proposal['declaration']['count'] == 2, proposal
    checked.append('callee-spelling-is-agnostic')

    # Single-row literal metadata owns exact mask bytes; no mask is created.
    text = 'T a(SSS),b(0);'
    span = dict(kind='literal', start=200 + text.index('SSS'),
                end=200 + text.index('SSS') + 3, source_text='"x"',
                owner='fixture-supplied-exact-literal-token')
    proposal = analyze(text, offset=200, type_prefix=_prefix('T'), literal_spans=(span,))
    assert proposal['declaration']['count'] == 2, proposal
    assert any(row.get('owner') == span['owner'] for row in proposal['evidence'])
    assert text == 'T a(SSS),b(0);'
    for bad in (dict(span, end=span['end'] + 1), dict(span, source_text='"xy"'),
                dict(span, kind='data'), dict(span, source_text='x\ny'),
                dict(span, start=span['start'] - 1, end=span['end'] - 1)):
        proposal = analyze(text, offset=200, type_prefix=_prefix('T'), literal_spans=(bad,))
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    proposal = analyze(text, offset=200, type_prefix=_prefix('T'), literal_spans=(span, span))
    assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('exact-literal-span-contract-and-row-gates')

    # Unsupported templates cannot split into admitted extra argument owners.
    proposal = analyze('T a(make<U,Other>()),b(0);', offset=0,
                       type_prefix=_prefix('T'), role_resolver=_source_roles)
    assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('template-call-role-gate')

    # Rejected/flagged source evidence cannot become safe bare-name ambiguity.
    for flag in ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed'):
        def flagged_name(name, at):
            return dict(role='value', kind='fixture-rejected-owner', **{flag: True})
        proposal = analyze('T a(ARG),b(0);', offset=0, type_prefix=_prefix('T'),
                           role_resolver=flagged_name)
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
        def flagged_type(text, at):
            return (len('Scalar'), dict(kind='fixture-rejected-type', **{flag: True})) if text.startswith('Scalar') else None
        proposal = analyze('T a((Scalar)value),b(0);', offset=0, type_prefix=_prefix('T'),
                           role_resolver=_source_roles, type_id_resolver=flagged_type)
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('rejected-owner-flags-remain-gates')

    # Literal ownership takes precedence over identical name/type spellings.
    def mask_name_type(name, at):
        return dict(role='type', kind='fixture-supplied-type') if name == 'SSS' else _source_roles(name, at)
    for text in ('T a((SSS)0),b(0);', 'T a(static_cast<SSS>(0)),b(0);',
                 'T SSS(0),b(0);', 'T a(0),SSS=0;', 'T a(obj.SSS()),b(0);',
                 'T a(ns::SSS()),b(0);', 'T a(0),b=SSS(0);'):
        literal = dict(kind='literal', start=text.index('SSS'), end=text.index('SSS') + 3,
                       source_text='"x"', owner='fixture-exact-literal-token')
        proposal = analyze(text, offset=0, type_prefix=_prefix('T'),
                           role_resolver=mask_name_type, literal_spans=(literal,),
                           legacy_declarators=legacy_declarators)
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('literal-mask-cannot-become-a-type')

    # The recursion bound applies to flat cast chains as well as delimiters.
    proposal = analyze('T a(' + '(int)' * 40 + '0),b(0);', offset=0, type_prefix=_prefix('T'))
    assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('flat-cast-recursion-bound')

    # Existing '=' fallback keeps outer assignments but gates argument work.
    for text in ('T a(0),b=factory(value++);', 'T a(0),b=factory(value=0);',
                 'T a(0),b=factory(\nvalue\n);', 'T a(0),b=[](){return 1;};',
                 'T a(0),b=[]() mutable {return 1;};',
                 'T a(0),b=[]() noexcept {return 1;};'):
        proposal = analyze(text, offset=0, type_prefix=_prefix('T'),
                           role_resolver=_source_roles, legacy_declarators=legacy_declarators)
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    checked.append('delegated-rhs-argument-work-remains-gated')

    for context in ('parameters', 'args', 'control', 'data', 'header', 'body'):
        proposal = analyze('T a(0),b(0);', offset=0, type_prefix=_prefix('T'), context=context)
        assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    proposal = analyze('T a(0),b(0);', offset=0, type_prefix=None)
    assert proposal['declaration'] is None and proposal['diagnostics'], proposal
    proposal = analyze('T a=0,b;', offset=0, type_prefix=_prefix('T'), legacy_declarators=legacy_declarators)
    assert not proposal['matched'] and not proposal['diagnostics'] and proposal['declaration'] is None
    checked.append('caller-context-and-prefix-proof-are-required')
    return tuple(checked)


# These are source-text imports, deliberately not executed during proposal
# preparation. Future unittest discovery must run inside its worker-owned lock.
import unittest
from c_direct_declarators import analyze_direct_decl, identify_direct_decl
from c_type_environment import _declarators


class CDirectDeclaratorFixtures(unittest.TestCase):
    def accepted(self, name):
        fixture = next(row for row in ACCEPTED_FIXTURES if row['name'] == name)
        proposal = analyze_direct_decl(fixture['text'], offset=1000,
            type_prefix=_prefix(fixture['type']), role_resolver=_source_roles,
            legacy_declarators=_declarators)
        declaration = proposal['declaration']
        self.assertIsNotNone(declaration, proposal)
        self.assertFalse(proposal['diagnostics'], proposal)
        self.assertEqual((declaration['count'], declaration['uninitialized']),
                         (fixture['count'], fixture['count']), proposal)

    def gated(self, name):
        fixture = next(row for row in GATED_FIXTURES if row['name'] == name)
        proposal = analyze_direct_decl(fixture['text'], offset=1000,
            type_prefix=_prefix('T'), role_resolver=_source_roles,
            legacy_declarators=_declarators)
        self.assertIsNone(proposal['declaration'], proposal)
        self.assertIn(fixture['kind'], [row['kind'] for row in proposal['diagnostics']], proposal)

    def packed_expanded(self, packed, expanded, spelling):
        together = analyze_direct_decl(packed, offset=0, type_prefix=_prefix(spelling),
                                       role_resolver=_source_roles)
        separate = [analyze_direct_decl(text, offset=0, type_prefix=_prefix(spelling),
                                       role_resolver=_source_roles) for text in expanded]
        self.assertEqual(together['declaration']['count'], 2)
        self.assertEqual(sum(row['declaration']['count'] for row in separate), 2)
        self.assertEqual([row['name'] for row in together['declaration']['declarators']],
                         [row['declaration']['declarators'][0]['name'] for row in separate])

    def test_spectralnorm_actual_packed_and_expanded(self):
        self.packed_expanded(SPECTRAL_PACKED, SPECTRAL_EXPANDED, 'vector<double>')

    def test_mandelbrot_actual_packed_and_expanded(self):
        self.packed_expanded(MANDELBROT_PACKED, MANDELBROT_EXPANDED, 'std::complex<double>')

    def test_spectralnorm_actual_member_call(self):
        self.accepted('spectralnorm-member-call')

    def test_callee_agnostic_nested_calls(self):
        self.accepted('callee-agnostic-nested-calls')

    def test_member_index_operator_composition(self):
        self.accepted('member-index-and-operators')

    def test_pointer_and_reference_direct_heads(self):
        self.accepted('pointer-reference-direct-heads')

    def test_named_prototype_structural_formals(self):
        self.accepted('prototype-structural-formals')

    def test_object_and_prototype_count_invariance(self):
        self.accepted('prototype-object-ambiguity')

    def test_bare_name_count_invariance(self):
        self.accepted('unknown-bare-role-count-invariance')

    def test_proven_c_and_cpp_cast_composition(self):
        self.accepted('explicit-cpp-cast')

    def test_relational_and_logical_composition(self):
        self.accepted('comparisons-are-composition')

    def test_separate_named_declaration_rows(self):
        self.accepted('separate-declaration-rows')

    def test_argument_assignment_gate(self):
        self.gated('argument-assignment')

    def test_formal_default_gate(self):
        self.gated('formal-default')

    def test_compound_assignment_gate(self):
        self.gated('argument-compound-assignment')

    def test_alternative_assignment_gate(self):
        self.gated('argument-alternative-assignment')

    def test_argument_update_gate(self):
        self.gated('argument-update')

    def test_nested_call_update_gate(self):
        self.gated('nested-call-update')

    def test_grouped_comma_sequence_gate(self):
        self.gated('grouped-comma-sequence')

    def test_index_comma_sequence_gate(self):
        self.gated('index-comma-sequence')

    def test_ternary_comma_sequence_gate(self):
        self.gated('ternary-comma-sequence')

    def test_semicolon_sequence_gate(self):
        self.gated('semicolon-sequence')

    def test_executable_lambda_argument_gate(self):
        self.gated('lambda-callback')

    def test_braced_data_argument_gate(self):
        self.gated('braced-data-argument')

    def test_control_argument_gate(self):
        self.gated('control-in-argument')

    def test_unowned_new_operand_gate(self):
        self.gated('new-operand-role')

    def test_unknown_expression_leaf_gate(self):
        self.gated('unknown-expression-leaf')

    def test_unknown_multiword_formal_gate(self):
        self.gated('unknown-multiword-formal')

    def test_mixed_formal_object_list_gate(self):
        self.gated('mixed-object-formal-list')

    def test_formal_function_pointer_gate(self):
        self.gated('unsupported-formal-function-pointer')

    def test_named_function_formal_gate(self):
        self.gated('unsupported-named-function-formal')

    def test_reserved_declarator_name_gate(self):
        self.gated('reserved-declarator-name')

    def test_reserved_callee_gate(self):
        self.gated('reserved-callee')

    def test_malformed_hex_float_gate(self):
        self.gated('malformed-hex-float')

    def test_malformed_float_suffix_gate(self):
        self.gated('malformed-float-suffix')

    def test_possible_template_callee_comma_role_gate(self):
        self.gated('template-call-comma-role')

    def test_possible_template_left_shift_comma_role_gate(self):
        self.gated('template-call-shift-comma-role')

    def test_possible_template_right_shift_comma_role_gate(self):
        self.gated('template-call-right-shift-comma-role')

    def test_trailing_noexcept_gate(self):
        self.gated('trailing-noexcept')

    def test_trailing_array_gate(self):
        self.gated('trailing-array')

    def test_trailing_function_gate(self):
        self.gated('trailing-function')

    def test_multiline_scalar_argument_rows_gate(self):
        self.gated('multiline-scalar-arguments')

    def test_multiline_formal_rows_gate(self):
        self.gated('multiline-formals')

    def test_api_offsets_literal_ownership_legacy_additivity_and_rejected_proofs(self):
        checked = exercise_direct_declarator_fixtures(analyze_direct_decl, identify_direct_decl, _declarators)
        self.assertIn('exact-original-offsets', checked)
        self.assertIn('literal-mask-cannot-become-a-type', checked)
        self.assertIn('legacy-additive-counter-and-byte-preservation', checked)
        self.assertIn('rejected-owner-flags-remain-gates', checked)
        self.assertIn('delegated-rhs-argument-work-remains-gated', checked)
