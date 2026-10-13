"""Canonical composed-value integration fixtures; run only under timing mutex."""
import unittest
from audit import timing_boundary
from normalizer import normalize


class ValueIntegrationFixtures(unittest.TestCase):
    def equivalent(self, sources, language, units):
        for source in sources:
            row = normalize(source, language)
            self.assertEqual(row['normalized_loc'], units, row)
            self.assertEqual(row['diagnostics'], [], row)
            self.assertTrue(any(e['kind'] == 'value-expression' for e in row['canonical_events']), row)

    def test_rust_binding_and_return_operands(self):
        self.equivalent(('let depth=if minimum+2>n {minimum+2}else{n};',
                         'let depth=if minimum+2>n {\n minimum+2\n}else{\n n\n};'), 'rust', 1)
        self.equivalent(('return if flag {a}else{b};', 'return if flag {\n a\n}else{\n b\n};'), 'rust', 1)

    def test_rust_named_function_tail(self):
        self.equivalent(('fn f(){if flag {node(a)}else{node(b)}}',
                         'fn f(){\n if flag {\n node(a)\n }else{\n node(b)\n }\n}'), 'rust', 3)

    def test_rust_match_patterns_and_fields_are_structural(self):
        self.equivalent(('fn f(){match value {None=>0,Some(Node{ref left,ref right})=>1+check(left)+check(right)}}',
                         'fn f(){\n match value {\n None=>0,\n Some(Node{ref left,ref right})=>1+check(left)+check(right)\n }\n}'), 'rust', 3)

    def test_julia_binding_operand(self):
        self.equivalent(('x=if flag; a; else; b; end', 'x=if flag\n a\nelse\n b\nend'), 'julia', 1)

    def test_julia_named_function_tail(self):
        self.equivalent(('function f(n); if n==0; node(); else; node(f(n-1)); end; end',
                         'function f(n)\n if n==0\n node()\n else\n node(f(n-1))\n end\nend'), 'julia', 3)

    def test_r_expression_only_function_and_value_binding(self):
        self.equivalent(('make<-function(depth) if(depth) list(make(depth-1),make(depth-1)) else list(NULL,NULL)',
                         'make<-compiler::cmpfun(function(depth) if(depth) list(make(depth-1),make(depth-1)) else list(NULL,NULL))'), 'r', 1)
        self.equivalent(('x<-if(flag){a}else{b}', 'x<-if(flag){\n a\n}else{\n b\n}'), 'r', 1)

    def test_arguments_do_not_add_a_second_expression(self):
        self.equivalent(('consume(if flag {a}else{b});',), 'rust', 1)
        self.equivalent(('consume(if flag; a; else; b; end)',), 'julia', 1)
        self.equivalent(('consume(if(flag){a}else{b})',), 'r', 1)

    def test_imperative_bindings_and_sequences_are_not_hidden(self):
        for source, language in (('let x=if flag {let y=work();y}else{z};', 'rust'),
                                 ('x=if flag\n y=work()\n y\nelse\n z\nend', 'julia'),
                                 ('x<-if(flag){y<-work();y}else{z}', 'r')):
            row = normalize(source, language)
            self.assertFalse(any(e['kind'] == 'value-expression' for e in row['canonical_events']), row)
            self.assertGreater(row['normalized_loc'], 1, row)

    def test_macro_callback_and_literal_wrapping_remain_reviewed(self):
        for source in ('let x=if flag {format!(value)}else{0};',
                       'let f=if flag {||{a=1;b=2;}}else{||x};',
                       'let x=if flag {[1,\n2]}else{[3,4]};'):
            row = normalize(source, 'rust')
            self.assertFalse(any(e['kind'] == 'value-expression' for e in row['canonical_events']), row)
            self.assertTrue(any(e['kind'] == 'value-form-review' for e in row['diagnostics']), row)


if __name__ == '__main__':
    with timing_boundary():
        unittest.main()
