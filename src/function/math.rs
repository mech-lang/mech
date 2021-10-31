use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// ParMul Vector : Scalar
#[derive(Debug)]
pub struct ParMultiplyVS<T>
where T: std::ops::Mul<Output = T> + Copy + Sync + Send + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for ParMultiplyVS<T> 
where T: std::ops::Mul<Output = T> + Copy + Sync + Send + Debug
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add Scalar : Vector
#[derive(Debug)]
pub struct AddSV<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for AddSV<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    let lhs = self.lhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs + *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add Scalar : Vector
#[derive(Debug)]
pub struct AddSIxSIx<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  pub lhs: Arg<T>, pub lix: usize, pub rhs: Arg<T>, pub rix: usize, pub out: Out<T>
}
impl<T> MechFunction for AddSIxSIx<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    let lhs = self.lhs.borrow()[self.lix];
    let rhs = self.rhs.borrow()[self.rix];
    self.out.borrow_mut().iter_mut().for_each(|out| *out = lhs + rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add Vector : Scalar
#[derive(Debug)]
pub struct AddVS<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for AddVS<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs + rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add Vector : Vector
#[derive(Debug)]
pub struct AddVV<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for AddVV<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs + *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add Scalar : Scalar
#[derive(Debug)]
pub struct AddSS<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for AddSS<T> 
where T: std::ops::Add<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] + (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// ParAdd Vector : Scalar
#[derive(Debug)]
pub struct ParAddVS<T>
where T: std::ops::Add<Output = T> + Copy + Sync + Send + Debug
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for ParAddVS<T> 
where T: std::ops::Add<Output = T> + Copy + Sync + Send + Debug
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = *lhs + rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}
