#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use std::cell::RefCell;
use std::rc::Rc;
use mech_syntax::parser2;
use mech_syntax::interpreter::*;

  /// Compare interpreter output to expected value
  macro_rules! test_interpreter {
    ($func:ident, $input:tt, $expected:expr) => (
      #[test]
      fn $func() {
        let s = $input;
        match parser2::parse(&s) {
            Ok(tree) => { 
              let mut intrp = Interpreter::new();
              let result = intrp.interpret(&tree).unwrap();

              assert_eq!(result, $expected);
            },
            Err(err) => {panic!("{:?}", err);}
        }   
      }
    )
  }

/////////////////////////////////////////////////////////////////////////////////

test_interpreter!(interpret_literal_integer, "123", Value::Number(123));
//test_interpreter!(interpret_literal_string, "Hello", Value::String("Hello".to_string()));
test_interpreter!(interpret_literal_true, "true", Value::Bool(true));

test_interpreter!(interpret_formula_add, "1 + 1", Value::Number(2));
test_interpreter!(interpret_formula_sub, "1 - 1", Value::Number(0));
test_interpreter!(interpret_formula_neg_result, "0 - 1", Value::Number(-1));
test_interpreter!(interpret_formula_multiple_terms, "1 + 2 + 3", Value::Number(6));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::Number(123));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::Number(1));