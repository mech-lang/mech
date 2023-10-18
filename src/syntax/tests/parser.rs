#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use mech_syntax::parser::{TextFormatter, parse};
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;

  /// Compare error locations (the reported row and col numbers).
  macro_rules! test_parser {
    ($func:ident, $input:tt, $($expected_err_loc:expr),*) => (
      #[test]
      fn $func() {
        let text = $input;
        let err_locations_exp = vec![$($expected_err_loc),*];
        let parse_result =  parse($input);
    
        // Parsing should succeed
        if (err_locations_exp.is_empty()) {
          assert!(parse_result.is_ok());
          return;
        }
    
        // Parsing should fail
        let error_report = match(parse_result) {
          Err(e) => match e.kind {
            MechErrorKind::ParserError(_, report, _) => report,
            _ => panic!("Expect mech error kind: ParserError"),
          }
          _ => panic!("Expect parser error"),
        };
    
        // Parser error should match with expected
        assert_eq!(error_report.len(), err_locations_exp.len());
        for i in 0..error_report.len() {
          let rng = error_report[i].cause_rng;
          // error range never ends at first column, so it's safe to `minus 1` here
          let reported_location = (rng.end.row, rng.end.col - 1);
          let expected_location = err_locations_exp[i];
          assert_eq!(reported_location, expected_location);
        }

        // Formatting function doesn't crash
        let tf = TextFormatter::new(text);
        let msg = tf.format_error(&error_report);
        assert_ne!(msg.len(), 0);
      }
    )
  }

/////////////////////////////////////////////////////////////////////////////////
test_parser!(err_empty_1, "", (1, 1));
test_parser!(err_empty_2, "\n", (1, 1));
test_parser!(err_empty_3, "\n\n  \n\n\n", (5, 1));
test_parser!(ok_simple_text, "Paragraph text", );

/////// LITERALS ///////
test_parser!(err_decimal_literal, r#"x = 0d0f1"#, (1, 8));
test_parser!(err_hexadecimal_literal, r#"x = 0x0g1"#, (1, 8));
test_parser!(err_octal_literal, r#"x = 0o081"#, (1, 8));
test_parser!(err_binary_literal, r#"x = 0b021"#, (1, 8));

///////// SUBSCRIPTS ///////
test_parser!(err_subscript_missing_index, r#"
block
  x = y{
  z = 7
"#, (3, 9));
test_parser!(err_subscript_missing_rbrace, r#"
block
  x = y{5 + 3
  z = 7
"#, (3, 14));
test_parser!(err_subscript_illegal_index, r#"
block
  x = y{$}
  z = 7
"#, (3, 9));

///////// DOT INDEX ///////
test_parser!(err_dot_index_missing_value, r#"
block
  x = y.
  z = 7
"#, (3, 9));
test_parser!(err_dot_index_illegal_value, r#"
block
  x = y.$
  z = 7
"#, (3, 9));

///////// SWIZZLE ///////
test_parser!(err_swizzle_missing_value_1, r#"
block
  x = a.b,
  z = 7
"#, (3, 11));
test_parser!(err_swizzle_missing_value_2, r#"
block
  x = a.b,c,
  z = 7
"#, (3, 12));
test_parser!(err_swizzle_illegal_value_1, r#"
block
  x = a.b,$
  z = 7
"#, (3, 11));
test_parser!(err_swizzle_illegal_value_2, r#"
block
  x = a.b,c,$
  z = 7
"#, (3, 12));

///////// KIND ANNOTATION ///////
test_parser!(err_kind_annotation_missing_value_1, r#"
block
  #x<> = 7
  z = 7
"#, (3, 6));
test_parser!(err_kind_annotation_missing_value_2, r#"
block
  #x<u32,u64,> = 7
  z = 7
"#, (3, 13));

///////// TABLE ///////
test_parser!(err_table_missing_name, r#"
block
  # = 7
  z = 7
"#, (3, 4));

///////// TABLE BINDING ///////
test_parser!(err_binding_extra_space_before_colon, r#"
block
  x = [a : 7, b: 8]
  z = 7
"#, (3, 9));
test_parser!(err_binding_missing_value, r#"
block
  x = [a: , b: 8]
  z = 7
"#, (3, 11));
test_parser!(err_binding_missing_separater, r#"
block
  x = [a: 8b: 8]
  z = 7
"#, (3, 12));
test_parser!(err_binding_missing_space_after_comma, r#"
block
  x = [a: 8,b: 8]
  z = 7
"#, (3, 13));
test_parser!(err_binding_missing_space_after_comma_sp, r#"
block
  x = [a: u.u1,b: 8]
  z = 7
"#, (3, 18));
test_parser!(err_binding_missing_after_second_colon, r#"
block
  x = [a: u.u1, b:8]
  z = 7
"#, (3, 19));

///////// FUNCTION BINDING ///////
test_parser!(err_function_binding_missing_colon, r#"
block
  x = math/sin(angle 90)
  z = 7
"#, (3, 21));
test_parser!(err_function_binding_missing_space, r#"
block
  x = math/sin(angle:90)
  z = 7
"#, (3, 22));
test_parser!(err_function_binding_missing_value, r#"
block
  x = math/sin(angle: )
  z = 7
"#, (3, 23));

///////// FUNCTION ///////
test_parser!(err_function_no_args, r#"
block
  x = math/sin()
  z = 7
"#, (3, 16));
test_parser!(err_function_unmatched_paren, r#"
block
  x = math/sin(angle: (((1 + 3) * 2))
  z = 7
"#, (3, 38));

///////// AMBIGIOUS TABLE ///////
test_parser!(ok_indexing_complex, r#"
block
  u = [u1: 1, u2: 2, u3: 3]
  t = [t1: u.u1, u2: 2, t3: u.u3]
  x = t.t1,t2,t3
  z = 7
"#,);
test_parser!(err_ambigious_table_as_annonymous, r#"
block
  u = [u1:1, u2: 2, u3: 3]
  z = 7
"#, (3, 17), (3, 24));
test_parser!(err_ambigious_table_as_inline, r#"
block
  u = [u1: 1, u2:2, u3: 3]
  z = 7
"#, (3, 18));
test_parser!(ok_ambigious_table_as_anonymous_ranges, r#"
block
  t = [ta:u.ua,tb:u.ub,tc:u.uc]
"#,);
test_parser!(ok_ambigious_table_as_inline_ranges, r#"
block
  t = [ta: u.ua,tb:u.ub,tc:u.uc]
"#,);
// NOTE: This test justifies a bad parser behavior.  Intuitively, the test input should
// be interpreted as inline table with 3 bindings (ta, tb, tc) and the error should be
// the missing space after each comma.  However by our grammar this is recongnized as
// inline table with a single binding (ta), and it prompts user to remove spaces after
// the colons.
test_parser!(err_ambigious_table_as_inline_range_err, r#"
block
  t = [ta: u.ua,tb: u.ub,tc: u.uc]
"#, (3, 20), (3, 29));

///////// COMMENT ///////
test_parser!(err_comment_missing_content, r#"
block
  x = 1
  --
  z =2
"#, (4, 5), (5, 6));
test_parser!(err_comment_illegal_content, r#"
block
  x = 1
  --abc$def
  z =2
"#, (4, 8), (5, 6));

///////// USER FUNCTION ///////
test_parser!(ok_user_function, r#"
[a<f32>] = foo(y<f32>)
  a = y * 3

[a<f32>] = bar(b<f32>)
  a = foo(y: b) + 2

block
  #y = bar(b: 20)
"#, );
test_parser!(err_user_function_missing_output_kind, r#"
[a<f32>,b] = foo(y<f32>)
  a = y * 3
"#, (2, 10));
test_parser!(err_user_function_missing_right_bracket, r#"
[a<f32> = foo(y<f32>)
  a = y * 3
"#, (2, 9));
test_parser!(err_user_function_no_space, r#"
[a<f32>]=foo(y<f32>)
  a = y * 3
"#, (2, 9));
test_parser!(err_user_function_missing_name, r#"
[a<f32>] = (y<f32>)
  a = y * 3
"#, (2, 12));
test_parser!(err_user_function_missing_paren, r#"
[a<f32>] = foo y<f32>
  a = y * 3
"#, (2, 15));
test_parser!(err_user_function_one_line_1, r#"
[a<f32>] = foo(y<f32>) a = y * 3
"#, (2, 23));
test_parser!(err_user_function_one_line_2, r#"
[a<f32>] bar() = foo(y: b) + 2
"#, (2, 10));
test_parser!(err_user_function_missing_body, r#"
[a<f32>] = foo(y<f32>)

block
  #y = bar(b: 20)
"#, (3, 1));
test_parser!(err_user_function_error_in_body, r#"
[a<f32>] = foo(y<f32>)
    a = y + 1

block
  #y = bar(b: 20)
"#, (3, 3));

///////// ERROR RECOVERY ///////
test_parser!(err_section_recovery_too_many_titles_1, r#"
Title
===========

block
  #x = 5

Title2
===========
"#, (9, 1));
test_parser!(err_section_recovery_too_many_titles_2, r#"
Title
===========

block
  #x = 5

Title2
===========

block
  #y = ()
"#, (9, 1), (12, 9));
/////////////////////////////////////////////////////////////////////////////////