"""Proposed integration fixtures; run only in a future reviewed mutex job."""
import unittest

from c_type_environment import (collect_typedef_aliases, identify_decl,
                                possible_packed_decl, _type_prefix)
from lexical import mask_source
from normalizer import normalize, _scope_map, _structural


class CStdTemplateIntegrationFixtures(unittest.TestCase):
    def environment(self, source):
        masked = mask_source(source, 'cpp')
        self.assertEqual(len(masked['cleaned']), len(source))
        self.assertEqual(masked['cleaned'].count('\n'), source.count('\n'))
        environment = collect_typedef_aliases(masked['cleaned'], 'cpp',
            semantic_imports=masked['semantic_imports'], preprocessor_source=source,
            protected_literal_spans=masked.get('masked_literal_spans', ()))
        return masked['cleaned'], environment

    def prefix(self, source, declaration):
        cleaned, environment = self.environment(source)
        at = source.index(declaration)
        result = _type_prefix(cleaned[at:at + len(declaration)], environment, at)
        return result, environment

    def test_actual_vector_and_complex_prefixes_keep_constructor_suffix_gate(self):
        cases = (
            ('#include <vector>\nusing namespace std;\n',
             'vector<double> u(N), v(N);', 'vector<double>', ['u', 'v']),
            ('#include <complex>\n',
             'std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);',
             'std::complex<double>', ['Z', 'C']),
        )
        for prefix, declaration, spelling, names in cases:
            source = prefix + declaration
            typed, environment = self.prefix(source, declaration)
            self.assertEqual(typed[1], spelling)
            self.assertEqual(typed[2]['units'], 0)
            self.assertEqual(possible_packed_decl(declaration)['possible_names'], names)
            # This template proposal deliberately does not invent ownership of
            # constructor declarator suffixes. A separate reviewed seam must
            # replace this gate with packed/expanded normalization parity.
            self.assertIsNone(identify_decl(declaration, environment,
                                           offset=source.index(declaration)))

    def test_plain_independent_names_and_nested_arguments_have_separate_owners(self):
        packed = '#include <vector>\n#include <complex>\nstd::vector<std::complex<double>> u,v;'
        expanded = '#include <vector>\n#include <complex>\nstd::vector<std::complex<double>> u;\nstd::vector<std::complex<double>> v;'
        cleaned, environment = self.environment(packed)
        declaration = 'std::vector<std::complex<double>> u,v;'
        row = identify_decl(declaration, environment, offset=packed.index(declaration))
        self.assertEqual([item['name'] for item in row['declarators']], ['u', 'v'])
        self.assertEqual(row['type_evidence']['units'], 0)
        self.assertEqual(row['type_evidence']['arguments'][0]['units'], 0)
        one, other = normalize(packed, 'cpp'), normalize(expanded, 'cpp')
        self.assertEqual(one['diagnostics'], [])
        self.assertEqual(other['diagnostics'], [])
        self.assertEqual(one['normalized_loc'], other['normalized_loc'])

    def test_fasta_source_argument_direct_member_and_constructor_initialization(self):
        source = ('#include <vector>\nstruct IUB{char c;float p;};'
                  'struct Random{Random(std::vector<IUB>& i):i(i){}'
                  'std::vector<IUB>& i;};')
        row = normalize(source, 'cpp')
        self.assertEqual(row['diagnostics'], [], row)
        owners = [entry for entry in row['cpp_initializer_evidence']
                  if entry['constructor_name'] == 'Random']
        self.assertEqual(len(owners), 1)
        self.assertEqual([entry['name'] for entry in owners[0]['members']], ['i'])
        field = next(entry for entry in owners[0]['member_declarations'] if entry['name'] == 'i')
        self.assertEqual(field['type_evidence']['kind'], 'proven-standard-template-specialization')
        self.assertEqual(field['type_evidence']['arguments'][0]['owner']['name'], 'IUB')

    def test_alias_rhs_proof_offsets_are_real_using_and_typedef_token_offsets(self):
        source = ('#include <vector>\nstruct IUB{};'
                  'using Vec = std::vector<IUB>;'
                  'typedef std::vector<IUB> Other;Vec a,b;')
        cleaned, environment = self.environment(source)
        for alias, marker in (('Vec', 'using Vec'), ('Other', 'typedef')):
            entry = environment['aliases'][alias][0]
            expected = source.index('std::vector<IUB>', source.index(marker))
            self.assertEqual(entry['source_type_probe_start'], expected)
            argument = entry['source_type_evidence']['arguments'][0]
            self.assertEqual(argument['start'], source.index('IUB>', expected))
            self.assertEqual(cleaned[argument['start']:argument['end']], 'IUB')

    def test_profiles_prototypes_arguments_and_for_header_stay_structural(self):
        source = ('#include <vector>\nusing namespace std;\n'
                  'void prototype(vector<double>& u,vector<double>& v);'
                  'void f(vector<double>& u,vector<double>& v){consume(u,v);'
                  'for(vector<int>*i=0,*j=0;again();step()){consume(i,j);}}')
        cleaned, environment = self.environment(source)
        contexts, _ = _scope_map(cleaned, 'cpp')
        for marker in ('& u', '& v', '*i=0', '*j=0'):
            at = source.index(marker) + 1
            self.assertTrue(_structural(contexts[at], language='cpp'))
        row = normalize(source, 'cpp')
        self.assertEqual(row['diagnostics'], [], row)

    def test_nested_array_formal_type_commas_are_protected_by_proven_prefix(self):
        source = ('#include <array>\nstruct IUB{};'
                  'void f(std::array<IUB,4>& values){consume(values);}')
        _, environment = self.environment(source)
        self.assertFalse(any('parameter-shadow' in row['kind']
                             for row in environment['diagnostics']), environment)
        typed, _ = self.prefix(source, 'std::array<IUB,4>& values')
        self.assertEqual(typed[2]['arguments'][1]['kind'], 'decimal-size-value')

    def test_namespace_order_and_block_lifetime_use_real_environment(self):
        source = ('#include <vector>\nvector<int> before,other;'
                  'void f(){using namespace std;vector<int> inside,second;}'
                  'vector<int> after,third;')
        cleaned, environment = self.environment(source)
        for declaration, owned in (('vector<int> before,other;', False),
                                   ('vector<int> inside,second;', True),
                                   ('vector<int> after,third;', False)):
            result = _type_prefix(declaration, environment, source.index(declaration))
            self.assertEqual(result is not None, owned)

    def test_auto_function_formal_and_source_class_shadows_remain_real(self):
        prefixes = (
            'using namespace std;void f(){auto vector=0;vector<int> a,b;}',
            'using namespace std;int vector();vector<int> a,b;',
            'using namespace std;void f(int vector){vector<int> a,b;}',
            'struct vector{};using namespace std;vector<int> a,b;',
            'using namespace std;struct vector{};vector<int> a,b;',
        )
        for body in prefixes:
            source = '#include <vector>\n' + body
            typed, environment = self.prefix(source, 'vector<int> a,b;')
            self.assertIsNone(typed, environment)
            self.assertTrue(any(entry.get('name') == 'vector'
                                for entry in environment['shadows'])
                            or any(entry['kind'] == 'source-local-cpp-named-type'
                                   for entry in environment['aliases'].get('vector', ())))
        source = '#include <vector>\nusing namespace std;void f(){auto vector=0;std::vector<int> a,b;}'
        self.assertIsNotNone(self.prefix(source, 'std::vector<int> a,b;')[0])

    def test_source_argument_shadow_blocks_specialization(self):
        source = '#include <vector>\nstruct IUB{};void f(){int IUB;std::vector<IUB> a,b;}'
        typed, environment = self.prefix(source, 'std::vector<IUB> a,b;')
        self.assertIsNone(typed)
        self.assertTrue(any(entry['name'] == 'IUB' for entry in environment['shadows']))

    def test_indented_scalar_alias_formal_shadow_starts_at_its_written_name(self):
        source='#include <vector>\ntypedef int T;void f(                    int T){std::vector<T> a,b;}T*outer;'
        typed,environment=self.prefix(source,'std::vector<T> a,b;')
        self.assertIsNone(typed,environment)
        formal=next(entry for entry in environment['shadows']
                    if entry['name']=='T' and entry['kind']=='function-parameter-shadow')
        self.assertEqual(formal['start'],source.index('int T)')+len('int T'))
        self.assertIsNotNone(_type_prefix('T*outer;',environment,source.index('T*outer;')))

    def test_final_shadows_gate_template_alias_and_alias_descendants(self):
        source = ('#include <vector>\nusing namespace std;'
                  'void f(){auto vector=0;using Bad=vector<int>;'
                  'using Further=Bad;Further a,b;}')
        _, environment = self.environment(source)
        self.assertNotIn('Bad', environment['aliases'])
        self.assertNotIn('Further', environment['aliases'])
        for marker in ('using Bad', 'using Further'):
            self.assertTrue(any(row['kind'] == 'c-typedef-declarator-owner-review'
                                and row['offset'] == source.index(marker)
                                for row in environment['diagnostics']), environment)

    def test_scalar_alias_shadow_cannot_prove_a_template_type_argument(self):
        source = ('#include <vector>\ntypedef int T;'
                  'void f(){int T;using Bad=T;std::vector<Bad> a,b;}')
        cleaned, environment = self.environment(source)
        self.assertNotIn('Bad', environment['aliases'])
        declaration = 'std::vector<Bad> a,b;'
        self.assertIsNone(_type_prefix(declaration, environment, source.index(declaration)))
        self.assertTrue(any(row['kind'] == 'c-typedef-declarator-owner-review'
                            and row['offset'] == source.index('using Bad')
                            for row in environment['diagnostics']))

    def test_completed_alias_survives_later_shadow_of_original_type_spelling(self):
        source = ('#include <vector>\nusing namespace std;using Vec=vector<int>;'
                  'void f(){auto vector=0;Vec a,b;}')
        cleaned, environment = self.environment(source)
        declaration = 'Vec a,b;'
        self.assertIsNotNone(identify_decl(declaration, environment,
                                          offset=source.index(declaration)))

    def test_unknown_dependent_named_array_size_and_conditional_headers_stay_gated(self):
        cases = (
            ('#include <vector>\n', 'std::vector<Unknown> a,b;'),
            ('#include <array>\n', 'std::array<int,SIZE> a,b;'),
            ('#include <array>\n', 'std::array<NUMBER_TYPE,SIZE> a,b;'),
            ('#include <vector>\n', 'std::vector<typename T::value_type> a,b;'),
            ('#if FEATURE\n#include <vector>\n#endif\n', 'std::vector<int> a,b;'),
            ('#include "vector"\n', 'std::vector<int> a,b;'),
        )
        for prefix, declaration in cases:
            typed, environment = self.prefix(prefix + declaration, declaration)
            self.assertIsNone(typed, environment)
        source = ('#include <array>\ntemplate<typename NUMBER_TYPE>'
                  'class VectorizedNumber{constexpr static int SIZE=8;'
                  'using NumericArray=std::array<NUMBER_TYPE,SIZE>;};')
        _, environment = self.environment(source)
        self.assertNotIn('NumericArray', environment['aliases'])
        self.assertTrue(environment['diagnostics'])

    def test_literal_data_wrapping_keeps_existing_data_ownership(self):
        data = '{\n  1,2,\n  3,4\n};'
        vector = '#include <vector>\nstd::vector<int> values=' + data
        control = '#include <vector>\nint values[]=' + data
        one, other = normalize(vector, 'cpp'), normalize(control, 'cpp')
        self.assertEqual(one['diagnostics'], [], one)
        self.assertEqual(other['diagnostics'], [], other)
        self.assertEqual(one['normalized_loc'], other['normalized_loc'])
        masked, environment = self.environment(vector)
        self.assertEqual(masked[vector.index(data):], data)


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
