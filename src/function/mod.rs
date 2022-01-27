use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

pub mod compare;
pub mod math;
pub mod stats;
pub mod table;
pub mod set;
pub mod logic;

pub type Arg<T> = ColumnV<T>;
pub type Out<T> = ColumnV<T>;
pub type ArgTable = Rc<RefCell<Table>>;
pub type OutTable = Rc<RefCell<Table>>;

pub trait MechNumArithmetic<T>: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + num_traits::Pow<T, Output = T> + Sized {}

pub trait MechFunctionCompiler {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError>;
}

pub trait MechFunction {
  fn solve(&mut self);
  fn to_string(&self) -> String;
}