"""Unhooked proposals; execute only in a frozen common-mutex fixture job."""
import unittest
from zig_odin import normalize,mask_odin,mask_zig


class ZigOdinFixtures(unittest.TestCase):
    def count(self,source,language,expected,status='candidate-parser-pass'):
        row=normalize(source,language)
        self.assertEqual(row['normalized_loc'],expected,row)
        self.assertEqual(row['review_status'],status,row)
        return row

    def equivalent(self,packed,expanded,language,expected):
        self.count(packed,language,expected);self.count(expanded,language,expected)

    def test_named_function_headers_and_bodies(self):
        self.equivalent('pub fn f(x:u64) u64 { const y=x+1; return y; }',
            'pub fn f(x:u64)\n u64\n{\n const y=x+1;\n return y;\n}','zig',4)
        self.equivalent('f :: proc(x:u64) -> u64 { y:=x+1; return y }',
            'f :: proc(x:u64)\n -> u64\n{\n y:=x+1\n return y\n}','odin',4)

    def test_type_fields_vs_formals(self):
        self.equivalent('const Node=struct {left:?*Node,right:?*Node};',
            'const Node=struct\n{\n left:?*Node,\n right:?*Node,\n};','zig',4)
        self.equivalent('Node :: struct {left,right:^Node}',
            'Node :: struct\n{\n left:^Node\n right:^Node\n}','odin',4)
        self.count('batch :: proc(depth,iterations:int) -> int {return depth}','odin',3)

    def test_data_constructors_do_not_create_executable_close(self):
        for language,source in (('zig','const x=Node{.left=null,.right=null};'),
            ('zig','spawn(.{},worker,.{&tasks[i]});'),
            ('odin','x:=Body{[3]f64{1,2,3},[3]f64{4,5,6}}')):
            self.count(source,language,1)

    def test_control_headers_protect_iteration_work(self):
        self.equivalent('while(i<n):(i+=1){a+=i;}',
            'while(i<n):(i+=1)\n{\n a+=i;\n}','zig',3)
        self.equivalent('for(s)|c|{a+=c;}',
            'for(s)|c|\n{\n a+=c;\n}','zig',3)
        self.equivalent('for i:=0;i<n;i+=1{a+=i}',
            'for i:=0;i<n;i+=1\n{\n a+=i\n}','odin',3)

    def test_packed_branches(self):
        self.count('if x{a=1}else{a=2}','odin',6)
        self.count('if(x){a=1;}else{a=2;}','zig',6)
        self.count('if(got==0) break;','zig',2)

    def test_odin_multiple_binding_vs_multiple_return(self):
        self.count('x,y:=1,2','odin',2)
        self.count('n,err:=read(input)','odin',1)
        self.count('x,y:int','odin',2)
        self.count('x,y:=0','odin',2,'needs-review')

    def test_odin_switch_arm_headers_and_bodies(self):
        self.equivalent('switch c{case 0,1:return 2;case:return 3}',
            'switch c\n{\n case 0,1:\n return 2\n case:\n return 3\n}','odin',6)

    def test_value_switch_is_gated_not_arrow_assignment(self):
        row=normalize('return switch(c){0=>1,else=>2};','zig')
        self.assertEqual(row['review_status'],'needs-review')
        self.assertTrue(any(d['kind']=='zig-value-control-review' for d in row['diagnostics']))
        self.assertFalse(any(a['operator']=='=' for event in row['expansions']
            for statement in event['events'] for a in statement['assignments']))

    def test_odin_programming_literal_mask_and_nested_comments(self):
        source='/* a /* b */ c */\nx:=`/* ;:= {} */`\ny:="; := {}" // comment'
        mask=mask_odin(source)
        self.assertEqual(len(mask['cleaned']),len(source))
        self.assertEqual(mask['diagnostics'],[])
        self.count(source,'odin',2)

    def test_meaningful_odin_annotation_is_separate(self):
        self.equivalent('@(optimization_mode="speed") f :: proc(){return}',
            '@(optimization_mode="speed")\nf :: proc()\n{\n return\n}','odin',4)

    def test_zig_builtin_return_type_owns_complete_function_signature(self):
        self.equivalent('inline fn scale(v:anytype,f:f64) @TypeOf(v){const y=v;return y;}',
            'inline fn scale(v:anytype,f:f64)\n @TypeOf(v)\n{\n const y=v;\n return y;\n}','zig',4)
        self.equivalent('fn vector(v:anytype) @Vector(4,f64){return v;}',
            'fn vector(v:anytype)\n @Vector(4,f64)\n{\n return v;\n}','zig',3)
        row=normalize('fn unknown() @Custom(){a=1;b=2;}','zig')
        self.assertTrue(any(d['kind']=='zig-unrecognized-brace-header-review' for d in row['diagnostics']))

    def test_odin_call_tuple_producer_accepts_nested_data_arguments(self):
        self.count('iterator,err:=create(input,pattern,{.Case_Insensitive})','odin',1)
        self.count('iterator,err:=create(input,Options{flags={.Case_Insensitive}})','odin',1)
        self.count('iterator,err:=create(input=input,pattern=pattern,flags={.Case_Insensitive})','odin',1)
        self.count('iterator,err:=create(flags={.Case_Insensitive})','odin',1)
        self.count('iterator,err:=create("proc; if; return",{.Case_Insensitive})','odin',1)
        self.count('first,second:=1,2','odin',2)
        self.count('first,second:=create((flags={.Case_Insensitive}))','odin',2,'needs-review')
        self.count('first,second:=create(flags=other={.Case_Insensitive})','odin',2,'needs-review')

    def test_odin_anonymous_executable_argument_is_not_tuple_data_proof(self):
        row=normalize('first,second:=create(proc(){a:=1;b:=2;})','odin')
        self.assertEqual(row['review_status'],'needs-review')
        self.assertTrue(any(d['kind']=='odin-multiple-binding-producer-review' for d in row['diagnostics']))

    def test_zig_switch_pattern_alternatives_and_transfer_bodies(self):
        self.equivalent('switch(c){0,1=>return 2,else=>return 3,}',
            'switch(c)\n{\n0,1=>\n return 2,\nelse=>\n return 3,\n}','zig',6)

    def test_zig_switch_explicit_arm_blocks_have_own_executable_close(self):
        for source in ('switch(c){0=>{a=1;b=2;},else=>{a=3;},}',
                'switch(c)\n{\n0=>\n{\na=1;\nb=2;\n},\nelse=>\n{\na=3;\n},\n}'):
            row=self.count(source,'zig',9)
            self.assertEqual(sum(e['kind']=='branch' and e['scope']=='zig-switch-arm' for e in row['canonical_events']),2)
            self.assertFalse(any(e['kind']=='branch' and e['scope']=='braced' for e in row['canonical_events']))

    def test_zig_catch_switch_transfers_are_proven_imperative_owners(self):
        self.equivalent('const n=read() catch |err| switch(err){error.End=>break,else=>return err};',
            'const n=read() catch |err|\n switch(err)\n {\nerror.End=>break,\nelse=>return err,\n };','zig',7)

    def test_zig_switch_simple_capture_and_data_constructors_are_structural(self):
        self.count('switch(c){.Item=>|value|consume(value),else=>consume(0),}','zig',6)
        self.count('switch(c){inline else=>|value,tag|consume(value,tag),}','zig',4)
        self.count('switch(c){0=>consume(.{1,2}),else=>consume(.{3,4}),}','zig',6)
        value=normalize('const n=switch(c){0=>1,else=>2};','zig')
        self.assertTrue(any(d['kind']=='zig-value-control-review' for d in value['diagnostics']))

    def test_zig_scalar_if_binding_packed_expanded_is_one_composition(self):
        self.equivalent('const used=if(produced%10==0) 10 else produced%10;',
            'const used=\n if(produced%10==0)\n  10\n else\n  produced%10;','zig',1)
        self.equivalent('var idx=if(transpose) baseIdx(ti-vec1to4(1)) else baseIdx(ti)+ti;',
            'var idx=\n if(transpose)\n  baseIdx(ti-vec1to4(1))\n else\n  baseIdx(ti)+ti;','zig',1)

    def test_zig_scalar_if_is_owned_with_surrounding_return_arithmetic(self):
        packed='fn check(node:*const Node) usize{return 1+(if(node.left)|left|check(left)else 0)+(if(node.right)|right|check(right)else 0);}'
        expanded='fn check(node:*const Node) usize\n{\n return 1+\n  (if(node.left) |left|\n   check(left)\n  else\n   0)+\n  (if(node.right) |right|\n   check(right)\n  else\n   0);\n}'
        self.equivalent(packed,expanded,'zig',3)
        row=self.count(packed,'zig',3)
        self.assertEqual(len(row['value_evidence']),1)
        self.assertEqual(len(row['value_evidence'][0]['ifs']),2)
        self.assertEqual(len(row['value_evidence'][0]['captures']),2)

    def test_zig_conditional_optional_and_error_captures_are_formals(self):
        self.equivalent('const count=if(map.get(Code.fromStr(s))) |x| x else 0;',
            'const count=\n if(map.get(Code.fromStr(s))) |x|\n  x\n else\n  0;','zig',1)
        self.equivalent('return if(result) |value| value else |err| recover(err);',
            'return if(result)\n |value| value\n else\n |err| recover(err);','zig',1)
        self.count('const x=if(optional) |*value| value.* else null;','zig',1)

    def test_zig_scalar_if_call_index_member_and_cast_arms(self):
        self.count('const x=if(c) @as(u8,@intCast(values[index])) else item.field;','zig',1)
        self.count('const x=if(c) values[lo..hi] else values[..];','zig',1)
        self.equivalent('consume(if(c) produce() else recover());',
            'consume(\n if(c)\n  produce()\n else\n  recover()\n);','zig',1)
        # Calls remain source expressions; no name-based purity whitelist.
        self.count('const x=if(c) mutate() else unknownCall();','zig',1)

    def test_zig_scalar_if_typed_and_anonymous_constructors_are_data(self):
        row=self.count('var ti=vec1to4(first)+if(transpose) vec4{1,2,3,4} else vec4{0,1,2,3};','zig',1)
        self.assertEqual(len(row['value_evidence'][0]['data']),2)
        self.assertFalse(any(e['kind']=='close' for e in row['canonical_events']))
        self.count('const value=if(c) .{.x=first,.y=second} else .{.x=second,.y=first};','zig',1)
        self.count('const value=if(c) .{&first,&second} else .{&second,&first};','zig',1)
        self.count('const value=if(c) [_]u8{1,2} else [2]u8{3,4};','zig',1)
        self.count('const value=if(c) @as([]const u8,input) else @as([*]const u8,pointer);','zig',1)

    def test_zig_scalar_if_keeps_written_constructor_rows(self):
        source='const value=if(c) .{\n .x=1,\n .y=2,\n} else .{\n .x=3,\n .y=4,\n};'
        row=self.count(source,'zig',7)
        self.assertEqual(row['value_evidence'][0]['preserved_data_rows'],list(range(1,8)))
        self.assertEqual(row['canonical_events'][0]['outer_value_units'],1)
        self.count('const value=\n if(c)\n  .{\n   .x=1,\n  }\n else\n  .{\n   .x=2,\n  };','zig',7)

    def test_zig_scalar_if_does_not_consume_following_function_close_or_binding(self):
        source='fn f() u8{const x=if(c) 1 else 2;const y=x+1;return y;} fn g() u8{return 3;}'
        row=self.count(source,'zig',8)
        self.assertEqual(sum(e['kind']=='close' for e in row['canonical_events']),2)
        self.assertEqual(len(row['value_evidence']),1)
        span=row['value_evidence'][0]
        self.assertEqual(source[span['start']:span['expression_end']],'const x=if(c) 1 else 2')
        self.assertEqual(source[span['start']:span['end']],'const x=if(c) 1 else 2;')

    def test_zig_nested_scalar_if_complete_branches_remain_one_value(self):
        self.equivalent('return if(a) if(b) f() else g() else h();',
            'return\n if(a)\n  if(b) f()\n  else g()\n else h();','zig',1)
        self.count('const value=if(a and b) left orelse fallback else right catch recover();','zig',1)
        row=self.count('const value=if(a) .{.x=if(b) 1 else 2} else .{.x=0};','zig',1)
        self.assertEqual(len(row['value_evidence'][0]['ifs']),2)

    def test_zig_statement_if_and_imperative_scalar_arms_are_not_collapsed(self):
        self.equivalent('if(c){a=1;}else{a=2;}',
            'if(c)\n{\n a=1;\n}\nelse\n{\n a=2;\n}','zig',6)
        for source in ('const x=if(c){a=1;b=2;}else{a=3;};',
                'return if(c) block:{break :block 1;} else 2;',
                'const x=if(c) a+=1 else b;', 'consume(if(c) a=1 else b);',
                'const x=if(c) for(xs)|x|consume(x) else 0;',
                'const x=if(c) fn()void{a=1;} else 0;',
                'const x=if(c) return 1 else 2;', 'const x=if(c) 1;',
                'format(if(c) a=1 else b);','if(c) doWork() else doOtherWork();'):
            row=normalize(source,'zig')
            self.assertEqual(row['review_status'],'needs-review',row)
            self.assertEqual(row['value_evidence'],[])

    def test_zig_actual_inline_loop_statement_ifs_stay_gated(self):
        for source in ('inline while(i<=max_n):(i+=1) if(i==n) return @shuffle(u8,a,undefined,mask(i));',
                'inline while(i<=max_n):(i+=1) if(i==n) return comptime factorial(i);',
                'if(m.get(code)) |count| total+=count;'):
            row=normalize(source,'zig')
            self.assertEqual(row['review_status'],'needs-review',row)
            self.assertEqual(row['value_evidence'],[])

    def test_zig_switch_policy_gates_and_imperative_arms_are_preserved(self):
        for source in ('return switch(c){0=>1,else=>unreachable};',
                'const c:u8=switch(c){encode(0)=>1,else=>2};'):
            row=normalize(source,'zig')
            self.assertTrue(any(d['kind']=='zig-value-control-review' for d in row['diagnostics']))
            self.assertEqual(row['value_evidence'],[])
        self.count('switch(c){0=>{a=1;b=2;},else=>{a=3;},}','zig',9)

    def test_zig_unknown_conditional_capture_and_mixed_data_code_stay_gated(self):
        for source in ('const x=if(c)|first,second|first else 0;',
                'const x=if(c) .{.x=1;} else .{};', 'const x=if(c) 1 else {doWork();};'):
            row=normalize(source,'zig')
            self.assertEqual(row['review_status'],'needs-review',row)
            self.assertEqual(row['value_evidence'],[])

    def test_zig_scalar_value_after_executable_close_has_its_own_window(self):
        row=self.count('fn f()void{} const x=if(c) 1 else 2;','zig',3)
        self.assertEqual(len(row['value_evidence']),1)
        self.count('fn f()void{if(c){work();}const x=if(d) 1 else 2;}','zig',6)
        # A preceding constructor close stays within the same value window.
        self.count('const x=.{1,2}++if(c) .{3,4} else .{5,6};','zig',1)

    def test_zig_scalar_values_keep_exact_literal_rows(self):
        self.count('const x=if(c) "a" ++ "b" else "c";','zig',1)
        row=self.count('const x=if(c) "a" ++\n "b" else "c";','zig',2)
        self.assertEqual(row['value_evidence'][0]['preserved_data_rows'],[1,2])
        self.count('const x=if(c) format("a",\n "b") else format("c");','zig',2)
        self.count('const x=if(c) S++\n S else S;','zig',1)
        source='const text=if(c) fallback else\n \\\\first\n \\\\second\n;'
        row=self.count(source,'zig',3)
        self.assertEqual(row['value_evidence'][0]['preserved_data_rows'],[2,3])
        self.assertEqual(row['value_evidence'][0]['literals'][0]['role'],'multiline-literal')

    def test_zig_literal_evidence_uses_raw_syntax_and_preserves_offsets(self):
        source='// "not a literal"\nconst x=if(c) "a \\" b" else \'x\'; const @"name"=1;'
        masked=mask_zig(source)
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'],[i for i,c in enumerate(source) if c=='\n'])
        self.assertEqual([source[s['start']:s['end']] for s in masked['literals']],['"a \\" b"',"'x'"])
        # This pinned Pygments Zig lexer has no quoted-identifier rule. Keep
        # its @ error gate rather than treating the name as string evidence.
        self.assertIn('foreign-lexer-error',{issue['kind'] for issue in masked['diagnostics']})
        self.count(source,'zig',2,'needs-review')

    def test_zig_value_terminator_on_its_own_line_is_structural(self):
        self.count('const x=\n if(c)\n  1\n else\n  2\n;','zig',1)
        row=self.count('fn f()u8{\n return if(c) 1 else 2\n ;\n}\n','zig',3)
        self.assertEqual(sum(e['kind']=='close' for e in row['canonical_events']),1)
