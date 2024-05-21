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



