"""Unexecuted bounded C++ initializer ownership fixtures.

Execute only in a parent-reviewed snapshot under the common timing mutex.
These tests do not read benchmark sources or certify chart counts.
"""
import unittest

from c_type_environment import collect_typedef_aliases, identify_decl
from cpp_initializers import classify_cpp_initializers
from normalizer import _scope_map


class CppInitializerProposalFixtures(unittest.TestCase):
    def proposal(self,source):
        environment=collect_typedef_aliases(source,'cpp',preprocessor_source=source)
        contexts,_=_scope_map(source,'cpp')
        return classify_cpp_initializers(source,'cpp',contexts,environment,
            declaration_resolver=identify_decl)

    def owned(self,source,expected):
        result=self.proposal(source)
        self.assertEqual(result['diagnostics'],[],result['diagnostics'])
        self.assertEqual(sum(event['units'] for event in result['events']),expected)
        self.assertTrue(result['proposal_only'])
        return result

    def gate(self,source,kind):
        result=self.proposal(source)
        self.assertIn(kind,{issue['kind'] for issue in result['diagnostics']})
        return result

    def test_other_languages_empty_without_aliasing(self):
        for language in ('c','c++','rust','java'):
            result=classify_cpp_initializers('unparsed',language,[],{},declaration_resolver=identify_decl)
            self.assertEqual(result['events'],[])
            self.assertEqual(result['diagnostics'],[])

    def test_node_header_two_members_written_close_is_four(self):
        result=self.owned('struct Node{Node*l,*r;Node(Node*a,Node*b):l(a),r(b){}};',4)
        owner=result['owners'][0]
        self.assertEqual((owner['header_units'],owner['member_units'],owner['written_close_units']),(1,2,1))
        self.assertEqual([event['kind'] for event in result['events']],['header','member-initializer','member-initializer','open','close'])

    def test_node_packed_expanded_same_initializer_units(self):
        self.owned('struct Node{Node*l,*r;Node(Node*a,Node*b):l(a),r(b){}};',4)
        self.owned('struct Node\n{\n Node*l;\n Node*r;\n Node(\n Node*a,Node*b)\n :l(a),\n r(b)\n {\n }\n};',4)

    def test_body_bindings_are_left_to_caller_not_masked(self):
        source='struct Node{Node*l,*r;Node(Node*a,Node*b):l(a),r(b){int x=y=0;int z;z=x;}};'
        result=self.owned(source,4);owner=result['owners'][0]
        body=source[owner['body_opening']+1:owner['body_closing']]
        self.assertEqual(body,'int x=y=0;int z;z=x;')
        self.assertFalse(any(event['start']<owner['body_closing'] and owner['body_opening']+1<event['end'] for event in result['events']))
        self.assertEqual(result['executable_regions'][0]['start'],owner['body_opening']+1)

    def test_braced_initializers_count_only_written_members(self):
        self.owned('struct V{double x=0,y=0,z=0,pad=0;constexpr V(double a,double b,double c):x{a},y{b},z{c}{}};',5)
        self.owned('struct V{double x=0,y=0,z=0,pad=0;constexpr V(double a,double b,double c)\n:x{a},\ny{b},\nz{c}\n{\n}};',5)

    def test_members_declared_after_constructor_are_proven_direct_members(self):
        self.owned('struct V{V():x{1},y{2}{}int x,y;};',4)
        self.owned('struct V{public:V():x(1),y(2){}private:int x,y;};',4)

    def test_plain_calls_nested_data_and_argument_commas_are_structural(self):
        self.owned('struct N{int x,y;N():x(make(1,2)),y{make(3,4)}{}};',4)
        self.owned('struct N{int values[4];N():values{1,2,3,4}{}};',3)
        self.owned('struct N{int values[2][2];N():values{{1,2},{3,4}}{}};',3)

    def test_literal_masked_arguments_keep_offsets_without_name_dispatch(self):
        source='struct N{const char*text;N():text(SSSSSSSS){}};'
        result=self.owned(source,3)
        span=result['owners'][0]['members'][0]
        self.assertEqual(source[span['argument_start']:span['argument_end']],'SSSSSSSS')

    def test_multiline_literal_or_data_arguments_require_written_row_proof(self):
        for source in ('struct N{int values[2];N():values{1,\n2}{}};',
                       'struct N{int values[2][2];N():values{{1,2},\n{3,4}}{}};',
                       'struct N{const char*text;N():text(SSS +\n SSS){}};'):
            result=self.gate(source,'cpp-wrapped-initializer-data-review')
            self.assertEqual(result['events'],[])

    def test_noexcept_and_modifiers_belong_constructor_header(self):
        self.owned('class N{public:int x;explicit N(int a) noexcept:x(a){}};',3)
        self.owned('struct N{int x;inline constexpr N(int a) noexcept(true):x(a){}};',3)
        result=self.proposal('struct N{int x;explicit(false) N(int a):x(a){}};')
        self.assertTrue({'cpp-initializer-list-owner-review',
            'cpp-initializer-constructor-owner-review'} &
            {item['kind'] for item in result['diagnostics']},result)
        self.assertEqual(result['events'],[])

    def test_initializer_commas_are_owned_without_extra_statement_units(self):
        source='struct N{int x,y;N():x(0),\n y(0){}};'
        result=self.owned(source,4);owner=result['owners'][0]
        chars=list(source)
        for event in result['events']:
            for index in range(event['start'],event['end']): chars[index]=' '
        self.assertFalse(''.join(chars)[owner['start']:owner['prefix_end']].strip())

    def test_qualified_global_constructor_needs_declared_owner(self):
        self.owned('struct N{int x;N();};N::N():x(1){}',3)
        self.gate('Missing::Missing():x(1){}','cpp-initializer-constructor-owner-review')

    def test_explicit_namespace_and_nested_class_qualification(self):
        self.owned('namespace n{struct N{int x;N();};}n::N::N():x(1){}',3)
        self.owned('struct Outer{struct N{int x;N();};};Outer::N::N():x(1){}',3)

    def test_local_expired_class_does_not_supply_qualified_owner(self):
        self.gate('void f(){struct N{int x;N();};}N::N():x(1){}','cpp-initializer-constructor-owner-review')

    def test_template_and_conditional_class_owners_remain_gated(self):
        self.gate('template<typename T>struct N{T x;N(T v):x(v){}};','cpp-initializer-constructor-owner-review')
        self.gate('#ifdef A\nstruct N{int x;N():x(0){}};\n#endif','cpp-initializer-constructor-owner-review')

    def test_conditional_constructor_or_member_is_not_unconditional_proof(self):
        self.gate('struct N{int x;\n#ifdef A\nN():x(0){}\n#endif\n};','cpp-initializer-conditional-owner-review')
        self.gate('struct N{\n#ifdef A\nint x;\n#endif\nN():x(0){}\n};','cpp-member-base-initializer-owner-review')

    def test_unknown_static_or_base_initializer_is_not_inferred(self):
        self.gate('struct N{int x;N():unknown(0),x(1){}};','cpp-member-base-initializer-owner-review')
        self.gate('struct N{static int x;N():x(0){}};','cpp-member-base-initializer-owner-review')
        self.gate('struct Base{};struct N:Base{int x;N():Base(),x(0){}};','cpp-initializer-constructor-owner-review')

    def test_delegating_constructor_is_separately_gated(self):
        result=self.gate('struct N{int x;N(int v):x(v){}N():N(0){}};','cpp-delegating-initializer-owner-review')
        self.assertEqual(len(result['owners']),1)
        self.assertEqual(result['owners'][0]['members'][0]['name'],'x')

    def test_duplicate_initializer_never_adds_an_admitted_unit(self):
        result=self.gate('struct N{int x;N():x(0),x(1){}};','cpp-duplicate-member-initializer-review')
        self.assertEqual(result['events'],[])

    def test_argument_assignments_updates_sequences_and_callbacks_are_visible_gates(self):
        for arg in ('y=0','y++','(y=0)','[](){int y=0;return y;}()','make(y=0)'):
            result=self.gate('struct N{int x,y;N():x('+arg+'){}};','cpp-initializer-argument-work-review')
            self.assertEqual(result['events'],[])

    def test_formal_executable_default_is_not_hidden_in_header(self):
        self.gate('struct N{int x;N(int a=[](){return 1;}()):x(a){}};','cpp-initializer-list-owner-review')

    def test_unbalanced_or_unmatched_groups_never_supply_owner(self):
        self.gate('struct N{int x;N():x(0){','cpp-initializer-delimiter-owner-review')
        self.gate('struct N{int x;N():x(0),{}};','cpp-initializer-list-owner-review')

    def test_ordinary_type_calls_and_ternaries_are_not_constructor_headers(self):
        result=self.proposal('struct N{int x;};N* make(){return flag?new N():nullptr;}void f(){N(flag?0:1);}')
        self.assertEqual(result['owners'],[])
        self.assertEqual(result['diagnostics'],[])

    def test_context_overrides_keep_nested_structural_frames(self):
        result=self.owned('struct N{int x;N(int a):x(make(a,2)){int y=3;}};',3)
        self.assertEqual([row['kind'] for row in result['context_overrides']],['parameters','args','code'])
        self.assertEqual([row['role'] for row in result['suppress_canonical_spans']],['constructor-prefix','constructor-open','constructor-close'])


if __name__=='__main__':
    unittest.main()
