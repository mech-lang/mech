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

test_interpreter!(interpret_literal_integer, "123", Value::I64(Rc::new(RefCell::new(123))));
test_interpreter!(interpret_literal_string, r#""Hello""#, Value::String("Hello".to_string()));
test_interpreter!(interpret_literal_true, "true", Value::Bool(Rc::new(RefCell::new(true))));
test_interpreter!(interpret_literal_false, "false", Value::Bool(Rc::new(RefCell::new(false))));
test_interpreter!(interpret_literal_atom, "`A", Value::Atom(55450514845822917));
test_interpreter!(interpret_literal_empty, "_", Value::Empty);

test_interpreter!(interpret_formula_math_add, "2 + 2", Value::I64(Rc::new(RefCell::new(4))));
test_interpreter!(interpret_formula_math_sub, "2 - 2", Value::I64(Rc::new(RefCell::new(0))));
test_interpreter!(interpret_formula_math_mul, "2 * 2", Value::I64(Rc::new(RefCell::new(4))));
test_interpreter!(interpret_formula_math_div, "2 / 2", Value::I64(Rc::new(RefCell::new(1))));
test_interpreter!(interpret_formula_math_exp, "2 ^ 2", Value::I64(Rc::new(RefCell::new(4))));

test_interpreter!(interpret_formula_math_neg, "-1", Value::I64(Rc::new(RefCell::new(-1))));
test_interpreter!(interpret_formula_math_multiple_terms, "1 + 2 + 3", Value::I64(Rc::new(RefCell::new(6))));
test_interpreter!(interpret_formula_comparison_gt, "10 > 11", Value::Bool(Rc::new(RefCell::new(false))));
test_interpreter!(interpret_formula_comparison_lt, "10 < 11", Value::Bool(Rc::new(RefCell::new(true))));
test_interpreter!(interpret_formula_unicode, "😃:=1;🤦🏼‍♂️:=2;y̆és:=🤦🏼‍♂️ + 😃", Value::I64(Rc::new(RefCell::new(3))));
test_interpreter!(interpret_formula_logic_and, "true & true", Value::Bool(Rc::new(RefCell::new(true))));
test_interpreter!(interpret_formula_logic_and2, "true & false", Value::Bool(Rc::new(RefCell::new(false))));
test_interpreter!(interpret_formula_logic_or, "true | false", Value::Bool(Rc::new(RefCell::new(true))));
test_interpreter!(interpret_formula_logic_or2, "false | false", Value::Bool(Rc::new(RefCell::new(false))));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::I64(Rc::new(RefCell::new(123))));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::MutableReference(Rc::new(RefCell::new(Value::I64(Rc::new(RefCell::new(1)))))));

test_interpreter!(interpret_matrix_range_exclusive, "1..4", Value::Matrix(Matrix::RowDVector(Rc::new(RefCell::new(RowDVector::from_vec(vec![1,2,3]))))));
test_interpreter!(interpret_matrix_range_inclusive, "1..=4", Value::Matrix(Matrix::RowDVector(Rc::new(RefCell::new(RowDVector::from_vec(vec![1,2,3,4]))))));

test_interpreter!(interpret_matrix_empty, "[]", Value::Matrix(Matrix::DMatrix(DMatrix::from_vec(0,0,vec![]))));
test_interpreter!(interpret_matrix_mat1, "[123]", Value::Matrix(Matrix::Matrix1(Matrix1::from_vec(vec![123]))));
test_interpreter!(interpret_matrix_mat2, "[1 2; 3 4]", Value::Matrix(Matrix::Matrix2(Rc::new(RefCell::new(Matrix2::from_vec(vec![1,3,2,4]))))));
test_interpreter!(interpret_matrix_transpose, "[1 2; 3 4]'", Value::Matrix(Matrix::Matrix2(Rc::new(RefCell::new(Matrix2::from_vec(vec![1,2,3,4]))))));
test_interpreter!(interpret_matrix_negate, "-[1 2; 3 4]", Value::Matrix(Matrix::Matrix2(Rc::new(RefCell::new(Matrix2::from_vec(vec![-1,-3,-2,-4]))))));
test_interpreter!(interpret_matrix_row3_add, "[1 2 3] + [4 5 6]", Value::Matrix(Matrix::RowVector3(Rc::new(RefCell::new(RowVector3::from_vec(vec![5,7,9]))))));
test_interpreter!(interpret_matrix_row3_sub, "[1 2 3] - [4 5 6]", Value::Matrix(Matrix::RowVector3(Rc::new(RefCell::new(RowVector3::from_vec(vec![-3,-3,-3]))))));
test_interpreter!(interpret_matrix_mat2_matmul_ref, "a := [1 2; 3 4]; b := [4 5; 6 7]; c := a ** b", Value::Matrix(Matrix::Matrix2(Rc::new(RefCell::new(Matrix2::from_vec(vec![16,36,19,43]))))));
test_interpreter!(interpret_matrix_row3_add_ref, "a := [1 2 3]; b := [4 5 6]; c := a + b", Value::Matrix(Matrix::RowVector3(Rc::new(RefCell::new(RowVector3::from_vec(vec![5,7,9]))))));

test_interpreter!(interpret_tuple, "(1,true)", Value::Tuple(MechTuple::from_vec(vec![Value::I64(Rc::new(RefCell::new(1))), Value::Bool(Rc::new(RefCell::new(true)))])));
test_interpreter!(interpret_tuple_nested, r#"(1,("Hello",false))"#, Value::Tuple(MechTuple::from_vec(vec![Value::I64(Rc::new(RefCell::new(1))), Value::Tuple(MechTuple::from_vec(vec![Value::String("Hello".to_string()), Value::Bool(Rc::new(RefCell::new(false)))]))])));

test_interpreter!(interpret_slice, "a := [1,2,3]; a[2]", Value::I64(Rc::new(RefCell::new(2))));
test_interpreter!(interpret_slice_2d, "a := [1,2,3]; a[1,2]", Value::I64(Rc::new(RefCell::new(2))));

test_interpreter!(interpret_set_empty,"{_}", Value::Set(MechSet::from_vec(vec![])));
test_interpreter!(interpret_set,"{1,2,3}", Value::Set(MechSet::from_vec(vec![Value::I64(Rc::new(RefCell::new(1))),Value::I64(Rc::new(RefCell::new(2))),Value::I64(Rc::new(RefCell::new(3)))])));
test_interpreter!(interpret_record,r#"{a: 1, b: "Hello"}"#, Value::Record(MechMap::from_vec(vec![(Value::Id(55170961230981453),Value::I64(Rc::new(RefCell::new(1)))),(Value::Id(44311847522083591),Value::String("Hello".to_string()))])));
test_interpreter!(interpret_record_field_access,r#"a := {x: 1,  y: 2}; a.y"#, Value::I64(Rc::new(RefCell::new(2))));
test_interpreter!(interpret_map,r#"{"a": 1, "b": 2}"#, Value::Map(MechMap::from_vec(vec![(Value::String("a".to_string()),Value::I64(Rc::new(RefCell::new(1)))),(Value::String("b".to_string()),Value::I64(Rc::new(RefCell::new(2))))])));

test_interpreter!(interpret_function_define_call,r#"foo(x<i64>) = z<i64> :=
z := x + 10.
foo(10)"#, Value::I64(Rc::new(RefCell::new(20))));

test_interpreter!(interpret_function_define_call_2_args,r#"foo(x<i64>, y<i64>) = z<i64> :=
z := x + y.
foo(10,20)"#, Value::I64(Rc::new(RefCell::new(30))));