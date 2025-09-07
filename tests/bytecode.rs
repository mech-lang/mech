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
bytecode_test!(bytecode_matrix_rowvector3,"[1 2 3]",Value::MatrixF64(Matrix::RowVector3(Ref::new(na::RowVector3::from_vec(vec![F64::new(1.0),F64::new(2.0),F64::new(3.0)])))));