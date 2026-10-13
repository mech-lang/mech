"""Unhooked Pony proposal fixtures; execution requires the common mutex."""
import unittest

from foreign_pony import mask_pony,normalize


class PonyFixtures(unittest.TestCase):
    def count(self,source,expected,status='candidate-parser-pass'):
        result=normalize(source)
        self.assertEqual(result['normalized_loc'],expected,result)
        self.assertEqual(result['review_status'],status,result)
        self.assertFalse(result['eligible_for_chart'])
        return result

    def equivalent(self,packed,expanded,expected):
        self.count(packed,expected);self.count(expanded,expected)

    def gate(self,source,kind):
        result=normalize(source)
        self.assertEqual(result['review_status'],'needs-review',result)
        self.assertTrue(any(d['kind']==kind for d in result['diagnostics']),result)
        self.assertFalse(result['eligible_for_chart'])
        return result

    def test_type_and_member_layout_have_no_imputed_end(self):
        self.count('class Body\n  var x:F64\n  var y:F64\n  fun ke():F64 => x*x+y*y',4)
        self.count('actor Main\n  let env:Env\n  new create(e:Env) =>\n    env=e',4)
        self.count('primitive Twice\n  fun apply(x:U64):U64 => x*2',2)

    def test_independent_constructor_statements(self):
        self.equivalent('class Pair\n  let a:U64\n  let b:U64\n  new create(x:U64,y:U64) => a=x; b=y',
            'class Pair\n  let a:U64\n  let b:U64\n  new create(x:U64,y:U64) =>\n    a=x\n    b=y',6)

    def test_pure_method_is_one_composition(self):
        self.equivalent('fun apply(x:U64):U64 => x+1',
            'fun apply(x:U64):U64 =>\n  x+1',1)
        self.count('fun ref print():None => env.out.print(value.string())',1)

    def test_imperative_method_exposes_locals_and_tail(self):
        self.equivalent('fun apply(x:U64):U64 => let y=x+1; y*2',
            'fun apply(x:U64):U64 =>\n  let y=x+1\n  y*2',3)
        self.count('fun ref f() =>\n  first()\n  second()',3)

    def test_capabilities_partial_markers_and_formals_are_structural(self):
        self.count("new val create(word':String,count':U64) =>\n  word=word'\n  count=count'",3)
        self.count('fun ref f(x:Array[U8] iso,y:(Node|None)):U64 ? =>\n  return x.size()',2)
        self.equivalent('fun f(x:U64=0,y:U64=1):U64 => return x',
            'fun f(\n  x:U64=0,\n  y:U64=1\n):U64 =>\n  return x',2)

    def test_written_control_end_and_branches(self):
        self.equivalent('if x then a=1 else a=2 end',
            'if x then\n  a=1\nelse\n  a=2\nend',5)
        self.count('if x then a=1 elseif y then a=2 else a=3 end',7)
        self.count('while x<n do x=x+1 end',3)

    def test_for_iteration_patterns_are_header_owned(self):
        self.equivalent('for (word,count) in rows.pairs() do total=total+count end',
            'for (word,count) in rows.pairs() do\n  total=total+count\nend',3)

    def test_control_nested_in_method_keeps_bodies(self):
        self.count('fun f(x:Bool) =>\n  if x then\n    a=1\n  else\n    a=2\n  end',6)
        self.count('fun f() =>\n  while x<n do\n    x=x+1\n  end\n  emit(x)',5)

    def test_value_if_operand_is_one_outer_binding(self):
        self.count('let last=if n>width then n-width+1 else 0 end',1)
        self.count('let x=if a then 1 elseif b then 2 else 3 end',1)
        self.equivalent('fun f(x:Bool):U64 => if x then 1 else 2 end',
            'fun f(x:Bool):U64 =>\n  if x then\n    1\n  else\n    2\n  end',1)

    def test_pure_match_patterns_are_not_local_bindings(self):
        self.count('fun check():U64 =>\n  match (left,right)\n  | (let a:Node box,let b:Node box) => 1+a.check()+b.check()\n  else\n    1\n  end',1)
        self.count('let x=match value\n| 0 => 1\nelse\n  2\nend',1)

    def test_imperative_match_arms_and_written_close(self):
        self.count('match phase\n| 0 => output=a\n| 1 => output=b\nelse\n  output=c\nend',8)

    def test_pure_recover_constructor_is_one_composition(self):
        self.equivalent('let values=recover val Array[U8](32) end',
            'let values=recover val\n  Array[U8](32)\nend',1)
        self.count('recover String end',1)
        self.count('env.input(recover Input(this) end,65536)',1)

    def test_imperative_recover_exposes_header_body_and_end(self):
        self.equivalent('recover iso let value=Array[U8]; value end',
            'recover iso\n  let value=Array[U8]\n  value\nend',4)
        self.equivalent('recover val let values=Array[U8]; values.push(1); values end',
            'recover val\n  let values=Array[U8]\n  values.push(1)\n  values\nend',5)
        self.count('fun apply():Array[U8] val =>\n  recover val\n    let values=Array[U8]\n    values.push(1)\n    values\n  end',6)

    def test_embedded_imperative_recover_keeps_outer_binding(self):
        self.equivalent('let result=recover iso let local=Array[U8]; local end',
            'let result=recover iso\n  let local=Array[U8]\n  local\nend',5)
        self.count('consume(recover iso let local=Array[U8]; local end)',5)
        self.gate('consume(first,recover iso let local=Array[U8]; local end)',
            'pony-imperative-recover-enclosing-review')

    def test_receiving_binding_and_reset_are_two_independent_writes(self):
        self.count('let result:Array[U8] iso=field=recover Array[U8] end',2)
        self.count('let value=field=other',2)
        self.count('let value=?field',1)

    def test_where_named_arguments_are_not_assignments(self):
        self.count('env.out.print(Format.float[F64](value where fmt=FormatFix,prec=9))',1)
        self.count('let text=pattern.replace(input,replacement where global=true)?',1)
        self.gate('replace input where global=true','pony-where-argument-owner-review')

    def test_union_tuple_and_generic_data_are_structural(self):
        self.count('let left:(Node|None)',1)
        self.count('let pair=(first,second)',1)
        self.count('let maps:Array[Map[String,U64] val]=Array[Map[String,U64] val]',1)
        self.count('(let first,let second)=pair()',1)

    def test_data_wrapping_is_preserved(self):
        self.count('let data=[\n  U8(1);U8(2)\n  U8(3)\n]',4)
        self.count('fun data():Array[U8] =>\n  [\n    U8(1);U8(2)\n    U8(3)\n  ]',5)

    def test_receiver_composition_keeps_written_wrapping(self):
        self.count('values\n  .>push(1)\n  .>push(2)',3)
        self.count('values.>push(1).>push(2)',1)

    def test_pure_partial_try_value_is_composition(self):
        self.count('let n=try args(1)?.usize()? else 100 end',1)
        self.count('fun f():U64 => try value()? else 0 end',1)

    def test_imperative_try_header_and_written_end(self):
        self.equivalent('try let x=read()?; emit(x) else failed() end',
            'try\n  let x=read()?\n  emit(x)\nelse\n  failed()\nend',6)
        self.count('try emit(value()?) end',3)

    def test_mixed_error_value_controls_remain_gated(self):
        self.gate('let n=if ok then 1 else error end','pony-mixed-value-control-review')
        self.gate('recover val error end','pony-recover-error-review')

    def test_expression_only_anonymous_lambda_is_composition(self):
        self.count('let action={(x:U64):U64 => x+1}',1)
        self.count('run({(x:U64):U64 => x+1})',1)
        self.count('{(x:U64):U64 => x+1}',1)
        self.gate('run({(x:U64):U64 => let y=x+1; y})',
            'pony-imperative-lambda-composition-review')

    def test_ffi_declaration_is_source_not_generated_body(self):
        self.count('use @strlen[USize](text:Pointer[U8] tag)',1)
        self.gate('use @function[U64]','pony-ffi-declaration-review')

    def test_unknown_type_annotation_and_control_forms_remain_gated(self):
        self.gate('trait Named\n  fun name():String','pony-extended-type-review')
        self.gate('class \\packed\\ Data\n  let x:U64','pony-compiler-annotation-review')
        self.gate('with file=open() do emit(file) end','pony-extended-control-review')
        self.gate('if x then emit(x)','pony-unclosed-keyword-scope')

    def test_nested_comments_literals_and_apostrophe_names(self):
        source="/* a /* b */ c */\nlet word'=\"end ; = /*\"\nlet count'=word'\nlet byte='A'"
        masked=mask_pony(source)
        self.assertEqual(len(source),len(masked['cleaned']))
        self.assertEqual(masked['diagnostics'],[])
        self.assertIn("word'",masked['cleaned'])
        self.count(source,3)

    def test_numeric_operators_and_partial_reference_are_not_bindings(self):
        self.count('let ok=(a==b) and (c!=d) and (e<=f) and (g>=h)',1)
        self.count('let x=consume value',1)
        self.count('let x=?field',1)
        self.count('fun f():U64 => (field=value)',2)
        self.gate('fun f():U64 => (field=1;field=2)',
            'pony-executable-expression-group-review')


if __name__=='__main__':
    unittest.main()
