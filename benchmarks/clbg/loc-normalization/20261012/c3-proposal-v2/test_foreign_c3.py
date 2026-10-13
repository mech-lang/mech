"""Meaningful C3 positive/negative grammar fixtures; unexecuted until mutex job."""
import unittest
from foreign_c3 import normalize
from c3_mask import mask_c3


class C3Fixtures(unittest.TestCase):
    def count(self, source, expected):
        result = normalize(source)
        self.assertEqual(result['diagnostics'], [], result)
        self.assertEqual(result['normalized_loc'], expected, result)
        self.assertEqual(result['language'], 'c3')
        self.assertFalse(result['eligible_for_chart'])
        return result

    def equivalent(self, packed, expanded, expected):
        self.count(packed, expected); self.count(expanded, expected)

    def gate(self, source, kind):
        result = normalize(source)
        self.assertEqual(result['review_status'], 'needs-review', result)
        self.assertIn(kind, {issue['kind'] for issue in result['diagnostics']}, result)
        self.assertFalse(result['eligible_for_chart'])
        return result

    def test_module_import_and_separate_packed_statements(self):
        self.equivalent('module bench; import libc; import std::collections::map;',
                        'module bench;\nimport libc;\nimport std::collections::map;', 3)

    def test_c3_function_formals_are_structural(self):
        self.equivalent('fn int main(int argc,char** argv){int n=10;return n;}',
                        'fn int main(\nint argc,\nchar** argv\n)\n{\nint n=10;\nreturn n;\n}', 4)

    def test_extern_abi_signature_and_attribute_are_one_header(self):
        self.equivalent('extern fn void init(Mpz* value) @cname("__gmpz_init");',
                        'extern fn void init(\nMpz* value\n)\n@cname("__gmpz_init");', 1)
        self.count('extern fn int printf(char* format, ...) @cname("printf");', 1)
        self.gate('extern fn void run(){mutate();}', 'c3-external-function-body-review')
        self.gate('extern fn int scalar()=>1;', 'c3-external-function-body-review')

    def test_single_external_variable_has_proved_typed_owner(self):
        self.equivalent('extern char* version @cname("__gmp_version");',
                        'extern char* version\n@cname("__gmp_version");', 1)
        self.count('extern gmp::Mpz* value;', 1)
        result = self.gate('extern int first,second;', 'c3-external-variable-owner-review')
        self.assertEqual(result['normalized_loc'], 2)
        self.gate('extern int value=stateful();', 'c3-external-variable-owner-review')

    def test_function_pointer_and_generic_aliases(self):
        self.count('alias Worker = fn void*(void*);', 1)
        self.count('alias Map = HashMap{String,long};', 1)
        self.count('typedef String = inline char[];', 1)
        self.gate('alias Hidden = fn int(){first();second();};', 'c3-alias-owner-review')
        self.gate('def Worker = fn void*(void*);', 'c3-legacy-alias-review')

    def test_struct_pointer_array_slice_fields_and_independent_declarators(self):
        self.equivalent('struct Node{Node* left,right;long check;char[] label;Node*[8] slots;}',
                        'struct Node\n{\nNode* left;\nNode* right;\nlong check;\nchar[] label;\nNode*[8] slots;\n}', 7)
        self.count('struct Body{double x,y,z,vx,vy,vz,mass;}', 9)

    def test_nested_struct_has_its_own_real_close(self):
        self.equivalent('struct Outer{struct inner{long value;}long other;}',
                        'struct Outer\n{\nstruct inner\n{\nlong value;\n}\nlong other;\n}', 6)

    def test_initialized_uninitialized_and_chained_bindings(self):
        self.equivalent('int a,b,c;', 'int a;\nint b;\nint c;', 3)
        self.count('int a=1;int b=2;int c;', 3)
        self.count('a=b=c=0;', 3)
        self.count('int a=b=0;int c;', 3)
        self.gate('int a=1,b=2,c;', 'c3-initialized-multi-declaration-review')
        self.gate('int a,b=0;', 'c3-initialized-multi-declaration-review')
        self.count('int[3] values={1,2,3};', 1)
        self.count('Node value={.left=null,.right=null};', 1)
        self.count('Result value={.first=call(a,b),.second=2};', 1)

    def test_for_foreach_headers_do_not_become_body_assignments(self):
        self.equivalent('fn void f(){for(int i=0;i<8;i++){sum+=i;}foreach(entry:map.iter()){emit(entry);}}',
                        'fn void f()\n{\nfor(int i=0;i<8;i++)\n{\nsum+=i;\n}\nforeach(entry:map.iter())\n{\nemit(entry);\n}\n}', 8)
        self.count('foreach(usz index, &entry : rows){sum+=entry.value;}', 3)
        self.count('foreach_r(entry : rows){emit(entry);}', 3)
        self.gate('foreach(entry rows){emit(entry);}', 'c3-foreach-header-review')

    def test_inline_control_branch_and_body_count(self):
        self.equivalent('fn void f(){if(ok) first();else second();while(n>0)n--;}',
                        'fn void f()\n{\nif(ok) first();\nelse second();\nwhile(n>0) n--;\n}', 8)
        self.count('if(a){first();}else if(b){second();}else{third();}', 9)
        self.gate('while(n>0)\nn--;', 'c3-unbraced-control-layout-review')
        self.gate('for(int i=0;i<8;i++)\nwork(i);', 'c3-unbraced-control-layout-review')
        self.gate('if(ok)\nwork();', 'c3-unbraced-control-layout-review')

    def test_defer_call_and_defer_block_owners(self):
        self.equivalent('fn int f(){defer map.free();return 0;}',
                        'fn int f()\n{\ndefer\nmap.free();\nreturn 0;\n}', 4)
        self.count('fn void f(){defer{first();second();}work();}', 7)
        self.gate('defer catch cleanup();', 'c3-extended-defer-review')
        self.gate('defer output=next();', 'c3-extended-defer-review')

    def test_pure_arrow_one_definition_and_mutating_arrow_is_gated(self):
        self.equivalent('fn double norm(double x)=>x*x;', 'fn double norm(double x)\n=>\nx*x;', 1)
        self.gate('fn int next()=>counter=counter+1;', 'c3-imperative-arrow-review')
        self.gate('fn void run()=>mutate();', 'c3-imperative-arrow-review')
        self.gate('fn int next()=>stateful();', 'c3-imperative-arrow-review')
        self.gate('fn int compose(int x)=>apply(fn int(int y){first();return y;});', 'c3-executable-call-argument-review')

    def test_named_call_arguments_and_data_assignments_are_structural(self):
        self.count('emit(value: call(1,2), count: 3);', 1)
        self.count('output={.x=1,.y=2};', 1)
        self.count('table.init(mem);table.free();', 2)
        self.count('table.init(&allocators::LIBC_ALLOCATOR);', 1)
        self.count('emit({.value=call(1,2),.count=3});', 1)
        self.gate('first(),second();', 'c3-comma-expression-review')
        self.gate('apply(fn int(int x){first();return x;});', 'c3-executable-call-argument-review')
        self.gate('call(value=next());', 'c3-call-argument-write-review')
        self.gate('call(object.field=next());', 'c3-call-argument-write-review')
        self.gate('call(counter++);', 'c3-call-argument-mutation-review')

    def test_initializer_and_index_owners_cannot_hide_executable_effects(self):
        self.count('Row value={.key=next(),.count=1};', 1)
        self.count('int[3] values={1,2,3};', 1)
        self.count('rows[index]={key,count};', 1)
        self.gate('Handler value={.callback=fn int(int x){first();return x;}};', 'c3-executable-data-owner-review')
        self.gate('Handler value={.callback=fn int(int x)=>stateful(x)};', 'c3-executable-data-owner-review')
        self.gate('int[2] values={first();second();};', 'c3-executable-data-owner-review')
        self.gate('values[index++]=next();', 'c3-data-owner-mutation-review')
        self.gate('values[index=next()]=0;', 'c3-data-owner-write-review')
        self.gate('Row value={.count=counter++};', 'c3-data-owner-mutation-review')
        self.gate('Row value={.count=(counter=next())};', 'c3-data-owner-write-review')

    def test_scalar_switch_arm_headers_preserve_each_real_body_statement(self):
        self.equivalent("switch(base){case 'A':return 'T';case 'C':return 'G';default:return base;}",
                        "switch(base)\n{\ncase 'A':\nreturn 'T';\ncase 'C':\nreturn 'G';\ndefault:\nreturn base;\n}", 8)
        self.count('switch(value){case 0:first();second();break;default:third();}', 8)
        self.count('switch(value){case 0:{first();}default:second();}', 7)
        self.count('{work();}', 2)
        self.count('switch(value){case LIMIT:return 1;default:return 0;}', 6)
        self.count("switch(value){case 'A':switch(other){case 1:return 3;default:return 4;}default:return 5;}", 11)
        self.gate('case 0:return 1;', 'c3-switch-arm-review')
        self.gate('switch(value){case 0..3:return 1;}', 'c3-switch-arm-review')
        self.gate('switch(value){case int:return 1;}', 'c3-switch-arm-review')
        self.gate('switch(value){case stateful():return 1;}', 'c3-switch-arm-review')
        self.gate('switch(value){case "A":return 1;}', 'c3-switch-arm-review')
        self.gate('switch(value){case `A`:return 1;}', 'c3-switch-arm-review')
        self.gate('switch(value) work();', 'c3-switch-arm-review')

    def test_nested_comments_literals_raw_ticks_and_exact_offsets(self):
        source = '/* outer /* nested */ end */\nString x="if(a){x=y;} // ";\nString y=`raw; `` tick {}`;'
        masked = mask_c3(source)
        self.assertEqual(len(masked['cleaned']), len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'], [i for i,c in enumerate(source) if c=='\n'])
        self.assertEqual(masked['diagnostics'], [])
        self.count(source, 2)
        self.count("char x=';';", 1)
        self.count('#!/usr/bin/c3c\nmodule bench;', 1)

    def test_contracts_and_macro_compile_time_forms_are_not_silent_comments(self):
        self.gate('<* @require x>0 *>\nfn int f(int x)=>x;', 'c3-documentation-contract-review')
        self.gate('$if FEATURE: int x=1; $endif', 'c3-compile-time-or-macro-review')
        self.gate('macro void twice(){first();second();}', 'c3-extended-statement-review')
        self.gate('asm{nop;}', 'c3-extended-statement-review')

    def test_unterminated_and_unbalanced_input_remains_gated(self):
        self.gate('String x="unfinished', 'c3-unterminated-literal')
        self.gate('/* outer /* inner */', 'c3-unterminated-block-comment')
        self.gate('fn void f(){work();', 'foreign-unclosed-delimiter')
        self.gate('int x=1\nint y=2;', 'c3-unproved-typed-declaration-review')
        self.gate('int x=1 int y=2;', 'c3-unproved-typed-declaration-review')

    def test_optional_unwrapping_unknown_attribute_and_switch_are_gated(self):
        self.gate('if(try result=call()){emit(result);}', 'c3-condition-binding-review')
        self.gate('fn void f() @unknown{work();}', 'c3-attribute-review')
        self.gate('switch(x){case SomeType:first();default:second();}', 'c3-switch-arm-review')

if __name__ == '__main__': unittest.main()
