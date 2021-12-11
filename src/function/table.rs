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
where T: Clone + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for CopyVV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = arg.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Scalar : Vector
#[derive(Debug)]
pub struct CopySV<T> 
where T: Clone + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Out<T>
}
impl<T> MechFunction for CopySV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    let arg = self.arg.borrow()[self.ix].clone();
    self.out.borrow_mut().iter_mut().for_each(|out| *out = arg.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector : Vector Ref
#[derive(Debug)]
pub struct CopyVVRef {
  pub arg: Arg<TableId>, pub out: Out<TableId>
}
impl MechFunction for CopyVVRef {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| {
      let id = TableId::Global(*arg.unwrap());
      *out = id;
    }); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Reference
#[derive(Debug)]
pub struct CopySVRef {
  pub arg: Arg<TableId>, pub ix: usize , pub out: Out<TableId>
}
impl MechFunction for CopySVRef 
{
  fn solve(&mut self) {
    let id = TableId::Global(*self.arg.borrow()[self.ix].unwrap());
    self.out.borrow_mut().iter_mut().for_each(|out| *out = id.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Scalar : Scalar
#[derive(Debug)]
pub struct CopySS<T> {
  pub arg: Arg<T>, pub ix: usize , pub out: Out<T>
}
impl<T> MechFunction for CopySS<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.arg.borrow())[self.ix].clone()
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Reference
#[derive(Debug)]
pub struct CopySSRef {
  pub arg: Arg<TableId>, pub ix: usize , pub out: Out<TableId>
}
impl MechFunction for CopySSRef 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = TableId::Global(*self.arg.borrow()[self.ix].unwrap())
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
    // Filter the column to include only elements with a "true" index
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
pub struct CopyVI<T>  {
  pub arg: Arg<T>, pub ix: Arg<usize>, pub out: Out<T>
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

// Set Scalar : Vector {Bool}
#[derive(Debug)]
pub struct SetSIxVB<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>, pub oix: Arg<bool>
}
impl<T> MechFunction for SetSIxVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let oix_brrw = self.oix.borrow();
    for row in 0..oix_brrw.len() {
      if oix_brrw[row] {
        (self.out.borrow_mut())[row] = (self.arg.borrow())[self.ix];
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Scalar : Vector {Bool}
#[derive(Debug)]
pub struct CopyTB {
  pub arg: ArgTable, pub ix: ArgTable, pub out: OutTable, 
}
impl MechFunction for CopyTB
{
  fn solve(&mut self) {
    let ix_brrw = self.ix.borrow();
    let rows = ix_brrw.logical_len();

    let src_brrw = self.arg.borrow();
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.resize(rows,1);
    let mut i = 0;
    for j in 0..ix_brrw.len() {
      match ix_brrw.get_linear(j) {
        Ok(Value::Bool(false)) => continue,
        Ok(Value::Bool(true)) => {
          let value = src_brrw.get_linear(j).unwrap();
          out_brrw.set_linear(i,value).unwrap();
          i+=1;
        }
        _ => (),
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Vector : Vector {Bool}
#[derive(Debug)]
pub struct SetVVB<T> {
  pub arg: Arg<T>, pub out: Arg<T>, pub oix: Arg<bool>
}

impl<T> MechFunction for SetVVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let oix_brrw = self.oix.borrow();
    for row in 0..oix_brrw.len() {
      if oix_brrw[row] {
        (self.out.borrow_mut())[row] = (self.arg.borrow())[row];
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Vector : Vector
#[derive(Debug)]
pub struct SetVV<T> {
  pub arg: Arg<T>, pub out: Arg<T>
}

impl<T> MechFunction for SetVV<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let rows = self.arg.borrow().len();
    for row in 0..rows {
      (self.out.borrow_mut())[row] = (self.arg.borrow())[row];
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Table : Table
#[derive(Debug)]
pub struct CopyT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for CopyT {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();

    out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
    for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
      out_brrw.set_col_kind(col, kind.clone());
    }
    out_brrw.column_ix_to_alias = arg_brrw.column_ix_to_alias.clone();
    out_brrw.column_alias_to_ix = arg_brrw.column_alias_to_ix.clone();
    for col in 0..arg_brrw.cols {
      for row in 0..arg_brrw.rows {
        let value = arg_brrw.get(row,col).unwrap();
        out_brrw.set(row,col,value);
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// AppendRow Table : Table
#[derive(Debug)]
pub struct AppendRowT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for AppendRowT {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    let ocols = out_brrw.cols;
    let arows = arg_brrw.rows;
    out_brrw.resize(orows + arows, ocols);
    if arg_brrw.has_col_aliases() {
      for col in 0..arg_brrw.cols {
        for row in 0..arows {
          let value = arg_brrw.get(row,col).unwrap();
          let alias = arg_brrw.column_ix_to_alias[col];
          match out_brrw.column_alias_to_ix.get(&alias) {
            Some(col_ix) => {out_brrw.set(orows + row,*col_ix,value);}
            None => (), // TODO Error
          }
        }
      }
    } else {
      for col in 0..arg_brrw.cols {
        for row in 0..arows {
          let value = arg_brrw.get(row,col).unwrap();
          out_brrw.set(orows + row,col,value);
        }
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// AppendRow Table : Table
#[derive(Debug)]
pub struct AppendRowSV {
  pub arg: ArgTable, pub ix: usize,  pub out: OutTable
}

impl MechFunction for AppendRowSV {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    out_brrw.resize(orows + 1, 1);
    let value = arg_brrw.get_linear(self.ix).unwrap();
    out_brrw.set(orows,0,value);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}