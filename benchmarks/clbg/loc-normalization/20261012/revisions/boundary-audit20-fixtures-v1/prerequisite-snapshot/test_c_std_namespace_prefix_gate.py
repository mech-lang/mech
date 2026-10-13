"""Unexecuted first-stage std-prefix fixtures for a future reviewed job."""

import unittest

from c_std_namespace_prefix_gate import collect_std_namespace_prefix_gates
from c_type_environment import collect_typedef_aliases, _type_prefix
from lexical import mask_source


class CStdNamespacePrefixGateFixtures(unittest.TestCase):
    def collect(self, source, **kwargs):
        return collect_std_namespace_prefix_gates(source, 'cpp', **kwargs)

    def test_only_qualified_std_tokens_do_not_create_potential_owner_gate(self):
        source = 'std::vector<std::complex<double>> a,b;std\n::array<int,4> c;'
        row = self.collect(source)
        self.assertFalse(row['blocked'])
        self.assertEqual(row['diagnostics'], [])
        self.assertFalse(row['per_use_scope_proof'])

    def test_collector_proven_exact_namespace_import_is_exempt(self):
        source = 'using namespace std;\nstd::vector<int> a,b;'
        end = source.index(';') + 1
        row = self.collect(source,
            owned_namespace_import_spans=[dict(definition_start=0, definition_end=end)])
        self.assertFalse(row['blocked'])

    def test_unproven_standard_namespace_import_is_retained(self):
        source = 'using namespace std;\nstd::vector<int> a,b;'
        row = self.collect(source)
        self.assertTrue(row['blocked'])
        self.assertTrue(any(item['role'] == 'unproved-standard-namespace-import'
                            for item in row['diagnostics']))

    def test_namespace_type_and_class_prefix_owners_have_exact_token_offsets(self):
        for source in ('namespace std=Custom;std::vector<int>*a,*b;',
                       'using std=Custom;std::vector<int>*a,*b;',
                       'struct std{};std::vector<int>*a,*b;'):
            row = self.collect(source)
            owners = [item for item in row['diagnostics']
                      if item['role'] == 'potential-source-std-prefix-owner']
            self.assertEqual([item['offset'] for item in owners], [source.index('std')])
            self.assertTrue(row['blocked'])

    def test_unknown_other_namespace_import_blocks_whole_proposal(self):
        source = 'using namespace Custom;std::vector<int> a,b;'
        row = self.collect(source)
        self.assertTrue(row['blocked'])
        self.assertEqual(row['diagnostics'][0]['offset'], 0)
        self.assertEqual(row['diagnostics'][0]['role'], 'unproved-other-namespace-import')

    def test_checked_comment_and_protected_literal_tokens_are_excluded(self):
        source = '/* std */ "std";std::vector<int> a,b;'
        first = source.index('"')
        cleaned = ' ' * len('/* std */') + source[len('/* std */'):]
        row = self.collect(cleaned,
            protected_literal_spans=[dict(start=first, end=first + len('"std"'))])
        self.assertFalse(row['blocked'])

    def test_conditional_owner_retains_source_wide_gate_without_branch_choice(self):
        source = 'std::vector<int> before;\n#if FEATURE\nnamespace std=Custom;\n#endif\nstd::vector<int> after;'
        row = self.collect(source)
        self.assertTrue(row['blocked'])
        self.assertTrue(all(item['source_wide_gate'] for item in row['diagnostics']))
        self.assertEqual(row['diagnostics'][0]['offset'], source.index('std=Custom'))

    def test_value_formal_and_function_names_remain_conservative_without_hide_claim(self):
        for source in ('int std=0;std::vector<int> a,b;',
                       'void f(int std){std::vector<int> a,b;}',
                       'void std();std::vector<int> a,b;'):
            row = self.collect(source)
            self.assertTrue(row['blocked'])
            self.assertTrue(any(item['role'] == 'potential-source-std-prefix-owner'
                                for item in row['diagnostics']))

    def test_invalid_import_or_literal_span_never_provides_exemption(self):
        source = 'using namespace std;std::vector<int> a,b;'
        for kwargs in (dict(owned_namespace_import_spans=[dict(definition_start=1, definition_end=20)]),
                       dict(protected_literal_spans=[dict(start=-1, end=3)])):
            row = self.collect(source, **kwargs)
            self.assertTrue(row['blocked'])
            self.assertTrue(any(item['kind'] == 'cpp-standard-namespace-prefix-provenance-review'
                                for item in row['diagnostics']))

    def test_exact_directive_metadata_protects_macro_tokens_as_directive_roles(self):
        source = '#define std Foreign\n#undef std\nstd::vector<int> a,b;'
        middle = source.index('#undef')
        end = source.index('std::')
        spans = [dict(start=start, end=stop, extent_proven=True,
                      directive_ownership_proven=True)
                 for start, stop in ((0, middle), (middle, end))]
        row = self.collect(source, protected_directive_spans=spans)
        self.assertFalse(row['blocked'])
        spans[0]['extent_proven'] = False
        self.assertTrue(self.collect(source, protected_directive_spans=spans)['blocked'])

    def test_gate_preserves_all_source_bytes_and_returns_no_units(self):
        source = '\nnamespace std = Custom;\nstd::vector<int> a,b;\n'
        before = source
        row = self.collect(source)
        self.assertEqual(source, before)
        self.assertEqual(row['source_length'], len(source))
        self.assertTrue(all(item['units'] is None for item in row['diagnostics']))


class CStdNamespacePrefixIntegrationFixtures(unittest.TestCase):
    def environment(self, source):
        masked = mask_source(source, 'cpp')
        return masked['cleaned'], collect_typedef_aliases(masked['cleaned'], 'cpp',
            semantic_imports=masked['semantic_imports'], preprocessor_source=source,
            protected_literal_spans=masked.get('masked_literal_spans', ()))

    def test_namespace_alias_value_template_cannot_promote_packed_expression(self):
        expression = 'std::vector<int>*a,*b;'
        source = ('#include <vector>\nnamespace Custom{template<class> inline int vector=0;}\n'
                  'void f(){namespace std=Custom;int a,b;' + expression + '}')
        cleaned, environment = self.environment(source)
        at = source.index(expression)
        self.assertIsNone(_type_prefix(cleaned[at:at + len(expression)], environment, at))
        self.assertTrue(environment['aliases']['std::vector'][0]['namespace_prefix_review'])
        self.assertTrue(any(row.get('kind') == 'cpp-standard-namespace-prefix-owner-review'
                            and row.get('offset') == source.index('std=Custom')
                            for row in environment['diagnostics']))

    def test_unknown_namespace_import_blocks_qualified_and_bare_contracts(self):
        qualified, bare = 'std::vector<int> a,b;', 'vector<int> q,r;'
        source = ('#include <vector>\nnamespace Custom{}\nusing namespace Custom;\n'
                  'using namespace std;\n' + qualified + '\n' + bare)
        cleaned, environment = self.environment(source)
        for declaration in (qualified, bare):
            at = source.index(declaration)
            self.assertIsNone(_type_prefix(cleaned[at:at + len(declaration)], environment, at))
        self.assertTrue(all(row['namespace_prefix_review']
                            for name in ('std::vector', 'vector')
                            for row in environment['aliases'][name]))

    def test_owned_namespace_directive_is_independent_of_bare_member_conflict(self):
        qualified, bare = 'std::vector<int> q,r;', 'vector<int> a,b;'
        source = ('#include <vector>\nstruct vector{};\nusing namespace std;\n'
                  + qualified + '\n' + bare)
        cleaned, environment = self.environment(source)
        directives = environment['std_template_namespace_directive_evidence']
        self.assertEqual(len(directives), 1)
        self.assertEqual(directives[0]['definition_start'], source.index('using namespace std'))
        self.assertNotIn('namespace_prefix_review', environment['aliases']['std::vector'][0])
        at = source.index(qualified)
        typed = _type_prefix(cleaned[at:at + len(qualified)], environment, at)
        self.assertEqual(typed[1], 'std::vector<int>')
        at = source.index(bare)
        self.assertIsNone(_type_prefix(cleaned[at:at + len(bare)], environment, at))
