"""Unhooked typedef grammar fixtures; execution takes the timing mutex."""

import re
import unittest

from audit import timing_boundary
from c_type_environment import (collect_typedef_aliases, identify_decl,
                                classify_alias_suffixes, possible_packed_decl)


class CTypeEnvironmentFixtures(unittest.TestCase):
    def env(self, source, language='c', diagnostics=()):
        # This suite supplies explicit token-role metadata without lexing. The
        # production lexical metadata pathway needs its own guarded fixtures.
        chars = list(source)
        imports = []
        for match in re.finditer(r'^[ \t]*#[ \t]*include[ \t]*([<"]([^>"\n]+)[>"])', source, re.M):
            start, end = match.span(1)
            imports.append(dict(start=start, end=end, header=match[2]))
            chars[start:end] = ['S'] * (end - start)
        result = collect_typedef_aliases(''.join(chars), language,
            semantic_imports=imports, preprocessor_source=source)
        self.assertEqual([entry['kind'] for entry in result['diagnostics']], list(diagnostics), result)
        return result

    def declaration(self, source, declaration, count, uninitialized, language='c'):
        environment = self.env(source, language)
        offset = source.rindex(declaration)
        result = identify_decl(declaration, environment, offset=offset)
        self.assertIsNotNone(result, environment)
        self.assertEqual((result['count'], result['uninitialized']), (count, uninitialized), result)
        return result

    def test_integral_typedef_and_independent_arrays(self):
        self.declaration('typedef int elem; elem s[16], t[16];',
                         'elem s[16], t[16];', 2, 2)

    def test_modifiers_and_pointer_scalar_declaration(self):
        self.declaration('typedef int elem; void f(){register elem *x, *y, c;}',
                         'register elem *x, *y, c;', 3, 3)

    def test_unsigned_char_array_alias_and_initializer_data(self):
        result = self.declaration('typedef unsigned char int_t; int_t perm[16],tperm[16],cnt[16]={0};',
                                 'int_t perm[16],tperm[16],cnt[16]={0};', 3, 2, 'cpp')
        self.assertEqual([entry['name'] for entry in result['declarators']], ['perm', 'tperm', 'cnt'])

    def test_typedef_aggregate_alias_and_independent_pointers(self):
        for spelling in ('typedef struct tn {struct tn*left;struct tn*right;} treeNode;',
                         'typedef struct tn {\nstruct tn*left;\nstruct tn*right;\n} treeNode;'):
            source = spelling + '\n treeNode *stretchTree,*longLivedTree,*tempTree;'
            self.declaration(source, 'treeNode *stretchTree,*longLivedTree,*tempTree;', 3, 3)

    def test_same_tag_and_alias_name_is_proven_after_tail(self):
        source = 'typedef struct node {struct node*left,*right;} node; node *a,*b;'
        self.declaration(source, 'node *a,*b;', 2, 2)

    def test_anonymous_union_and_enum_aliases(self):
        for owner in ('typedef union {int a;double b;} value;',
                      'typedef enum {One,Two} value;'):
            self.declaration(owner + ' value x,y;', 'value x,y;', 2, 2)

    def test_aggregate_alias_tail_collection_has_no_loc_units(self):
        environment = self.env('typedef struct Tag {int a,b;} Alias,*AliasPtr;')
        self.assertEqual(set(environment['aliases']), {'Alias', 'AliasPtr'}, environment)
        self.assertTrue(all('units' not in entry for entry in environment['evidence']), environment)
        # Tail LOC ownership is a separate canonical policy decision.
        self.assertIsNone(identify_decl('Alias,*AliasPtr;', environment))

    def test_aggregate_primary_alias_suffix_structural_packed_and_expanded(self):
        for source in ('typedef struct Tag {int a,b;} Alias;',
                       'typedef struct Tag {\nint a,b;\n}\nAlias;'):
            environment = self.env(source)
            result = classify_alias_suffixes(source, 'c', environment)
            self.assertEqual(result['diagnostics'], [], result)
            self.assertEqual(len(result['spans']), 1, result)
            span = result['spans'][0]
            self.assertEqual((span['primary_alias'], span['additional_aliases'], span['units']),
                             ('Alias', [], 0), span)
            self.assertEqual(source[span['closing_brace']], '}')

    def test_aggregate_additional_aliases_are_independent_beyond_primary(self):
        for source in ('typedef struct Tag {int a;} Alias,*AliasPtr,AliasArray[2];',
                       'typedef struct Tag {\nint a;\n}\nAlias,\n*AliasPtr,\nAliasArray[2];'):
            result = classify_alias_suffixes(source, 'c', self.env(source))
            self.assertEqual(result['diagnostics'], [], result)
            span = result['spans'][0]
            self.assertEqual((span['primary_alias'], span['additional_aliases'], span['units']),
                             ('Alias', ['AliasPtr', 'AliasArray'], 2), span)

    def test_aggregate_instance_tail_is_never_hidden_as_typedef_alias(self):
        for source, count in (('struct Tag {int a;} instance;', 1),
                              ('struct Tag {int a;} instance,*other;', 2),
                              ('struct Tag {int a;} instance={0};', 1)):
            result = classify_alias_suffixes(source, 'c', self.env(source))
            self.assertEqual(result['spans'], [], result)
            self.assertEqual(result['diagnostics'][0]['kind'], 'c-aggregate-instance-suffix-review')
            self.assertEqual(result['diagnostics'][0]['required_independent_instance_units'], count)

    def test_aggregate_bare_semicolon_has_no_alias_or_instance_binding(self):
        source = 'struct Tag {int a,b;};'
        result = classify_alias_suffixes(source, 'c', self.env(source))
        self.assertEqual((result['spans'], result['diagnostics']), ([], []), result)

    def test_previously_proven_pointer_aliases(self):
        self.declaration('typedef unsigned int ui;typedef ui *ptr;ptr a,b;', 'ptr a,b;', 2, 2)

    def test_cpp_using_alias_requires_a_proven_rhs(self):
        self.declaration('typedef int elem;using ptr=elem*;ptr a,b;', 'ptr a,b;', 2, 2, 'cpp')
        environment = self.env('using ptr=foreign_type;ptr a,b;', 'cpp',
                               ('c-typedef-declarator-owner-review',))
        self.assertIsNone(identify_decl('ptr a,b;', environment, offset=23))

    def test_c_using_does_not_define_cpp_alias(self):
        environment = self.env('using ptr=int;ptr a,b;', 'c')
        self.assertNotIn('ptr', environment['aliases'])

    def test_builtin_double_pointer_and_named_pointer_groups(self):
        environment = self.env('')
        for declaration in ('char **a, **b;', 'int (*a),(*b);', 'int (*a)[4],(*b)[4];'):
            result = identify_decl(declaration, environment)
            self.assertEqual((result['count'], result['uninitialized']), (2, 2), result)

    def test_chained_initializers_keep_uninitialized_additive(self):
        self.declaration('typedef int elem;elem a=b=0,c;', 'elem a=b=0,c;', 2, 1)
        self.declaration('typedef int elem;elem a=produce(1,2),c;', 'elem a=produce(1,2),c;', 2, 1)

    def test_unknown_case_spelling_does_not_prove_type(self):
        environment = self.env('')
        for declaration in ('node *a,*b;', 'Node *a,*b;', 'thing x,y;'):
            self.assertIsNone(identify_decl(declaration, environment))

    def test_uppercase_operand_multiplication_comma_is_not_a_type(self):
        source = 'int X=2;void f(){X *a,b;}'
        environment = self.env(source)
        self.assertIsNone(identify_decl('X *a,b;', environment, offset=source.index('X *a')))
        self.declaration('typedef int X;X *a,b;', 'X *a,b;', 2, 2)

    def test_uppercase_possible_declaration_gate_never_adds_units(self):
        proposal = possible_packed_decl('X*a,b;')
        self.assertEqual(proposal['type_spelling'], 'X')
        self.assertEqual(proposal['possible_names'], ['a', 'b'])
        self.assertIsNone(proposal['units'])

    def test_qualified_template_type_ownership_stays_gated(self):
        source = 'std::vector<int> a,b;'
        environment = self.env(source, 'cpp')
        self.assertIsNone(identify_decl(source, environment, offset=0))
        proposal = possible_packed_decl(source)
        self.assertEqual((proposal['type_spelling'], proposal['possible_names']),
                         ('std::vector<int>', ['a', 'b']))

    def test_nested_qualified_template_member_gate(self):
        for spelling in ('std::map<int,std::vector<int>>',
                         'std::vector<int>::iterator', 'typename T::value_type'):
            proposal = possible_packed_decl(spelling + ' a,b;')
            self.assertIsNotNone(proposal, spelling)
            self.assertEqual(proposal['possible_names'], ['a', 'b'])

    def test_possible_constructor_initializers_and_data_commas_are_protected(self):
        for source in ('Foreign a(1,2),b(3,4);', 'Foreign a{1,2},b{3,4};',
                       'Foreign *a=build<A,B>(1,2),b;', 'std::vector<int> const a,b;'):
            proposal = possible_packed_decl(source)
            self.assertEqual(proposal['possible_names'], ['a', 'b'], source)
            self.assertTrue(proposal['review_required'])

    def test_possible_pointer_array_declarators_stay_unproven(self):
        proposal = possible_packed_decl('Unknown *a[3],**b,c[2];')
        self.assertEqual(proposal['possible_names'], ['a', 'b', 'c'])
        self.assertIsNone(proposal['units'])

    def test_value_shadow_starts_after_its_declared_name_not_before_its_type(self):
        source='typedef int T;void f(){T T=1,other=2;T*a,b;}'
        env=self.env(source)
        declaration='T T=1,other=2;'
        self.assertEqual(identify_decl(declaration,env,offset=source.index(declaration))['count'],2)
        self.assertIsNone(identify_decl('T*a,b;',env,offset=source.index('T*a')))

    def test_typedef_aggregate_body_does_not_hide_real_value_field_shadows(self):
        source='typedef int X;typedef struct Box{int X;X*a,b;}Box;'
        env=self.env(source)
        self.assertIsNone(identify_decl('X*a,b;',env,offset=source.index('X*a')))
        self.assertTrue(any(shadow['name']=='X' for shadow in env['shadows']))

    def test_possible_gate_does_not_promote_control_call_or_data_prefixes(self):
        for source in ('return a*b,c;', 'for(X*a,b;flag;step())', 'run(a,b);',
                       'a,b;', 'table={a,b};', 'Unknown a=build(1,2);'):
            self.assertIsNone(possible_packed_decl(source), source)

    def test_comma_expressions_and_calls_are_not_declarations(self):
        environment = self.env('typedef int elem;')
        for expression in ('a*b,c;', 'a=*x,*x++=*y,*y--=a;', 'elem(a,b);',
                           'consume(elem *x,*y);', '++permcount,next_permutation(perm,n,cnt);'):
            self.assertIsNone(identify_decl(expression, environment), expression)

    def test_control_header_wrappers_are_not_declarations(self):
        environment = self.env('typedef int elem;')
        for header in ('for(elem *a=nodes,*b=nodes;flag;step())',
                       'while(elem(a,b))', 'f(elem *a,elem *b)'):
            self.assertIsNone(identify_decl(header, environment), header)
        # The caller must also reject the inner formal/for/data context before
        # calling identify_decl; an isolated typed spelling is a declaration.

    def test_use_before_typedef_is_not_proven(self):
        source = 'elem *a,*b;typedef int elem;elem *c,*d;'
        environment = self.env(source)
        self.assertIsNone(identify_decl('elem *a,*b;', environment, offset=0))
        self.assertEqual(identify_decl('elem *c,*d;', environment,
                                      offset=source.index('elem *c'))['count'], 2)

    def test_block_alias_is_not_visible_outside(self):
        source = 'void f(){typedef int elem;elem a,b;} elem c,d;'
        environment = self.env(source)
        self.assertEqual(identify_decl('elem a,b;', environment,
                                      offset=source.index('elem a'))['count'], 2)
        self.assertIsNone(identify_decl('elem c,d;', environment,
                                       offset=source.index('elem c')))
        self.assertIsNone(identify_decl('elem a,b;', environment))

    def test_ordinary_local_identifier_shadows_type_alias(self):
        source = 'typedef int elem;void f(){int elem;elem *a,*b;}elem c,d;'
        environment = self.env(source)
        self.assertIsNone(identify_decl('elem *a,*b;', environment,
                                       offset=source.index('elem *a')))
        self.assertEqual(identify_decl('elem c,d;', environment,
                                      offset=source.index('elem c'))['count'], 2)
        self.assertIsNone(identify_decl('elem c,d;', environment))

    def test_braced_initializer_does_not_hide_identifier_shadow(self):
        for initializer in ('int elem={0};', 'int elem[2]={0,1};'):
            source = 'typedef int elem;void f(){' + initializer + 'elem *a,*b;}'
            environment = self.env(source)
            self.assertIsNone(identify_decl('elem *a,*b;', environment,
                                           offset=source.index('elem *a')))

    def test_braced_for_initializer_identifier_shadows_alias_in_loop(self):
        source = 'typedef int elem;void f(){for(int elem=0;elem<3;elem++){elem *a,*b;}elem c,d;}'
        environment = self.env(source)
        self.assertIsNone(identify_decl('elem *a,*b;', environment,
                                       offset=source.index('elem *a')))
        self.assertEqual(identify_decl('elem c,d;', environment,
                                      offset=source.index('elem c'))['count'], 2)

    def test_function_parameter_identifier_shadows_type_alias(self):
        source = 'typedef int elem;void f(int elem){elem *a,*b;}elem c,d;'
        environment = self.env(source)
        self.assertIsNone(identify_decl('elem *a,*b;', environment,
                                       offset=source.index('elem *a')))
        self.assertEqual(identify_decl('elem c,d;', environment,
                                      offset=source.index('elem c'))['count'], 2)

    def test_nested_alias_uses_its_own_scope_and_order(self):
        source = 'typedef int elem;void f(){typedef double elem;elem a,b;}elem c,d;'
        environment = self.env(source)
        inner = identify_decl('elem a,b;', environment, offset=source.index('elem a'))
        outer = identify_decl('elem c,d;', environment, offset=source.index('elem c'))
        self.assertEqual(inner['type_evidence']['type_name'], 'double')
        self.assertEqual(outer['type_evidence']['type_name'], 'int')

    def test_conditional_typedef_stays_gated(self):
        source = '#ifdef CONFIG\ntypedef int elem;\n#endif\nelem a,b;'
        environment = self.env(source, diagnostics=('c-type-conditional-owner-review',))
        self.assertIsNone(identify_decl('elem a,b;', environment, offset=source.index('elem a')))
        self.assertIsNone(identify_decl('elem a,b;', environment))

    def test_conditional_redefinition_blocks_prior_alias(self):
        source = 'typedef int elem;\n#ifdef CONFIG\ntypedef double elem;\n#endif\nelem a,b;'
        environment = self.env(source, diagnostics=('c-type-conditional-owner-review',))
        self.assertIsNone(identify_decl('elem a,b;', environment, offset=source.index('elem a')))

    def test_macro_typedef_does_not_create_alias(self):
        for source in ('#define TYPE typedef int elem;\nelem a,b;',
                       '#define TYPE \\\n typedef int elem;\nelem a,b;'):
            environment = self.env(source)
            self.assertNotIn('elem', environment['aliases'])

    def test_macro_braces_do_not_create_lexical_typedef_scopes(self):
        source = '#define OPEN {\n#define CLOSE }\ntypedef int elem;elem a,b;'
        environment = self.env(source)
        self.assertIsNone(environment['aliases']['elem'][0]['scope_opening'])
        self.assertEqual(identify_decl('elem a,b;', environment,
                                      offset=source.index('elem a'))['count'], 2)

    def test_function_pointer_typedef_is_gated(self):
        environment = self.env('typedef int (*Fn)(int);Fn a,b;', diagnostics=
                               ('c-typedef-declarator-owner-review',))
        self.assertNotIn('Fn', environment['aliases'])
        self.assertIsNone(identify_decl('int (*a)(int),(*b)(int);', environment))

    def test_explicit_gmp_import_contract(self):
        self.declaration('#include <gmp.h>\nmpz_t n1,n2,d,u,v,w;', 'mpz_t n1,n2,d,u,v,w;', 6, 6)
        self.declaration('#include <gmpxx.h>\nstatic mpz_class tmp0,tmp1;',
                         'static mpz_class tmp0,tmp1;', 2, 2, 'cpp')

    def test_gnu_wide_integer_builtin_and_signed_unsigned_forms(self):
        for language in ('c', 'cpp'):
            for spelling in ('__int128', 'unsigned __int128', 'signed __int128'):
                self.declaration(spelling + ' a,b;', spelling + ' a,b;', 2, 2, language)
        environment = self.env('')
        self.assertIsNone(identify_decl('__int128_extra a,b;', environment))

    def test_explicit_sse_header_owns_float_vector_type(self):
        for language in ('c', 'cpp'):
            self.declaration('#include <xmmintrin.h>\n__m128 a,b;', '__m128 a,b;', 2, 2, language)

    def test_explicit_sse2_header_owns_sse_and_sse2_vector_types(self):
        for language in ('c', 'cpp'):
            for spelling in ('__m128', '__m128d', '__m128i'):
                self.declaration('#include <emmintrin.h>\n' + spelling + ' a,b;',
                                 spelling + ' a,b;', 2, 2, language)

    def test_explicit_umbrella_header_owns_proven_vector_families(self):
        for language in ('c', 'cpp'):
            for spelling in ('__m128', '__m128d', '__m128i', '__m256', '__m256d',
                             '__m256i', '__m512', '__m512d', '__m512i'):
                self.declaration('#include <immintrin.h>\n' + spelling + ' a,b;',
                                 spelling + ' a,b;', 2, 2, language)

    def test_intrinsic_vector_spelling_requires_explicit_header_proof(self):
        for spelling in ('__m128', '__m128d', '__m256d', '__m512i', '__m999i'):
            source = spelling + ' a,b;'
            environment = self.env(source)
            self.assertIsNone(identify_decl(source, environment, offset=0), spelling)
            proposal = possible_packed_decl(source)
            self.assertTrue(proposal['review_required'], spelling)
            self.assertIsNone(proposal['units'], spelling)

    def test_sse_header_does_not_guess_sse2_or_wider_vector_types(self):
        for header, spelling in (('xmmintrin.h', '__m128d'), ('xmmintrin.h', '__m256d'),
                                 ('emmintrin.h', '__m512i'), ('immintrin.h', '__m999i')):
            source = '#include <' + header + '>\n' + spelling + ' a,b;'
            environment = self.env(source)
            self.assertIsNone(identify_decl(spelling + ' a,b;', environment,
                                           offset=source.index(spelling)), (header, spelling))

    def test_standard_string_qualified_and_namespace_import(self):
        self.declaration('#include <string>\nstd::string a,b;', 'std::string a,b;', 2, 2, 'cpp')
        self.declaration('#include <iostream>\nusing namespace std;\nstring str,out;',
                         'string str,out;', 2, 2, 'cpp')
        environment = self.env('#include <string>\nstring a,b;', 'cpp')
        self.assertIsNone(identify_decl('string a,b;', environment, offset=18))

    def test_explicit_standard_integer_headers_own_pointer_max_least_fast_types(self):
        for header in ('stdint.h','inttypes.h'):
            for spelling in ('intptr_t','uintptr_t','intmax_t','uintmax_t',
                             'int_least64_t','uint_least32_t','int_fast16_t','uint_fast8_t'):
                source='#include <'+header+'>\n'+spelling+' a,b;'
                self.declaration(source,spelling+' a,b;',2,2)

    def test_standard_integer_typedef_requires_explicit_header_not_spelling(self):
        source='#include <stdint.h>\ntypedef intptr_t Native;Native *a,*b;'
        self.declaration(source,'Native *a,*b;',2,2)
        for unknown in ('intptr_t a,b;','#include <stdio.h>\nintptr_t a,b;',
                        '#include <foreign.h>\nintmax_t a,b;'):
            env=self.env(unknown)
            declaration=unknown.rsplit('\n',1)[-1]
            self.assertIsNone(identify_decl(declaration,env,offset=unknown.index(declaration)))

    def test_explicit_c_standard_headers_own_size_type(self):
        for header in ('stdio.h','stdlib.h','string.h','stddef.h'):
            source='#include <'+header+'>\nsize_t capacity,size;'
            self.declaration(source,'size_t capacity,size;',2,2)
        env=self.env('size_t capacity,size;')
        self.assertIsNone(identify_decl('size_t capacity,size;',env,offset=0))

    def test_pcre2_explicit_size_alias_contract_is_not_prefix_guessing(self):
        source='#include <pcre2.h>\nPCRE2_SIZE erroroffset,pos=0;'
        self.declaration(source,'PCRE2_SIZE erroroffset,pos=0;',2,1)
        self.declaration('#include <pcre2.h>\nsize_t capacity,size;','size_t capacity,size;',2,2)
        for code in ('PCRE2_SIZE a,b;','PCRE2_FAKE a,b;'):
            prefix='' if code.startswith('PCRE2_SIZE') else '#include <pcre2.h>\n'
            env=self.env(prefix+code)
            self.assertIsNone(identify_decl(code,env,offset=len(prefix)))

    def test_standard_namespace_alias_is_visible_to_later_typedef(self):
        self.declaration('#include <iostream>\nusing namespace std;\ntypedef string Text;Text a,b;',
                         'Text a,b;', 2, 2, 'cpp')

    def test_import_contract_requires_explicit_unchanged_header_name(self):
        for source in ('mpz_t a,b;', '#include SSSSSSS\nmpz_t a,b;',
                       '#include <foreign.h>\nmpz_t a,b;'):
            environment = self.env(source)
            self.assertIsNone(identify_decl('mpz_t a,b;', environment,
                                           offset=source.index('mpz_t')))

    def test_conditional_header_contract_stays_gated(self):
        source = '#ifdef CONFIG\n#include <gmp.h>\n#endif\nmpz_t a,b;'
        environment = self.env(source, diagnostics=('c-type-conditional-owner-review',))
        self.assertIsNone(identify_decl('mpz_t a,b;', environment, offset=source.index('mpz_t')))

    def test_import_metadata_cannot_claim_an_unowned_filename(self):
        source = 'consume(SSSSSSSS);mpz_t a,b;'
        result = collect_typedef_aliases(source, 'c', semantic_imports=[
            dict(start=8, end=16, header='gmp.h')])
        self.assertEqual(result['diagnostics'][0]['kind'], 'c-type-import-span-owner-review')
        self.assertNotIn('mpz_t', result['aliases'])

    def test_import_metadata_requires_translation_unit_directive(self):
        source = 'void f(){\n#include SSSSSSS\nmpz_t a,b;}'
        start = source.index('SSSSSSS')
        result = collect_typedef_aliases(source, 'c', semantic_imports=[
            dict(start=start, end=start + 7, header='gmp.h')])
        self.assertEqual(result['diagnostics'][0]['kind'], 'c-type-import-scope-owner-review')
        self.assertNotIn('mpz_t', result['aliases'])

    def test_template_alias_and_direct_initializers_stay_gated(self):
        environment = self.env('using Vec=std::vector<int>;', 'cpp',
                               ('c-typedef-declarator-owner-review',))
        self.assertIsNone(identify_decl('int a(value),b;', environment))
        self.assertIsNone(identify_decl('int a=call<Type,Other>(),b;', environment))


if __name__ == '__main__':
    with timing_boundary():
        unittest.main()
