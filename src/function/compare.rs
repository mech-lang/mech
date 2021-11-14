use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// Greater Than Vector : Vector
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

// Less Than Vector : Vector
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

// Less Than Equal Vector : Vector
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

// Equal Vector : Vector
#[derive(Debug)]
pub struct EqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs == *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Equal Vector : Vector
#[derive(Debug)]
pub struct NotEqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs != *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Greater Than Vector : Scalar
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

// Less Than Vector : Scalar
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

// Less Than Equal Vector : Scalar
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

// Equal Vector : Scalar
#[derive(Debug)]
pub struct EqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = &self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs == *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Equal Vector : Scalar
#[derive(Debug)]
pub struct NotEqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = &self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs != *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// == Scalar : Scalar
#[derive(Debug)]
pub struct EqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] == (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// != Equal Scalar : Scalar
#[derive(Debug)]
pub struct NotEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] != (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// > Scalar : Scalar
#[derive(Debug)]
pub struct GreaterSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] > (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// < Scalar : Scalar
#[derive(Debug)]
pub struct LessSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] < (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// >= Scalar : Scalar
#[derive(Debug)]
pub struct GreaterEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] >= (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// < Scalar : Scalar
#[derive(Debug)]
pub struct LessEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] <= (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}