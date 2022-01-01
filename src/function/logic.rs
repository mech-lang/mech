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

// And Scalar : Scalar
#[derive(Debug)]
pub struct AndSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for AndSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] && (self.rhs.borrow())[0];
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

// Or Scalar : Scalar
#[derive(Debug)]
pub struct OrSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for OrSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] || (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Xor Vector : Vector
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

// Xor Scalar : Scalar
#[derive(Debug)]
pub struct XorSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for XorSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] ^ (self.rhs.borrow())[0];
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

// Not Scalar
#[derive(Debug)]
pub struct NotS
{
  pub arg: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for NotS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = !(self.arg.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}
