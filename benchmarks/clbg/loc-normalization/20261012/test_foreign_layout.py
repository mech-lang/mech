"""Unexecuted F#/Roc/Elm layout fixtures for common-mutex qualification.

These expectations are a parser proposal, never source/pin/count admission.
The fixture text reproduces language forms, not benchmark-specific dispatch.
No benchmark report or original program is mutated/read by this test module.
"""
import unittest

from foreign_layout import classify_scopes, mask_layout, normalize, _scalar


class ForeignLayoutProposalFixtures(unittest.TestCase):
    def assert_units(self,source,language,expected):
        result=normalize(source,language)
        self.assertEqual(result['diagnostics'],[],result['diagnostics'])
        self.assertEqual(result['normalized_loc'],expected,result['expansions'])
        self.assertTrue(result['proposal_only'])
        self.assertEqual(result['proposal_status'],'unhooked-no-admission')
        return result

    def assert_gate(self,source,language,kind):
        result=normalize(source,language)
        self.assertIn(kind,{issue['kind'] for issue in result['diagnostics']},result['diagnostics'])
        self.assertTrue(result['proposal_only'])
        return result

    def test_expression_only_named_functions_are_one_composition(self):
        cases=[('fsharp','let inc x = x+1','let inc x =\n    x+1'),
            ('roc','inc = |x| x+1','inc = |x|\n    x+1'),
            ('elm','inc x = x+1','inc x =\n    x+1')]
        for language,packed,expanded in cases:
            self.assert_units(packed,language,1)
            self.assert_units(expanded,language,1)

    def test_value_conditionals_do_not_hide_extra_locals(self):
        cases=[('fsharp','let choose x = if x>0 then x else 0','let choose x =\n    if x>0 then\n        x\n    else\n        0'),
            ('roc','choose = |x| if x>0 then x else 0','choose = |x|\n    if x>0 then\n        x\n    else\n        0'),
            ('elm','choose x = if x>0 then x else 0','choose x =\n    if x>0 then\n        x\n    else\n        0')]
        for language,packed,expanded in cases:
            self.assert_units(packed,language,1)
            self.assert_units(expanded,language,1)

    def test_fsharp_packed_and_layout_independent_locals(self):
        self.assert_units('let compute x = let a=x+1; let b=a+1; a+b','fsharp',4)
        self.assert_units('let compute x =\n    let a=x+1\n    let b=a+1\n    a+b','fsharp',4)

    def test_fsharp_mutable_bindings_and_writes_remain_independent(self):
        self.assert_units('let compute x = let mutable a=x; a<-a+1; a','fsharp',4)
        self.assert_units('let compute x =\n    let mutable a=x\n    a<-a+1\n    a','fsharp',4)

    def test_fsharp_body_parenthesis_semicolon_sequence(self):
        self.assert_units('let change x = (a<-1;b<-2;x)','fsharp',4)
        self.assert_units('let change x =\n    (\n      a<-1\n      b<-2\n      x\n    )','fsharp',4)

    def test_fsharp_iteration_in_is_not_local_scope_close(self):
        self.assert_units('let advance n = for i in 0..n do a<-a+i','fsharp',3)
        self.assert_units('let advance n =\n    for i in 0..n do\n        a<-a+i','fsharp',3)

    def test_fsharp_imperative_if_and_else_headers(self):
        self.assert_units('let change x = if x then a<-1 else a<-2','fsharp',5)
        self.assert_units('let change x =\n    if x then\n        a<-1\n    else\n        a<-2','fsharp',5)

    def test_fsharp_tuple_binding_one_producer(self):
        self.assert_units('let dx,dy,dz = a.x-b.x,a.y-b.y,a.z-b.z','fsharp',1)
        self.assert_units('let change a b =\n    let dx,dy,dz = a.x-b.x,a.y-b.y,a.z-b.z\n    dx+dy+dz','fsharp',3)

    def test_fsharp_formal_record_semicolons_structural(self):
        result=self.assert_units('let combine {first=first; second=second} extra = first+second+extra','fsharp',1)
        self.assertFalse(any(e.get('declaration_bindings',0)>1 for row in result['expansions'] for e in row['events']))

    def test_fsharp_independent_record_type_fields(self):
        packed='type Body = { mutable X: float; mutable Y: float; mutable Z: float; Mass: float; mutable VX: float; mutable VY: float; mutable VZ: float }'
        expanded='type Body =\n{\nmutable X: float\nmutable Y: float\nmutable Z: float\nMass: float\nmutable VX: float\nmutable VY: float\nmutable VZ: float\n}'
        self.assert_units(packed,'fsharp',9)
        self.assert_units(expanded,'fsharp',9)

    def test_record_instances_are_data_not_independent_fields(self):
        for language,source in [('fsharp','value = { X=1; Y=2; Z=3 }'),
            ('roc','value = {x:1,y:2,z:3}'),('elm','value = {x=1,y=2,z=3}')]:
            # F# bindings require their own written let keyword.
            if language=='fsharp': source='let '+source
            self.assert_units(source,language,1)

    def test_data_wrapping_retains_original_rows(self):
        for language in ('fsharp','roc','elm'):
            source=('let ' if language=='fsharp' else '')+'values = [\n1,2,\n3,4\n]'
            self.assert_units(source,language,4)

    def test_fsharp_data_function_first_row_not_counted_twice(self):
        self.assert_units('let values x = [\n1,2\n]','fsharp',3)
        self.assert_units('let body x = {\nX=x;Y=2\n}','fsharp',3)

    def test_fsharp_async_inside_array_resets_data_owner(self):
        packed='let tasks = [| async { let a=1; return a } |]'
        expanded='let tasks = [| async\n{\n    let a=1\n    return a\n}\n|]'
        for source in (packed,expanded):
            result=self.assert_units(source,'fsharp',5)
            self.assertTrue(any(e['scope']=='foreign-fsharp-computation' and e['kind']=='header' for e in result['canonical_events']))
            self.assertTrue(any(e['scope']=='foreign-fsharp-computation' and e['kind']=='close' for e in result['canonical_events']))

    def test_expression_lambdas_remain_composed_expressions(self):
        self.assert_units('let transform xs = List.map (fun x -> x+1) xs','fsharp',1)
        self.assert_units('transform = |xs| List.map(xs, |x| x+1)','roc',1)
        self.assert_units('transform xs = List.map (\\x -> x+1) xs','elm',1)

    def test_elm_local_scope_open_zero_and_written_in_close_one(self):
        packed='compute x = let a=x+1 in a+x'
        expanded='compute x =\n    let\n        a=x+1\n    in\n    a+x'
        for source in (packed,expanded):
            result=self.assert_units(source,'elm',4)
            self.assertTrue(any(e['kind']=='open' and e['units']==0 and e['scope']=='foreign-elm-local-declarations' for e in result['canonical_events']))
            self.assertTrue(any(e['kind']=='close' and e['units']==1 and e['scope']=='foreign-elm-local-declarations' for e in result['canonical_events']))

    def test_fsharp_explicit_local_in_same_four_units(self):
        self.assert_units('let compute x = let a=x+1 in a+x','fsharp',4)
        self.assert_units('let compute x =\n    let a=x+1\n    in\n    a+x','fsharp',4)

    def test_elm_two_locals_and_nested_local_scopes(self):
        self.assert_units('compute x =\n    let\n        a=x+1\n        b=a+1\n    in\n    a+b','elm',5)
        self.assert_units('compute x = let a=x+1 in let b=a+1 in a+b','elm',6)

    def test_parenthesized_local_scope_argument_is_not_blanket_data(self):
        self.assert_units('compute x = (let a=x+1 in a+x)','elm',4)
        self.assert_units('let compute x = (let a=x+1 in a+x)','fsharp',4)

    def test_roc_layout_independent_bindings_and_tail(self):
        self.assert_units('compute = |x|\n    a=x+1\n    b=a+1\n    a+b','roc',4)

    def test_scalar_case_arm_patterns_are_structural(self):
        cases=[('fsharp','let check tree = match tree with | Empty -> 0 | Node(left,right) -> 1+check left+check right',
                'let check tree =\n    match tree with\n    | Empty -> 0\n    | Node(left,right) -> 1+check left+check right'),
            ('roc','check = |tree|\n    when tree is\n        Empty -> 0\n        Node(_,left,right) -> 1+check(left)+check(right)',None),
            ('elm','check tree =\n    case tree of\n        Empty -> 0\n        Node _ left right -> 1+check left+check right',None)]
        for language,packed,expanded in cases:
            self.assert_units(packed,language,1)
            if expanded: self.assert_units(expanded,language,1)

    def test_fsharp_array_case_pattern_pipes_not_arm_separators(self):
        self.assert_units('let n = match args with [|n|] -> int n | _ -> 0','fsharp',1)

    def test_elm_case_arm_local_bindings_stay_visible(self):
        source='compute xs =\n    case xs of\n        [] ->\n            let\n                a=1\n            in\n            a\n        x::rest -> x'
        self.assert_units(source,'elm',8)

    def test_roc_and_elm_named_record_type_fields(self):
        self.assert_units('Body : {x:F64,y:F64,z:F64}','roc',5)
        self.assert_units('type alias Body = {x:Float,y:Float,z:Float}','elm',5)

    def test_elm_record_update_pipe_is_not_case_or_lambda(self):
        self.assert_units('advance xs = List.map (\\b -> {b|x=b.x+1,y=b.y+1}) xs','elm',1)
        self.assert_units('advance = |xs| List.map(xs, |b| {b & x:b.x+1,y:b.y+1})','roc',1)

    def test_elm_port_module_import_signature_units(self):
        self.assert_units('port module Program exposing (main)\nimport Platform\nport input : ({n:Int,input:String} -> msg) -> Sub msg','elm',3)

    def test_roc_division_is_not_comment(self):
        source='whole = scaled // factor # arithmetic\nnext = whole+1'
        masked=mask_layout(source,'roc')
        self.assertIn('//',masked['cleaned'])
        self.assertNotIn('# arithmetic',masked['cleaned'])
        self.assert_units(source,'roc',2)

    def test_comments_literals_and_offsets(self):
        cases=[('fsharp','(* outer (* nested *) done *)\nlet text="if x then a=1; else b=2" // end'),
            ('roc','# if let a=1\ntext="a=1; // division inside literal"'),
            ('elm','{- outer {- nested -} done -}\ntext="let a=1 in a" -- tail')]
        for language,source in cases:
            masked=mask_layout(source,language)
            self.assertEqual(len(masked['cleaned']),len(source))
            self.assertEqual([i for i,c in enumerate(source) if c=='\n'],[i for i,c in enumerate(masked['cleaned']) if c=='\n'])
            self.assert_units(source,language,1)

    def test_fsharp_attribute_semicolon_is_structural(self):
        self.assert_units('[<Sealed; AllowNullLiteral>]\ntype Node(left: Node,right: Node) =\n    member this.check() = 1','fsharp',3)

    def test_complex_forms_remain_source_review_gated(self):
        self.assert_gate('let computation = builder { let a=1; a }','fsharp','foreign-fsharp-computation-builder-review')
        self.assert_gate('let rec f x=x\nand g x=x','fsharp','foreign-layout-complex-declaration-review')
        self.assert_gate('let! value = fetch()','fsharp','foreign-layout-complex-declaration-review')
        self.assert_gate('f = |{x}| x','roc','foreign-roc-function-formals-review')
        self.assert_gate('compute x = if x then 1','elm','foreign-layout-missing-else-review')
        self.assert_gate('value = in x','elm','foreign-layout-in-owner-review')
        self.assert_gate('value = let x=1','elm','foreign-layout-let-owner-review')
        self.assert_gate('a=1;b=2','roc','foreign-layout-semicolon-review')
        self.assert_gate('infix right 5 (++) = append','elm','foreign-layout-complex-declaration-review')

    def test_interpolation_and_directives_are_not_silently_hidden(self):
        self.assert_gate('let text=$"{mutate()}"','fsharp','foreign-interpolation-expression-review')
        self.assert_gate('text="${value=mutate()}"','roc','foreign-interpolation-expression-review')
        self.assert_gate('#if FEATURE\nlet value=1\n#endif','fsharp','foreign-directive-policy-review')

    def test_imperative_lambda_surrounding_application_needs_review(self):
        self.assert_gate('let compute xs = List.fold (fun x -> let y=x+1 in y) 0 xs','fsharp','foreign-layout-lambda-surrounding-expression-review')

    def test_unknown_languages_are_not_aliased(self):
        for language in ('julia','rust','haskell','smalltalk','nim'):
            with self.assertRaises(ValueError): normalize('value=1',language)
            with self.assertRaises(ValueError): classify_scopes('value=1',language)

    def test_elm_operator_declaration_heads_never_become_opaque_functions(self):
        for source in ('infix right 5 (++) = append','infix right 5 (++) =\n    append',
                'infixl 5 (++) = append','infixr 5 (++) = append'):
            self.assert_gate(source,'elm','foreign-layout-complex-declaration-review')
        self.assert_units('description = "infix right 5 (++) = append"','elm',1)

    def test_fsharp_formal_record_vs_executable_body_semicolons(self):
        self.assert_units('let combine {first=first; second=second} extra =\n    first+second+extra','fsharp',1)
        packed='let change {first=first; second=second} extra = first.Value<-extra;second.Value<-extra'
        expanded='let change {first=first; second=second} extra =\n    first.Value<-extra\n    second.Value<-extra'
        self.assert_units(packed,'fsharp',3)
        self.assert_units(expanded,'fsharp',3)
        self.assert_gate('let value = builder { let a=1; a }','fsharp','foreign-fsharp-computation-builder-review')

    def test_fsharp_tuple_and_nested_loop_iteration_ownership(self):
        for source in ('let advance pairs = for first,second in pairs do first.Value<-second.Value',
                'let advance pairs =\n    for first,second in pairs do\n        first.Value<-second.Value'):
            self.assert_units(source,'fsharp',3)
        for source in ('let advance n = for i in 0..n do for j in 0..i do target.Value<-i+j',
                'let advance n =\n    for i in 0..n do\n        for j in 0..i do\n            target.Value<-i+j'):
            self.assert_units(source,'fsharp',4)

    def test_proven_type_field_separators_zero_and_type_tuple_commas_structural(self):
        self.assert_units('Body : {x:F64,y:F64,z:F64}','roc',5)
        self.assert_units('Body : {\n    x:F64,\n    y:F64,\n    z:F64\n}','roc',5)
        self.assert_units('type alias Body = {x:Float,y:Float,z:Float}','elm',5)
        self.assert_units('type alias Body =\n{ x:Float\n, y:Float\n, z:Float\n}','elm',5)
        self.assert_units('body : {x:F64,y:F64,z:F64}','roc',1)
        self.assert_units('Body : {xy:(F64,F64),z:F64}','roc',4)
        self.assert_units('type alias Body = {xy:(Float,Float),z:Float}','elm',4)

    def test_same_indent_else_continuations_keep_own_conditional_scope(self):
        roc_packed='choose = |x|\n    if x>0 then x else\n        a=x+1\n        a'
        roc_expanded='choose = |x|\n    if x>0 then\n        x\n    else\n        a=x+1\n        a'
        self.assert_units(roc_packed,'roc',6)
        self.assert_units(roc_expanded,'roc',6)
        elm_packed='choose x =\n    if x>0 then x else\n        let a=x+1 in a'
        elm_expanded='choose x =\n    if x>0 then\n        x\n    else\n        let\n            a=x+1\n        in\n        a'
        self.assert_units(elm_packed,'elm',7)
        self.assert_units(elm_expanded,'elm',7)

    def test_same_indent_else_if_and_final_else_are_not_new_definitions(self):
        source='choose = |x|\n    if x>1 then\n        a=x+1\n        a\n    else if x>0 then\n        b=x+2\n        b\n    else\n        c=x+3\n        c'
        self.assert_units(source,'roc',10)

    def test_conditional_binding_else_extent_and_independent_outer_locals(self):
        packed='compute = |x|\n    a=x+1\n    sign=if a>0 then 1 else 0\n    a+sign'
        expanded='compute = |x|\n    a=x+1\n    sign=if a>0 then\n        1\n    else\n        0\n    a+sign'
        self.assert_units(packed,'roc',4)
        self.assert_units(expanded,'roc',4)
        self.assert_gate('compute xs = List.map (\\x -> if x>0 then x else 0) xs','elm','foreign-layout-nested-value-composition-review')

    def test_standard_fsharp_seq_computation_exposes_mutation_and_loop(self):
        packed='let values = seq { let mutable x=0; while x<2 do yield x; x<-x+1 }'
        expanded='let values = seq\n{\n    let mutable x=0\n    while x<2 do\n        yield x\n        x<-x+1\n}'
        self.assert_units(packed,'fsharp',7)
        result=self.assert_units(expanded,'fsharp',7)
        self.assertFalse(any(e['kind']=='composed-definition' and e['start']==0 for e in result['canonical_events']))
        for builder in ('async','task'):
            self.assert_units('let value = '+builder+'\n{\n    return 1\n}','fsharp',4)
        self.assert_units('let values x = seq { yield x }','fsharp',4)
        self.assert_gate('let values = custom { let x=1; x }','fsharp','foreign-fsharp-computation-builder-review')

    def test_fsharp_val_fields_have_named_layout_type_owner(self):
        self.assert_units('[<Struct>]\ntype State =\n    val alloc:int; val size:int; val ptr:System.IntPtr','fsharp',5)
        self.assert_units('[<Struct>]\ntype State =\n    val alloc:int\n    val size:int\n    val ptr:System.IntPtr','fsharp',5)
        self.assert_gate('val unknown:int','fsharp','foreign-layout-complex-declaration-review')
        self.assert_gate('type State =\n    member this.compute() =\n        val unknown:int','fsharp','foreign-layout-complex-declaration-review')

    def test_roc_literal_alternative_patterns_are_structural(self):
        self.assert_units('translate = |byte|\n    when byte is\n        65|97 -> 84u8\n        84 | 116 | 85 | 117 -> 65u8\n        _ -> 78u8','roc',1)
        self.assert_units('translate = |byte|\n    when byte is\n        65 | 97 -> 84u8\n        84|116|85|117 -> 65u8\n        _ -> 78u8','roc',1)
        self.assert_gate('translate = |tag|\n    when tag is\n        Ok(x) | Err(x) -> x\n        _ -> 0','roc','foreign-layout-arm-pattern-review')

    def test_roc_scalar_interpolation_calls_members_indices_operators(self):
        for operand in ('value','Num.to_str(whole)','values[index+1]','fixed(100*Num.to_f64(total)/denominator,3)',
                'decode(width,key,[])','mutate()','value |> fixed(9)','left // right','left ?? right'):
            source='text="${'+operand+'}"'
            metric=self.assert_units(source,'roc',1)
            self.assertEqual(len(metric['interpolation_evidence']),1)
            span=metric['interpolation_evidence'][0]
            self.assertEqual(span['classification'],'scalar')
            self.assertEqual(source[span['payload_start']:span['payload_end']],operand)
            self.assertEqual(source[span['start']:span['end']],'${'+operand+'}')

    def test_roc_complete_scalar_conditional_interpolation_packed_expanded(self):
        self.assert_units('text="${if value>0 then fixed(value,9) else fixed(-value,9)}"','roc',1)
        source='text="""\n${if value>0 then\n    fixed(value,9)\nelse\n    fixed(-value,9)}\n"""'
        metric=self.assert_units(source,'roc',6)
        self.assertEqual(metric['interpolation_evidence'][0]['classification'],'scalar')
        self.assert_gate('text="${if value>0 then value}"','roc','foreign-interpolation-expression-review')

    def test_roc_nested_payload_quotes_and_nested_scalar_interpolation(self):
        for source in ('text="${format("x",value)}"', 'text="${format("inner ${value}",value)}"'):
            metric=self.assert_units(source,'roc',1)
            self.assertTrue(all(e['classification']=='scalar' for e in metric['interpolation_evidence']))
            masked=mask_layout(source,'roc')
            self.assertEqual(len(masked['cleaned']),len(source))
            self.assertNotIn('format',masked['cleaned'])
        metric=self.assert_units('text="${format("inner ${value}",value)}"','roc',1)
        self.assertEqual([e['payload'] for e in metric['interpolation_evidence']],['format("inner ${value}",value)','value'])
        self.assert_gate('text="${format("inner ${value=1}",value)}"','roc','foreign-interpolation-expression-review')

    def test_roc_interpolation_preserves_literal_rows_offsets_and_escapes(self):
        source='text="""\n${value}\n"""'
        self.assert_units(source,'roc',3)
        masked=mask_layout(source,'roc')
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'],[i for i,c in enumerate(source) if c=='\n'])
        escaped='text="\\${not_an_interpolation}"'
        metric=self.assert_units(escaped,'roc',1)
        self.assertEqual(metric['interpolation_evidence'],[])
        # The adjacent independent binding stays a separate executable unit.
        self.assert_units('text="${value}"\nother=1','roc',2)

    def test_roc_interpolation_actual_general_string_compositions(self):
        self.assert_units('text="${sign}${Num.to_str(whole)}.${padding}${digits}"','roc',1)
        self.assert_units('Stdout.write!("${fixed(first,9)}\\n${fixed(last,9)}\\n")','roc',1)
        self.assert_units('line="${Num.to_str(iterations)}\\t trees of depth ${Num.to_str(depth)}\\t check: ${Num.to_str(total)}\\n"','roc',1)
        self.assert_units('line="${decode(width,key,[])} ${fixed(100*Num.to_f64(total)/denominator,3)}\\n"','roc',1)

    def test_roc_interpolation_executable_or_unknown_payloads_remain_gated(self):
        for operand in ('a=1','a:=1','a; b','a\nb','|x| x+1','for x in xs do x',
                'when x is A -> 1','return x','{x:1}','call(a=1)','if x then a=1 else 2'):
            self.assert_gate('text="${'+operand+'}"','roc','foreign-interpolation-expression-review')
        for source in ('text="${call(1]}"','text="${value"','text="${call("unfinished)}"'):
            self.assert_gate(source,'roc','foreign-unclosed-literal')

    def test_roc_parenthesized_conditional_operand_packed_expanded(self):
        self.assert_units('total = checksum+(if Num.is_even(index) then current else -current)','roc',1)
        self.assert_units('total = checksum+(if Num.is_even(index) then\n    current\nelse\n    -current)','roc',1)

    def test_elm_parenthesized_conditional_operand_packed_expanded(self):
        self.assert_units('total = checksum+(if modBy 2 index == 0 then current else -current)','elm',1)
        self.assert_units('total = checksum+(if modBy 2 index == 0 then\n    current\nelse\n    -current)','elm',1)

    def test_fsharp_conditional_call_operand_is_one_composition(self):
        self.assert_units('let result = consume (if flag then left else right)','fsharp',1)
        self.assert_units('let result = consume (if flag then\n    left\nelse\n    right)','fsharp',1)

    def test_nested_parenthesized_conditional_values_remain_one_operand(self):
        for language in ('fsharp','roc','elm'):
            self.assertTrue(_scalar('size*(if a then 2 else if b then 3 else 5)',language))
            self.assertTrue(_scalar('size*(if a then (if b then 2 else 3) else 5)',language))

    def test_two_separate_parenthesized_conditional_operands(self):
        for language in ('fsharp','roc','elm'):
            prefix='let ' if language=='fsharp' else ''
            self.assert_units(prefix+'value=(if a then 1 else 2)+(if b then 3 else 4)',language,1)

    def test_following_binding_is_not_absorbed_by_conditional_operand(self):
        for language in ('fsharp','roc','elm'):
            prefix='let ' if language=='fsharp' else ''
            self.assert_units(prefix+'value=(if a then 1 else 2)\n'+prefix+'other=4',language,2)

    def test_incomplete_nested_conditional_has_no_scalar_proof(self):
        for language in ('fsharp','roc','elm'):
            self.assertFalse(_scalar('consume (if a then 1)',language))

    def test_nested_written_bindings_and_assignments_have_no_scalar_proof(self):
        self.assertFalse(_scalar('consume (if a then x<-1 else x<-2)','fsharp'))
        self.assertFalse(_scalar('consume (if a then let x=1 in x else 0)','elm'))
        self.assertFalse(_scalar('consume(if a then x=1 else 0)','roc'))

    def test_nested_wrapped_data_never_becomes_a_scalar_mask(self):
        for language in ('fsharp','roc','elm'):
            self.assertFalse(_scalar('consume(if a then [\n1,2\n] else [])',language))

    def test_proven_local_value_in_computation_discharges_only_outer_gate(self):
        source='let compute x = async {\n    let answer = if x>0 then x else 0\n    return answer\n}'
        row=self.assert_units(source,'fsharp',5)
        answer=source.index('let answer')
        self.assertTrue(any(event['kind']=='composed-definition' and event['start']==answer
                            for event in row['canonical_events']))
        self.assertEqual(sum(event['kind']=='close' for event in row['canonical_events']),1)

    def test_unproved_inline_conditional_owner_keeps_its_specific_gate(self):
        self.assert_gate('value = consume if flag then left else right','elm',
                         'foreign-layout-nested-value-composition-review')

    def test_executable_lambda_region_alone_does_not_discharge_value_owner_gate(self):
        source=('compute = |xs|\n    List.walk(xs,0,|total,x|\n'
                '        a = if x>0 then x else 0\n        total+a,\n    )')
        self.assert_gate(source,'roc','foreign-layout-nested-value-composition-review')


if __name__=='__main__':
    unittest.main()
