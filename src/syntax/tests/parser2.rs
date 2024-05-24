#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;
use mech_syntax::parser2;

  /// Compare hashed parse tree traces
  macro_rules! test_parser {
    ($func:ident, $input:tt, $expected:expr) => (
      #[test]
      fn $func() {
        let s = $input;
        match parser2::parse(&s) {
            Ok(tree) => { 
              let hashed_parse = hash_str(&format!("{:#?}", tree));
              assert_eq!(hashed_parse, $expected);
            },
            Err(err) => {panic!("Should have worked");}
        }   
      }
    )
  }

/////////////////////////////////////////////////////////////////////////////////

test_parser!(parse_literal_number_integer, "123", 62568158498624598);
test_parser!(parse_literal_number_integer_neg, "-123", 16685225171239470);
test_parser!(parse_literal_number_float, "123.456", 49724774253782161);
test_parser!(parse_literal_number_rational, "123/456", 38518217377960831);
test_parser!(parse_literal_number_hex, "0x1234567890ABCDEF", 10208025603092252);
test_parser!(parse_literal_number_dec, "0d1234567890", 57432846543525412);
test_parser!(parse_literal_number_oct, "0o12345670", 36107841685676795);
test_parser!(parse_literal_number_bin, "0b1010101", 51428896740892327);
test_parser!(parse_literal_number_sci, "123.456E789", 16735846146196743);
test_parser!(parse_literal_number_underscores, "1_000_000", 17117948062822050);
test_parser!(parse_literal_number_bin_leading_zeros, "0b00010101", 31115173340120627);

test_parser!(parse_literal_string, r#""Hello World""#, 64968622345197628);
test_parser!(parse_literal_string_escaped_quote, r#""Hello \" World""#, 9347612743027557);
test_parser!(parse_literal_string_escaped_backslash, r#""Hello \\" World""#, 69411547946998585);

test_parser!(parse_literal_true, "true", 1252109378846295);
test_parser!(parse_literal_false, "false", 18374905389476967);

test_parser!(parse_literal_empty, "_", 42646767556506866);

test_parser!(parse_kind_annotation, "10<m/s^2>", 23566671171775747);
test_parser!(parse_kind_annotation_size, "foo<u8:3,4>", 23754552381603812);
test_parser!(parse_kind_annotation_lhs, "z<u8> := 10", 1328561829991962);
test_parser!(parse_kind_annotation_both, "z<u8> := 10<u8>", 48854411622876658);
test_parser!(parse_kind_annotation_tuple, "z<(u8,u8)>", 65064718600897177);
test_parser!(parse_kind_annotation_tuple_nested, "z<((u8,u8),u8)>", 23648802851573596);

test_parser!(parse_range, "1:10", 66930478939264165);
test_parser!(parse_range_increment, "1:2:10", 18888666111954368);

test_parser!(parse_slice, "a[1]", 16516262270243137);
test_parser!(parse_slice_nested, "a[a[1]]", 13793932459857128);
test_parser!(parse_slice_3d, "a[1,2,3]", 66069081409915865);
test_parser!(parse_slice_range, "a[1:3]", 42134448022382694);

test_parser!(parse_empty_table, "[]", 46610421933005859);
test_parser!(parse_matrix_scalar_integer, "[123]", 13075771302721700);
test_parser!(parse_matrix_vector, "[1 2 3]", 58888609671561603);
test_parser!(parse_matrix_vector_transpose, "[1 2 3]'", 51008949150648919);
test_parser!(parse_matrix_vector_vars, "[a,b,c]", 49551394880404050);
test_parser!(parse_matrix_column_vector, "[1; 2; 3]", 24137050493281632);
test_parser!(parse_matrix_2x2, "[1 2; 3 4]", 12435940958099457);
test_parser!(parse_matrix_tuples, "[(1,2), (3,4)]", 65497773797987574);

test_parser!(parse_function_call, "a(b)", 11123136008908087);
test_parser!(parse_function_call_nested, "a(a(a(a(a(a(a(a(a))))))))", 37209815955448119);
test_parser!(parse_function_call_multi_args, "a(b,c,d)", 63772171465944686);
test_parser!(parse_function_call_named_args, "a(b: 1, c: 2 ,d: 3)", 51521379098738857);

test_parser!(parse_mega_formula, "((2 + 3 * sin(3.14)) > (10 - 3 + cos(2 * 3.14))) & (4 < 5 | (2 ^ 3 + 10 / 2) == 5) & (tan(45) < 1 | log(10, 2) + 3 > 5) & (sqrt(16) - 2 == 2 | 3 * 2 - 5 + 1 != 2 + 1)", 49936918439020726);

test_parser!(parse_matrix_fancy1,
r#"╭───┬───┬───╮
│ 1 │ 2 │ 3 │
├───┼───┼───┤
│ 4 │ 5 │ 6 │
├───┼───┼───┤
│ 7 │ 8 │ 9 │
╰───┴───┴───╯"#,44412314364066378);
test_parser!(parse_matrix_fancy2,
r#"╭───┬───┬───╮
│ 1 │ 2 │ 3 │
│ 4 │ 5 │ 6 │
│ 7 │ 8 │ 9 │
╰───┴───┴───╯"#,64979024920836908);
test_parser!(parse_matrix_fancy3,
r#"╭───────────╮
│ 1   2   3 │
├───────────┤
│ 4   5   6 │
├───────────┤
│ 7   8   9 │
╰───────────╯"#,44412314364066378);
test_parser!(parse_matrix_fancy4,
r#"╭───────────╮
│ 1   2   3 │
│ 4   5   6 │
│ 7   8   9 │
╰───────────╯"#,64979024920836908);

test_parser!(parse_table_inline,r#"[x<f32> y<u8> | 1.2 9 ; 1.3 8 ]"#,65004581493517295);
test_parser!(parse_table_empty, "[ x<f32> y<u8> | _ ]", 49124109782989357);
test_parser!(parse_table,
r#"[x<f32> y<u8> 
1.2    9 
1.3    8   ]"#,414818813821773);
test_parser!(parse_table_header_facy,
r#"╭───────────────────────────╮
│ x<u8>   y<string>  z<f32> │
├───────┬──────────┬────────┤
│   1   │  "a"     │ 3.14   │
├───────┼──────────┼────────┤
│   4   │  "b"     │ 6.15   │
├───────┼──────────┼────────┤
│   7   │  "c"     │ 9.19   │
╰───────┴──────────┴────────╯"#,20506962846954977);

test_parser!(parse_table_header_facy_variable,
r#"x := 
╭─────────────────────────────╮
│ x<u8>   y<string>  z<u8:3>  │
├───────┬──────────┬──────────┤
│   1   │  "a"     │ [1 2 3]  │
├───────┼──────────┼──────────┤
│   4   │  "b"     │ [4 5 6]  │
├───────┼──────────┼──────────┤
│   7   │  "c"     │ [7 8 9]  │
╰───────┴──────────┴──────────╯"#,31210612369796226);


test_parser!(parse_tuple_empty, "()", 46625237035827900);
test_parser!(parse_tuple_scalar, "(1)", 41050214404370146);
test_parser!(parse_tuple_three, "(1,2,3)", 34973964530646587);
test_parser!(parse_tuple_nested, "(1,(2,3))", 1496208466301128);
test_parser!(parse_tuple_hetero, r#"(1, true, "Hello")"#, 1090619636774422);
test_parser!(parse_tuple_hetero_nested, r#"(1, (true, "Hello"))"#, 52985721568108321);
test_parser!(parse_tuple_expressions, r#"(1 + 2, x > y, true | false)"#, 27548167311049490);

test_parser!(parse_formula, "1 + 2 * 3", 53314686960653757);
test_parser!(parse_formula_vars, "a + b * c", 26596788877301348);
test_parser!(parse_formula_slices, "a[1] + b[2] * c", 14997858166465448);
test_parser!(parse_formula_paren_expr, "(1 + 2) * 3", 22002356562256589);

test_parser!(parse_record, "[a: 1, b: 2, c: 3]", 23513968729906793);
test_parser!(parse_record_column, r#"[a: 1
 b: 2
 c: 3]"#, 41121906894714823);
test_parser!(parse_record_nested, r#"[a: [a: 1 b: 2 c: 3] b: 2 c: 3]"#, 34734170064490835);

test_parser!(parse_statement_variable_define, "x := 123", 61318328524297221);
test_parser!(parse_statement_variable_define_annotated_tuple, "z<(u8, u8)> := (10,11)", 5743532714881875);
test_parser!(parse_statement_variable_define_annotated_tuple_both, "z<(u8, u16)> := (10<u8>,11<u16>)", 58071829579428918);
test_parser!(parse_statement_variable_define_annotated_tuple_rhs, "z := (10<u8>,11<u16>)", 64984498152023673);

test_parser!(parse_statement_variable_assign, "a = 2", 61938044825647035);
test_parser!(parse_statement_variable_assign_slice, "a[1] = 2", 23943233967889861);
test_parser!(parse_statement_kind_define, "<pos> := <(u8,u8,u8)>", 62624658898678961);
test_parser!(parse_statement_kind_define_size, "<foo> := <(u8:1,2, u8:3,3)>", 37979414279321074);
test_parser!(parse_statement_kind_define_size_hex, "<bar> := <foo:0x01, 0xFF>", 62296951259330595);


test_parser!(parse_mechdown_paragraph, "Hello World", 44055055244553644);

test_parser!(parse_mechdown_heading, r#"Hello World
=============

This is a program."#, 33399644466523221);

test_parser!(parse_mechdown_subheadings, r#"A
====

1. B
----

(a) C

A thing"#, 31292392503547082);

test_parser!(parse_mechdown_unordered_list, r#"- one
- two
- three"#, 32571997588793248);

test_parser!(parse_fsm_implementation,
r#"#bubble-sort(arr) => Start(arr)
  Start(arr, swaps) => Comparison(arr, swaps)
  Comparison([], swaps) => Check(arr, swaps)
  Comparison([a, b, tail], swaps)
      │ a > b => Comparison([b, a, tail], swaps + 1)
      └ * => Comparison([tail], swaps)
  Check(arr, 0) => Done(arr)
  Check(arr, swaps) => Comparison(arr,0)
  Done(arr) -> arr."#, 57218274469469508);