use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// And Vector : Vector
#[derive(Debug)]
pub struct AndVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for AndVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs && *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Or Vector : Vector
#[derive(Debug)]
pub struct OrVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for OrVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs || *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Or Vector : Vector
#[derive(Debug)]
pub struct XorVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for XorVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs ^ *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Vector
#[derive(Debug)]
pub struct NotV {
  pub arg: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for NotV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = !(*arg)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}