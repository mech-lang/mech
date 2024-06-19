#![allow(warnings)]
#![feature(iter_intersperse)]

#![feature(test)]
#![feature(get_mut_unchecked)]

extern crate test;

use test::Bencher;

use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};

use std::collections::VecDeque;
use std::thread;
use mech_core::*;
use mech_core::function::table;

use std::fmt::*;
use std::ops::*;

use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::*;
use mech_syntax::parser2;
//use mech_syntax::analyzer::*;
use mech_syntax::interpreter::*;
use mech_syntax::parser::{parse};
use nalgebra::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};



#[bench]
fn matrix_multiply(b:&mut Bencher){
  let s = r#"a := [1 2; 3 4]
b := [4 5; 6 7]
c := a ** b"#;
  match parser2::parse(&s) {
    Ok(tree) => { 
      let mut intrp = Interpreter::new();
      let result = intrp.interpret(&tree);
      let fxn = &intrp.plan.borrow()[0];
      b.iter(|| {
        let result = fxn.solve();
      });
    }
    _ => (),
  }
}

#[bench]
fn matrix_add_scalar(b:&mut Bencher){
  let s = r#"a := 1
b := 2
c := a + b"#;
  match parser2::parse(&s) {
    Ok(tree) => { 
      let mut intrp = Interpreter::new();
      let result = intrp.interpret(&tree);
      let fxn = &intrp.plan.borrow()[0];
      b.iter(|| {
        let result = fxn.solve();
      });
    }
    _ => (),
  }
}

#[bench]
fn matrix_add(b:&mut Bencher){
  let s = r#"a := [1 2 3]
b := [4 5 6]
c := a + b"#;
  match parser2::parse(&s) {
    Ok(tree) => { 
      let mut intrp = Interpreter::new();
      let result = intrp.interpret(&tree);
      let fxn = &intrp.plan.borrow()[0];
      b.iter(|| {
        let result = fxn.solve();
      });
    }
    _ => (),
  }
}

#[bench]
fn matrix_add_baseline_nalgebra(b:&mut Bencher){
  b.iter(|| {
    let a: RowVector3<i64> = RowVector3::from_vec(vec![1,2,3]);
    let b: RowVector3<i64> = RowVector3::from_vec(vec![4,5,6]);
    let c = a + b;
  });
}

#[bench]
fn matrix_add_baseline_rust(b:&mut Bencher){
  b.iter(|| {
    let a: [i64;3] = [1, 2, 3];
    let b: [i64;3] = [4, 5, 6];
    let mut c = [0, 0, 0];
    for i in 0..3 {
      c[i] = a[i] + b[i];
    }
  });
}

#[bench]
fn matrix_add_baseline_heap(b:&mut Bencher){
  b.iter(|| {
    let a: Vec<i64> = vec![1, 2, 3];
    let b: Vec<i64> = vec![4, 5, 6];
    let mut c = vec![0, 0, 0];
    for i in 0..3 {
      c[i] = a[i] + b[i];
    }
  });
}
