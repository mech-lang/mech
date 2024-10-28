#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;
use mech_syntax::parser;
use std::env;

  /// Compare hashed parse tree traces
  macro_rules! test_parser {
    ($func:ident, $input:tt, $expected:expr) => (
      #[test]
      fn $func() {
        let s = $input;
        match parser::parse(&s) {
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
test_parser!(parse_literal_number_integer_neg, "-123", 31175819317228376);
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

test_parser!(parse_literal_negated, "-a", 28095792625106764);
test_parser!(parse_literal_negated_transpose, "-[a, b, c]'", 18032051464716518);
test_parser!(parse_literal_negated_multi, "-a + -b * -(-c - -b)", 29443585629021246);
test_parser!(parse_literal_negated_bool, "!a", 25781072094432306);

test_parser!(parse_literal_atom, "`A", 29631792893088166);

test_parser!(parse_literal_string, r#""Hello World""#, 64968622345197628);
test_parser!(parse_literal_string_escaped_quote, r#""Hello \" World""#, 9347612743027557);
test_parser!(parse_literal_string_escaped_backslash, r#""Hello \\" World""#, 69411547946998585);

test_parser!(parse_literal_true, "true", 1252109378846295);
test_parser!(parse_literal_false, "false", 18374905389476967);

test_parser!(parse_literal_empty, "_", 42646767556506866);
test_parser!(parse_literal_empty_multi, "_____", 16517769417968924);

test_parser!(parse_kind_annotation, "10<m/s^2>", 41451390958973903);
test_parser!(parse_kind_annotation_size, "foo<u8:3,4>", 8411444293349319);
test_parser!(parse_kind_annotation_lhs, "z<u8> := 10", 71403132938397338);
test_parser!(parse_kind_annotation_both, "z<u8> := 10<u8>", 35142481711361869);
test_parser!(parse_kind_annotation_tuple, "z<(u8,u8)>", 57987489394315533);
test_parser!(parse_kind_annotation_tuple_nested, "z<((u8,u8),u8)>", 64061479951167009);
test_parser!(parse_kind_annotation_empty, "z<_>", 15408982683395009);
test_parser!(parse_kind_annotation_atom, "z<`A>", 4301426029334554);
test_parser!(parse_kind_annotation_vector_dynamic, "z<[u8]:1,_>", 63916155854859674);
test_parser!(parse_kind_annotation_vector_3d, "z<[u8]:2,3,4>", 54073698199977163);
test_parser!(parse_kind_annotation_record, "z<{u8,string}:1,2>", 63423700931504609);
test_parser!(parse_kind_annotation_table, "z<{u8,string,[u8]:3}:3,3>", 65499865810553426);
test_parser!(parse_kind_annotation_set, "z<{u8}>", 16458084577498196);
test_parser!(parse_kind_annotation_map, "z<{string:u8}>", 24709209651786249);
test_parser!(parse_kind_annotation_map_nested, "z<{string:{string:u8}}>", 12787900685791211);
test_parser!(parse_kind_annotation_function, "z<(u8,u8)=(u8)>", 7505600452235802);

test_parser!(parse_range, "1..10", 12982910244952926);
test_parser!(parse_range_increment, "1..2..10", 55953760740329370);

test_parser!(parse_slice, "a[1]", 44592826654700758);
test_parser!(parse_slice_nested, "a[a[1]]", 68026688928900201);
test_parser!(parse_slice_3d, "a[1,2,3]", 22383159065466977);
test_parser!(parse_slice_range, "a[1..3]", 1587219038137663);
test_parser!(parse_slice_range_inclusive, "a[1..=3]", 53965541241723299);
test_parser!(parse_slice_dot, "a.b", 45658871590006420);
test_parser!(parse_slice_dot_chain, "a.b.c", 41359157262287512);
test_parser!(parse_slice_formula, "a[1 + 1]", 31476935489771180);
test_parser!(parse_slice_all, "a[:]", 59486585015065589);
test_parser!(parse_slice_multi, "a[:,1,1 + 1]", 32148654415944679);
test_parser!(parse_slice_logical, "a[[true false true]]", 22988697843305658);
test_parser!(parse_slice_swizzle, "a.x,x,y", 45252841116611977);
test_parser!(parse_slice_key, r#"a{"foo"}"#, 56850274883809298);
test_parser!(parse_slice_mega, r#"a.x.y[1,1 + 1,[1 2 3],1..3,1..=3].a,b,b,c{"foo"}"#, 68153246169941525);


test_parser!(parse_matrix_empty, "[]", 20166184779250868);
test_parser!(parse_matrix_scalar_integer, "[123]", 21675146718618610);
test_parser!(parse_matrix_vector, "[1 2 3]", 30693086370542127);
test_parser!(parse_matrix_vector_transpose, "[1 2 3]'", 35071653597190083);
test_parser!(parse_matrix_vector_transposes, "[A' + B' C']'", 39809887119621634);
test_parser!(parse_matrix_vector_vars, "[a,b,c]", 60770341462741177);
test_parser!(parse_matrix_column_vector, "[1; 2; 3]", 38030129547778653);
test_parser!(parse_matrix_2x2, "[1 2; 3 4]", 33529581225564663);
test_parser!(parse_matrix_tuples, "[(1,2), (3,4)]", 16921021255927653);

test_parser!(parse_set, "{1}", 35956285171394015);
test_parser!(parse_set_empty, "{_}", 46862853373603272);
test_parser!(parse_set_multiple_elements, "{1,2,3}", 27836895338180221);

test_parser!(parse_map, r#"{"a":10}"#, 40163603282332712);
test_parser!(parse_map_empty, "{}", 55962694842201166);
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
╰───┴───┴───╯"#,29958166929332346);
test_parser!(parse_matrix_fancy2,
r#"╭───┬───┬───╮
│ 1 │ 2 │ 3 │
│ 4 │ 5 │ 6 │
│ 7 │ 8 │ 9 │
╰───┴───┴───╯"#,60073775801197611);
test_parser!(parse_matrix_fancy3,
r#"╭───────────╮
│ 1   2   3 │
├───────────┤
│ 4   5   6 │
├───────────┤
│ 7   8   9 │
╰───────────╯"#,29958166929332346);
test_parser!(parse_matrix_fancy4,
r#"╭───────────╮
│ 1   2   3 │
│ 4   5   6 │
│ 7   8   9 │
╰───────────╯"#,60073775801197611);

test_parser!(parse_table_inline,r#"{x<f32> y<u8> | 1.2 9 ; 1.3 8 }"#,18779183519589985);
test_parser!(parse_table_empty, "{ x<f32> y<u8> | _ }", 15413160474115045);
test_parser!(parse_table,
r#"{x<f32> y<u8> |
1.2    9 
1.3    8   }"#,47449831800460666);
test_parser!(parse_table_header_fancy,
r#"╭───────────────────────────╮
│ x<u8>   y<string>  z<f32> │
├───────┬──────────┬────────┤
│   1   │  "a"     │ 3.14   │
├───────┼──────────┼────────┤
│   4   │  "b"     │ 6.15   │
├───────┼──────────┼────────┤
│   7   │  "c"     │ 9.19   │
╰───────┴──────────┴────────╯"#,40931531245879404);

test_parser!(parse_table_header_fancy_variable,
r#"x := 
╭─────────────────────────────────╮
│ x<u8>   y<string>  z<[u8]:1,3>  │
├───────┬──────────┬──────────────┤
│   1   │   "a"    │   [1 2 3]    │
├───────┼──────────┼──────────────┤
│   4   │   "b"    │   [4 5 6]    │
├───────┼──────────┼──────────────┤
│   7   │   "c"    │   [7 8 9]    │
╰───────┴──────────┴──────────────╯"#,37234775238921438);


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
test_parser!(parse_formula_slices, "a[1] + b[2] * c", 8392692448570008);
test_parser!(parse_formula_paren_expr, "(1 + 2) * 3", 37070150120883219);
test_parser!(parse_formula_cross, "a ⨯ b", 35316863412583795);
test_parser!(parse_formula_dot, "a · b", 32414125599205190);

test_parser!(parse_record, "{a: 1, b: 2, c: 3}", 26546496782427794);
test_parser!(parse_record_column, r#"{a: 1
 b: 2
 c: 3}"#, 34810819596571683);
test_parser!(parse_record_nested, r#"{a: {a: 1 b: 2 c: 3} b: 2 c: 3}"#, 42954662984815976);

test_parser!(parse_statement_variable_define, "x := 123", 62190040362503998);
test_parser!(parse_statement_variable_define_emoji, "Δx^2 := 123", 42324157149985255);
test_parser!(parse_statement_variable_define_annotated_tuple, "z<(u8, u8)> := (10,11)", 7667347478522863);
test_parser!(parse_statement_variable_define_annotated_tuple_both, "z<(u8, u16)> := (10<u8>,11<u16>)", 10505668968110378);
test_parser!(parse_statement_variable_define_annotated_tuple_rhs, "z := (10<u8>,11<u16>)", 30360312150734751);

test_parser!(parse_statement_variable_assign, "a = 2", 47312424597726258);
test_parser!(parse_statement_variable_assign_slice, "a[1] = 2", 38641881983528183);
test_parser!(parse_statement_kind_define, "<pos> := <(u8,u8,u8)>", 19272007189561377);
test_parser!(parse_statement_kind_define_size, "<foo> := <([u8]:1,2, [u8]:3,3)>", 19568235100036623);
test_parser!(parse_statement_kind_define_size_hex, "<bar> := <[foo]:0x01, 0xFF>", 52794480162632018);

test_parser!(parse_statement_enum_define, "<my-type> := A | B", 64572902068503820);
test_parser!(parse_statement_enum_define_typed, "<my-type> := A(<u8>) | B", 62980205579073513);
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
test_parser!(parse_statement_fsm_declare_args_kind, "#a<foo> := #b", 22678544250457847);
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
  Done(arr) => arr."#, 12713972902824909);

test_parser!(parse_function_define,r#"a() = b<c> := 
    a := 1;
    b := 2;
    c := 3."#, 62997233809516734);

test_parser!(parse_function_define_args,r#"foo(x<u8>, y<u8>) = z<u8> :=
    x2 := x + 1
    y2 := y + 2
    z := x2 + y2."#, 26397243087465788);

test_parser!(parse_function_define_inline,r#"a() = b<c> := a := 1;b := 2;c := 3."#, 36259460740270037);
    
test_parser!(parse_function_define_real,r#"time-update(μ<pose>, Σ<cov>) = (μ<pose>, Σ<cov>) :=
  θ :=  μ.θ
  Gt := [1  0 -u.v * math/sin(θ) * Δt
         0  1  u.v * math/cos(θ) * Δt
         0  0  1]
  Vt := [math/cos(θ) * Δt  0
         math/sin(θ) * Δt  0
         0                 Δt]
  μ := μ + u.v,v,ω * [math/cos(θ), math/sin(θ), 1] * Δt
  Σ := Gt ** Σ ** Gt' + Vt ** Q ** Vt'.

measurement-update(μ<[f32]:3>, Σ<[f32]:3,3>) = (μ<[f32]:3>, Σ<[f32]:3,3>) :=
  Δy := camera.y - μ.y
  Δx := camera.x - μ.x
  q := Δx ^ 2 + Δy ^ 2
  Ẑ := math/atan2(y: Δy, x: Δx) - μ.θ
  H := [Δy / q, -Δx / q, -1]
  S := H ** Σ ** H' + Q
  K := Σ ** H' / S
  μ := (μ + K * (z -  Ẑ))
  Σ := ([1 0 0; 0 1 0; 0 0 1] - K ** H) ** Σ."#,17138746356126439);

test_parser!(parse_comment_inline,r#"a := 0b10101   -- bin
b := 0x123abc  -- hex
c := 0o1234    -- oct
a := 0d1234    -- dec"#, 32628914429923900);

test_parser!(parse_comment_block,r#"/* Hello 
   World */

a := 123"#, 18698165613865065);