"""Unhooked value-form fixtures; standalone execution takes the timing mutex."""

import unittest

from audit import timing_boundary
from lexical import mask_source
from normalizer import _scope_map
from value_forms import classify_value_forms


class ValueFormFixtures(unittest.TestCase):
    def classify(self, source, language):
        masked = mask_source(source, language)
        self.assertEqual(masked['diagnostics'], [], masked)
        contexts, diagnostics = _scope_map(masked['cleaned'], language)
        self.assertEqual(diagnostics, [], diagnostics)
        return classify_value_forms(masked['cleaned'], language, contexts)

    def one(self, source, language, units, position):
        row = self.classify(source, language)
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(len(row['spans']), 1, row)
        span = row['spans'][0]
        self.assertEqual((span['units'], span['position']), (units, position), row)
        self.assertTrue(span['exclude_internal_boundaries'])
        self.assertGreater(span['end'], span['start'])
        return span

    def test_rust_binding_value_if_packed_and_expanded(self):
        for source in ('let depth=if minimum+2>n {minimum+2}else{n};',
                       'let depth=if minimum+2>n {\n minimum+2\n}else{\n n\n};'):
            self.one(source, 'rust', 0, 'binding-operand')

    def test_rust_return_and_argument_value_if(self):
        self.one('return if flag {a}else{b};', 'rust', 0, 'return-operand')
        self.one('consume(if flag {a}else{b});', 'rust', 0, 'argument-operand')

    def test_rust_function_tail_value_if(self):
        for source in ('fn f(){if flag {node(a)}else{node(b)}}',
                       'fn f(){\n if flag {\n node(a)\n }else{\n node(b)\n }\n}'):
            self.one(source, 'rust', 1, 'function-tail')

    def test_rust_match_patterns_are_structural(self):
        for source in ('fn f(){match value {None=>0,Some(Node{ref left,ref right})=>1+check(left)+check(right)}}',
                       'fn f(){\n match value {\n None=>0,\n Some(Node{ref left,ref right})=>1+check(left)+check(right)\n }\n}'):
            span = self.one(source, 'rust', 1, 'function-tail')
            self.assertEqual(span['form'], 'match')
            self.assertEqual(span['branches'], 2)

    def test_rust_imperative_result_blocks_remain_expanded(self):
        row = self.classify('fn f(){if flag {let x=allocate(node());Some(x)}else{None}}', 'rust')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['state'] == 'imperative' for item in row['evidence']))
        row = self.classify('let x=if flag {work();a}else{b};', 'rust')
        self.assertEqual(row['spans'], [], row)

    def test_nested_imperative_callbacks_are_not_hidden(self):
        row = self.classify('let f=if flag {||{a=1;b=2;}}else{||x};', 'rust')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['reason'] == 'nested-function-or-callback' for item in row['diagnostics']))

    def test_julia_binding_value_if_packed_and_expanded(self):
        for source in ('x=if flag; a; else; b; end',
                       'x=if flag\n a\nelse\n b\nend'):
            self.one(source, 'julia', 0, 'binding-operand')

    def test_julia_and_r_argument_value_conditionals(self):
        self.one('consume(if flag; a; else; b; end)', 'julia', 0, 'argument-operand')
        self.one('consume(if(flag){a}else{b})', 'r', 0, 'argument-operand')

    def test_outer_julia_arguments_do_not_hide_branch_bindings(self):
        row = self.classify('consume(if flag; y=work(); y; else; z; end)', 'julia')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['state'] == 'imperative' for item in row['evidence']))

    def test_julia_expression_only_function_tail(self):
        for source in ('function f(n); if n==0; node(); else; node(f(n-1)); end; end',
                       'function f(n)\n if n==0\n node()\n else\n node(f(n-1))\n end\nend'):
            self.one(source, 'julia', 1, 'function-tail')

    def test_julia_imperative_and_distributed_loops_are_not_values(self):
        row = self.classify('x=if flag\n y=work()\n y\nelse\n z\nend', 'julia')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['state'] == 'imperative' for item in row['evidence']))
        row = self.classify('x=@distributed (+) for i=1:n\n work(i)\nend', 'julia')
        self.assertEqual(row['spans'], [], row)

    def test_r_expression_function_conditionals(self):
        for source in ('make<-function(depth) if(depth) list(make(depth-1),make(depth-1)) else list(NULL,NULL)',
                       'make<-compiler::cmpfun(function(depth) if(depth) list(make(depth-1),make(depth-1)) else list(NULL,NULL))'):
            self.one(source, 'r', 0, 'expression-function-body')

    def test_r_binding_value_if_packed_and_expanded(self):
        for source in ('x<-if(flag){a}else{b}',
                       'x<-if(flag){\n a\n}else{\n b\n}'):
            self.one(source, 'r', 0, 'binding-operand')

    def test_r_imperative_branches_and_statement_controls_remain(self):
        row = self.classify('x<-if(flag){y<-work();y}else{z}', 'r')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['state'] == 'imperative' for item in row['evidence']))
        row = self.classify('if(flag) next', 'r')
        self.assertEqual(row['spans'], [], row)

    def test_guards_macros_and_literal_wrapping_stay_gated(self):
        for source in ('let x=match v {Some(a) if a>0=>a,_=>0};',
                       'let x=if flag {format!(value)}else{0};',
                       'let x=if flag {[1,\n2]}else{[3,4]};'):
            row = self.classify(source, 'rust')
            self.assertEqual(row['spans'], [], row)
            self.assertTrue(row['diagnostics'], row)

    def test_opaque_owns_all_covered_rows_without_owning_outer_binding(self):
        source = 'let x=if flag {\n a\n}else{\n b\n};'
        span = self.one(source, 'rust', 0, 'binding-operand')
        self.assertEqual(span['start'], source.index('if'))
        self.assertEqual(source[span['end']:], ';')
        self.assertEqual(span['owned_rows'], [1, 2, 3, 4, 5])

    def test_unproven_statement_position_is_not_automatically_opaque(self):
        row = self.classify('if flag {a}else{b};', 'rust')
        self.assertEqual(row['spans'], [], row)
        self.assertTrue(any(item['reason'] == 'unproven-value-position' for item in row['diagnostics']))

    def test_recorded_original_value_forms_are_generic_fixtures(self):
        self.one('let max_depth = if min_depth + 2 > n {min_depth + 2} else {n};',
                 'rust', 0, 'binding-operand')
        self.one("fn item_check(t: &Option<&Tree>) -> i32 {\n"
                 " match *t {\n None => 0,\n"
                 " Some(&Tree { ref l, ref r }) => 1 + item_check(l) + item_check(r)\n }\n}",
                 'rust', 1, 'function-tail')
        self.one('function make(d)\n if d == 0\n Node(Empty(), Empty())\n else\n Node(make(d-1), make(d-1))\n end\nend',
                 'julia', 1, 'function-tail')
        self.one('make <- function(depth) if (depth) list(make(depth-1L),make(depth-1L)) else list(NULL,NULL)',
                 'r', 0, 'expression-function-body')


if __name__ == '__main__':
    with timing_boundary():
        unittest.main()
