"""Unhooked named C++ type owner fixtures; execution takes timing mutex."""

import unittest

from audit import timing_boundary
from c_named_types import collect_named_type_owners, collect_named_type_shadows
from c_type_environment import (_scope_ranges, _conditional_regions, _type_prefix,
                                _collect_shadows, identify_decl)


class NamedCppTypeFixtures(unittest.TestCase):
    def environment(self, source, language='cpp'):
        ranges, diagnostics = _scope_ranges(source)
        self.assertEqual(diagnostics, [], diagnostics)
        conditional = _conditional_regions(source)
        owners = collect_named_type_owners(source, language, ranges, conditional)
        environment = dict(language=language, aliases={}, shadows=[], alias_spans=[], diagnostics=[])
        for entry in owners['entries']:
            environment['aliases'].setdefault(entry['name'], []).append(entry)
        _collect_shadows(source, environment, ranges)
        shadows = collect_named_type_shadows(source, environment, ranges, _type_prefix,
                                             conditional_regions=conditional)
        environment['shadows'].extend(shadows['shadows'])
        environment['shadows'].extend(shadows['blocked_owners'])
        return environment, owners, shadows

    def declaration(self, source, declaration, count=2):
        environment, owners, shadows = self.environment(source)
        self.assertEqual(owners['diagnostics'], [], owners)
        self.assertEqual(shadows['diagnostics'], [], shadows)
        self.assertEqual(environment['diagnostics'], [], environment)
        row = identify_decl(declaration, environment, offset=source.index(declaration))
        self.assertIsNotNone(row, environment)
        self.assertEqual(row['count'], count, row)
        return environment, owners, shadows

    def test_named_struct_type_visible_inside_its_own_body(self):
        for source in ('struct Node {Node *l,*r;};',
                       'struct Node\n{\nNode *l,*r;\n};'):
            self.declaration(source, 'Node *l,*r;')

    def test_lowercase_and_uppercase_named_types_have_equal_proof_rules(self):
        for name in ('node', 'Node', 'NODE'):
            self.declaration('struct ' + name + '{' + name + '*l,*r;};',
                             name + '*l,*r;')

    def test_named_class_and_union_types_visible_in_later_scopes(self):
        for keyword in ('class', 'union'):
            self.declaration(keyword + ' Value {int a;};void f(){Value x,y;}', 'Value x,y;')

    def test_pidigits_source_owned_field_type(self):
        self.declaration('struct Mpz {int data;};struct Context {Mpz tmp1,tmp2;};',
                         'Mpz tmp1,tmp2;')

    def test_header_span_never_covers_or_hides_fields(self):
        source = 'struct Node{Node *l,*r;int x,y;};'
        _, owners, _ = self.declaration(source, 'Node *l,*r;')
        entry = owners['entries'][0]
        self.assertEqual(entry['source_text'], 'struct Node{')
        self.assertLessEqual(entry['definition_end'], source.index('Node *l'))
        self.assertTrue(all('units' not in item for item in owners['evidence']))
        self.assertEqual(owners['header_spans'][0]['end'], source.index('{') + 1)

    def test_forward_declaration_proves_later_pointer_types(self):
        self.declaration('class Node;void f(){Node *l,*r;}', 'Node *l,*r;')

    def test_forward_then_definition_preserve_type_order(self):
        source = 'struct Node;Node *a,*b;struct Node{Node *l,*r;};'
        environment, _, _ = self.declaration(source, 'Node *a,*b;')
        self.assertEqual(identify_decl('Node *l,*r;', environment,
                                      offset=source.index('Node *l'))['count'], 2)

    def test_type_is_not_visible_before_named_header(self):
        source = 'Node *a,*b;struct Node{int x;};Node *c,*d;'
        environment, _, _ = self.environment(source)
        self.assertIsNone(identify_decl('Node *a,*b;', environment, offset=0))
        self.assertEqual(identify_decl('Node *c,*d;', environment,
                                      offset=source.index('Node *c'))['count'], 2)

    def test_block_local_named_type_does_not_escape(self):
        source = 'void f(){struct Node{int x;};Node a,b;}Node c,d;'
        environment, _, _ = self.declaration(source, 'Node a,b;')
        self.assertIsNone(identify_decl('Node c,d;', environment,
                                       offset=source.index('Node c')))

    def test_namespace_local_bare_type_does_not_escape(self):
        source = 'namespace N{struct Node{int x;};Node a,b;}Node c,d;'
        environment, _, _ = self.declaration(source, 'Node a,b;')
        self.assertIsNone(identify_decl('Node c,d;', environment,
                                       offset=source.index('Node c')))
        self.assertIsNone(identify_decl('N::Node a,b;', environment, offset=len(source)))

    def test_c_named_tag_never_becomes_a_bare_typedef(self):
        source = 'struct Node{struct Node*l,*r;};Node *a,*b;'
        environment, owners, _ = self.environment(source, 'c')
        self.assertEqual(owners['entries'], [])
        self.assertIsNone(identify_decl('Node *a,*b;', environment,
                                       offset=source.index('Node *a')))
        self.assertEqual(identify_decl('struct Node*l,*r;', environment,
                                      offset=source.index('struct Node*l'))['count'], 2)

    def test_explicit_cxx_tag_in_typedef_has_its_own_type_owner(self):
        self.declaration('typedef struct Node{Node*l,*r;} Alias;', 'Node*l,*r;')

    def test_enum_class_is_not_a_class_header(self):
        environment, owners, _ = self.environment('enum class Mode{One,Two};Mode a,b;')
        self.assertEqual(owners['entries'], [])
        self.assertNotIn('Mode', environment['aliases'])

    def test_template_and_inherited_headers_remain_gated(self):
        for source in ('template<class T>struct Node{Node*l,*r;};',
                       'struct Node:Base{Node*l,*r;};',
                       'struct Outer::Node{int x;};'):
            environment, owners, _ = self.environment(source)
            self.assertEqual(owners['entries'], [], owners)
            self.assertTrue(owners['diagnostics'], owners)
            self.assertNotIn('Node', environment['aliases'])

    def test_ordinary_class_name_operand_is_never_a_type_by_capitalization(self):
        source = 'int X=2;void f(){X*a,b;}'
        environment, owners, _ = self.environment(source)
        self.assertEqual(owners['entries'], [])
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))

    def test_ordinary_object_value_hides_class_name(self):
        source = 'struct X{};void f(){int X=2;X*a,b;}X*c,*d;'
        environment, _, _ = self.environment(source)
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))
        self.assertEqual(identify_decl('X*c,*d;', environment,
                                      offset=source.index('X*c'))['count'], 2)

    def test_typed_ordinary_function_name_hides_class_name(self):
        for function in ('int X(int);', 'int X(int){return 1;}'):
            source = 'struct X{};' + function + 'void f(){X*a,b;}'
            environment, _, shadows = self.environment(source)
            self.assertEqual(shadows['diagnostics'], [], shadows)
            self.assertTrue(any(row['kind'] == 'cpp-named-type-ordinary-call-declarator-shadow'
                                for row in shadows['shadows']), shadows)
            self.assertIsNone(identify_decl('X*a,b;', environment,
                                           offset=source.index('X*a')))

    def test_constructor_destructor_and_conversion_operator_never_hide_type(self):
        for member in ('Node(){}~Node(){}', 'explicit Node(int){}', 'operator Node(){}'):
            self.declaration('struct Node{' + member + 'Node*l,*r;};', 'Node*l,*r;')

    def test_qualified_constructor_destructor_names_never_hide_type(self):
        self.declaration('struct Node{Node();~Node();};Node::Node(){}Node::~Node(){}Node*l,*r;',
                         'Node*l,*r;')

    def test_explicit_new_operand_does_not_declare_a_same_name_function(self):
        for expression in ('return new Node(1,2);',
                           'auto p=new Node(make(1,2));',
                           'consume(new Node(1,2));',
                           'return flag?new Node(1):new Node(2);'):
            source='struct Node{};void f(){'+expression+'Node*l,*r;}'
            environment,_,shadows=self.environment(source)
            self.assertEqual(shadows['diagnostics'],[],shadows)
            self.assertFalse(any(row['name']=='Node' for row in shadows['blocked_owners']),shadows)
            self.assertEqual(identify_decl('Node*l,*r;',environment,
                offset=source.index('Node*l'))['count'],2)

    def test_direct_access_prefix_does_not_become_a_constructor_return_type(self):
        for access in ('public','private','protected'):
            source='class Node{int x;'+access+':Node():x(0){}Node*l,*r;};'
            environment,_,shadows=self.environment(source)
            self.assertEqual(shadows['diagnostics'],[],shadows)
            self.assertEqual(identify_decl('Node*l,*r;',environment,
                offset=source.index('Node*l'))['count'],2)

    def test_return_binding_and_owned_call_arguments_do_not_declare_type_name(self):
        for expression in ('return Node(1);','auto p=Node(1);',
                           'consume(1,Node(2),3);','qualified::consume(Node(2));'):
            source='struct Node{};void f(){'+expression+'Node*l,*r;}'
            environment,_,shadows=self.environment(source)
            self.assertEqual(shadows['diagnostics'],[],shadows)
            self.assertEqual(identify_decl('Node*l,*r;',environment,
                offset=source.index('Node*l'))['count'],2)

    def test_unknown_function_profiles_and_real_names_remain_shadowed(self):
        source='struct Node{};Foreign Node(int);void g(){Node*l,*r;}'
        environment,_,shadows=self.environment(source)
        self.assertTrue(shadows['diagnostics'],shadows)
        self.assertIsNone(identify_decl('Node*l,*r;',environment,
            offset=source.index('Node*l')))
        source='struct Node{};void f(Foreign Node(int)){Node*a,b;}Node*l,*r;'
        environment,_,shadows=self.environment(source)
        self.assertTrue(shadows['diagnostics'],shadows)
        self.assertIsNone(identify_decl('Node*a,b;',environment,
            offset=source.index('Node*a')))
        self.assertEqual(identify_decl('Node*l,*r;',environment,
            offset=source.index('Node*l'))['count'],2)
        # A prototype has no body in which its parameter name can hide the
        # global source class. Do not invent a global value declaration.
        source='struct Node{};void f(Foreign Node(int));Node*l,*r;'
        environment,_,shadows=self.environment(source)
        self.assertEqual(shadows['diagnostics'],[],shadows)
        self.assertEqual(identify_decl('Node*l,*r;',environment,
            offset=source.index('Node*l'))['count'],2)

    def test_unproved_aggregate_constructor_profile_is_not_promoted_to_call_args(self):
        source='struct Node{};template<class T>struct Wrapper{Wrapper(Foreign Node(int)){Node*x;} };Node*l,*r;'
        environment,_,shadows=self.environment(source)
        self.assertTrue(any(row['name']=='Node' for row in shadows['blocked_owners']),shadows)
        self.assertIsNone(identify_decl('Node*x;',environment,offset=source.index('Node*x')))
        self.assertEqual(identify_decl('Node*l,*r;',environment,
            offset=source.index('Node*l'))['count'],2)
        source='struct Node{};template<class T>void f(){consume(Node(1));Node*l,*r;}'
        environment,_,shadows=self.environment(source)
        self.assertEqual(shadows['diagnostics'],[],shadows)
        self.assertEqual(identify_decl('Node*l,*r;',environment,
            offset=source.index('Node*l'))['count'],2)

    def test_typed_parameters_use_type_without_hiding_it(self):
        for formal in ('Node*left', 'const Node&left', 'Node*const left'):
            self.declaration('struct Node{};void f(' + formal + '){Node*l,*r;}', 'Node*l,*r;')

    def test_formal_value_name_hides_class_name_in_function_body_only(self):
        source = 'struct Node{};void f(int Node){Node*a,b;}Node*l,*r;'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'], [], shadows)
        self.assertIsNone(identify_decl('Node*a,b;', environment,
                                       offset=source.index('Node*a')))
        self.assertEqual(identify_decl('Node*l,*r;', environment,
                                      offset=source.index('Node*l'))['count'], 2)

    def test_lambda_and_call_operator_formal_values_hide_class_name(self):
        for function in ('auto f=[](int Node){Node*a,b;};',
                         'struct F{int operator()(int Node){Node*a,b;}};'):
            source = 'struct Node{};' + function + 'Node*l,*r;'
            environment, _, shadows = self.environment(source)
            self.assertEqual(shadows['diagnostics'], [], shadows)
            self.assertIsNone(identify_decl('Node*a,b;', environment,
                                           offset=source.index('Node*a')))
            self.assertEqual(identify_decl('Node*l,*r;', environment,
                                          offset=source.index('Node*l'))['count'], 2)

    def test_auto_value_name_hides_class_name(self):
        for declaration in ('auto X=1;', 'auto& X=value;', 'auto X(1);'):
            source = 'struct X{};void f(){' + declaration + 'X*a,b;}X*c,*d;'
            environment, _, shadows = self.environment(source)
            self.assertEqual(shadows['diagnostics'], [], shadows)
            self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))
            self.assertEqual(identify_decl('X*c,*d;', environment,
                                          offset=source.index('X*c'))['count'], 2)

    def test_unscoped_enumerator_hides_class_name_but_scoped_member_does_not(self):
        for enumeration in ('enum{X=1};', 'enum Value:unsigned{X=1};'):
            source = 'struct X{};' + enumeration + 'void f(){X*a,b;}'
            environment, _, shadows = self.environment(source)
            self.assertEqual(shadows['diagnostics'], [], shadows)
            self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))
        self.declaration('struct X{};enum class Value{X=1};X*a,*b;', 'X*a,*b;')

    def test_pointer_value_declarator_hides_class_name(self):
        source = 'struct X{};int(*X)(int);void f(){X*a,b;}'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'], [], shadows)
        self.assertTrue(any(row['kind'] == 'cpp-named-type-pointer-value-shadow'
                            for row in shadows['shadows']), shadows)
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))

    def test_uncertain_same_name_function_owner_stays_blocked(self):
        source = 'struct X{};Foreign X(int);void f(){X*a,b;}'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'][0]['kind'], 'cpp-named-type-function-shadow-owner-review')
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))

    def test_uncertain_parameter_owner_stays_blocked(self):
        source = 'struct X{};void f(Foreign X){X*a,b;}'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'][0]['kind'], 'cpp-named-type-parameter-shadow-owner-review')
        self.assertEqual([row['kind'] for row in environment['diagnostics']],
                         ['c-type-parameter-shadow-owner-review'], environment)
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))

    def test_constructor_initializer_parameter_lifetime_stays_gated(self):
        source = 'struct Node{int data;Node(int Node):data(0){Node*a,b;}Node*l,*r;};'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'][0]['kind'], 'cpp-named-type-parameter-lifetime-owner-review')
        self.assertIsNone(identify_decl('Node*a,b;', environment,
                                       offset=source.index('Node*a')))

    def test_conditional_named_header_and_value_shadow_stay_gated(self):
        source = '#ifdef OPTION\nstruct X{};\n#endif\nX*a,*b;'
        environment, owners, _ = self.environment(source)
        self.assertEqual(owners['diagnostics'][0]['kind'], 'cpp-named-type-conditional-owner-review')
        self.assertIsNone(identify_decl('X*a,*b;', environment, offset=source.index('X*a')))
        source = 'struct X{};\n#ifdef OPTION\nint X(int);\n#endif\nX*a,b;'
        environment, _, shadows = self.environment(source)
        self.assertEqual(shadows['diagnostics'][0]['kind'], 'cpp-named-type-conditional-shadow-owner-review')
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))

    def test_macro_headers_and_literal_placeholders_do_not_create_types(self):
        source = '#define TYPE struct X {int a;};\nSSSSSSSSSSSS\nX*a,b;'
        environment, owners, _ = self.environment(source)
        self.assertEqual(owners['entries'], [])
        self.assertIsNone(identify_decl('X*a,b;', environment, offset=source.index('X*a')))


if __name__ == '__main__':
    with timing_boundary():
        unittest.main()
