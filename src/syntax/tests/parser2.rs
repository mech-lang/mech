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
            Err(err) => {panic!("{:?}", err);}
        }   
      }
    )
  }

/////////////////////////////////////////////////////////////////////////////////

test_parser!(parse_identifier, "abc", 48171905589132044);
test_parser!(parse_identifier_number, "abc123", 32986501350920458);
test_parser!(parse_identifier_dash, "dash-delinates-words", 19078698641356489);
test_parser!(parse_identifier_slash, "slash/delinates/scope", 71507821257673205);
test_parser!(parse_identifier_qualified, "io/print", 72034991961564183);
test_parser!(parse_identifier_emoji, "🤖", 12564702933130716);
test_parser!(parse_identifier_star, "A*", 47514170547507386);
test_parser!(parse_identifier_greek, "Δx^2", 34800204971269505);

test_parser!(parse_literal_number_integer, "123", 47158019211217915);
test_parser!(parse_literal_number_integer_neg, "-123", 35870853261236691);
test_parser!(parse_literal_number_float, "123.456", 35039068852936934);
test_parser!(parse_literal_number_rational, "123/456", 51796036056154014);
test_parser!(parse_literal_number_hex, "0x1234567890ABCDEF", 43012669827828490);
test_parser!(parse_literal_number_dec, "0d1234567890", 54131753369551440);
test_parser!(parse_literal_number_oct, "0o12345670", 46511574515455561);
test_parser!(parse_literal_number_bin, "0b1010101", 17669206941430379);
test_parser!(parse_literal_number_sci, "123.456E789", 48647166186388907);
test_parser!(parse_literal_number_underscores, "1_000_000", 1283988130449453);
test_parser!(parse_literal_number_imaginary, "1234i", 56634961804533704);
test_parser!(parse_literal_number_complex, "1234+567i", 52164299928210322);
test_parser!(parse_literal_number_complex_fractions, "12.34+5.67i", 49522358830348899);
test_parser!(parse_literal_number_hex_underscores, "0xAB_CD_EF_GH", 11525069796638697);

test_parser!(parse_literal_atom, "`A", 29631792893088166);

test_parser!(parse_literal_string, r#""Hello World""#, 64968622345197628);
test_parser!(parse_literal_string_escaped_quote, r#""Hello \" World""#, 9347612743027557);
test_parser!(parse_literal_string_escaped_backslash, r#""Hello \\" World""#, 69411547946998585);

test_parser!(parse_literal_true, "true", 1252109378846295);
test_parser!(parse_literal_false, "false", 18374905389476967);

test_parser!(parse_literal_empty, "_", 42646767556506866);

test_parser!(parse_kind_annotation, "10<m/s^2>", 71616930996186052);
test_parser!(parse_kind_annotation_size, "foo<u8:3,4>", 47950682744332223);
test_parser!(parse_kind_annotation_lhs, "z<u8> := 10", 23247483275563923);
test_parser!(parse_kind_annotation_both, "z<u8> := 10<u8>", 1385214578996481);
test_parser!(parse_kind_annotation_tuple, "z<(u8,u8)>", 65064718600897177);
test_parser!(parse_kind_annotation_tuple_nested, "z<((u8,u8),u8)>", 23648802851573596);

test_parser!(parse_range, "1..10", 5844225421276229);
test_parser!(parse_range_increment, "1..2..10", 47858485653184183);

test_parser!(parse_slice, "a[1]", 70005361819083756);
test_parser!(parse_slice_nested, "a[a[1]]", 66601766791999547);
test_parser!(parse_slice_3d, "a[1,2,3]", 45450758362188861);
test_parser!(parse_slice_range, "a[1..3]", 16760939247954643);

test_parser!(parse_matrix_empty, "[]", 20166184779250868);
test_parser!(parse_matrix_scalar_integer, "[123]", 54310964423322192);
test_parser!(parse_matrix_vector, "[1 2 3]", 13246292939325121);
test_parser!(parse_matrix_vector_transpose, "[1 2 3]'", 35774200196470992);
test_parser!(parse_matrix_vector_vars, "[a,b,c]", 49551394880404050);
test_parser!(parse_matrix_column_vector, "[1; 2; 3]", 71460606371207459);
test_parser!(parse_matrix_2x2, "[1 2; 3 4]", 55450029560457659);
test_parser!(parse_matrix_tuples, "[(1,2), (3,4)]", 54992719886778865);

test_parser!(parse_set, "{1}", 35956285171394015);
test_parser!(parse_set_empty, "{_}", 46610421933005859);
test_parser!(parse_set_multiple_elements, "{1,2,3}", 27836895338180221);

test_parser!(parse_map, r#"{"a":10}"#, 40163603282332712);
test_parser!(parse_map_empty, "{}", 46610421933005859);
test_parser!(parse_map_multiple_elements, r#"{"a":10, "b":20, "c": 30}"#, 14390678166496455);
test_parser!(parse_map_vert, r#"{"a":10 
"b":20
"c": 30}"#, 5426766675804062);
test_parser!(parse_map_nested, r#"{"a":{"a":10}}"#, 28398496069573765);
test_parser!(parse_map_nested_multiline, r#"{"a":
    {
        "a":10
    }
}"#, 15836217214311594);

test_parser!(parse_function_call, "a(b)", 11123136008908087);
test_parser!(parse_function_call_nested, "a(a(a(a(a(a(a(a(a))))))))", 37209815955448119);
test_parser!(parse_function_call_multi_args, "a(b,c,d)", 63772171465944686);
test_parser!(parse_function_call_named_args, "a(b: 1, c: 2 ,d: 3)", 23007367454260292);

test_parser!(parse_mega_formula, "((2 + 3 * sin(3.14)) > (10 - 3 + cos(2 * 3.14))) & (4 < 5 | (2 ^ 3 + 10 / 2) == 5) & (tan(45) < 1 | log(10, 2) + 3 > 5) & (sqrt(16) - 2 == 2 | 3 * 2 - 5 + 1 != 2 + 1)", 33778397953693497);

test_parser!(parse_matrix_fancy1,
r#"╭───┬───┬───╮
│ 1 │ 2 │ 3 │
├───┼───┼───┤
│ 4 │ 5 │ 6 │
├───┼───┼───┤
│ 7 │ 8 │ 9 │
╰───┴───┴───╯"#,6793140575280764);
test_parser!(parse_matrix_fancy2,
r#"╭───┬───┬───╮
│ 1 │ 2 │ 3 │
│ 4 │ 5 │ 6 │
│ 7 │ 8 │ 9 │
╰───┴───┴───╯"#,16339589348817943);
test_parser!(parse_matrix_fancy3,
r#"╭───────────╮
│ 1   2   3 │
├───────────┤
│ 4   5   6 │
├───────────┤
│ 7   8   9 │
╰───────────╯"#,6793140575280764);
test_parser!(parse_matrix_fancy4,
r#"╭───────────╮
│ 1   2   3 │
│ 4   5   6 │
│ 7   8   9 │
╰───────────╯"#,16339589348817943);

test_parser!(parse_table_inline,r#"{x<f32> y<u8> | 1.2 9 ; 1.3 8 }"#,12469378324178801);
test_parser!(parse_table_empty, "{ x<f32> y<u8> | _ }", 49124109782989357);
test_parser!(parse_table,
r#"{x<f32> y<u8> |
1.2    9 
1.3    8   }"#,60702597920596872);
test_parser!(parse_table_header_fancy,
r#"╭───────────────────────────╮
│ x<u8>   y<string>  z<f32> │
├───────┬──────────┬────────┤
│   1   │  "a"     │ 3.14   │
├───────┼──────────┼────────┤
│   4   │  "b"     │ 6.15   │
├───────┼──────────┼────────┤
│   7   │  "c"     │ 9.19   │
╰───────┴──────────┴────────╯"#,64660003063383141);

test_parser!(parse_table_header_fancy_variable,
r#"x := 
╭─────────────────────────────╮
│ x<u8>   y<string>  z<u8:3>  │
├───────┬──────────┬──────────┤
│   1   │  "a"     │ [1 2 3]  │
├───────┼──────────┼──────────┤
│   4   │  "b"     │ [4 5 6]  │
├───────┼──────────┼──────────┤
│   7   │  "c"     │ [7 8 9]  │
╰───────┴──────────┴──────────╯"#,60046088409113153);


test_parser!(parse_tuple_empty, "()", 46625237035827900);
test_parser!(parse_tuple_scalar, "(1)", 16922997190951392);
test_parser!(parse_tuple_three, "(1,2,3)", 53311318706243013);
test_parser!(parse_tuple_nested, "(1,(2,3))", 39749631098848845);
test_parser!(parse_tuple_hetero, r#"(1, true, "Hello")"#, 21595373940482904);
test_parser!(parse_tuple_hetero_nested, r#"(1, (true, "Hello"))"#, 8881324351186990);
test_parser!(parse_tuple_expressions, r#"(1 + 2, x > y, true | false)"#, 61094866984856895);

test_parser!(parse_tuple_struct, "`A(1)", 46038618144278401);
test_parser!(parse_tuple_struct_tuple, "`A((1,2,3))", 21800174027292727);

test_parser!(parse_formula, "1 + 2 * 3", 35118734439232812);
test_parser!(parse_formula_vars, "a + b * c", 26596788877301348);
test_parser!(parse_formula_slices, "a[1] + b[2] * c", 22590931187307951);
test_parser!(parse_formula_paren_expr, "(1 + 2) * 3", 37070150120883219);

test_parser!(parse_record, "{a: 1, b: 2, c: 3}", 26546496782427794);
test_parser!(parse_record_column, r#"{a: 1
 b: 2
 c: 3}"#, 34810819596571683);
test_parser!(parse_record_nested, r#"{a: {a: 1 b: 2 c: 3} b: 2 c: 3}"#, 42954662984815976);

test_parser!(parse_statement_variable_define, "x := 123", 62190040362503998);
test_parser!(parse_statement_variable_define_emoji, "Δx^2 := 123", 42324157149985255);
test_parser!(parse_statement_variable_define_annotated_tuple, "z<(u8, u8)> := (10,11)", 70189018235132426);
test_parser!(parse_statement_variable_define_annotated_tuple_both, "z<(u8, u16)> := (10<u8>,11<u16>)", 6440057661285952);
test_parser!(parse_statement_variable_define_annotated_tuple_rhs, "z := (10<u8>,11<u16>)", 68216837866507296);

test_parser!(parse_statement_variable_assign, "a = 2", 5448552719387223);
test_parser!(parse_statement_variable_assign_slice, "a[1] = 2", 45199166767527015);
test_parser!(parse_statement_kind_define, "<pos> := <(u8,u8,u8)>", 62624658898678961);
test_parser!(parse_statement_kind_define_size, "<foo> := <(u8:1,2, u8:3,3)>", 59365484348435996);
test_parser!(parse_statement_kind_define_size_hex, "<bar> := <foo:0x01, 0xFF>", 42590915248956376);

test_parser!(parse_statement_enum_define, "<my-type> := A | B", 64572902068503820);
test_parser!(parse_statement_enum_define_typed, "<my-type> := A(<u8>) | B", 41352039959953377);
test_parser!(parse_statement_enum_define_grave, "<my-type> := `A | `B", 24306883787841449);

test_parser!(parse_fsm_instance, "#a", 25265165668657972);
test_parser!(parse_fsm_instance_args, "#a(a,b,c)", 3224655551426456);
test_parser!(parse_fsm_instance_args_named, "#a(foo: 1, bar: 2)", 71278746694291017);
test_parser!(parse_fsm_pipe_transition, "#a -> #b", 428983498488167);
test_parser!(parse_fsm_pipe_async, "#a ~> #b", 49667412332171174);
test_parser!(parse_fsm_pipe_out, "#a => #b", 52005814634192125);
test_parser!(parse_fsm_pipe_all, "#a -> #b ~> #c => #d", 27532108367535129);

test_parser!(parse_statement_fsm_declare, "#a := #b", 52031770603132409);
test_parser!(parse_statement_fsm_declare_args, "#a := #b(a,b,c)", 57320567753593041);
test_parser!(parse_statement_fsm_declare_args_named, "#a := #b(foo: 1, bar: 2)", 20318356213698921);
test_parser!(parse_statement_fsm_declare_args_kind, "#a<foo> := #b", 3182624260594038);
test_parser!(parse_statement_fsm_declare_pipe, "#a := #b -> #c", 4372245302078996);
test_parser!(parse_statement_fsm_declare_pipe_output, "#a := #b -> #c -> #d => out", 28918541910487698);

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

test_parser!(parse_fsm_specification,
r#"#bubble-sort(arr) => arr := 
    │ Start(arr,ix)
    │ Comparison(arr,ix) 
    │ Check(arr,ix)
    └ Done(arr)."#, 36857026007363078);

test_parser!(parse_fsm_implementation,
r#"#bubble-sort(arr) -> Start(arr)
  Start(arr, swaps) -> Comparison(arr, swaps)
  Comparison([], swaps) -> Check(arr, swaps)
  Comparison([a, b, tail], swaps)
      │ a > b -> Comparison([b, a, tail], swaps + 1)
      └ * -> Comparison([tail], swaps)
  Check(arr, 0) -> Done(arr)
  Check(arr, swaps) -> Comparison(arr,0)
  Done(arr) => arr."#, 44068451390416423);

test_parser!(parse_function_define,r#"a() = b<c> := 
    a := 1;
    b := 2;
    c := 3."#, 63243368957527106);

test_parser!(parse_function_define_args,r#"foo(x<u8>, y<u8>) = z<u8> :=
    x2 := x + 1
    y2 := y + 2
    z := x2 + y2."#, 7012778753667092);

test_parser!(parse_function_define_inline,r#"a() = b<c> := a := 1;b := 2;c := 3."#, 50534530857365659);
    