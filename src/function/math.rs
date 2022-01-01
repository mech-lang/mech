use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

impl MechNum<u8> for u8 {}
impl MechNum<f32> for f32 {}

// Scalar : Scalar
binary_infix_ss!(AddSS,add);
binary_infix_ss!(SubSS,sub);
binary_infix_ss!(MulSS,mul);
binary_infix_ss!(DivSS,div);
binary_infix_ss!(ExpSS,pow);

// Scalar : Vector
binary_infix_sv!(AddSV,add);
binary_infix_sv!(SubSV,sub);
binary_infix_sv!(MulSV,mul);
binary_infix_sv!(DivSV,div);
binary_infix_sv!(ExpSV,pow);

// Vector : Scalar
binary_infix_vs!(AddVS,add);
binary_infix_vs!(SubVS,sub);
binary_infix_vs!(MulVS,mul);
binary_infix_vs!(DivVS,div);
binary_infix_vs!(ExpVS,pow);

// Vector : Vector
binary_infix_vv!(AddVV,add);
binary_infix_vv!(SubVV,sub);
binary_infix_vv!(MulVV,mul);
binary_infix_vv!(DivVV,div);
binary_infix_vv!(ExpVV,pow);

// Parallel Vector : Scalar
binary_infix_par_vs!(AddParVS,add);
binary_infix_par_vs!(SubParVS,sub);
binary_infix_par_vs!(MulParVS,mul);
binary_infix_par_vs!(DivParVS,div);
binary_infix_par_vs!(ExpParVS,pow);

// Parallel Vector : Vector
binary_infix_par_vv!(AddParVV,add);
binary_infix_par_vv!(SubParVV,sub);
binary_infix_par_vv!(MulParVV,mul);
binary_infix_par_vv!(DivParVV,div);
binary_infix_par_vv!(ExpParVV,pow);

// Parallel Scalar : Vector
binary_infix_par_vv!(AddParSV,add);
binary_infix_par_vv!(SubParSV,sub);
binary_infix_par_vv!(MulParSV,mul);
binary_infix_par_vv!(DivParSV,div);
binary_infix_par_vv!(ExpParSV,pow);


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
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug
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
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug
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
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug
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
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug + Send + Sync
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
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug + Send + Sync
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
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub lix: usize, pub rhs: Arg<T>, pub rix: usize, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug
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
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNum<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}