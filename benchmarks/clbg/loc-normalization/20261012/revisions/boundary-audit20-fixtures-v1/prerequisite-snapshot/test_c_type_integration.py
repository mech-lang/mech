"""General typedef/import binding integration; guarded execution only."""
import unittest
from normalizer import normalize


class CTypeIntegrationFixtures(unittest.TestCase):
    def count(self, source, language, expected):
        row = normalize(source, language)
        self.assertEqual(row['normalized_loc'], expected, row)
        self.assertEqual(row['diagnostics'], [], row)
        return row

    def test_source_alias_independent_pointer_and_array_declarations(self):
        for source in ('typedef int elem; elem a[4],b[4];',
                       'typedef int elem;\nelem a[4];\nelem b[4];'):
            self.count(source, 'c', 3)
        self.count('typedef int node; node *a,*b,*c;', 'c', 4)

    def test_cpp_named_type_header_establishes_independent_fields_and_storage(self):
        self.count('struct Node{Node*l,*r;};','cpp',4)
        self.count('struct Mpz{int data;};struct Context{Mpz tmp1,tmp2;};','cpp',7)
        self.count('struct Node{};void f(){Node Node,*other;}','cpp',6)

    def test_cpp_named_tag_in_typedef_does_not_hide_field_body(self):
        self.count('typedef struct Node{Node*l,*r;}Alias;','cpp',4)
        row=normalize('typedef int X;typedef struct Box{int X;X*a,b;}Box;','cpp')
        self.assertTrue(any(d['kind']=='possible-packed-declaration-type-owner-review' for d in row['diagnostics']))

    def test_source_alias_chain_initializers_are_additive(self):
        self.count('typedef int elem; elem a=b=0,c;', 'c', 4)

    def test_primary_aggregate_alias_belongs_to_header(self):
        for source in ('typedef struct Node{int a,b;}Alias;',
                       'typedef\nstruct Node\n{\nint a;\nint b;\n}\nAlias;'):
            row = self.count(source, 'c', 4)
            self.assertEqual(sum(e['units'] for e in row['canonical_events']
                                 if e['kind']=='typedef-alias-suffix'), 0)

    def test_additional_attached_and_simple_aliases_are_independent(self):
        self.count('typedef struct{int a;}Alias,*AliasPtr;', 'c', 4)
        self.count('typedef int A,B; A a,b;', 'c', 4)

    def test_explicit_import_contracts_do_not_require_raw_filename_code(self):
        row = self.count('#include <gmp.h>\nmpz_t a,b,c;', 'c', 4)
        self.assertEqual([r['header'] for r in row['semantic_imports']], ['gmp.h'])
        self.count('#include <iostream>\nusing namespace std;\nstring a,b;', 'cpp', 4)

    def test_standard_integer_import_alias_has_independent_names(self):
        self.count('#include <stdint.h>\ntypedef intptr_t Native;\nNative *a,*b;','c',4)
        self.count('#include <inttypes.h>\nintmax_t a,b;','c',3)

    def test_standard_size_and_pcre2_groups_are_independent_not_header_arguments(self):
        self.count('#include <stdio.h>\nsize_t size,len;\nsize_t ilen,clen,slen;','c',6)
        self.count('#include <pcre2.h>\nPCRE2_SIZE capacity,size;\nPCRE2_SIZE offset,pos=0;','c',5)
        self.count('#include <stdlib.h>\nsize_t length=produce(1,2),capacity,end=0;','c',4)

    def test_unknown_type_use_before_alias_and_expression_ambiguity_stay_gated(self):
        for source in ('Node *a,*b;', 'X*a,b;',
                       'std::vector<int> a,b;', 'elem a,b; typedef int elem;'):
            row = normalize(source, 'cpp')
            self.assertTrue(any(d['kind']=='possible-packed-declaration-type-owner-review'
                                for d in row['diagnostics']), row)
            self.assertEqual(row['normalized_loc'], 2 if 'typedef' in source else 1, row)

    def test_argument_data_and_for_headers_never_become_independent_declarations(self):
        self.count('typedef int elem; consume(a,b);', 'c', 2)
        self.count('typedef int elem; for(elem i=0,j=0;i<2;i++){work();}', 'c', 4)

    def test_actual_aggregate_object_tail_is_never_hidden_as_alias(self):
        row = normalize('struct Node{int x;}a,b;', 'c')
        self.assertTrue(any(d['kind']=='c-aggregate-instance-suffix-review'
                            and d['required_independent_instance_units']==2
                            for d in row['diagnostics']), row)
        self.assertFalse(any(e['kind']=='typedef-alias-suffix' for e in row['canonical_events']), row)


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
