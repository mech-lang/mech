"""Nim ownership fixtures; execute only in an immutable common-mutex job."""
import unittest

from foreign_nim import mask_nim, normalize


class NimFixtures(unittest.TestCase):
    def count(self,source,expected,status='candidate-parser-pass'):
        result=normalize(source,'nim')
        self.assertEqual(result['normalized_loc'],expected,result)
        self.assertEqual(result['review_status'],status,result)
        self.assertFalse(result['eligible_for_chart'])
        return result

    def equivalent(self,packed,expanded,expected):
        self.count(packed,expected)
        self.count(expanded,expected)

    def gate(self,source,kind):
        result=normalize(source)
        self.assertEqual(result['review_status'],'needs-review',result)
        self.assertTrue(any(d['kind']==kind for d in result['diagnostics']),result)
        self.assertFalse(result['eligible_for_chart'])
        return result

    def test_declaration_sections_are_structural(self):
        self.equivalent('var a,b:int','var\n  a,b:int',2)
        self.equivalent('let a=1; b=2','let\n  a=1\n  b=2',2)
        self.equivalent('const a=1; b=2','const\n  a=1\n  b=2',2)

    def test_independent_initialized_and_uninitialized_names(self):
        self.count('var a,b,c: int = 0',3)
        self.count('var a,b,c: int',3)
        self.count('var a=1; var b=2; var c=3',3)

    def test_tuple_binding_is_one_explicit_producer(self):
        self.count('let (a,b) = pair()',1)
        self.count('(a,b) = (b,a)',1)
        self.count('let a = b = value',2)

    def test_named_object_type_fields_are_independent(self):
        self.equivalent('type\n  Node = ref object\n    left,right: Node',
            'type Node = ref object\n  left: Node\n  right: Node',3)
        self.count('type Body = object\n  x,y,z:float64\n  mass:float64',5)

    def test_named_tuple_type_has_field_owners(self):
        self.equivalent('type Job = tuple[input,output: ptr seq[float64], first,last: int]',
            'type\n  Job = tuple[\n    input,output: ptr seq[float64],\n    first,last: int\n  ]',5)
        self.count('type Entry = tuple[word:string,count:int64]',3)

    def test_generic_type_and_constructor_data_do_not_expand(self):
        self.count('var table: Table[uint64,int64]',1)
        self.count('let body=Body(x:1,y:2,z:3)',1)
        self.count('launch(worker,(addr input,addr output,0,n,false))',1)
        self.count('let record=(word:"abc",count:3)',1)

    def test_literal_data_wrapping_is_preserved(self):
        self.count('let body=Body(\n  x:1,\n  y:2,\n  z:3)',4)
        self.count('let values=[\n  1,2,\n  3,4\n]',4)

    def test_pure_named_procedure_is_composition(self):
        self.equivalent('proc denominator(i,j:int):float64 = float64(i+j)',
            'proc denominator(i,j:int):float64 =\n  float64(i+j)',1)
        self.equivalent('func twice(x:int):int = x * 2',
            'func twice(x:int):int =\n  x *\n    2',1)

    def test_imperative_procedure_body_is_not_composition(self):
        self.equivalent('proc f(x:int):int = var y=x+1; return y',
            'proc f(x:int):int =\n  var y=x+1\n  return y',3)
        self.count('proc product(x:var seq[float64]) =\n  first(x)\n  second(x)',3)

    def test_wrapped_named_header_and_formal_defaults(self):
        self.equivalent('proc f(x,y:int=0,z:int=1):int = return x',
            'proc f(\n  x,y:int=0,\n  z:int=1\n):int =\n  return x',2)

    def test_procedure_formal_groups_are_not_fields(self):
        self.count('proc offset(b:var Body,px,py,pz:float) =\n  b.vx=px\n  b.vy=py\n  b.vz=pz',4)

    def test_control_headers_and_body_bindings(self):
        self.equivalent('for i,j in pairs: a+=i; b+=j',
            'for i,j in pairs:\n  a+=i\n  b+=j',3)
        self.equivalent('if x: a=1; b=2',
            'if x:\n  a=1\n  b=2',3)
        self.count('while x<n:\n  x+=1',2)

    def test_layout_has_no_imputed_closers(self):
        self.count('if x:\n  a=1\nelse:\n  a=2\nb=3',5)
        self.count('block:\n  let local=1\n  emit(local)\nemit(2)',4)

    def test_case_patterns_belong_to_branch_headers(self):
        self.count('case ch\nof 1,2: return 3\nelse: raise bad()',5)
        self.count('proc code(c:char):int =\n  case c\n  of \'A\': return 0\n  of \'C\': return 1\n  else: raise bad()',8)

    def test_value_only_if_and_case_procedures(self):
        self.equivalent('proc choose(x:int):int = if x>0: 1 else: 2',
            'proc choose(x:int):int =\n  if x>0:\n    1\n  else:\n    2',1)
        self.count('proc code(x:int):int =\n  case x\n  of 0,1: 2\n  else: 3',1)

    def test_value_conditional_operand_is_not_statement_header(self):
        self.count('let v = if x: 1 else: 2',1)
        self.count('total += value / (if transpose: A(j,i) else: A(i,j))',1)
        self.gate('let v = if x: var y=1; y else: 2','nim-value-control-review')
        self.gate('consume((if x: var y=1; y else: 2))','nim-value-control-review')

    def test_explicit_callback_retains_header_and_body(self):
        # The earlier gated draft's proposed fragment count is superseded by
        # root's common composition rule; unknown outer forms remain gated.
        self.count('let action = proc(x:int):int = x+1',1)
        self.count('run(proc(x:int):int = x+1)',1)
        self.count('let action=proc(x:int):int = let y=x+1; y',3)
        self.count('run(proc(x:int):int = let y=x+1; y)',3)
        self.gate('run(1,proc(x:int):int = x+1)','nim-callback-enclosing-owner-review')
        self.gate('let actions=[proc(x:int):int = x+1]','nim-callback-enclosing-owner-review')
        self.gate('run(proc(x:int):int = (let y=x+1; y))','nim-executable-expression-group-review')

    def test_standalone_meaningful_pragma_is_retained(self):
        self.count('{.push checks:off.}\nlet x=1\n{.pop.}',3)
        self.count('{.warning[UnusedImport]:off.}\nimport std/math',2)

    def test_attached_meaningful_pragmas_are_independent(self):
        self.equivalent('proc f(x:int):int {.inline.} = x+1',
            '{.inline.}\nproc f(x:int):int =\n  x+1',2)
        self.equivalent('type Node {.acyclic.} = ref object\n  left:Node',
            '{.acyclic.}\ntype Node = ref object\n  left:Node',3)
        self.equivalent('proc worker(x:int) {.thread.} =\n  run(x)',
            '{.thread.}\nproc worker(x:int) =\n  run(x)',2)

    def test_codegen_and_custom_pragmas_remain_gated(self):
        self.gate('{.emit:"foreign code".}','nim-codegen-or-unknown-pragma-review')
        self.gate('proc f(x:int) {.custom.} = x','nim-codegen-or-unknown-pragma-review')

    def test_nested_comments_strings_chars_and_numeric_suffixes(self):
        source='#[ a #[ b ]# c ]#\nlet a="; # { } ="\nlet b=re"[A-Z]"\nlet c=\'A\'\nlet d=0\'u64\nlet e=0xFF\'u8'
        result=mask_nim(source)
        self.assertEqual(len(result['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(source) if c in '\r\n'],
            [i for i,c in enumerate(result['cleaned']) if c in '\r\n'])
        self.assertEqual(result['diagnostics'],[])
        self.count(source,5)

    def test_quoted_identifiers_and_raw_literals_preserve_positions(self):
        source='let `type` = r"a""b"\nlet text="""x\ny"""'
        result=mask_nim(source)
        self.assertEqual(len(source),len(result['cleaned']))
        self.assertEqual(result['diagnostics'],[])
        self.count(source,3)

    def test_comparison_operators_are_not_assignments(self):
        self.count('let ok=(a==b) and (c!=d) and (e<=f) and (g>=h)',1)

    def test_unknown_macro_template_and_extended_type_are_gated(self):
        self.gate('template f(x:untyped)=x','nim-metaprogram-or-declaration-review')
        self.gate('macro f(x:typed):untyped = x','nim-metaprogram-or-declaration-review')
        self.gate('type Node = object of RootObj\n  x:int','nim-type-header-suffix-review')

    def test_unqualified_lexical_forms_remain_gated(self):
        self.gate('let text=fmt"{value}"','future-nim-custom-literal-review')
        self.gate('let text="unterminated','future-unclosed-string')


if __name__=='__main__':
    unittest.main()
