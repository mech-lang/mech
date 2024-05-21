#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;
use mech_syntax::parser2;

  /// Compare error locations (the reported row and col numbers).
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
test_parser!(parse_literal_number_float, "123.456", 49724774253782161);
test_parser!(parse_literal_number_rational, "123/456", 38518217377960831);

test_parser!(parse_literal_string, r#""Hello World""#, 64968622345197628);
test_parser!(parse_literal_string_escaped_quote, r#""Hello \" World""#, 9347612743027557);

test_parser!(parse_literal_true, "true", 1252109378846295);
test_parser!(parse_literal_false, "false", 18374905389476967);

test_parser!(parse_literal_empty, "_", 42646767556506866);

test_parser!(parse_table_empty, "[]", 59794664552129197);
test_parser!(parse_table_scalar_integer, "[123]", 66082959252429624);
test_parser!(parse_table_vector, "[1 2 3]", 26494628560603194);
test_parser!(parse_table_column_vector, "[1; 2; 3]", 55330048942590530);
test_parser!(parse_table_2x2, "[1 2; 3 4]", 27276319635453143);

test_parser!(parse_record, "[a: 1, b: 2, c: 3]", 13220390494180657);
test_parser!(parse_record_column, r#"[a: 1
 b: 2
 c: 3]"#, 35126957775100680);
test_parser!(parse_record_nested, r#"[a: [a: 1 b: 2 c: 3] b: 2 c: 3]"#, 67293969229524370);

test_parser!(parse_statement_variable_define, "x = 123", 1173180602711415);

test_parser!(parse_mechdown_paragraph, "Hello World", 44055055244553644);

test_parser!(parse_mechdown_heading, r#"Hello World
=============

This is a program."#, 33399644466523221);

test_parser!(parse_mechdown_unordered_list, r#"- one
- two
- three"#, 32571997588793248);