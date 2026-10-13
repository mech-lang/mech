"""Unexecuted standard-template proposal fixtures for a future guarded suite.

These fixtures supply masks/semantic records directly, never lex benchmark
sources. Import/execution is reserved for a parent-reviewed immutable snapshot
under the common timing mutex; writing this file does not run its fixtures.
"""

import unittest

from c_std_templates import collect_std_template_contracts, prove_std_template_prefix
from c_type_environment import _resolve


class CStdTemplateProposalFixtures(unittest.TestCase):
    def fixture(self, body, headers=('vector',), *, ranges=None, conditional=(),
                aliases=None, shadows=(), language='cpp'):
        source = ''
        imports = []
        for header in headers:
            prefix = '#include '
            spelling = '<' + header + '>'
            start = len(source) + len(prefix)
            imports.append(dict(start=start, end=start + len(spelling),
                                header=header, spelling=spelling,
                                kind='explicit-c-family-include-literal'))
            source += prefix + 'S' * len(spelling) + '\n'
        source += body
        environment = dict(language=language, aliases=aliases or {},
                           shadows=list(shadows), source_length=len(source))
        original_aliases = {name: [dict(row) for row in rows]
                            for name, rows in environment['aliases'].items()}
        ranges = ranges or [(0, len(source), None)]
        result = collect_std_template_contracts(source, language, environment,
            resolver=_resolve, semantic_imports=imports, scope_ranges=ranges,
            conditional_regions=conditional)
        self.assertEqual(environment['aliases'], original_aliases)
        proposed = dict(environment, aliases={name: list(rows) for name, rows in
                                             environment['aliases'].items()})
        for entry in result['entries']:
            proposed['aliases'].setdefault(entry['name'], []).append(entry)
        return source, proposed, result

    def prove(self, source, environment, text, *, resolver=_resolve):
        return prove_std_template_prefix(text, environment, source.index(text),
                                         resolver=resolver)

    def test_exact_vector_contract_and_structural_default(self):
        text = 'std::vector<double> a,b;'
        source, environment, collected = self.fixture(text)
        result = self.prove(source, environment, text)
        self.assertEqual(result[:2], (len('std::vector<double>'), 'std::vector<double>'))
        self.assertEqual(result[2]['omitted_defaults'], ('allocator<T>',))
        self.assertEqual(result[2]['units'], 0)
        self.assertEqual(len(collected['entries']), 1)
        self.assertEqual(collected['diagnostics'], [])
        self.assertEqual(result[2]['contract']['primary_source'],
                         'https://eel.is/c++draft/vector.syn')

    def test_complex_actual_packed_constructor_prefix(self):
        text = 'std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);'
        source, environment, _ = self.fixture(text, ('complex',))
        result = self.prove(source, environment, text)
        self.assertEqual(result[1], 'std::complex<double>')
        self.assertEqual(text[result[0]:], ' Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);')
        self.assertEqual(len(result[2]['arguments']), 1)
        self.assertEqual(result[2]['units'], 0)

    def test_actual_fasta_source_type_argument_and_reference(self):
        text = 'std::vector<IUB>& i;'
        body = 'struct IUB{}; ' + text
        scope_end = len('#include SSSSSSSS\n' + body)
        owner = dict(name='IUB', kind='source-local-cpp-named-type',
                     visible_start=18 + len('struct IUB'), scope_start=0,
                     scope_end=scope_end, scope_opening=None, conditional=False)
        source, environment, _ = self.fixture(body, aliases={'IUB': [owner]})
        seen = []

        def resolver(env, name, offset):
            seen.append((name, offset))
            return _resolve(env, name, offset)

        result = self.prove(source, environment, text, resolver=resolver)
        self.assertEqual(text[result[0]:], '& i;')
        self.assertIn(('IUB', source.index('IUB>', source.index(text))), seen)
        self.assertEqual(result[2]['arguments'][0]['owner']['name'], 'IUB')

    def test_outer_adjacent_pointer_and_cv_are_left_for_declarators(self):
        for text, remaining in (('std::vector<int>*a,*b;', '*a,*b;'),
                                ('const std::vector<int> volatile&a,b;', '&a,b;')):
            source, environment, _ = self.fixture(text)
            result = self.prove(source, environment, text)
            self.assertEqual(text[result[0]:], remaining)

    def test_explicit_using_namespace_std_owns_spectralnorm_prefix(self):
        text = 'vector<double> u(N), v(N);'
        source, environment, collected = self.fixture('using namespace std;\n' + text)
        result = self.prove(source, environment, text)
        self.assertEqual(result[1], 'vector<double>')
        self.assertEqual(result[2]['owner']['kind'],
                         'explicit-standard-template-namespace-import')
        self.assertEqual(len(collected['entries']), 2)

    def test_unqualified_name_requires_prior_exact_namespace_owner(self):
        for body in ('vector<int> a,b;',
                     'vector<int> a,b; using namespace std;'):
            source, environment, _ = self.fixture(body)
            self.assertIsNone(self.prove(source, environment, 'vector<int> a,b;'))

    def test_namespace_import_before_include_does_not_own_later_added_names(self):
        source = 'using namespace std;\n#include SSSSSSSS\nvector<int> a,b;'
        start = source.index('SSSSSSSS')
        environment = dict(language='cpp', aliases={}, shadows=[], source_length=len(source))
        result = collect_std_template_contracts(source, 'cpp', environment,
            resolver=_resolve, semantic_imports=[dict(start=start, end=start + 8,
                header='vector', spelling='<vector>')],
            scope_ranges=[(0, len(source), None)])
        self.assertEqual([entry['name'] for entry in result['entries']], ['std::vector'])

    def test_scoped_namespace_import_expires_at_block_close(self):
        body = 'void f(){using namespace std; vector<int> a,b;} vector<int> c,d;'
        full = '#include SSSSSSSS\n' + body
        opening, closing = full.index('{'), full.index('}') + 1
        source, environment, _ = self.fixture(body,
            ranges=[(0, len(full), None), (opening, closing, opening)])
        self.assertIsNotNone(self.prove(source, environment, 'vector<int> a,b;'))
        self.assertIsNone(self.prove(source, environment, 'vector<int> c,d;'))

    def test_value_shadow_and_scope_lifetime_gate_unqualified_owner(self):
        body = 'using namespace std; void f(){int vector; vector<int> a,b;} vector<int> c,d;'
        full = '#include SSSSSSSS\n' + body
        opening, closing = full.index('{'), full.index('}') + 1
        shadow = dict(name='vector', start=full.index('vector;') + len('vector'),
                      end=closing, scope_start=opening)
        source, environment, _ = self.fixture(body, shadows=[shadow],
            ranges=[(0, len(full), None), (opening, closing, opening)])
        self.assertIsNone(self.prove(source, environment, 'vector<int> a,b;'))
        self.assertIsNotNone(self.prove(source, environment, 'vector<int> c,d;'))

    def test_qualified_owner_rechecked_after_namespace_import(self):
        body = 'using namespace std; vector<int> a,b;'
        full = '#include SSSSSSSS\n' + body
        shadow = dict(name='std::vector', start=full.index('vector<int>'),
                      end=len(full), scope_start=0)
        source, environment, _ = self.fixture(body, shadows=[shadow])
        self.assertIsNone(self.prove(source, environment, 'vector<int> a,b;'))

    def test_nested_templates_and_argument_cv_pointers_are_structural(self):
        text = 'std::vector<std::array<const int*, 4>> a,b;'
        source, environment, _ = self.fixture(text, ('vector', 'array'))
        result = self.prove(source, environment, text)
        inner = result[2]['arguments'][0]
        self.assertEqual(inner['contract']['name'], 'std::array')
        self.assertEqual(inner['arguments'][1]['kind'], 'decimal-size-value')
        self.assertEqual(inner['arguments'][0]['type_id'], 'const int*')
        self.assertEqual(inner['units'], 0)

    def test_wrapping_is_preserved_in_exact_spelling_and_offsets(self):
        text = 'std::vector<\n std::complex<double>\n> a,b;'
        source, environment, _ = self.fixture(text, ('vector', 'complex'))
        result = self.prove(source, environment, text)
        self.assertEqual(result[1], text[:text.index(' a,b;')])
        self.assertEqual(source[result[2]['start']:result[2]['end']], result[1])
        self.assertEqual(source.count('\n'), 4)

    def test_exact_arity_and_parameter_roles(self):
        cases = ('std::vector<> a,b;', 'std::vector<int,> a,b;',
                 'std::vector<int,int,int> a,b;', 'std::complex<int,int> a,b;',
                 'std::array<int> a,b;', 'std::array<int,int> a,b;',
                 'std::vector<4> a,b;', 'std::array<int,4,5> a,b;')
        for text in cases:
            source, environment, _ = self.fixture(text, ('vector', 'complex', 'array'))
            self.assertIsNone(self.prove(source, environment, text), text)

    def test_size_names_formals_macros_and_expressions_remain_gated(self):
        for size in ('SIZE', 'max_n', 'sizeof(int)', '1+2', '(4)', '4<5', '4>>1', '4u', '08'):
            text = 'std::array<int,' + size + '> a,b;'
            source, environment, _ = self.fixture(text, ('array',))
            self.assertIsNone(self.prove(source, environment, text), text)

    def test_unknown_source_alias_and_dependent_template_formals_remain_gated(self):
        for text in ('std::vector<Unknown> a,b;', 'std::array<NUMBER_TYPE,SIZE> a,b;',
                     'std::vector<typename T::value_type> a,b;',
                     'std::vector<T...> a,b;', 'std::vector<std::allocator<int>> a,b;'):
            source, environment, _ = self.fixture(text, ('vector', 'array'))
            self.assertIsNone(self.prove(source, environment, text), text)

    def test_resolver_cannot_supply_unresolved_or_dependent_source_type(self):
        text = 'std::vector<T> a,b;'
        for flags in ({'kind': 'unresolved-source-alias'},
                      {'kind': 'source-local-using', 'dependent': True},
                      {'kind': 'source-local-cpp-named-type', 'template_formal': True}):
            owner = dict(name='T', visible_start=0, scope_start=0,
                         scope_end=1000, scope_opening=None, **flags)
            source, environment, _ = self.fixture(text, aliases={'T': [owner]})
            self.assertIsNone(self.prove(source, environment, text))

    def test_source_type_argument_order_shadow_and_scope_lifetime(self):
        text = 'std::vector<IUB> a,b;'
        owner = dict(name='IUB', kind='source-local-cpp-named-type',
                     visible_start=0, scope_start=0, scope_end=1000,
                     scope_opening=None)
        source, environment, _ = self.fixture(text, aliases={'IUB': [owner]})
        argument_at = source.index('IUB>')
        environment['shadows'].append(dict(name='IUB', start=argument_at,
                                          end=len(source), scope_start=0))
        self.assertIsNone(self.prove(source, environment, text))
        environment['shadows'].clear()
        owner['visible_start'] = argument_at + 1
        self.assertIsNone(self.prove(source, environment, text))
        owner['visible_start'] = 0
        owner['scope_end'] = argument_at
        self.assertIsNone(self.prove(source, environment, text))

    def test_unknown_templates_members_relations_and_malformed_groups_remain_gated(self):
        for text in ('std::map<int,int> a,b;', 'std::vector<int>::iterator a,b;',
                     'A < B > c,d;', 'std::vector<int <= 2> a,b;',
                     'std::vector<int>> a,b;', 'std::vector<(int)> a,b;',
                     'std::vector<int[3]> a,b;', 'std::vector<int&*> a,b;',
                     'std::vector<int&&&> a,b;', 'std::vector<std::complex<int> a,b;'):
            source, environment, _ = self.fixture(text, ('vector', 'complex'))
            self.assertIsNone(self.prove(source, environment, text), text)

    def test_missing_transitive_or_quoted_header_never_proves_owner(self):
        text = 'std::vector<int> a,b;'
        for headers in ((), ('iostream',)):
            source, environment, _ = self.fixture(text, headers)
            self.assertIsNone(self.prove(source, environment, text))
        source = '#include SSSSSSSS\n' + text
        start = source.index('SSSSSSSS')
        environment = dict(language='cpp', aliases={}, source_length=len(source))
        result = collect_std_template_contracts(source, 'cpp', environment,
            resolver=_resolve, semantic_imports=[dict(start=start, end=start + 8,
                header='vector', spelling='"vector"')], scope_ranges=[(0, len(source), None)])
        self.assertEqual(result['entries'], [])
        self.assertEqual(result['diagnostics'][0]['kind'], 'cpp-template-import-span-owner-review')

    def test_metadata_cannot_claim_expression_or_macro_as_include_filename(self):
        for source in ('consume(SSSSSSSS);', '#include VECTOR\n'):
            start = source.index('SSSSSSSS') if 'SSSSSSSS' in source else source.index('VECTOR')
            environment = dict(language='cpp', aliases={}, source_length=len(source))
            result = collect_std_template_contracts(source, 'cpp', environment,
                resolver=_resolve, semantic_imports=[dict(start=start, end=start + 8,
                    header='vector', spelling='<vector>')],
                scope_ranges=[(0, len(source), None)])
            self.assertEqual(result['entries'], [])

    def test_conditional_header_and_unknown_scope_proof_remain_gated(self):
        text = 'std::vector<int> a,b;'
        source, environment, collected = self.fixture(text, conditional=[(0, 18)])
        self.assertIsNone(self.prove(source, environment, text))
        self.assertEqual(collected['diagnostics'][0]['kind'], 'cpp-template-conditional-owner-review')
        result = collect_std_template_contracts(source, 'cpp', environment,
            resolver=_resolve, semantic_imports=[dict(start=9, end=17, header='vector',
                spelling='<vector>')], scope_ranges=[])
        self.assertEqual(result['entries'], [])
        self.assertEqual(result['diagnostics'][0]['kind'], 'cpp-template-import-scope-owner-review')

    def test_unbalanced_source_environment_remains_gated(self):
        text = 'std::vector<int> a,b;'
        source, environment, _ = self.fixture(text)
        environment['diagnostics'] = [dict(kind='c-type-environment-unclosed-scope-review', offset=0)]
        self.assertIsNone(self.prove(source, environment, text))
        result = collect_std_template_contracts(source, 'cpp', environment,
            resolver=_resolve, semantic_imports=[dict(start=9, end=17, header='vector',
                spelling='<vector>')], scope_ranges=[(0, len(source), None)])
        self.assertEqual(result['entries'], [])
        self.assertEqual(result['diagnostics'][0]['kind'], 'cpp-template-unbalanced-source-owner-review')

    def test_header_scope_and_language_require_cpp_translation_unit(self):
        source = 'void f(){\n#include SSSSSSSS\n}'
        start = source.index('SSSSSSSS')
        opening = source.index('{')
        environment = dict(language='cpp', aliases={}, source_length=len(source))
        result = collect_std_template_contracts(source, 'cpp', environment,
            resolver=_resolve, semantic_imports=[dict(start=start, end=start + 8,
                header='vector', spelling='<vector>')],
            scope_ranges=[(0, len(source), None), (opening, len(source), opening)])
        self.assertEqual(result['entries'], [])
        source, environment, collected = self.fixture('std::vector<int> a,b;', language='c')
        self.assertEqual(collected['entries'], [])
        self.assertIsNone(self.prove(source, environment, 'std::vector<int> a,b;'))

    def test_missing_exact_offset_and_recursion_limit_remain_gated(self):
        text = 'std::vector<int> a,b;'
        source, environment, _ = self.fixture(text)
        for offset in (None, -1, True, len(source)):
            self.assertIsNone(prove_std_template_prefix(text, environment, offset, resolver=_resolve))
        nested = 'std::vector<' * 18 + 'int' + '>' * 18 + ' a,b;'
        source, environment, _ = self.fixture(nested)
        self.assertIsNone(self.prove(source, environment, nested))
