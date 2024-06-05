#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
extern crate nalgebra as na;
use std::cell::RefCell;
use std::rc::Rc;
use mech_syntax::parser2;
use mech_syntax::interpreter::*;
use na::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};

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

test_interpreter!(interpret_literal_integer, "123", Value::Number(123.0));
//test_interpreter!(interpret_literal_string, "Hello", Value::String("Hello".to_string()));
test_interpreter!(interpret_literal_true, "true", Value::Bool(true));

test_interpreter!(interpret_formula_add, "1 + 1", Value::Number(2.0));
test_interpreter!(interpret_formula_sub, "1 - 1", Value::Number(0.0));
test_interpreter!(interpret_formula_neg_result, "0 - 1", Value::Number(-1.0));
test_interpreter!(interpret_formula_multiple_terms, "1 + 2 + 3", Value::Number(6.0));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::Number(123.0));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::Number(1.0));

test_interpreter!(interpret_matrix2, "[1 2; 3 4]", Value::Matrix(Matrix::Matrix2(Matrix2::from_vec(vec![1.0,3.0,2.0,4.0]))));
test_interpreter!(interpret_matrix2_transpose, "[1 2; 3 4]'", Value::Matrix(Matrix::Matrix2(Matrix2::from_vec(vec![1.0,2.0,3.0,4.0]))));
