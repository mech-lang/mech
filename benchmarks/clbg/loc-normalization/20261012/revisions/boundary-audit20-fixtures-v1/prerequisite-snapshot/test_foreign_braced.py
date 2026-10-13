"""Unexecuted fixtures for the unhooked C#/Dart proposal.

Run only in a reviewed common-mutex job. This module has not been imported or
executed while preparing the proposal; fixtures are expectations for review,
not a receipt. No benchmark source/report is read, mutated, or certified here.
"""
import unittest

from foreign_braced import classify_scopes, mask_csharp, mask_dart, normalize


class ForeignBracedProposalFixtures(unittest.TestCase):
    def assert_units(self, source, language, expected):
        metric = normalize(source,language)
        self.assertEqual(metric['diagnostics'],[],metric['diagnostics'])
        self.assertEqual(metric['normalized_loc'],expected)
        self.assertTrue(metric['proposal_only'])
        self.assertEqual(metric['proposal_status'],'unhooked-no-admission')
        return metric

    def assert_gate(self, source, language, kind):
        metric = normalize(source,language)
        self.assertIn(kind,{issue['kind'] for issue in metric['diagnostics']})
        self.assertTrue(metric['proposal_only'])
        return metric

    def test_csharp_uninitialized_fields_packed_expanded(self):
        packed = 'class Tree { private Tree left, right; }'
        expanded = 'class Tree\n{\nprivate Tree left;\nprivate Tree right;\n}'
        self.assert_units(packed,'csharp',4)
        self.assert_units(expanded,'csharp',4)

    def test_dart_uninitialized_seven_fields(self):
        packed = 'class Body { double x,y,z,vx,vy,vz,mass; }'
        expanded = 'class Body\n{\n'+'\n'.join('double '+name+';' for name in ('x','y','z','vx','vy','vz','mass'))+'\n}'
        self.assert_units(packed,'dart',9)
        self.assert_units(expanded,'dart',9)

    def test_chains_and_uninitialized_declarators_add(self):
        for language in ('csharp','dart'):
            self.assert_units('x=y=z=0;',language,3)
            self.assert_units('void F(){int a=b=0,c;} ',language,5)
            self.assert_units('void F()\n{\nint a=b=0;\nint c;\n}',language,5)

    def test_no_extra_own_line_executable_open(self):
        for language in ('csharp','dart'):
            self.assert_units('void F(){x=1;y=2;}',language,4)
            metric=self.assert_units('void F()\n{\nx=1;\ny=2;\n}',language,4)
            self.assertTrue(any(e['kind']=='open' and e['units']==0 for e in metric['canonical_events']))

    def test_for_header_parameters_do_not_become_bindings(self):
        for language in ('csharp','dart'):
            self.assert_units('void F(int x,int y){for(int i=0;i<2;i++){x+=y;}}',language,5)

    def test_inline_unbraced_controls(self):
        for language in ('csharp','dart'):
            self.assert_units('void F(){if(x) a=1;else a=2;}',language,6)

    def test_explicit_csharp_block_callback(self):
        metric=self.assert_units('Use((x)=>{return x;});','csharp',3)
        self.assertTrue(any(e['scope']=='foreign-callback' for e in metric['canonical_events']))
        self.assert_units('Use((x)=>\n{\nreturn x;\n});','csharp',3)
        self.assert_units('Use((x)=>{a=1;b=2;});','csharp',4)

    def test_explicit_dart_block_callback(self):
        self.assert_units('use((x){return x;});','dart',3)
        self.assert_units('use((x)\n{\nreturn x;\n});','dart',3)

    def test_expression_lambdas_are_expression_forms(self):
        self.assert_units('var f=(x)=>x+1;','csharp',1)
        self.assert_units('final f=(x)=>x+1;','dart',1)

    def test_named_args_tuple_destructuring_and_values(self):
        self.assert_units('var (x,y)=(1,2);','csharp',1)
        self.assert_units('final (x,y)=(1,2);','dart',1)
        self.assert_units('var x=Make(first:1,second:2);','csharp',1)
        self.assert_units('final x=make(first:1,second:2);','dart',1)

    def test_formal_defaults_in_prototypes_are_not_body_assignments(self):
        self.assert_units('abstract class C { public abstract int F(int x=1,int y=2); }','csharp',3)
        self.assert_units('class C { C({int x=1,int y=2}); }','dart',3)

    def test_explicit_csharp_attribute_groups_are_independent_units(self):
        self.assert_units('[MethodImpl(Options.Fast)] static void F(){x=1;}','csharp',4)
        self.assert_units('[MethodImpl(Options.Fast)]\nstatic void F()\n{\nx=1;\n}','csharp',4)
        self.assert_units('[A][B(Value=1,Other=2)] public abstract int F(int x=1,int y=2);','csharp',3)
        self.assert_units('[A]\n[B(Value=1,Other=2)]\npublic abstract int F(int x=1,int y=2);','csharp',3)
        self.assert_units('[A,B(1,2)] class C{int x,y;}','csharp',5)
        self.assert_units('[assembly: Version("1.0")]','csharp',1)
        self.assert_units('x=items[index];','csharp',1)
        self.assert_gate('[Unknown<T>] void F(){}','csharp','foreign-csharp-attribute-policy-review')
        self.assert_gate('void F([In] int x){}','csharp','foreign-csharp-formal-attribute-review')

    def test_generic_declared_type_commas_are_structural(self):
        self.assert_units('Dictionary<int,List<string>> first,second;','csharp',2)
        self.assert_units('Map<int,List<String>> first,second;','dart',2)

    def test_generic_constructor_commas_are_type_structure(self):
        self.assert_units('Dictionary<int,List<string>> first=new Dictionary<int,List<string>>(),second;','csharp',2)
        self.assert_units('Dictionary<int,List<string>> first=new Dictionary<int,List<string>>();\nDictionary<int,List<string>> second;','csharp',2)
        self.assert_units('HashMap<String,int> first=new HashMap<String,int>(),second;','dart',2)
        self.assert_units('HashMap<String,int> first=new HashMap<String,int>();\nHashMap<String,int> second;','dart',2)

    def test_generic_method_type_arguments_are_callee_agnostic(self):
        self.assert_units('var value=Unsafe.As<Vector128<double>,double>(ref sum);','csharp',1)
        self.assert_units('var value=Other.Convert<Pair<int,int>,string>(input);','csharp',1)
        self.assert_units('final value=other.convert<Map<int,String>,List<double>>(input);','dart',1)

    def test_generic_new_array_type_owner_is_exact(self):
        self.assert_units('var tasks=new Task<int>[count];','csharp',1)
        self.assert_units('Vector128<byte>[] masks=new Vector128<byte>[16],other;','csharp',2)
        self.assert_gate('var x=new Unknown<int + value,string>();','csharp','foreign-generic-initializer-review')
        self.assert_gate('var x=new Unknown<List<int + value>,string>();','csharp','foreign-generic-initializer-review')
        self.assert_gate('var x=Make<int,string> + y;','csharp','foreign-complex-declaration-review')

    def test_generic_initializers_chains_and_bare_bindings_add(self):
        packed='Dictionary<int,int> first=new Dictionary<int,int>(),second=third=null,fourth;'
        expanded='Dictionary<int,int> first=new Dictionary<int,int>();\nDictionary<int,int> second=third=null;\nDictionary<int,int> fourth;'
        self.assert_units(packed,'csharp',4)
        self.assert_units(expanded,'csharp',4)

    def test_wrapped_declaration_has_whole_statement_owner(self):
        self.assert_units('var u=new Float64List(n)..fillRange(0,n,1.0),\n v=new Float64List(n),\n w=new Float64List(n),\n vBv=0.0,\n vv=0.0;','dart',5)
        self.assert_units('var u=new Float64List(n)..fillRange(0,n,1.0),v=new Float64List(n),w=new Float64List(n),vBv=0.0,vv=0.0;','dart',5)
        self.assert_units('Dictionary<int,int> first=new Dictionary<int,int>(),\n second,third=0;','csharp',3)
        self.assert_gate('var first=Make(),\n unresolved + value;','dart','foreign-complex-declaration-review')

    def test_wrapped_argument_literals_do_not_become_declarations(self):
        self.assert_units('Console.WriteLine(string.Concat("a",\n "b",n));','csharp',2)
        self.assert_units("use(Request(\n 'header',\n 'data',[],n));",'dart',3)
        self.assert_units('Use(\n "a" "b" "c",\n next);','csharp',3)
        self.assert_units('Use<int,string>(\n "a" "b" "c",\n next);','csharp',3)

    def test_argument_context_does_not_hide_independent_assignments(self):
        for language in ('csharp','dart'):
            self.assert_units('Use(\n first:x=y=0,\n second:1);',language,4)

    def test_scalar_expression_methods_own_formal_commas(self):
        self.assert_units('internal static Vector128<byte> AndNot(Vector128<byte> a,Vector128<byte> b)=>~a&b;','csharp',1)
        self.assert_units('private static V128d SumLanes(this V256d x)=>Vector128.Add(x.GetLower(),x.GetUpper());','csharp',1)
        self.assert_units('[A]\nstatic int F(int x=1,int y=2)=>Mutate(x,y);','csharp',2)
        self.assert_units('int f(int x,int y)=>mutate(x,y);','dart',1)
        self.assert_gate('int F()=>x=y=0;','csharp','foreign-expression-method-body-review')

    def test_multiline_extern_prototype_is_one_header_with_separate_attribute(self):
        packed='[DllImport("lib",EntryPoint="compile")] extern static IntPtr Compile(string pattern,long length,uint options,out int error,out long offset,IntPtr context);'
        expanded='[DllImport("lib",EntryPoint="compile")]\nextern static IntPtr Compile(string pattern,long length,uint options,\n out int error,out long offset,IntPtr context);'
        self.assert_units(packed,'csharp',2)
        metric=self.assert_units(expanded,'csharp',2)
        self.assertEqual([event['scope'] for event in metric['canonical_events']],['foreign-csharp-attribute','foreign-method-prototype'])
        self.assertEqual(metric['method_statement_owners'][0]['header_units'],1)
        self.assertEqual(metric['method_statement_owners'][0]['written_close_units'],0)

    def test_prototype_pointer_generic_and_default_formals_stay_structural(self):
        packed='class Pcre{[DllImport("lib")] extern unsafe static int Match(IntPtr code,byte* subject,Dictionary<int,string> values,int options=0);}'
        expanded='class Pcre\n{\n[DllImport("lib")]\nextern unsafe static int Match(IntPtr code,byte* subject,\n Dictionary<int,string> values,\n int options=0);\n}'
        self.assert_units(packed,'csharp',4)
        self.assert_units(expanded,'csharp',4)
        self.assert_units('external static int f(int x,\n int y);','dart',1)
        self.assert_gate('extern static int F([In] int x,\n int y);','csharp','foreign-csharp-formal-attribute-review')

    def test_prototype_owner_does_not_consume_wrapped_call_argument_work(self):
        self.assert_units('Use(x=1,\n y=2);','csharp',2)
        self.assert_units('use(x=1,\n y=2);','dart',2)
        self.assert_gate('Use((x)=>{return x;},another);','csharp','foreign-callback-surrounding-expression-review')

    def test_assignment_arrow_has_header_and_body_without_imputed_close(self):
        pure='static uint Rand()=>(seed*IA+IC)%IM;'
        packed='static uint Rand()=>seed=(seed*IA+IC)%IM;'
        expanded='static uint Rand()=>\n seed=(seed*IA+IC)%IM;'
        self.assert_units(pure,'csharp',1)
        self.assert_units(packed,'csharp',2)
        metric=self.assert_units(expanded,'csharp',2)
        owner=metric['method_statement_owners'][0]
        self.assertEqual((owner['header_units'],owner['body_assignment_units'],owner['written_close_units']),(1,1,0))
        self.assertFalse(owner['body_scope_review'])
        self.assertFalse(any(event['kind']=='close' for event in metric['canonical_events']))
        self.assert_units('static int F(\n int x,int y)=>\n seed=(seed+x+y)%17;','csharp',2)
        self.assert_units('int next()=>seed=(seed*multiplier+increment)%limit;','dart',2)

    def test_chained_arrow_exposes_units_but_retains_exact_body_scope_gate(self):
        metric=self.assert_gate('int F()=>x=y=z=0;','csharp','foreign-expression-method-body-review')
        self.assertEqual(metric['normalized_loc'],4)
        owner=metric['method_statement_owners'][0]
        self.assertEqual((owner['header_units'],owner['body_assignment_units'],owner['written_close_units']),(1,3,0))
        self.assertTrue(owner['body_scope_review'])
        for source in ('int F()=>seed=Use(other=1);','int F()=>seed=(other=1);','int F()=>seed=1,other=2;'):
            self.assert_gate(source,'csharp','foreign-expression-method-body-review')

    def test_tuple_return_proves_method_and_following_type_boundaries(self):
        packed='static (byte[] buf,long len) Read(Stream stream){return Get();} class Pcre{int x,y;}'
        expanded='static (byte[] buf,long len) Read(Stream stream)\n{\nreturn Get();\n}\nclass Pcre\n{\nint x;\nint y;\n}'
        self.assert_units(packed,'csharp',7)
        self.assert_units(expanded,'csharp',7)
        self.assert_gate('static (Unknown buf,long len) Read(Stream stream){return Get();}','csharp','foreign-ambiguous-brace-review')

    def test_tuple_type_binding_groups_are_independent(self):
        self.assert_units('(int x,int y) first,second;','csharp',2)
        self.assert_units('(int x,int y) first;\n(int x,int y) second;','csharp',2)
        self.assert_units('((int x,int y) pair,double z) first,second;','csharp',2)
        self.assert_gate('(Unknown x,int y) first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('(x,y) first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('(Unknown x,int y) F()=>Get();','csharp','foreign-complex-type-declaration-review')

    def test_tuple_alias_components_are_type_structure_once(self):
        packed='using Body=((double X,double Y,double Z) P,(double X,double Y,double Z) V,double M);'
        expanded='using Body=(\n (double X,double Y,double Z) P,\n (double X,double Y,double Z) V,double M);'
        self.assert_units(packed,'csharp',1)
        metric=self.assert_units(expanded,'csharp',1)
        self.assertEqual(metric['canonical_events'][0]['scope'],'foreign-csharp-type-alias')
        self.assertEqual(metric['source_type_owners'][0]['name'],'Body')

    def test_named_and_namespace_alias_have_explicit_owner_lifetime(self):
        self.assert_units('using Word=int;\nunsafe void F(){Word* first,second;}','csharp',5)
        self.assert_units('namespace N{using Body=(double x,double y);unsafe void F(){Body* first,second;}}','csharp',7)
        self.assert_gate('namespace N{using Body=(double x,double y);} Body* first,second;','csharp','foreign-complex-type-declaration-review')

    def test_alias_unknown_owners_and_duplicates_keep_gates(self):
        self.assert_gate('void F(){using Body=(double x,double y);}','csharp','foreign-csharp-type-alias-owner-review')
        self.assert_gate('class C{using Body=(double x,double y);}','csharp','foreign-csharp-type-alias-owner-review')
        self.assert_gate('using Body=(1,2);','csharp','foreign-csharp-type-alias-owner-review')
        self.assert_gate('using Body=int;using Body=double;','csharp','foreign-csharp-type-alias-shadow-review')

    def test_builtin_pointer_declarators_are_independent(self):
        self.assert_units('unsafe void F(){int* first,second;}','csharp',4)
        self.assert_units('unsafe void F()\n{\nint* first;\nint* second;\n}','csharp',4)
        self.assert_units('char* builder=stackalloc char[count];','csharp',1)

    def test_source_type_pointer_declaration_needs_visible_owner(self):
        self.assert_units('struct Body{double x,y;} unsafe void F(){Body* first,second;}','csharp',8)
        self.assert_units('using Body=(double x,double y); unsafe void F(){Body* first=Make(),second;}','csharp',5)
        self.assert_gate('Body* first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('int X=1;X* first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('External.Body* first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('class Outer{class Body{}} Body* first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('namespace N{} N* first,second;','csharp','foreign-complex-type-declaration-review')

    def test_pointer_alias_value_and_formal_shadows_are_gated(self):
        self.assert_gate('using Body=(double x,double y);void F(){int Body=1;Body* first,second;}','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('using Body=(double x,double y);void F(int Body){Body* first,second;}','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('using Body=(double x,double y);void F(Body* Body){Body* first,second;}','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('using Body=(double x,double y);class C{void F(){Body* first,second;}int Body;}','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('delegate*<int,void> first,second;','csharp','foreign-complex-type-declaration-review')

    def test_default_values_do_not_acquire_switch_label_gates(self):
        self.assert_units('mpz_t first=default,second=default;','csharp',2)
        self.assert_units('mpz_t first=default;\nmpz_t second=default;','csharp',2)
        self.assert_units('var first=default(int);','csharp',1)
        self.assert_gate('switch(x){default:break;}','csharp','foreign-pattern-switch-record-review')

    def test_actual_callback_argument_and_chain_surrounds_stay_gated(self):
        self.assert_gate('Parallel.For(1,n,()=>0,(x)=>{return x;},x=>Add(x));','csharp','foreign-callback-surrounding-expression-review')
        self.assert_gate('return stream.forEach((x){add(x);}).then((x){return x;});','dart','foreign-callback-surrounding-expression-review')

    def test_object_initializers_are_data(self):
        metric=self.assert_units('var p=new Point { X=1,Y=2 };','csharp',1)
        self.assertEqual(metric['canonical_events'],[])
        self.assert_units('var p=new Point() { X=1,Y=2 };','csharp',1)

    def test_dart_arrow_returns_map_not_executable_block(self):
        metric=self.assert_units("final f=(x)=>{'x':x,'y':2};",'dart',1)
        self.assertEqual(metric['canonical_events'],[])

    def test_dart_named_default_parameter_braces_are_formal_data(self):
        self.assert_units('void F({int x=1,int y=2}){use(x,y);}','dart',3)

    def test_data_wrapping_is_retained(self):
        self.assert_units('final values=[\n1,2,\n3,4\n];','dart',4)

    def test_csharp_comments_char_and_verbatim_keep_offsets(self):
        source='// { x=1; }\nvar text=@"a; ""quote"" // }";\nchar c=\'}\';'
        masked=mask_csharp(source)
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'],[i for i,c in enumerate(source) if c=='\n'])
        self.assertEqual(masked['diagnostics'],[])
        self.assertNotIn('{',masked['cleaned'])
        self.assertNotIn('}',masked['cleaned'])
        self.assert_units(source,'csharp',2)

    def test_dart_nested_comments_raw_and_triple_strings(self):
        source="/* outer /* nested */ still outer */\nfinal a=r'${not_code};';\nfinal b='''one; {\ntwo; }''';"
        masked=mask_dart(source)
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual(masked['diagnostics'],[])
        self.assertNotIn('{',masked['cleaned'])
        self.assertNotIn('}',masked['cleaned'])
        self.assert_units(source,'dart',3)

    def test_interpolation_is_not_silently_hidden(self):
        self.assert_gate('var s=$"{x=1}";','csharp','foreign-interpolation-expression-review')
        self.assert_gate("final s='${x=1}';",'dart','foreign-interpolation-expression-review')
        self.assert_gate('var s=$"{first();second()}";','csharp','foreign-interpolation-expression-review')
        self.assert_gate("final s='${((){x=1;return x;})()}';",'dart','foreign-interpolation-expression-review')

    def test_csharp_scalar_interpolation_does_not_guess_callee_purity(self):
        for source in ('var s=$"{Mutate()}";','var s=$"{GetErrorMessage(ret)} {table[index]}";',
                       'Console.WriteLine($"{magicTask.Result} {flag ? left : right}");'):
            self.assert_units(source,'csharp',1)
        self.assert_units('throw new ArgumentException($@"Error compiling ""{pattern}"": {GetErrorMessage(ret)}");','csharp',1)

    def test_dart_unbraced_variables_and_scalar_calls_members_indices(self):
        for source in ("print('$key $count');","final s='${mutate()}';",
                       "return '${(freq[string] ?? 0)}\\t$string';",
                       "print('$each ${regex.allMatches(s).length}');"):
            self.assert_units(source,'dart',1)

    def test_interpolation_format_arguments_literal_braces_and_literal_arguments_structural(self):
        self.assert_units('var s=$"{{literal}} {value,12:F3} {call(\"a;=b\")}";','csharp',1)
        self.assert_units("final s='\\$escaped ${call(\"a;=b\")}';",'dart',1)
        self.assert_units('var s=$"plain dollar prefix without expressions";','csharp',1)

    def test_interpolation_executable_callbacks_control_and_updates_are_gated(self):
        for source in ('var s=$"{()=>Run()}";','var s=$"{if(x){Run();}}";',
                       'var s=$"{x++}";'):
            self.assert_gate(source,'csharp','foreign-interpolation-expression-review')
        self.assert_gate("final s='${if(flag) call()}';",'dart','foreign-interpolation-expression-review')

    def test_nested_interpolation_unknown_and_malformed_remain_explicit_gates(self):
        self.assert_gate('var s=$"{call($\"{x=1}\")}";','csharp','foreign-interpolation-expression-review')
        self.assert_gate("final s='${call(\"${x=1}\")}';",'dart','foreign-interpolation-expression-review')
        self.assert_gate('var s=$"{call()";','csharp','foreign-interpolation-expression-review')
        self.assert_gate("final s='${call()';",'dart','foreign-interpolation-expression-review')

    def test_interpolation_source_positions_and_exact_scalar_owner_evidence(self):
        for source,language,mask in (('var s=$"{table[index]}\\n{call(x)}";','csharp',mask_csharp),
                                     ("final s='${table[index]}\\n$word';",'dart',mask_dart)):
            result=mask(source)
            self.assertEqual(len(result['cleaned']),len(source))
            self.assertEqual(result['diagnostics'],[])
            parts=result['literals'][0]['interpolations']
            self.assertEqual(len(parts),2)
            self.assertTrue(all(part['scalar_composition'] for part in parts))
            self.assert_units(source,language,1)

    def test_complex_and_unknown_syntax_is_pending(self):
        self.assert_gate('var s="""raw""";','csharp','foreign-csharp-raw-string-review')
        self.assert_gate('#if FEATURE\nx=1;\n#endif','csharp','foreign-directive-policy-review')
        self.assert_gate('class C { int P { get;set; } }','csharp','foreign-ambiguous-brace-review')
        self.assert_gate('Use((x)=>{return x;},another);','csharp','foreign-callback-surrounding-expression-review')
        self.assert_gate('final x=[if(test) a];','dart','foreign-dart-collection-control-review')
        self.assert_units('class C{[Attr]\nvoid F(){}}','csharp',5)
        self.assert_gate('(Unknown x,int y) first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('Unknown* first,second;','csharp','foreign-complex-type-declaration-review')
        self.assert_gate('Dictionary<int,int> a=new Dictionary<int + x,int>(),b;','csharp','foreign-complex-declaration-review')

    def test_unsupported_grammar_not_aliased(self):
        for language in ('zig','odin','java','c'):
            with self.assertRaises(ValueError):
                normalize('x=1;',language)
        classified=classify_scopes('x=1;','zig')
        self.assertEqual(classified['diagnostics'][0]['kind'],'foreign-unsupported-language')


if __name__=='__main__':
    unittest.main()
