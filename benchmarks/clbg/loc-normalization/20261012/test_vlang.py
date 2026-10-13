"""Unhooked V proposal fixtures. Run only after the whole-pool audit review.

Main refuses to execute while the common timing mutex is held elsewhere.
The frozen whole-pool job intentionally does not include this later proposal.
"""
import unittest
from audit import timing_boundary
from vlang import mask_vlang_source,normalize_vlang


class VlangNormalizationFixtures(unittest.TestCase):
    def count(self,source,expected,status='candidate-parser-pass'):
        result=normalize_vlang(source)
        self.assertEqual(result['normalized_loc'],expected,result)
        self.assertEqual(result['review_status'],status,result)
        self.assertFalse(result['chart_pool_eligible'])
        return result

    def equivalent(self,packed,expanded,expected):
        self.count(packed,expected)
        self.count(expanded,expected)

    def test_named_function_packing(self):
        self.equivalent('fn f() int { mut x := 1; return x }',
                        'fn f() int\n{\n mut x := 1\n return x\n}',4)

    def test_for_header_separators_are_structural(self):
        self.equivalent('for i:=0;i<n;i++ { a+=i }',
                        'for i:=0;i<n;i++\n{\n a+=i\n}',3)

    def test_imperative_branches(self):
        self.equivalent('if x { a=1 } else { a=2 }',
                        'if x\n{\n a=1\n}\nelse\n{\n a=2\n}',6)
        self.count('if x {a=1}; if y {a=2}',6)

    def test_independent_bindings_and_multi_return(self):
        self.count('a:=1; b:=2',2)
        self.count('quotient,remainder:=divide()',1)
        self.count('a,b:=1,2',2)

    def test_typed_literals_remain_data(self):
        for source in ('m:=map[string]u64{}','a:=[]u8{len:n,init:0}',
                       "p:=Entry{word:'x',count:1}",'a:=[32]int{}'):
            with self.subTest(source=source):self.count(source,1)

    def test_declarations_do_not_get_literal_packing_discount(self):
        self.equivalent('struct S { a int; b int }',
                        'struct S\n{\n a int\n b int\n}',4)
        self.equivalent('struct S { mut: a int; b int }',
                        'struct S {\n mut:\n a int\n b int\n}',5)

    def test_defer_block_packing(self):
        self.equivalent('defer {a=1; b=2}',
                        'defer {\n a=1\n b=2\n}',4)

    def test_meaningful_compiler_pragmas_are_not_packing_discounts(self):
        self.equivalent('@[inline] fn f() int {return 1}',
                        '@[inline]\nfn f() int {\n return 1\n}',4)
        self.count('#flag -Dfoo=1 -Dbar=2\nx:=1',2)

    def test_value_if_is_one_composed_binding(self):
        self.equivalent('x:=if c {1} else {2}',
                        'x:=if c {\n 1\n}\nelse {\n 2\n}',1)
        self.count('x:=\n if c {1} else {2}',1)
        self.count('return if a<b {-1} else if a>b {1} else {0}',1)
        self.count('sum += if c {i64(x)} else {-i64(x)}',1)
        self.count('denominator := 2 + (if c {j} else {i}) + 1',1)

    def test_value_fallback_and_unsafe_are_composition(self):
        self.equivalent('x:=atoi(s) or {0}',
                        'x:=atoi(s) or {\n 0\n}',1)
        self.count('x:=unsafe {nil}',1)

    def test_imperative_or_handler_packing(self):
        self.equivalent('x:=read() or {panic(err)}',
                        'x:=read() or {\n panic(err)\n}',4)
        self.equivalent('x:=read() or {y=1; panic(err)}',
                        'x:=read() or {\n y=1\n panic(err)\n}',5)
        self.count('x:=read() or {counter++}',4)

    def test_block_callback_function_packing(self):
        self.equivalent('callback:=fn(x int) int { y:=x+1; return y }',
                        'callback:=fn(x int) int {\n y:=x+1\n return y\n}',4)

    def test_nested_comments_and_literal_separators(self):
        result=self.count("/* outer /* nested */ tail */\nx:='; := { }'",1)
        self.assertEqual(result['physical_loc'],1)
        self.count("x:=r'\\n;:=/* not comment */'",1)
        self.count('x:=`\\n`',1)

    def test_interpolation_and_nested_quotes_are_masked(self):
        self.count("println('${call(x):.9f}')",1)
        self.count("println('${call(\"inner\")}')",1)
        self.count("println('${call(\"; := { }\")}')",1)
        source="println('${if x { 1 } else { 2 }}')"
        result=normalize_vlang(source)
        self.assertTrue(any(d['kind']=='vlang-interpolation-code-review' for d in result['diagnostics']))

    def test_literal_offset_and_crlf_preservation(self):
        source="\tx:='${call(\"x\")}' // comment\r\ny:=`\\n`\r\n"
        result=mask_vlang_source(source)
        self.assertEqual(len(result['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(result['cleaned']) if c in '\r\n'],
                         [i for i,c in enumerate(source) if c in '\r\n'])
        self.assertEqual(result['diagnostics'],[])

    def test_ambiguous_expression_and_compile_time_forms_are_gated(self):
        for source in ('x:=if c {a:=1; a} else {2}',
                       'x:=get() or {custom_fallback()}',
                       '$if linux { println(1) }',
                       'x:=get() or {obj.print()}',
                       'x:=if c {obj.int()} else {0}',
                       'x:=get() or {array << value}',
                       'x:=(if c {\n 1\n} else {\n 2\n})+1',
                       'for i:=read() or {panic(err)};i<n;i++ {a+=i}',
                       "for item in []Entry{Entry{word:'x',count:1}} {println(item.word)}",
                       'callback:=fn [capture](x int) int {return x}'):
            with self.subTest(source=source):
                self.assertEqual(normalize_vlang(source)['review_status'],'needs-review')

    def test_verilog_alias_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'vlang'):
            normalize_vlang('x:=1','v')


if __name__=='__main__':
    with timing_boundary():unittest.main()
