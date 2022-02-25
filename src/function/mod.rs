use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

//pub mod compare;
pub mod math;
pub mod stats;
pub mod table;
//pub mod set;
//pub mod logic;

pub type Arg<T> = ColumnV<T>;
pub type Out<T> = ColumnV<T>;
pub type ArgTable = Rc<RefCell<Table>>;
pub type OutTable = Rc<RefCell<Table>>;

pub trait MechNumArithmetic<T>: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Sized {}

pub trait MechFunctionCompiler {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError>;
}

pub trait MechFunction {
  fn solve(&self);
  fn to_string(&self) -> String;
}

pub fn resize_one(block: &mut Block, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
  let (out_table_id,_,_) = out;
  let out_table = block.get_table(out_table_id)?;
  let mut out_brrw = out_table.borrow_mut();
  out_brrw.resize(1,1);
  Ok(())
}