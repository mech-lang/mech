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
use indexmap::set::IndexSet;
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

test_interpreter!(interpret_literal_integer, "123", Value::Number(123));
test_interpreter!(interpret_literal_string, r#""Hello""#, Value::String("Hello".to_string()));
test_interpreter!(interpret_literal_true, "true", Value::Bool(true));
test_interpreter!(interpret_literal_atom, "`A", Value::Atom(55450514845822917));

test_interpreter!(interpret_formula_add, "1 + 1", Value::Number(2));
test_interpreter!(interpret_formula_sub, "1 - 1", Value::Number(0));
test_interpreter!(interpret_formula_neg, "-1", Value::Number(-1));
test_interpreter!(interpret_formula_multiple_terms, "1 + 2 + 3", Value::Number(6));
test_interpreter!(interpret_formula_unicode, "😃:=1;🤦🏼‍♂️:=2;y̆és:=🤦🏼‍♂️ + 😃", Value::Number(3));
test_interpreter!(interpret_formula_gt, "10 > 11", Value::Bool(false));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::Number(123));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::Number(1));

test_interpreter!(interpret_matrix2, "[1 2; 3 4]", Value::Matrix(Matrix::Matrix2(Matrix2::from_vec(vec![1,3,2,4]))));
test_interpreter!(interpret_matrix2_transpose, "[1 2; 3 4]'", Value::Matrix(Matrix::Matrix2(Matrix2::from_vec(vec![1,2,3,4]))));
test_interpreter!(interpret_matrix2_negate, "-[1 2; 3 4]", Value::Matrix(Matrix::Matrix2(Matrix2::from_vec(vec![-1,-3,-2,-4]))));

test_interpreter!(interpret_tuple, "(1,true)", Value::Tuple(MechTuple::from_vec(vec![Value::Number(1), Value::Bool(true)])));
test_interpreter!(interpret_tuple_nested, r#"(1,("Hello",false))"#, Value::Tuple(MechTuple::from_vec(vec![Value::Number(1), Value::Tuple(MechTuple::from_vec(vec![Value::String("Hello".to_string()), Value::Bool(false)]))])));

test_interpreter!(interpret_slice, "a := [1,2,3]; a[2]", Value::Number(2));
test_interpreter!(interpret_slice_2d, "a := [1,2,3]; a[1,2]", Value::Number(2));

test_interpreter!(interpret_set,"{1,2,3}", Value::Set(MechSet::from_vec(vec![Value::Number(1),Value::Number(2),Value::Number(3)])));
test_interpreter!(interpret_record,r#"{a: 1, b: "Hello"}"#, Value::Record(MechMap::from_vec(vec![(Value::Id(55170961230981453),Value::Number(1)),(Value::Id(44311847522083591),Value::String("Hello".to_string()))])));
test_interpreter!(interpret_record_field_access,r#"a := {x: 1,  y: 2}; a.y"#, Value::Number(2));
test_interpreter!(interpret_map,r#"{"a": 1, "b": 2}"#, Value::Map(MechMap::from_vec(vec![(Value::String("a".to_string()),Value::Number(1)),(Value::String("b".to_string()),Value::Number(2))])));
