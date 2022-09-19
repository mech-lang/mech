use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref MATRIX_MULTIPLY: u64 = hash_str("matrix/multiply");
  pub static ref MATRIX_TRANSPOSE: u64 = hash_str("matrix/transpose");
}


pub struct MatrixMul{}
impl MechFunctionCompiler for MatrixMul {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    Ok(())
  }
}

pub struct MatrixTranspose{}
impl MechFunctionCompiler for MatrixTranspose {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    Ok(())
  }
}