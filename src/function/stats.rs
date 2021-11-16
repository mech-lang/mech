use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumCol<T> {
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


// stats/sum(table: x)
#[derive(Debug)]
pub struct StatsSumTable {
  pub table: ArgTable, pub out: Out<u8>
}

impl MechFunction for StatsSumTable {
  fn solve(&mut self) {
    let mut sum = 0;
    let table_brrw = self.table.borrow();
    let table_els = table_brrw.rows * table_brrw.cols;
    for i in 0..table_els {
      match table_brrw.get_linear(i) {
        Ok(Value::U8(val)) => {
          sum += val
        },
        _ => (),
      }
    }
    (*self.out.borrow_mut())[0] = sum;
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct StatsSumRow {
  pub table: ArgTable, pub out: Out<u8>
}

impl MechFunction for StatsSumRow {
  fn solve(&mut self) {
    let table_brrw = self.table.borrow();
    for row in 0..table_brrw.rows {
      let mut sum = 0;
      for col in 0..table_brrw.cols {
        match table_brrw.get(row,col) {
          Ok(Value::U8(val)) => {
            sum += val
          },
          _ => (),
        }
      }
      (*self.out.borrow_mut())[row] = sum;
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumColVIx<T> {
  pub col: Arg<T>, pub ix: Arg<bool>, pub out: Out<T>
}

impl<T> MechFunction for StatsSumColVIx<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num {
  fn solve(&mut self) {
    let result = self.col.borrow()
                         .iter()
                         .zip(self.ix.borrow().iter())
                         .fold(identities::Zero::zero(),|sum, (n,ix)| if *ix {sum + *n} else {sum});
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// stats/sum(column: x{ix})
#[derive(Debug)]
pub struct StatsSumColTIx {
  pub col: ArgTable, pub ix: Arg<bool>, pub out: Out<u8>
}

impl MechFunction for StatsSumColTIx {
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

