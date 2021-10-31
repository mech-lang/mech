use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// Concat Vectors
#[derive(Debug)]
pub struct ConcatV<T> 
where T: Copy + Debug
{
  pub args: Vec<Arg<T>>, 
  pub out: Out<T>,
}

impl<T> MechFunction for ConcatV<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let mut arg_ix = 0;
    let mut ix = 0;
    let mut arg_brrw = self.args[arg_ix].borrow();
    for r in 0..out_brrw.len() {
      out_brrw[r] = arg_brrw[ix];
      ix += 1;
      if ix == arg_brrw.len() {
        ix = 0;
        arg_ix += 1;
        if arg_ix == self.args.len() {
          return;
        } else {
          arg_brrw = self.args[arg_ix].borrow();
        }
      } 
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector : Vector
#[derive(Debug)]
pub struct CopyVV<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for CopyVV<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = *arg); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector : Vector
#[derive(Debug)]
pub struct CopySS<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: usize , pub out: Out<T>
}
impl<T> MechFunction for CopySS<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.arg.borrow())[self.ix]
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}