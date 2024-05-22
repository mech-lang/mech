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

test_parser!(parse_literal_string, r#""Hello World""#, 64968622345197628);
test_parser!(parse_literal_string_escaped_quote, r#""Hello \" World""#, 9347612743027557);

test_parser!(parse_literal_true, "true", 1252109378846295);
test_parser!(parse_literal_false, "false", 18374905389476967);

test_parser!(parse_literal_empty, "_", 42646767556506866);

test_parser!(parse_kind_annotation, "10<m/s^2>", 23566671171775747);
test_parser!(parse_kind_annotation_size, "foo<u8:3,4>", 23754552381603812);
test_parser!(parse_kind_annotation_lhs, "z<u8> := 10", 1328561829991962);
test_parser!(parse_kind_annotation_both, "z<u8> := 10<u8>", 48854411622876658);

test_parser!(parse_range, "1:10", 2668291670556464);
test_parser!(parse_range_increment, "1:2:10", 49290182314426234);

test_parser!(parse_slice, "a[1]", 16516262270243137);
test_parser!(parse_slice_nested, "a[a[1]]", 13793932459857128);
test_parser!(parse_slice_3d, "a[1,2,3]", 66069081409915865);
test_parser!(parse_slice_range, "a[1:3]", 41984673176314825);

test_parser!(parse_table_empty, "[]", 59794664552129197);
test_parser!(parse_table_scalar_integer, "[123]", 66082959252429624);
test_parser!(parse_table_vector, "[1 2 3]", 26494628560603194);
test_parser!(parse_table_vector_transpose, "[1 2 3]'", 13707685070224489);
test_parser!(parse_table_vector_vars, "[a,b,c]", 70295520128197781);
test_parser!(parse_table_column_vector, "[1; 2; 3]", 55330048942590530);
test_parser!(parse_table_2x2, "[1 2; 3 4]", 27276319635453143);

test_parser!(parse_formula, "1 + 2 * 3", 16381879269635102);
test_parser!(parse_formula_vars, "a + b * c", 57105902454770510);
test_parser!(parse_formula_slices, "a[1] + b[2] * c", 20245506777075164);
test_parser!(parse_formula_paren_expr, "(1 + 2) * 3", 29006423147868544);

test_parser!(parse_record, "[a: 1, b: 2, c: 3]", 13220390494180657);
test_parser!(parse_record_column, r#"[a: 1
 b: 2
 c: 3]"#, 35126957775100680);
test_parser!(parse_record_nested, r#"[a: [a: 1 b: 2 c: 3] b: 2 c: 3]"#, 67293969229524370);

test_parser!(parse_statement_variable_define, "x := 123", 61318328524297221);
test_parser!(parse_statement_variable_assign, "a = 2", 61938044825647035);
test_parser!(parse_statement_variable_assign_slice, "a[1] = 2", 23943233967889861);


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
  Done(arr) -> arr."#, 67774157737283226);