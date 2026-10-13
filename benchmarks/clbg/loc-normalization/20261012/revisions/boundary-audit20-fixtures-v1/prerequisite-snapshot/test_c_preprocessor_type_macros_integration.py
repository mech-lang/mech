"""Unexecuted integration fixtures for the separate future macro proposal."""

import unittest

from c_type_environment import collect_typedef_aliases, identify_decl, _type_prefix
from lexical import mask_source


class CPreprocessorTypeMacroIntegrationFixtures(unittest.TestCase):
    def environment(self, source, *, cleaned=None, semantic_imports=(), literal_spans=()):
        if cleaned is None:
            masked = mask_source(source, 'cpp')
            cleaned = masked['cleaned']
            semantic_imports = masked.get('semantic_imports', ())
            literal_spans = masked.get('masked_literal_spans', ())
        self.assertEqual(len(cleaned), len(source))
        self.assertEqual(cleaned.count('\n'), source.count('\n'))
        environment = collect_typedef_aliases(cleaned, 'cpp',
            semantic_imports=semantic_imports, preprocessor_source=source,
            protected_literal_spans=literal_spans)
        return cleaned, environment

    def prefix(self, source, declaration):
        cleaned, environment = self.environment(source)
        at = source.index(declaration)
        return _type_prefix(cleaned[at:at + len(declaration)], environment, at), environment

    def test_nested_argument_macro_gate_reports_actual_use_token(self):
        declaration = 'std::vector<std::array<IUB,4>> a,b;'
        source = ('#include <vector>\n#include <array>\nstruct IUB{};\n'
                  '#define IUB int\n' + declaration)
        typed, environment = self.prefix(source, declaration)
        self.assertIsNone(typed)
        at = source.index('IUB,4')
        self.assertTrue(any(row.get('token_name') == 'IUB' and row.get('offset') == at
                            and row.get('role') == 'proven-standard-template-type-id'
                            for row in environment['diagnostics']))

    def test_header_contract_exposure_remains_after_unconditional_undef(self):
        declaration = 'std::vector<int> a,b;'
        source = '#define vector Other\n#include <vector>\n#undef vector\n' + declaration
        typed, environment = self.prefix(source, declaration)
        self.assertIsNone(typed)
        owner = environment['aliases']['std::vector'][0]
        self.assertTrue(owner['macro'])
        self.assertTrue(any(row.get('role') == 'imported-type-contract-macro-exposure'
                            and row.get('source_token') is False
                            for row in environment['diagnostics']))

    def test_self_macro_source_class_retains_bare_lookup_ambiguity(self):
        bare = 'vector<int> a,b;'
        qualified = 'std::vector<int> q,r;'
        source = ('#include <vector>\n#define vector vector\nstruct vector{};\n'
                  '#undef vector\nusing namespace std;\n' + bare + '\n' + qualified)
        cleaned, environment = self.environment(source)
        owner = next(row for row in environment['aliases']['vector']
                     if row['kind'] == 'source-local-cpp-named-type')
        self.assertTrue(owner['macro'])
        at = source.index(bare)
        self.assertIsNone(_type_prefix(cleaned[at:at + len(bare)], environment, at))
        at = source.index(qualified)
        typed = _type_prefix(cleaned[at:at + len(qualified)], environment, at)
        self.assertEqual(typed[1], 'std::vector<int>')
        self.assertEqual(typed[2]['units'], 0)
        self.assertTrue(environment['diagnostics'])

    def test_macro_affected_alias_blocks_inner_lookup_and_expires_with_scope(self):
        inside, outside = 'T inside,second;', 'T outside,other;'
        source = ('typedef int T;\nvoid f(){\n#define T T\ntypedef int T;\n'
                  '#undef T\n' + inside + '\n}\n' + outside)
        cleaned, environment = self.environment(source)
        owners = environment['aliases']['T']
        self.assertEqual(len(owners), 2)
        self.assertNotIn('macro', owners[0])
        self.assertTrue(owners[1]['macro'])
        at = source.index(inside)
        self.assertIsNone(identify_decl(cleaned[at:at + len(inside)], environment, offset=at))
        at = source.index(outside)
        row = identify_decl(cleaned[at:at + len(outside)], environment, offset=at)
        self.assertEqual([item['name'] for item in row['declarators']], ['outside', 'other'])

    def test_temporary_vector_macro_does_not_invalidate_written_namespace_import(self):
        declaration = 'vector<int> a,b;'
        source = ('#include <vector>\n#define vector Other\nusing namespace std;\n'
                  '#undef vector\n' + declaration)
        cleaned, environment = self.environment(source)
        imports = [row for row in environment['aliases']['vector']
                   if row['kind'] == 'explicit-standard-template-namespace-import']
        self.assertEqual(len(imports), 1)
        self.assertNotIn('macro', imports[0])
        at = source.index(declaration)
        row = identify_decl(cleaned[at:at + len(declaration)], environment, offset=at)
        self.assertEqual([item['name'] for item in row['declarators']], ['a', 'b'])

    def test_phase2_apparent_include_never_establishes_contract_after_undef(self):
        declaration = 'std::vector<int> a,b;'
        source = '// comment\\\n#include <vector>\n#undef vector\n' + declaration
        first = source.index('#include')
        header = source.index('<vector>')
        cleaned = ''.join(char if char.isspace() else ' ' for char in source[:first]) + source[first:]
        cleaned = cleaned[:header] + 'S' * 8 + cleaned[header + 8:]
        semantic = dict(start=header, end=header + 8, header='vector', spelling='<vector>')
        cleaned, environment = self.environment(source, cleaned=cleaned,
            semantic_imports=[semantic], literal_spans=[dict(start=header, end=header + 8)])
        self.assertTrue(environment['aliases']['std::vector'][0]['macro'])
        at = source.index(declaration)
        self.assertIsNone(_type_prefix(cleaned[at:at + len(declaration)], environment, at))
        self.assertTrue(any(row.get('kind') == 'c-type-preprocessor-directive-ownership-review'
                            for row in environment['diagnostics']))

    def test_clean_repeated_include_does_not_erase_earlier_contract_uncertainty(self):
        declaration = 'std::vector<int> a,b;'
        source = ('#define vector Other\n#include <vector>\n#undef vector\n'
                  '#include <vector>\n' + declaration)
        typed, environment = self.prefix(source, declaration)
        self.assertIsNone(typed)
        owners = environment['aliases']['std::vector']
        self.assertEqual(len(owners), 2)
        self.assertTrue(all(row['macro'] for row in owners))
        self.assertTrue(any(row.get('role') == 'prior-imported-type-contract-macro-exposure'
                            for row in environment['diagnostics']))

    def test_macro_affected_unresolved_alias_retains_lookup_block_before_add(self):
        declaration = 'Alias a,b;'
        qualified = 'std::vector<int> q,r;'
        source = ('#include <vector>\ntypedef int Alias;\nstruct IUB{};\n'
                  '#define IUB int\nusing Alias=std::vector<IUB>;\n'
                  '#undef IUB\n' + declaration + '\n' + qualified)
        cleaned, environment = self.environment(source)
        self.assertEqual(len(environment['aliases']['Alias']), 1)
        blocks = environment['type_macro_lookup_blocks']
        self.assertTrue(any(row['source_owner_kind'] == 'source-local-unresolved-using'
                            and row['definition_start'] == source.index('using Alias')
                            and row['accepted_type_owner'] is False for row in blocks))
        at = source.index(declaration)
        self.assertIsNone(_type_prefix(cleaned[at:at + len(declaration)], environment, at))
        at = source.index(qualified)
        typed = _type_prefix(cleaned[at:at + len(qualified)], environment, at)
        self.assertEqual(typed[1], 'std::vector<int>')
