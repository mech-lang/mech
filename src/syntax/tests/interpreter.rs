#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
extern crate nalgebra as na;
use std::cell::RefCell;
use std::rc::Rc;
use mech_syntax::matrix::Matrix;
use mech_syntax::*;
use indexmap::set::IndexSet;
use na::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix3x2, Matrix2x3, Matrix6, Matrix2};

  /// Compare interpreter output to expected value
  macro_rules! test_interpreter {
    ($func:ident, $input:tt, $expected:expr) => (
      #[test]
      fn $func() {
        let s = $input;
        match parser::parse(&s) {
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

test_interpreter!(interpret_literal_integer, "123", Value::I64(new_ref(123)));
test_interpreter!(interpret_literal_float, "1.23", Value::F64(new_ref(F64::new(1.23))));
test_interpreter!(interpret_literal_string, r#""Hello""#, Value::String("Hello".to_string()));
test_interpreter!(interpret_literal_true, "true", Value::Bool(new_ref(true)));
test_interpreter!(interpret_literal_false, "false", Value::Bool(new_ref(false)));
test_interpreter!(interpret_literal_atom, "`A", Value::Atom(55450514845822917));
test_interpreter!(interpret_literal_empty, "_", Value::Empty);

test_interpreter!(interpret_formula_math_add, "2 + 2", Value::I64(new_ref(4)));
test_interpreter!(interpret_formula_math_sub, "2 - 2", Value::I64(new_ref(0)));
test_interpreter!(interpret_formula_math_mul, "2 * 2", Value::I64(new_ref(4)));
test_interpreter!(interpret_formula_math_div, "2 / 2", Value::I64(new_ref(1)));
test_interpreter!(interpret_formula_math_exp, "2<u8> ^ 2<u8>", Value::U8(new_ref(4)));
test_interpreter!(interpret_formula_math_exp_f64, "2.0 ^ 2.0", Value::F64(new_ref(F64::new(4.0))));

test_interpreter!(interpret_kind_annotation, "1<u64>", Value::U64(new_ref(1)));
test_interpreter!(interpret_kind_annotation_math, "1<u64> + 1<u64>", Value::U64(new_ref(2)));
test_interpreter!(interpret_kind_overflow, "256<u8>", Value::U8(new_ref(0)));
test_interpreter!(interpret_kind_math_overflow, "255<u8> + 1<u8>", Value::U8(new_ref(0)));
test_interpreter!(interpret_kind_math_no_overflow, "255<u16> + 1<u16>", Value::U16(new_ref(256)));
test_interpreter!(interpret_kind_matrix_row3, "[1<u8> 2<u8> 3<u8>]", Value::MatrixU8(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![1,2,3])))));
test_interpreter!(interpret_kind_lhs_define, "x<u64> := 1", Value::U64(new_ref(1)));
test_interpreter!(interpret_kind_lhs_define_overflow, "x<u8> := 256", Value::U8(new_ref(0)));
test_interpreter!(interpret_kind_convert_twice, "x<u64> := 1; y<i8> := x", Value::I8(new_ref(1)));
test_interpreter!(interpret_kind_convert_float, "x<f32> := 123;", Value::F32(new_ref(F32::new(123.0))));

test_interpreter!(interpret_formula_math_neg, "-1", Value::I64(new_ref(-1)));
test_interpreter!(interpret_formula_math_multiple_terms, "1 + 2 + 3", Value::I64(new_ref(6)));
test_interpreter!(interpret_formula_comparison_bool, "true == false", Value::Bool(new_ref(false)));
test_interpreter!(interpret_formula_comparison_bool2, "true == true", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_eq, "10 == 11", Value::Bool(new_ref(false)));
test_interpreter!(interpret_formula_comparison_neq, "10 != 11", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_neq_bool, "false != true", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_gt, "10 > 11", Value::Bool(new_ref(false)));
test_interpreter!(interpret_formula_comparison_lt, "10 < 11", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_gte, "10 >= 10", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_lte, "10 <= 10", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_comparison_gt_vec, "[1 8; 10 5] > [7 2; 4 11]", Value::MatrixBool(Matrix::Matrix2(new_ref(Matrix2::from_vec(vec![false,true,true,false])))));
test_interpreter!(interpret_formula_comparison_lt_vec, "[1 8 10 5] < [7 2 4 11]", Value::MatrixBool(Matrix::RowVector4(new_ref(RowVector4::from_vec(vec![true,false,false,true])))));
test_interpreter!(interpret_formula_unicode, "😃:=1;🤦🏼‍♂️:=2;y̆és:=🤦🏼‍♂️ + 😃", Value::I64(new_ref(3)));
test_interpreter!(interpret_formula_logic_and, "true & true", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_logic_and_vec, "[true false true] & [false false true]", Value::MatrixBool(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![false,false,true])))));
test_interpreter!(interpret_formula_logic_and2, "true & false", Value::Bool(new_ref(false)));
test_interpreter!(interpret_formula_logic_or_vec, "[true false true] | [false false true]", Value::MatrixBool(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![true,false,true])))));
test_interpreter!(interpret_formula_logic_or, "true | false", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_logic_or2, "false | false", Value::Bool(new_ref(false)));
test_interpreter!(interpret_formula_logic_xor_vec, "[true false false] ⊕ [true true false]", Value::MatrixBool(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![false,true,false])))));
test_interpreter!(interpret_formula_logic_not, "!false", Value::Bool(new_ref(true)));
test_interpreter!(interpret_formula_logic_not_vec, "![false true false]", Value::MatrixBool(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![true,false,true])))));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::I64(new_ref(123)));

test_interpreter!(interpret_reference_bool, "x := false; y := true; x & y", Value::Bool(new_ref(false)));
test_interpreter!(interpret_reference_bool2, "x := false; x & true", Value::Bool(new_ref(false)));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::MutableReference(new_ref(Value::I64(new_ref(1)))));

test_interpreter!(interpret_matrix_range_exclusive, "1..4", Value::MatrixI64(Matrix::RowDVector(new_ref(RowDVector::from_vec(vec![1,2,3])))));
test_interpreter!(interpret_matrix_range_exclusive_u8, "1<u8>..4<u8>", Value::MatrixU8(Matrix::RowDVector(new_ref(RowDVector::from_vec(vec![1,2,3])))));
test_interpreter!(interpret_matrix_range_inclusive, "1..=4", Value::MatrixI64(Matrix::RowDVector(new_ref(RowDVector::from_vec(vec![1,2,3,4])))));
test_interpreter!(interpret_matrix_range_inclusive_u8, "1<u8>..=4<u8>", Value::MatrixU8(Matrix::RowDVector(new_ref(RowDVector::from_vec(vec![1,2,3,4])))));

test_interpreter!(interpret_matrix_empty, "[]", Value::MatrixF64(Matrix::DMatrix(new_ref(DMatrix::from_vec(0,0,vec![])))));
test_interpreter!(interpret_matrix_row3, "[1 2 3]", new_ref(RowVector3::from_vec(vec![1i64,2,3])).to_value());
test_interpreter!(interpret_matrix_mat1, "[123]", Value::MatrixI64(Matrix::Matrix1(new_ref(Matrix1::from_vec(vec![123])))));
test_interpreter!(interpret_matrix_row3_float, "[1.2 2.3 3.4]", Value::MatrixF64(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![F64::new(1.2),F64::new(2.3),F64::new(3.4)])))));
test_interpreter!(interpret_matrix_mat2, "[1 2; 3 4]", new_ref(Matrix2::from_vec(vec![1i64,3,2,4])).to_value());
test_interpreter!(interpret_matrix_transpose, "[1 2; 3 4]'", new_ref(Matrix2::from_vec(vec![1i64,2,3,4])).to_value());
test_interpreter!(interpret_matrix_transpose_u8, "[1<u8> 2<u8> 3<u8>]'", new_ref(Vector3::from_vec(vec![1u8,2,3,4])).to_value());
test_interpreter!(interpret_matrix_transpose_float, "[1.0 2.0 3.0; 4.0 5.0 6.0]'", new_ref(Matrix3x2::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0),F64::new(4.0),F64::new(5.0),F64::new(6.0),])).to_value());
test_interpreter!(interpret_matrix_mat2_f64, "[1.1 2.2; 3.3 4.4]", Value::MatrixF64(Matrix::Matrix2(new_ref(Matrix2::from_vec(vec![F64::new(1.1),F64::new(3.3),F64::new(2.2),F64::new(4.4)])))));
test_interpreter!(interpret_matrix_negate, "-[1 2; 3 4]", new_ref(Matrix2::from_vec(vec![-1i64,-3,-2,-4])).to_value());
test_interpreter!(interpret_matrix_negate_float, "-[1.0 2.0; 3.0 4.0]", new_ref(Matrix2::from_vec(vec![F64::new(-1.0),F64::new(-3.0),F64::new(-2.0),F64::new(-4.0)])).to_value());
test_interpreter!(interpret_matrix_row3_add, "[1 2 3] + [4 5 6]", new_ref(RowVector3::from_vec(vec![5i64,7,9])).to_value());
test_interpreter!(interpret_matrix_row3_mul_scalar, "[1 2 3] * 3", new_ref(RowVector3::from_vec(vec![3i64,6,9])).to_value());
test_interpreter!(interpret_matrix_row3_mul_scalar2, "3 * [1 2 3]", new_ref(RowVector3::from_vec(vec![3i64,6,9])).to_value());
test_interpreter!(interpret_matrix_row3_add_float, "[1.0 2.0 3.0] + [4.0 5.0 6.0]", new_ref(RowVector3::from_vec(vec![F64::new(5.0),F64::new(7.0),F64::new(9.0)])).to_value());
test_interpreter!(interpret_matrix_row3_sub, "[1 2 3] - [4 5 6]", new_ref(RowVector3::from_vec(vec![-3i64,-3,-3])).to_value());
test_interpreter!(interpret_matrix_row3_sub_float, "[1.0 2.0 3.0] - [4.0 5.0 6.0]", new_ref(RowVector3::from_vec(vec![F64::new(-3.0),F64::new(-3.0),F64::new(-3.0)])).to_value());
test_interpreter!(interpret_matrix_row3_add_ref, "a := [1 2 3]; b := [4 5 6]; c := a + b", new_ref(RowVector3::from_vec(vec![5i64,7,9])).to_value());
test_interpreter!(interpret_matrix_dynamic_add, "[1 2 3 4; 5 6 7 8] + [1 2 3 4; 5 6 7 8]", new_ref(DMatrix::from_vec(2,4,vec![2i64,10,4,12,6,14,8,16])).to_value());
test_interpreter!(interpret_matrix_dynamic_div, "[2 4 6 8] / [2 2 2 2]", new_ref(RowVector4::from_vec(vec![1i64,2,3,4])).to_value());
test_interpreter!(interpret_matrix_gt, "x := [66.0 2.0 3.0; 66.0 5.0 66.0]; y := [1.0 2.0 3.0; 4.0 5.0 6.0]; x > y", new_ref(Matrix2x3::from_vec(vec![true,true,false,false,false,true])).to_value());
test_interpreter!(interpret_matrix_lt, "x := [66.0 2.0 3.0; 66.0 4.0 66.0]; y := [1.0 2.0 3.0; 4.0 5.0 6.0]; x < y", new_ref(Matrix2x3::from_vec(vec![false,false,false,true,false,false])).to_value());
test_interpreter!(interpret_matrix_lt_int, "x := [66 2 3; 66 4 66]; y := [1 2 3; 4 5 6]; x < y", new_ref(Matrix2x3::from_vec(vec![false,false,false,true,false,false])).to_value());

test_interpreter!(interpret_matrix_mat2_matmul_ref, "a := [1 2; 3 4]; b := [4 5; 6 7]; c := a ** b", new_ref(Matrix2::from_vec(vec![16i64,36,19,43])).to_value());
test_interpreter!(interpret_matrix_mat2x3_matmul_ref, "a := [1.0 2.0 3.0; 4.0 5.0 6.0]; b := [4.0 5.0; 6.0 7.0; 8.0 9.0]; c := a ** b", new_ref(Matrix2::from_vec(vec![F64::new(40.0),F64::new(94.0),F64::new(46.0),F64::new(109.0)])).to_value());


test_interpreter!(interpret_tuple, "(1,true)", Value::Tuple(MechTuple::from_vec(vec![Value::I64(new_ref(1)), Value::Bool(new_ref(true))])));
test_interpreter!(interpret_tuple_nested, r#"(1,("Hello",false))"#, Value::Tuple(MechTuple::from_vec(vec![Value::I64(new_ref(1)), Value::Tuple(MechTuple::from_vec(vec![Value::String("Hello".to_string()), Value::Bool(new_ref(false))]))])));

test_interpreter!(interpret_slice, "a := [1,2,3]; a[2]", Value::I64(new_ref(2)));
test_interpreter!(interpret_slice_2d, "a := [1,2;3,4]; a[1,2]", Value::I64(new_ref(2)));
test_interpreter!(interpret_slice_f64, "a := [1.0,2.0,3.0]; a[2]", Value::F64(new_ref(F64::new(2.0))));
test_interpreter!(interpret_slice_2d_f64, "a := [1,2;3,4]; a[2,1]", Value::I64(new_ref(3)));
test_interpreter!(interpret_slice_range, "x := 4..10; x[1..=3]", Value::MatrixI64(Matrix::RowVector3(new_ref(RowVector3::from_vec(vec![4,5,6])))));
test_interpreter!(interpret_slice_range_2d, "x := [1 2 3; 4 5 6; 7 8 9]; x[2..=3, 2..=3]", Value::MatrixI64(Matrix::Matrix2(new_ref(Matrix2::from_vec(vec![5,8,6,9])))));


test_interpreter!(interpret_set_empty,"{_}", Value::Set(MechSet::from_vec(vec![])));
test_interpreter!(interpret_set,"{1,2,3}", Value::Set(MechSet::from_vec(vec![Value::I64(new_ref(1)),Value::I64(new_ref(2)),Value::I64(new_ref(3))])));
test_interpreter!(interpret_record,r#"{a: 1, b: "Hello"}"#, Value::Record(MechMap::from_vec(vec![(Value::Id(55170961230981453),Value::I64(new_ref(1))),(Value::Id(44311847522083591),Value::String("Hello".to_string()))])));
test_interpreter!(interpret_record_field_access,r#"a := {x: 1,  y: 2}; a.y"#, Value::I64(new_ref(2)));
test_interpreter!(interpret_map,r#"{"a": 1, "b": 2}"#, Value::Map(MechMap::from_vec(vec![(Value::String("a".to_string()),Value::I64(new_ref(1))),(Value::String("b".to_string()),Value::I64(new_ref(2)))])));

test_interpreter!(interpret_function_define,r#"foo(x<i64>) = z<i64> :=
z := x + 10.
foo(10)"#, Value::I64(new_ref(20)));
test_interpreter!(interpret_function_define_2_args,r#"foo(x<i64>, y<i64>) = z<i64> :=
z := x + y.
foo(10,20)"#, Value::I64(new_ref(30)));
test_interpreter!(interpret_function_define_statements,r#"foo(x<i64>, y<i64>) = z<i64> :=
    a := 1 + x
    b := y + 1
    z := a + b.
foo(10,20)"#, Value::I64(new_ref(32)));

test_interpreter!(interpret_function_call_native_vector,"math/sin([1.570796327 1.570796327])", new_ref(RowVector2::from_vec(vec![F64::new(1.0),F64::new(1.0)])).to_value());
test_interpreter!(interpret_function_call_native,r#"math/sin(1.5707963267948966)"#, Value::F64(new_ref(F64::new(1.0))));
test_interpreter!(interpret_function_call_native_cos,r#"math/cos(0.0)"#, Value::F64(new_ref(F64::new(1.0))));
test_interpreter!(interpret_function_call_native_vector2,"math/cos([0.0 0.0])", new_ref(RowVector2::from_vec(vec![F64::new(1.0),F64::new(1.0)])).to_value());