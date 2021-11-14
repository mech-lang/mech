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
where T: Clone + Debug
{
  pub args: Vec<Arg<T>>, 
  pub out: Out<T>,
}

impl<T> MechFunction for ConcatV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let mut arg_ix = 0;
    let mut ix = 0;
    let mut arg_brrw = self.args[arg_ix].borrow();
    for r in 0..out_brrw.len() {
      out_brrw[r] = arg_brrw[ix].clone();
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



// Copy Vector{Bool Ix} : Vector
#[derive(Debug)]
pub struct CopyVB<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: Arg<bool>, pub out: Out<T>
}

impl<T> MechFunction for CopyVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    // Filter the column to include only elements with a true index
    let filtered: Vec<T>  = 
      self.arg.borrow()
         .iter()
         .zip(self.ix.borrow().iter())
         .filter_map(|(x,ix)| if *ix {Some(*x)} else {None})
         .collect::<Vec<T>>();
    let mut out_brrw = self.out.borrow_mut();
    let rows = filtered.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,filtered[0]);
    }
    for row in 0..filtered.len() {
      out_brrw[row] = filtered[row];
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector{Int Ix} : Vector
#[derive(Debug)]
pub struct CopyVI<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: Arg<u8>, pub out: Out<T>
}

impl<T> MechFunction for CopyVI<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let ix_brrw = self.ix.borrow();

    let rows = ix_brrw.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,arg_brrw[0]);
    }
    for (out_ix, row) in ix_brrw.iter().enumerate() {
      out_brrw[out_ix] = arg_brrw[*row as usize - 1];
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Scalar : Scalar
#[derive(Debug)]
pub struct SetSIxSIx<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>, pub oix: usize
}
impl<T> MechFunction for SetSIxSIx<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[self.oix] = (self.arg.borrow())[self.ix];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Table : Table
#[derive(Debug)]
pub struct CopyT 
{
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for CopyT 
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
    for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
      out_brrw.set_col_kind(col, kind.clone());
    }
    for col in 0..arg_brrw.cols {
      for row in 0..arg_brrw.rows {
        let value = arg_brrw.get(row,col).unwrap();
        out_brrw.set(row,col,value);
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}