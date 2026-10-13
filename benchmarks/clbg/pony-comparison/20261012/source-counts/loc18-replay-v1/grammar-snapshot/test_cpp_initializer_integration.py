"""Exact constructor ownership integration; execution requires the timing lock."""
import unittest
from normalizer import normalize


class CppInitializerIntegrationFixtures(unittest.TestCase):
    def count(self,source,expected):
        row=normalize(source,'cpp')
        self.assertEqual(row['normalized_loc'],expected,row)
        self.assertEqual(row['diagnostics'],[],row)
        self.assertEqual(len(row['cpp_initializer_evidence']),1,row)
        return row

    def test_explicit_member_initializers_match_separate_body_assignments(self):
        initializer='struct Node{Node*l,*r;Node(Node*l,Node*r):l(l),r(r){}};'
        assignments='struct Node{Node*l,*r;Node(Node*left,Node*right){l=left;r=right;}};'
        row=self.count(initializer,8)
        other=normalize(assignments,'cpp')
        self.assertEqual(other['normalized_loc'],row['normalized_loc'],other)
        self.assertEqual(other['diagnostics'],[],other)

    def test_packed_and_expanded_constructor_initialization_are_equivalent(self):
        packed='struct Node{int left,right;Node(int a,int b):left(a),right(b){}};'
        expanded='struct Node\n{\nint left;\nint right;\nNode(int a,int b)\n:\nleft(a),\nright(b)\n{\n}\n};'
        for source in (packed,expanded):self.count(source,8)

    def test_member_data_initializers_do_not_hide_written_body_bindings(self):
        self.count('struct Box{int a,b;Box(int x):a{x},b{make(1,2)}{int c=0,d=1;}};',10)

    def test_nested_body_controls_retain_their_own_headers_and_closes(self):
        self.count('struct Box{int x;Box(int a):x(a){if(a){x=2;}}};',9)

    def test_argument_effects_and_unknown_members_remain_review_gated(self):
        for source in ('struct Box{int x;Box(int a):x(a=2){}};',
                       'struct Box{int x;Box(int a):unknown(a){}};',
                       'struct Box{static int x;Box(int a):x(a){}};'):
            row=normalize(source,'cpp')
            self.assertTrue(row['diagnostics'],row)
            self.assertEqual(row['cpp_initializer_evidence'],[],row)

    def test_ordinary_same_name_function_is_not_constructor_proof(self):
        row=normalize('struct Box{int x;};int Box(int a){return a;}','cpp')
        self.assertEqual(row['cpp_initializer_evidence'],[],row)
        self.assertEqual(row['normalized_loc'],6,row)

    def test_constructor_calls_after_proved_constructor_do_not_invent_shadows(self):
        source='struct Node{Node*l,*r;Node(Node*a,Node*b):l(a),r(b){}};Node* make(){return new Node(0,0);}'
        self.count(source,11)


if __name__=='__main__':
    from audit import timing_boundary
    with timing_boundary():unittest.main()
