#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
extern crate nalgebra as na;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::matrix::Matrix;
use mech_syntax::*;
use mech_core::*;
use mech_interpreter::*;
use indexmap::set::IndexSet;

macro_rules! bytecode_test {
  ($name:ident, $code:expr, $expected:expr) => {
    #[test]
    fn $name() {
      let mut intrp = Interpreter::new(0);
      match parser::parse($code) {
        Ok(tree) => {
          let mut intrp = Interpreter::new(0);
          let _ = intrp.interpret(&tree).unwrap();
          let bytecode = intrp.compile().unwrap();
          match ParsedProgram::from_bytes(&bytecode) {
            Ok(prog) => {
              match intrp.run_program(&prog) {
                Ok(result) => {
                  assert_eq!(result, $expected);
                },
                Err(e) => {
                  eprintln!("Error running program: {:?}", e);
                }
              }
            },
            Err(e) => {
              eprintln!("Error deserializing program: {:?}", e);
            }
          }
        },
        Err(err) => { panic!("{:?}", err); }
      }
    }
  };
}

bytecode_test!(bytecode_var_def,"x := 10",Value::F64(Ref::new(F64::new(10.0))));
bytecode_test!(bytecode_math,"1 + 2",Value::F64(Ref::new(F64::new(3.0))));
bytecode_test!(bytecode_math_def,"x := 1 + 2; y := x + 4",Value::F64(Ref::new(F64::new(7.0))));
bytecode_test!(bytecode_math_mul,"x := 2 * 2; y := x * 4",Value::F64(Ref::new(F64::new(16.0))));
bytecode_test!(bytecode_math_add_assign,"~x := 10; x += 20",Value::F64(Ref::new(F64::new(30.0))));
bytecode_test!(bytecode_math_add_assign_vv, "~x := [1 2 3]; x += [10 20 30]", Value::MatrixF64(Matrix::RowVector3(Ref::new(na::RowVector3::from_vec(vec![F64::new(11.0),F64::new(22.0),F64::new(33.0)])))));
bytecode_test!(bytecode_math_add_assign_vr, "x := [1 1]; y := [1 2]; z := [10 20]; x[y] += z;", Value::MatrixF64(Matrix::RowVector2(Ref::new(na::RowVector2::from_vec(vec![F64::new(11.0),F64::new(21.0)])))));
bytecode_test!(bytecode_math_sub_assign,"~x := 30; x -= 20",Value::F64(Ref::new(F64::new(10.0))));
bytecode_test!(bytecode_math_sub_assign_vv, "~x := [10 20 30]; x -= [1 2 3]", Value::MatrixF64(Matrix::RowVector3(Ref::new(na::RowVector3::from_vec(vec![F64::new(9.0),F64::new(18.0),F64::new(27.0)])))));
bytecode_test!(bytecode_math_sub_assign_vr, "x := [11 21]; y := [1 2]; z := [10 20]; x[y] -= z;", Value::MatrixF64(Matrix::RowVector2(Ref::new(na::RowVector2::from_vec(vec![F64::new(1.0),F64::new(1.0)])))));
bytecode_test!(bytecode_matrix_rowvector3,"[1 2 3]",Value::MatrixF64(Matrix::RowVector3(Ref::new(na::RowVector3::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0)])))));
bytecode_test!(bytecode_matrix_vector2,"[1; 2]",Value::MatrixF64(Matrix::Vector2(Ref::new(na::Vector2::from_vec(vec![F64::new(1.0),F64::new(2.0)])))));
bytecode_test!(bytecode_matrix_matrix2x2,"[1 2; 3 4]",Value::MatrixF64(Matrix::Matrix2(Ref::new(na::Matrix2::from_vec(vec![F64::new(1.0),F64::new(3.0),F64::new(2.0),F64::new(4.0)])))));
bytecode_test!(bytecode_combinatorics_n_choose_k,"combinatorics/n-choose-k(10,2)",Value::F64(Ref::new(F64::new(45.0))));
bytecode_test!(bytecode_compare_gt,"1 > 2",Value::Bool(Ref::new(false)));
bytecode_test!(bytecode_compare_eq,r#""foo" == "bar""#,Value::Bool(Ref::new(false)));
bytecode_test!(bytecode_logic_and,"true && false",Value::Bool(Ref::new(false)));
bytecode_test!(bytecode_logic_or,"true || false",Value::Bool(Ref::new(true)));
bytecode_test!(bytecode_logic_not,"!true",Value::Bool(Ref::new(false)));
bytecode_test!(bytecode_math_cos,"math/cos(0)",Value::F64(Ref::new(F64::new(1.0))));
bytecode_test!(bytecode_math_sin,"math/sin(0)",Value::F64(Ref::new(F64::new(0.0))));
bytecode_test!(bytecode_math_atan2,"math/atan2(1, 1)",Value::F64(Ref::new(F64::new(std::f64::consts::FRAC_PI_4))));
bytecode_test!(bytecode_matrix_matmul_transpose,"[1 2 3] ** [4 5 6]'",Value::MatrixF64(Matrix::Matrix1(Ref::new(na::Matrix1::from_vec(vec![F64::new(32.0)])))));
bytecode_test!(bytecode_matrix_dot,"matrix/dot([1 2 3],[4 5 6])",Value::F64(Ref::new(F64::new(32.0))));

bytecode_test!(bytecode_range_inclusive,"1..=4",Value::MatrixF64(Matrix::RowVector4(Ref::new(na::RowVector4::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0),F64::new(4.0)])))));
bytecode_test!(bytecode_range_inclusive_d,"1..=5",Value::MatrixF64(Matrix::RowVector4(Ref::new(na::RowVector4::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0),F64::new(4.0),F64::new(5.0)])))));
bytecode_test!(bytecode_range_inclusive_refs,"a := 1; b :=4 ; a..=b",Value::MatrixF64(Matrix::RowVector4(Ref::new(na::RowVector4::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0),F64::new(4.0)])))));
bytecode_test!(bytecode_range_exclusive,"1..5",Value::MatrixF64(Matrix::RowDVector(Ref::new(na::RowDVector::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0),F64::new(4.0)])))));
bytecode_test!(bytecode_stats_sum_column,"stats/sum/column([1 2 3])",Value::MatrixF64(Matrix::Matrix1(Ref::new(na::Matrix1::from_vec(vec![F64::new(6.0)])))));