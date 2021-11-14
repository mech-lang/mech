use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

pub trait MechNum<T>: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + num_traits::Pow<T, Output = T> + Sized {}

impl MechNum<u8> for u8 {}

// Scalar : Scalar
binary_infix_ss!(AddSS,add,MechNum);
binary_infix_ss!(SubSS,sub,MechNum);
binary_infix_ss!(MulSS,mul,MechNum);
binary_infix_ss!(DivSS,div,MechNum);
binary_infix_ss!(ExpSS,pow,MechNum);

// Scalar : Vector
binary_infix_sv!(AddSV,add,MechNum);
binary_infix_sv!(SubSV,sub,MechNum);
binary_infix_sv!(MulSV,mul,MechNum);
binary_infix_sv!(DivSV,div,MechNum);
binary_infix_sv!(ExpSV,pow,MechNum);

// Vector : Scalar
binary_infix_vs!(AddVS,add,MechNum);
binary_infix_vs!(SubVS,sub,MechNum);
binary_infix_vs!(MulVS,mul,MechNum);
binary_infix_vs!(DivVS,div,MechNum);
binary_infix_vs!(ExpVS,pow,MechNum);

// Vector : Vector
binary_infix_vv!(AddVV,add,MechNum);
binary_infix_vv!(SubVV,sub,MechNum);
binary_infix_vv!(MulVV,mul,MechNum);
binary_infix_vv!(DivVV,div,MechNum);
binary_infix_vv!(ExpVV,pow,MechNum);

// Parallel Vector : Scalar
binary_infix_par_vs!(AddParVS,add,MechNum);
binary_infix_par_vs!(SubParVS,sub,MechNum);
binary_infix_par_vs!(MulParVS,mul,MechNum);
binary_infix_par_vs!(DivParVS,div,MechNum);
binary_infix_par_vs!(ExpParVS,pow,MechNum);

// Parallel Vector : Vector
binary_infix_par_vv!(AddParVV,add,MechNum);
binary_infix_par_vv!(SubParVV,sub,MechNum);
binary_infix_par_vv!(MulParVV,mul,MechNum);
binary_infix_par_vv!(DivParVV,div,MechNum);
binary_infix_par_vv!(ExpParVV,pow,MechNum);

// Parallel Scalar : Vector
binary_infix_par_vv!(AddParSV,add,MechNum);
binary_infix_par_vv!(SubParSV,sub,MechNum);
binary_infix_par_vv!(MulParSV,mul,MechNum);
binary_infix_par_vv!(DivParSV,div,MechNum);
binary_infix_par_vv!(ExpParSV,pow,MechNum);


// Negate Vector
#[derive(Debug)]
pub struct NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = -((self.arg.borrow())[0]);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Negate Vector
#[derive(Debug)]
pub struct NegateV<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for NegateV<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = -(*arg)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[macro_export]
macro_rules! binary_infix_sv {
  ($func_name:ident, $op:tt, $types:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vs {
  ($func_name:ident, $op:tt, $types:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = (*lhs).$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vv {
  ($func_name:ident, $op:tt, $types:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug
    {
      fn solve(&mut self) {
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = (*lhs).$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vv {
  ($func_name:ident, $op:tt, $types:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).zip(self.rhs.borrow().par_iter()).for_each(|((out, lhs), rhs)| *out = (*lhs).$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vs {
  ($func_name:ident, $op:tt, $types:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = (*lhs).$op(rhs));
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_ss {
  ($func_name:ident, $op:tt, $types:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub lix: usize, pub rhs: Arg<T>, pub rix: usize, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[self.lix];
        let rhs = self.rhs.borrow()[self.rix];
        self.out.borrow_mut().iter_mut().for_each(|out| *out = lhs.$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_sv {
  ($func_name:ident, $op:tt, $types:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: $types<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}