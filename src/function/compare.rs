use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// GreaterThan Vector : Vector
#[derive(Debug)]
pub struct GreaterThanVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterThanVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs > *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// LessThan Vector : Vector
#[derive(Debug)]
pub struct LessThanVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessThanVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs < *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// LessThan Vector : Vector
#[derive(Debug)]
pub struct LessThanEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessThanEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs <= *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Vector
#[derive(Debug)]
pub struct GreaterThanEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterThanEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs >= *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Vector
#[derive(Debug)]
pub struct EqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs == *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Vector
#[derive(Debug)]
pub struct NotEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs != *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThan Vector : Scalar
#[derive(Debug)]
pub struct GreaterThanVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterThanVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs > rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// LessThan Vector : Scalar
#[derive(Debug)]
pub struct LessThanVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessThanVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs < rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// LessThan Vector : Scalar
#[derive(Debug)]
pub struct LessThanEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessThanEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs <= rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Scalar
#[derive(Debug)]
pub struct GreaterThanEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterThanEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs >= rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Scalar
#[derive(Debug)]
pub struct EqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs == rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Scalar
#[derive(Debug)]
pub struct NotEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs != rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}