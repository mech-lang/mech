use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumCol<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num
{
  pub col: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for StatsSumCol<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num
{
  fn solve(&mut self) {
    let result = self.col.borrow().iter().fold(identities::Zero::zero(),|sum, n| sum + *n);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

/*
// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumRow<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num
{
  pub col: ArgTable, pub out: Out<T>
}

CopyTable((ArgTable,OutTable)),


// stats/sum(row: x)
Function::CopyTable((arg,out)) => {
  let mut out_brrw = out.borrow_mut();
  let arg_brrw = arg.borrow();
  out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
  for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
    out_brrw.set_col_kind(col, kind.clone());
  }
  for col in 0..arg_brrw.cols {
    for row in 0..arg_brrw.rows {
      let value = arg_brrw.get(row,col).unwrap();
      out_brrw.set(row,col,value);
    }
  }
}*/


// stats/sum(column: x{ix})
#[derive(Debug)]
pub struct StatsSumColIx
{
  pub col: ArgTable, pub ix: Arg<bool>, pub out: Out<u8>
}

impl MechFunction for StatsSumColIx
{
  fn solve(&mut self) {
    let mut sum = 0;
    let table_brrw = self.col.borrow();
    let ix_brrw = self.ix.borrow();
    for i in 0..ix_brrw.len() {
      match (table_brrw.get_linear(i),ix_brrw[i]) {
        (Ok(Value::U8(val)),ix_value) => {
          if ix_value {
            sum += val
          }
        },
        _ => (),
      }
    }
    (*self.out.borrow_mut())[0] = sum;
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

