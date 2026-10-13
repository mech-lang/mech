"""Focused fixtures; main acquires the common timing mutex before lexing."""
import unittest
from pathlib import Path
from audit import timing_boundary
from lexical import mask_source
from normalizer import normalize, _scope_map
from canonical import _rust_function_start


class NormalizationFixtures(unittest.TestCase):
    def count(self, source, language, expected):
        row = normalize(source, language)
        self.assertEqual(row["normalized_loc"], expected, row)
        return row

    def test_mixed_statements_and_chained_bindings(self):
        self.count("a=1; f(); b=c=2;", "c", 4)

    def test_php_seven_chained_bindings_are_seven_units(self):
        row = self.count('$f=$flips=$nperm=$checksum=$i=$k=$r=0;', 'php', 7)
        self.assertEqual(len(row['expansions'][0]['events'][0]['assignments']),7)
        self.assertEqual(row['expansions'][0]['extra_lines'],6)

    def test_c_multiple_declaration_bindings(self):
        self.count("double x, y, z;", "c", 3)
        self.count("int w, h, bit_num=0;", "c", 3)
        self.count("int a=b=0,c;", "c", 3)

    def test_c_for_header_semicolons_are_structural(self):
        self.count("for(int i=0; test(i); step(i)) { x=1; y=2; }", "c", 4)

    def test_c_nested_assignment_is_executable(self):
        self.count("x=(y=0);", "c", 2)

    def test_c_compound_literals_are_data(self):
        self.count("v=(__m128d){1.0,1.0};", "c", 1)
        self.count("return (Point){x,y};", "c", 1)

    def test_ordinary_bare_blocks_have_no_fabricated_header(self):
        packed = 'int f(){{a=1;b=2;}return a;}'
        expanded = 'int f()\n{\n {\n  a=1;\n  b=2;\n }\n return a;\n}'
        for source in (packed, expanded):
            row = self.count(source, 'c', 6)
            self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))
        for language, signature in (('cpp', 'int f()'), ('java', 'void f()'),
                                    ('chapel', 'proc f()'), ('rust', 'fn f()')):
            with self.subTest(language=language):
                row = self.count(signature + '{\n a=1;\n {\n  b=2;\n }\n}', language, 5)
                self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))

    def test_go_bare_block_does_not_consume_prior_multiple_binding(self):
        self.count('func f(){\n a,b:=1,2\n {\n  c:=3\n }\n}', 'go', 6)
        self.count('func f(){\n call()\n {\n  c:=3\n }\n}', 'go', 5)

    def test_openmp_pragma_following_scope_is_executable(self):
        row = self.count('void f(){\n#pragma omp parallel\n{\n x=1; y=2;\n}\n}', 'c', 6)
        self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))

    def test_named_go_struct_fields_are_independent_declarations(self):
        for source in ('type Node struct {left,right *Node}',
                       'type Node struct {\nleft,right *Node\n}'):
            row = self.count(source, 'go', 4)
            self.assertFalse(any(d['kind'] == 'canonical-unrecognized-block-review' for d in row['diagnostics']))
        self.count('x:=Node{left:a,right:b}', 'go', 1)

    def test_rust_generic_struct_body_and_c_anonymous_union(self):
        row = self.count("struct Tree<'a> {\n l: Option<&'a Tree<'a>>,\n}", 'rust', 3)
        self.assertFalse(any(d['kind'] == 'canonical-unrecognized-block-review' for d in row['diagnostics']))
        row = self.count('struct S {union{int a;char b;};};', 'c', 6)
        self.assertFalse(any(d['kind'] == 'canonical-unrecognized-block-review' for d in row['diagnostics']))

    def test_js_destructured_parameters_remain_data(self):
        row = self.count('function f({a,b}){return a;}', 'javascript', 3)
        self.assertFalse(any(d['kind'] == 'canonical-unrecognized-block-review' for d in row['diagnostics']))
        self.count('const {a,b}=pair();', 'javascript', 1)

    def test_php_anonymous_class_body_is_a_declaration_scope(self):
        row = self.count('$x=new class {\n public $a;\n public $b;\n};', 'php', 4)
        self.assertFalse(any(d['kind'] == 'canonical-unrecognized-block-review' for d in row['diagnostics']))

    def test_java_static_initializer_header_is_format_independent(self):
        self.count('class S {static{x=1;}}', 'java', 5)
        self.count('class S {\n static\n {\n x=1;\n }\n}', 'java', 5)

    def test_declarator_cleanup_with_newline_and_spaces(self):
        self.count("int a=b=0,c;  \n", "c", 3)

    def test_rust_array_semicolon_is_structural(self):
        self.count("let buf=[0;32];", "rust", 1)

    def test_rust_destructuring_is_one_binding(self):
        self.count("let (a,b)=pair();", "rust", 1)

    def test_python_keyword_arguments_are_structural(self):
        self.count("x=dict(a=1,b=2)", "python", 1)

    def test_r_bindings_and_named_arguments(self):
        self.count("x <- y <- matrix(0,nrow=2,ncol=2)","r",2)
        self.count("f <- function(x) { y <- x+1; y }","r",4)

    def test_python_chains_and_tuple_destructuring(self):
        self.count("x=y=0", "python", 2)
        self.count("x,y=(0,0)", "python", 1)

    def test_julia_keyword_arguments_are_structural(self):
        self.count("sort!(x; rev=true, by=f)", "julia", 1)

    def test_chapel_named_constructor_and_call_arguments_are_structural(self):
        for source in ('new body(pos=p,vel=v,mass=m)',
                       'const pool=new bumpPtrMemPool(size,alignment=0);',
                       'const pool=new Pool(size,alignment=0),tree=new Tree(pool,depth=d);'):
            row = self.count(source,'chapel',2 if 'tree=' in source else 1)
            self.assertEqual(row['diagnostics'], [])

    def test_julia_matrix_rows_are_structural(self):
        self.count("x=[1 2;3 4]", "julia", 1)

    def test_julia_nested_comments(self):
        row=self.count("#= outer #= inner =# tail =#\nx=y=0", "julia", 2)
        self.assertEqual(row["physical_loc"], 1)

    def test_julia_comment_like_literal_and_adjoint(self):
        self.count('x="#= fake =#"\ny=A\' #= comment =#', "julia", 2)

    def test_julia_raw_regex_escapes_are_literal_text(self):
        source = 'pattern=r"a\\|b; x=y=0"\nrawtext=raw"#= literal =#; z=w=0"'
        row = self.count(source, 'julia', 2)
        self.assertEqual(row['diagnostics'], [])
        self.assertEqual(len(mask_source(source, 'julia')['cleaned']), len(source))

    def test_bom_before_comment_is_encoding_metadata(self):
        source = '\ufeff// comment x=y=0;\nx=1;'
        row = self.count(source, 'cpp', 1)
        self.assertEqual(row['physical_loc'], 1)
        self.assertEqual(row['diagnostics'], [])

    def test_perl_numeric_division_is_not_masked_as_regex(self):
        source = 'while(($d=($f=$a+$b)/$c)!=($f+$a)/$c){$x=1;}'
        row = self.count(source,'perl',3)
        self.assertEqual(row['diagnostics'], [])
        self.assertGreaterEqual(len(row['lexical_adjustments']),1)
        cleaned = mask_source(source,'perl')['cleaned']
        self.assertEqual(cleaned.count(')/$c'),2)
        self.assertIn('$f+$a',cleaned)
        self.assertEqual(len(cleaned),len(source))
        self.assertEqual(mask_source('$x = 9/$a + 8/$b;', 'perl')['diagnostics'], [])

    def test_perl_actual_regex_forms_stay_literal(self):
        for source in ('$ok=$x=~/a;b=c/;', '$ok=$x=~/a\\/b;c=d/;',
                       'if($ready) /$pattern/;', 'run(/$pattern/);'):
            masked = mask_source(source,'perl')
            self.assertEqual(masked['diagnostics'], [])
            self.assertEqual(masked['lexical_adjustments'], [])
            self.assertNotIn('b=c',masked['cleaned'])

    def test_perl_modern_postdereference_has_specific_operator_evidence(self):
        for source in ('my @items=$_[0]->@*;', 'my @items=$_[0]-> @*;'):
            with self.subTest(source=source):
                row = self.count(source,'perl',1)
                self.assertEqual(row['diagnostics'], [])
                self.assertTrue(any(a['kind'] == 'recognized-perl-postdereference'
                                    for a in row['lexical_adjustments']))
                cleaned = mask_source(source,'perl')['cleaned']
                self.assertEqual(cleaned, source)
                self.assertEqual(len(cleaned),len(source))

    def test_julia_loop_macro_symbol_option_is_one_header(self):
        for source in ('Threads.@threads :static for i=1:n\n x=i\nend',
                       '@inbounds for i=1:n\n x=i\nend'):
            row = self.count(source, 'julia', 3)
            self.assertEqual(row['diagnostics'], [])

    def test_julia_typed_keyword_functions_have_explicit_body_boundary(self):
        for source in ('function f(x::Int)::Int\n x+1\nend',
                       'function f(x::T) where T\n x+1\nend',
                       'function f(x::Int)::Int; x+1; end'):
            row = self.count(source, 'julia', 3)
            self.assertEqual(row['diagnostics'], [])

    def test_explicit_callback_body_resets_outer_argument_scope(self):
        for language, packed, expanded in (
            ('cpp', 'run([&x]{a=1;b=2;});', 'run([&x]\n{\na=1;\nb=2;\n});'),
            ('javascript', 'run(()=>{a=1;b=2;});', 'run(()=>\n{\na=1;\nb=2;\n});'),
            ('rust', 'run(||{a=1;b=2;});', 'run(||\n{\na=1;\nb=2;\n});'),
            ('php', 'run(function(){a=1;b=2;});', 'run(function()\n{\na=1;\nb=2;\n});')):
            for source in (packed, expanded):
                with self.subTest(language=language, source=source):
                    row = self.count(source, language, 4)
                    self.assertFalse(any(d['kind'] in {'canonical-lambda-body-review',
                        'canonical-braced-callback-review'} for d in row['diagnostics']))

    def test_callback_structural_suffix_can_wrap_without_new_statement(self):
        for source in ('run([&x]{a=1;b=2;});',
                       'run([&x]\n{\n a=1;\n b=2;\n}\n);'):
            row=self.count(source,'cpp',4)
            self.assertEqual(row['diagnostics'],[])
            self.assertTrue(any(e['kind']=='structural-suffix' and e['units']==0
                for e in row['canonical_events']))
        self.count('run([&x]{a=1;b=2;}); c=3;','cpp',5)
        # Literal-only data wrapping retains its existing physical treatment.
        self.count('values=[1,2,\n 3,4\n];','javascript',3)

    def test_callback_real_following_composition_is_reviewed(self):
        for source in ('run([&x]{a=1;b=2;}).collect();',
                       'run([&x]{a=1;b=2;},other());'):
            row=normalize(source,'cpp')
            self.assertTrue(any(d['kind']=='canonical-callback-composition-review'
                for d in row['diagnostics']))

    def test_named_chapel_function_is_not_a_lambda(self):
        row = self.count('proc main(){a=1;b=2;}', 'chapel', 4)
        self.assertFalse(any(d['kind'] == 'canonical-lambda-body-review' for d in row['diagnostics']))

    def test_chapel_proc_do_defaults_are_structural_and_body_separate(self):
        for source in ('inline proc poolSize(depth,num=1) do return num*depth;',
                       'inline proc poolSize(depth,num=1) do\n return num*depth;'):
            row = self.count(source, 'chapel', 2)
            self.assertEqual(row['diagnostics'], [])
        self.count('proc f(x) do for i in 1..x do run(i);', 'chapel', 3)

    def test_chapel_multiline_task_intent_is_one_loop_header(self):
        for source in ('forall i in xs with (+ reduce sum){sum+=i;}',
                       'forall i in xs\n with (+ reduce sum) {\n sum+=i;\n }'):
            row = self.count(source, 'chapel', 3)
            self.assertEqual(row['diagnostics'], [])

    def test_chapel_repeat_condition_is_a_written_header(self):
        row = self.count('do {x+=1;} while x<n;', 'chapel',4)
        self.assertEqual(row['diagnostics'], [])

    def test_python_soft_keywords_and_explicit_header_continuation(self):
        row = self.count('match=lookup()\ncase=0','python',2)
        self.assertEqual(row['diagnostics'], [])
        row = self.count('if ready \\\n and waiting():\n run()','python',2)
        self.assertEqual(row['diagnostics'], [])

    def test_julia_expression_blocks_keep_ends_tracked_and_reviewed(self):
        pure = self.count('function f()\n x=if c\n 1\n else\n 2\n end\nend', 'julia', 3)
        self.assertEqual(pure['diagnostics'], [])
        self.assertTrue(any(e['kind'] == 'value-expression' for e in pure['canonical_events']))
        for source in ('function f()\n x=@distributed (+) for i=1:n\n i\n end\nend',):
            row = normalize(source,'julia')
            self.assertFalse(any(d['kind'] in {'canonical-unmatched-keyword-close',
                'canonical-unmatched-keyword-branch','canonical-unclosed-keyword-block'}
                for d in row['diagnostics']))
            self.assertTrue(any(d['kind'] == 'canonical-expression-block-review'
                                for d in row['diagnostics']))

    def test_ruby_result_method_keeps_preceding_keyword_close(self):
        source = 'if x\n ys.map do |y|\n y\n end.each{|z| puts z}\nelse\n f()\nend'
        row = normalize(source, 'ruby')
        self.assertFalse(any(d['kind'] in {'canonical-unclosed-keyword-block',
            'canonical-unmatched-keyword-close', 'canonical-overlapping-spans-review'}
            for d in row['diagnostics']))
        self.assertFalse(any(d['kind'] == 'canonical-ruby-callback-expression-review'
                            for d in row['diagnostics']))

    def test_ruby_hash_literals_are_data_but_explicit_callbacks_are_blocks(self):
        for source in ('{a:1,b:2}', 'return {a:1,b:2}',
                       'x = c ? {a:1} : {b:2}'):
            row = self.count(source, 'ruby', 1)
            self.assertFalse(any(e['scope'] == 'braced' for e in row['canonical_events']))
        for source in ('xs.map{|x| x+1}', 'xs.map do |x|\n x+1\nend'):
            row = self.count(source, 'ruby', 3)
            self.assertEqual(row['diagnostics'], [])

    def test_simple_statement_modifiers_have_header_body_without_invented_close(self):
        for language, source in (
            ('ruby', 'return 1 if left.nil?'),
            ('ruby', 'break if index>=array.size'),
            ('ruby', 'run() unless ready'),
            ('perl', "die 'x' if ($h)->{lib} ne 'x';"),
            ('perl', 'print $x for @xs;'),
            ('perl', 'next unless $ready;')):
            row = self.count(source,language,2)
            self.assertEqual(row['diagnostics'], [])
            self.assertFalse(any(e['kind'] == 'close' for e in row['canonical_events']))

    def test_ruby_assignment_if_tracks_stack_but_keeps_expression_review(self):
        source = 'if x\n a = if y\n b()\n else\n c()\n end\nelse\n d()\nend'
        row = normalize(source, 'ruby')
        self.assertFalse(any(d['kind'] in {'canonical-unclosed-keyword-block',
            'canonical-unmatched-keyword-close', 'canonical-unmatched-keyword-branch'}
            for d in row['diagnostics']))
        self.assertTrue(any(d['kind'] == 'canonical-expression-block-review'
                            for d in row['diagnostics']))

    def test_ruby_nested_callback_header_excludes_outer_parameters(self):
        row = normalize('xs.map{|i| Worker.new{a=1;b=2}}', 'ruby')
        self.assertFalse(any(d['kind'] == 'canonical-overlapping-spans-review'
                            for d in row['diagnostics']))

    def test_rust_meaningful_attribute_and_function_are_separate_units(self):
        for source in ('#[inline] fn f(){return 1;}',
                       '#[inline]\nfn f()\n{\nreturn 1;\n}'):
            row = self.count(source, 'rust', 4)
            self.assertEqual(row['diagnostics'], [])

    def test_lua_multiple_independent_locals(self):
        self.count("local x,y,z=0,0,0", "lua", 3)
        self.count("local x,y,z", "lua", 3)

    def test_lua_single_producer_is_destructuring(self):
        self.count("local x,y=pair()", "lua", 1)

    def test_go_multi_return_and_independent_initializers(self):
        self.count("x,y:=pair()", "go", 1)
        self.count("x,y:=0,0", "go", 2)

    def test_typed_rust_and_go_bodies_remain_executable(self):
        self.count("fn f() -> f64 { x=1; y=2; }", "rust", 4)
        self.count("func f() float64 { x=1; y=2; }", "go", 4)
        self.count("for i:=0;i<n;i++ { x=1; y=2; }", "go", 4)

    def test_js_declarations_and_object_destructuring(self):
        self.count("let a=1,b=2;", "javascript", 2)
        self.count("const {a,b}=pair();", "javascript", 1)
        self.count("const f=(a=1)=>a;", "javascript", 1)

    def test_comments_and_strings_do_not_add_units(self):
        self.count('x="a=1;b=2"; // x=y=1;', "c", 1)

    def test_lexical_eof_line_comments_match_newline_terminated_comments(self):
        for language, marker in (("c", "//"), ("cpp", "//"),
                                 ("rust", "//"), ("go", "//"),
                                 ("java", "//"), ("chapel", "//"),
                                 ("javascript", "//"), ("python", "#"),
                                 ("ruby", "#"), ("perl", "#"),
                                 ("julia", "#"), ("r", "#"),
                                 ("lua", "--")):
            with self.subTest(language=language):
                source = 'x=1; ' + marker + ' y=z=1;'
                eof = mask_source(source, language)
                terminated = mask_source(source + '\n', language)
                self.assertEqual(eof["cleaned"], terminated["cleaned"][:-1])
                self.assertEqual(len(eof["cleaned"]), len(source))
                self.assertNotIn('y=z', eof["cleaned"])
                self.assertEqual(eof["diagnostics"], [])

    def test_lexical_comment_only_eof_has_no_virtual_source_line(self):
        source = '// x=y=1;'
        result = mask_source(source, "c")
        self.assertEqual(len(result["cleaned"]), len(source))
        self.assertEqual(result["cleaned"].strip(), '')
        self.assertNotIn('\n', result["cleaned"])
        self.assertEqual(result["diagnostics"], [])

    def test_lexical_eof_comment_retains_existing_crlf_offsets(self):
        source = 'x=1;\r\n// y=z=1;'
        result = mask_source(source, "c")
        self.assertEqual(len(result["cleaned"]), len(source))
        self.assertEqual([i for i, c in enumerate(result["cleaned"]) if c in '\r\n'],
                         [i for i, c in enumerate(source) if c in '\r\n'])
        self.assertNotIn('y=z', result["cleaned"])
        self.assertEqual(result["diagnostics"], [])

    def test_existing_closing_lines_are_retained(self):
        self.count("x=1;\n}", "c", 2)
        self.count("x=1\nend", "lua", 2)

    def test_mask_preserves_offsets_and_crlf(self):
        source='\tlocal x=";=--" -- comment\r\nlocal y=1\r\n'
        masked=mask_source(source,"lua")["cleaned"]
        self.assertEqual(len(source),len(masked))
        self.assertEqual([i for i,c in enumerate(source) if c in "\r\n"],
                         [i for i,c in enumerate(masked) if c in "\r\n"])

    def test_inline_and_expanded_control_spellings_agree(self):
        self.count("if x then f() end", "lua", 3)
        self.count("if x then\n  f()\nend", "lua", 3)
        self.count("if(x){a=1;}","c",3)
        self.count("if(x)\n{\n a=1;\n}","c",3)
        self.count("if x: f()","python",2)
        self.count("if x:\n f()","python",2)

    def test_function_and_combined_branches(self):
        self.count("int f(int x){return x;}","c",3)
        self.count("int f(int x)\n{\nreturn x;\n}","c",3)
        self.count("if(x){a=1;}else{b=2;}","c",6)
        self.count("if(x){\na=1;\n}\nelse{\nb=2;\n}","c",6)

    def test_mech_expression_and_explicit_block_functions(self):
        self.count("Plus(x<u64>) => <u64> := x+1","mech",1)
        for source in ("Plus(x<u64>) = (next<u64>) :=\n next := x+1.",
                       "Plus(x<u64>) => <u64> :=\n next := x+1."):
            row = self.count(source,"mech",3)
            self.assertEqual(row['diagnostics'], [])

    def test_semantic_compiler_pragma_retained(self):
        row=self.count("#pragma omp simd\nx=1;","c",2)
        self.assertEqual(row["physical_loc"],2)

    def test_known_legacy_lua_source(self):
        import os
        root=Path(os.environ.get("MECH_LOC_CLBG_ROOT",Path(__file__).resolve().parents[2]))
        row=normalize((root/"compact/upstream/nbody-lua-4.lua").read_text(),"lua")
        self.assertEqual(row["physical_loc"],107)
        self.assertEqual(row["normalized_loc"],123)
        self.assertEqual(sum(e["extra_lines"] for e in row["expansions"]),16)

    def test_known_julia_comment_and_chains(self):
        import os
        root=Path(os.environ.get("MECH_LOC_CLBG_ROOT",Path(__file__).resolve().parents[2]))
        row=normalize((root/"spectralnorm/upstream/spectralnorm-julia-4.jl").read_text(),"julia")
        self.assertEqual((row["physical_loc"],row["normalized_loc"]),(36,38))

    def test_known_r_nbody_physical_and_canonical_agree(self):
        import os
        root=Path(os.environ.get("MECH_LOC_CLBG_ROOT",Path(__file__).resolve().parents[2]))
        row=normalize((root/"r-apl-uiua-comparison/20261011/sources/r/nbody/compact.R").read_text(),"r")
        self.assertEqual((row["physical_loc"],row["normalized_loc"]),(19,19))

    def test_perl_dereference_braces_stay_data_after_control_words(self):
        for source in ("die 'x' if Foo->config->{lib} ne 'x';",
                       "die 'x' if Foo->config-> {lib} ne 'x';",
                       "my $value=($h)->{lib};"):
            with self.subTest(source=source):
                row = self.count(source, 'perl', 2 if source.startswith('die') else 1)
                self.assertFalse(any(e['scope'] == 'braced' for e in row['canonical_events']))

    def test_perl_adjacent_sigil_subscripts_are_data(self):
        for receiver in ('$h', '@h', '%h', '$Pkg::h', '$_'):
            with self.subTest(receiver=receiver):
                row = self.count("die 'x' if " + receiver + "{lib} ne 'x';", 'perl', 2)
                self.assertFalse(any(e['scope'] == 'braced' for e in row['canonical_events']))

    def test_perl_real_control_and_subroutine_blocks_remain_executable(self):
        self.count('if($x){$y=1;}', 'perl', 3)
        self.count('for(1..$n){$y=1;}', 'perl', 3)
        self.count('sub f{$y=1;}', 'perl', 3)

    def test_cpp_capture_without_parameter_parentheses_is_executable(self):
        for source in ('run([&x,y]{a=1;b=2;});',
                       'run([&x,y]\n{\n a=1;\n b=2;\n});'):
            row = self.count(source, 'cpp', 4)
            self.assertFalse(any(d['kind'] in {'cpp-complex-lambda-header-review',
                'canonical-braced-callback-review'} for d in row['diagnostics']))

    def test_cpp_lambda_mutable_noexcept_and_named_return_type(self):
        source = 'auto f=[&x,y]() mutable noexcept -> int {a=1;b=2;return a;};'
        row = self.count(source, 'cpp', 5)
        self.assertFalse(any(d['kind'] == 'cpp-complex-lambda-header-review' for d in row['diagnostics']))
        self.count('return [x]{a=1;b=2;};', 'cpp', 4)

    def test_cpp_complex_capture_is_gated_without_erasing_known_body(self):
        row = normalize('auto f=[x=values[0]]{a=1;b=2;};', 'cpp')
        self.assertTrue(any(d['kind'] == 'cpp-complex-lambda-header-review' for d in row['diagnostics']))
        row = self.count('auto f=[x=make()](){a=1;b=2;};', 'cpp', 4)
        self.assertTrue(any(d['kind'] == 'cpp-complex-lambda-header-review' for d in row['diagnostics']))

    def test_rust_parameter_closure_body_is_executable(self):
        for source in ('run(|x,y|{a=1;b=2;});',
                       'run(|x,y|\n{\n a=1;\n b=2;\n});'):
            row = self.count(source, 'rust', 4)
            self.assertFalse(any(d['kind'] in {'rust-complex-closure-header-review',
                'canonical-braced-callback-review'} for d in row['diagnostics']))
        self.count('let f=move |x: i32,mut y: i32| -> i32 {a=1;return x;};', 'rust', 4)

    def test_rust_complex_parameter_patterns_remain_gated(self):
        for source in ('let f=|(x,y)|{a=1;b=2;};',
                       'let f=async move |x|{a=1;b=2;};'):
            row = normalize(source, 'rust')
            self.assertTrue(any(d['kind'] == 'rust-complex-closure-header-review' for d in row['diagnostics']))

    def test_anonymous_grammar_does_not_capture_ordinary_array_data(self):
        for language, source in (('cpp', 'int a[2]={1,2};'),
                                 ('cpp', 'int a[2]{1,2};'),
                                 ('cpp', 'auto x=a[0];'),
                                 ('rust', 'let a=[1,2];')):
            row = self.count(source, language, 1)
            self.assertFalse(any('complex-lambda' in d['kind'] or 'complex-closure' in d['kind']
                                 for d in row['diagnostics']))

    def test_named_c_aggregate_pointer_declarators_are_independent(self):
        for language in ('c', 'cpp'):
            self.count('struct node *left,*right;', language, 2)
            self.count('struct worker_args *args,*targs,*hargs;', language, 3)
            self.count('struct Node*left,*right;', language, 2)
            self.count('union Value *left,*right;', language, 2)
            self.count('enum Mode first,second;', language, 2)

    def test_compiler_vector_and_wide_integer_declarators(self):
        for language in ('c', 'cpp'):
            self.count('#include <emmintrin.h>\n__m128d dx,dy,dz;', language, 4)
            self.count('#include <immintrin.h>\n__m256d p[N],v[N];', language, 3)
            self.count('#include <immintrin.h>\n__m512i x,y;', language, 3)
            self.count('__int128 x,y;', language, 2)
            self.count('unsigned __int128 x,y;', language, 2)

    def test_c_signed_and_unsigned_shorthand_declarators(self):
        for language in ('c', 'cpp'):
            self.count('unsigned x,y;', language, 2)
            self.count('signed x,y;', language, 2)
            self.count('unsigned*x,*y;', language, 2)

    def test_unknown_lowercase_type_does_not_guess_multiplication(self):
        for source in ('node *left,*right;', 'node*left,*right;',
                       'a * b,c;', 'a*b,c;'):
            row = self.count(source, 'c', 1)
            self.assertTrue(any(d['kind'] == 'unknown-c-declaration-type-review' for d in row['diagnostics']))
        for language, source in (('c', 'return a,b;'),
                                 ('cpp', 'auto [x,y]=pair();'),
                                 ('c', 'int a[]={1,2};'),
                                 ('c', 'f(a,b);')):
            row = self.count(source, language, 1)
            self.assertFalse(any(d['kind'] == 'unknown-c-declaration-type-review' for d in row['diagnostics']))

    def test_lua_single_values_bind_implicit_nil_targets_independently(self):
        for value in ('0', 'nil', 'true', 'value', 'a+b', 'not f()',
                      '{1,2}', '"hello world"', 'items[index]'):
            row = self.count('local x,y=' + value, 'lua', 2)
            self.assertFalse(any(d['kind'].startswith('lua-multiple-') for d in row['diagnostics']))

    def test_lua_only_proven_unparenthesized_producers_are_tuple_bindings(self):
        for value in ('pair()', 'module.pair()', 'object:pair(x)', 'pair(other())', '...'):
            row = self.count('local x,y=' + value, 'lua', 1)
            self.assertFalse(any(d['kind'].startswith('lua-multiple-') for d in row['diagnostics']))

    def test_lua_parentheses_suppress_multiple_returns(self):
        self.count('local x,y=(pair())', 'lua', 2)
        self.count('local x,y=(...)', 'lua', 2)

    def test_lua_complex_producers_and_mismatched_rhs_remain_gated(self):
        for value in ('(pair)()', 'functions[index]()', 'pair "hello"', 'pair{1,2}'):
            row = self.count('local x,y=' + value, 'lua', 2)
            self.assertTrue(any(d['kind'] == 'lua-multiple-return-producer-review' for d in row['diagnostics']))
        row = self.count('local x,y,z=1,pair()', 'lua', 3)
        self.assertTrue(any(d['kind'] == 'lua-multiple-rhs-review' for d in row['diagnostics']))

    def test_lua_explicit_binding_arity_and_go_comma_ok_are_preserved(self):
        self.count('local x,y', 'lua', 2)
        self.count('local x,y=0,1', 'lua', 2)
        self.count('x,ok:=lookup()', 'go', 1)

    def test_java_throws_signature_body_is_executable(self):
        for source in ('static final void f(int n) throws IOException {a=1;b=2;}',
                       'static final void f(int n) throws IOException\n{\n a=1;\n b=2;\n}',
                       'static final void f(\n int n)\n throws IOException,InterruptedException\n{a=1;b=2;}'):
            row = self.count(source, 'java', 4)
            self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))

    def test_java_implements_class_body_is_a_declaration_scope(self):
        for source in ('public final class C implements Runnable {int a;int b;}',
                       'public final class C implements Runnable\n{\n int a;\n int b;\n}'):
            row = self.count(source, 'java', 4)
            self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))

    def test_rust_multiline_for_iterator_has_executable_body(self):
        for source in ('for i in first..last {a=1;b=2;}',
                       'for i in\n first..last\n{\n a=1;\n b=2;\n}'):
            row = self.count(source, 'rust', 4)
            self.assertFalse(any(d['kind'] == 'canonical-bare-block-review' for d in row['diagnostics']))

    def test_rust_multiline_named_function_signature_is_one_header(self):
        pairs = (
            ("fn f<'a>(x:&'a T)->Option<&'a T>{a=1;b=2;}",
             "fn f<'a>(x:&'a T)\n ->Option<&'a T>\n{\n a=1;\n b=2;\n}", 4),
            ('pub fn f<T>(x:T)->T where T:Copy {x}',
             'pub fn f<T>(x:T)\n ->T\n where T:Copy\n{\n x\n}', 3),
            ('fn f(x:fn(u64)->u64)->u64 {x(1)}',
             'fn f(\n x:fn(u64)->u64\n)\n ->u64\n{\n x(1)\n}', 3))
        for packed, expanded, expected in pairs:
            for source in (packed, expanded):
                with self.subTest(source=source):
                    row = self.count(source, 'rust', expected)
                    self.assertEqual(row['diagnostics'], [])
                    headers = [event for event in row['canonical_events'] if event['kind'] == 'header']
                    self.assertEqual(len(headers), 1)
                    self.assertEqual(headers[0]['start'], 0)
                    cleaned = mask_source(source, 'rust')['cleaned']
                    contexts, diagnostics = _scope_map(cleaned, 'rust')
                    self.assertEqual(diagnostics, [])
                    opening = cleaned.index('{')
                    self.assertEqual(contexts[opening+1][-1][1], 'code')

    def test_rust_named_signature_proof_rejects_constructor_and_macro(self):
        for source in ('let x=Foo{}', 'm!(fn f()->T) {}'):
            self.assertIsNone(_rust_function_start(source, source.index('{')))

    def test_rust_expression_closures_do_not_gate_enclosing_iterator_body(self):
        source = 'for j in (0..).map(|x| x*k).take_while(|&j| j<k*n) {a=1;b=2;}'
        row = self.count(source, 'rust', 4)
        self.assertFalse(any(d['kind'] == 'rust-complex-closure-header-review' for d in row['diagnostics']))

    def test_go_bare_struct_and_interface_braces_are_type_data(self):
        for source in ('x:=make(chan struct{})', 'var x struct{}',
                       'var x interface{}', 'x:=struct{}{}'):
            row = self.count(source, 'go', 1)
            cleaned = mask_source(source, 'go')['cleaned']
            contexts, diagnostics = _scope_map(cleaned, 'go')
            self.assertEqual(diagnostics, [])
            for index, char in enumerate(cleaned):
                if char == '{':
                    self.assertEqual(contexts[index + 1][-1][1], 'data')

    def test_go_empty_anonymous_return_type_precedes_executable_function_body(self):
        for source in ('f:=func() interface{} {a=1;return a}',
                       'f:=func() interface{}\n{\n a=1;\n return a\n}'):
            row = self.count(source, 'go', 4)
            cleaned = mask_source(source, 'go')['cleaned']
            contexts, diagnostics = _scope_map(cleaned, 'go')
            self.assertEqual(diagnostics, [])
            kinds = [contexts[index + 1][-1][1] for index, char in enumerate(cleaned) if char == '{']
            self.assertEqual(kinds, ['data', 'code'])
        source = 'func f() struct{} {a:=1;return struct{}{}}'
        self.count(source, 'go', 4)
        cleaned = mask_source(source, 'go')['cleaned']
        contexts, diagnostics = _scope_map(cleaned, 'go')
        self.assertEqual(diagnostics, [])
        self.assertEqual([contexts[index + 1][-1][1] for index, char in enumerate(cleaned) if char == '{'],
                         ['data', 'code', 'data', 'data'])
        source = 'func outer(){f:=func() interface{}{a=1;return a}}'
        self.count(source, 'go', 6)
        cleaned = mask_source(source, 'go')['cleaned']
        contexts, diagnostics = _scope_map(cleaned, 'go')
        self.assertEqual(diagnostics, [])
        self.assertEqual([contexts[index + 1][-1][1] for index, char in enumerate(cleaned) if char == '{'],
                         ['code', 'data', 'code'])
        row = normalize('f:=func() interface{M()} {a=1;return a}', 'go')
        self.assertTrue(any(d['kind'] == 'go-complex-anonymous-return-type-review' for d in row['diagnostics']))

    def test_go_completed_type_declaration_can_precede_separate_bare_block(self):
        source = 'func f(){var x interface{}\n{a=1;}}'
        self.count(source, 'go', 5)
        cleaned = mask_source(source, 'go')['cleaned']
        contexts, diagnostics = _scope_map(cleaned, 'go')
        self.assertEqual(diagnostics, [])
        self.assertEqual([contexts[index + 1][-1][1] for index, char in enumerate(cleaned) if char == '{'],
                         ['code', 'data', 'bare-code'])

    def test_go_if_init_semicolon_belongs_to_control_header(self):
        source = 'if idx++;idx==idxMax {break}'
        self.count(source, 'go', 3)
        cleaned = mask_source(source, 'go')['cleaned']
        contexts, diagnostics = _scope_map(cleaned, 'go')
        self.assertEqual(diagnostics, [])
        self.assertEqual(contexts[source.index(';')][-1][1], 'control')

    def test_perl_uninitialized_declaration_names_are_independent(self):
        self.count('my ($vBv,$vv);', 'perl', 2)
        self.count('my($begin,$end,@threads);', 'perl', 3)
        self.count('my($k,$s,$d,$f);', 'perl', 4)
        for keyword in ('our', 'state', 'local'):
            self.count(keyword + '($x,$y);', 'perl', 2)

    def test_perl_assigned_tuple_producer_stays_one_binding(self):
        self.count('my($x,$y)=pair();', 'perl', 1)
        self.count('my($x,$y)=(pair());', 'perl', 1)
        self.count('my($x,$y)=@values;', 'perl', 1)

    def test_php_property_and_static_names_are_independent(self):
        self.count('public $left,$right;', 'php', 2)
        self.count('static $x,$y;', 'php', 2)
        self.count('var $left,$right;', 'php', 2)
        self.count('public readonly ?Node $left,$right;', 'php', 2)

    def test_php_mixed_chained_and_uninitialized_declarators(self):
        self.count('static $x=$y=0,$z;', 'php', 3)
        self.count('static $x=1,$y=2;', 'php', 2)
        self.count('public $x=1,$y;', 'php', 2)

    def test_other_calls_and_container_fields_are_not_declarations(self):
        self.count('attr_accessor :left,:right', 'ruby', 1)
        self.count('[$left,$right]', 'php', 1)
        self.count('f($left,$right);', 'php', 1)
        self.count('f($x,$y);', 'perl', 1)

    def test_java_arrow_callback_packed_and_expanded(self):
        for source in ('executor.execute(() -> {int a=1;int b=2;});',
                       'executor.execute(() -> {\n int a=1;\n int b=2;\n}\n);'):
            row = self.count(source, 'java', 4)
            self.assertEqual(row['diagnostics'], [], row)

    def test_go_empty_anonymous_type_constructor_is_data(self):
        row = self.count('func f(){done <- struct{}{};}', 'go', 3)
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(sum(e['units'] for e in row['canonical_events']), 2)


if __name__ == "__main__":
    with timing_boundary():
        unittest.main()
