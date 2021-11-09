use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// set/any(column: x)
#[derive(Debug)]
pub struct SetAnyCol {
  pub col: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for SetAnyCol {
  fn solve(&mut self) {
    let result = self.col.borrow().iter().any(|x| *x == true);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// set/all(column: x)
#[derive(Debug)]
pub struct SetAllCol {
  pub col: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for SetAllCol {
  fn solve(&mut self) {
    let result = self.col.borrow().iter().all(|x| *x == true);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}