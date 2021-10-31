use crate::{Column, ColumnV, humanize, ValueKind, MechString, Table, TableId, TableIndex, Value, Register, NumberLiteralKind};
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// binop vector-vector          -- lhs: &Vec<f64>,     rhs: &Vec<f64>    out: &mut Vec<f64>
// binop vector-vector in-place -- lhs: &mut Vec<f64>  rhs: &Vec<f64>
// binop vector-scalar          -- lhs: &Vec<f64>,     rhs: f64          out: &mut Vec<f64>
// binop vector-scalar in-place -- lhs: &mut Vec<f64>  rhs: f64
// truth vector-vector          -- lhs: &Vec<bool>     rhs: &Vec<bool>   out: &mut Vec<bool>
// comp  vector-scalar          -- lhs: &Vec<f64>      rhs: f64          out: &mut Vec<bool>
// set   vector-scalar          -- ix: &Vec<bool>      x:   f64          out: &mut Vec<f64>
// set   vector-vector          -- ix: &Vec<bool>      x:   &Vec<f64>    out: &mut Vec<f64>

pub type Arg<T> = ColumnV<T>;
pub type Out<T> = ColumnV<T>;
pub type ArgTable = Rc<RefCell<Table>>;
pub type OutTable = Rc<RefCell<Table>>;

pub trait MechFunction {
  fn solve(&mut self);
  fn to_string(&self) -> String;
}

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

// Add Vector : Vector
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

// stats/sum(column: x{ix})
#[derive(Debug)]
pub struct StatsSumColIx
{
  pub col: ArgTable, pub ix: Arg<bool>, pub out: Out<u8>
}

impl MechFunction for StatsSumColIx
{
  fn solve(&mut self) {
    let mut sum = 0;
    let table_brrw = self.col.borrow();
    let ix_brrw = self.ix.borrow();
    for i in 0..ix_brrw.len() {
      match (table_brrw.get_linear(i),ix_brrw[i]) {
        (Some(Value::U8(val)),ix_value) => {
          if ix_value {
            sum += val
          }
        },
        _ => (),
      }
    }
    (*self.out.borrow_mut())[0] = sum;
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumCol<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num
{
  pub col: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for StatsSumCol<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num
{
  fn solve(&mut self) {
    let result = self.col.borrow().iter().fold(identities::Zero::zero(),|sum, n| sum + *n);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

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

#[derive(Clone, Debug)]
pub enum Function {
  DivideSSU8((Arg<u8>, Arg<u8>, Out<u8>)),
  MultiplySSU8((Arg<u8>, Arg<u8>, Out<u8>)),
  SubtractSSU8((Arg<u8>, Arg<u8>, Out<u8>)),
  ExponentSSU8((Arg<u8>, Arg<u8>, Out<u8>)),
  AddSSIPF32((Out<f32>, Arg<f32>)),
  AddVVIPF32((Out<f32>, Arg<f32>)),
  ParAddVVIPF32(Vec<ColumnV<f32>>),  
  ParAddVSIPF32(Vec<ColumnV<f32>>),
  ParMultiplyVSF32(Vec<ColumnV<f32>>),
  ParOrVV(Vec<ColumnV<bool>>),
  ParLessThanVS((Arg<f32>,f32,Out<bool>)),
  ParGreaterThanVS((Arg<f32>,f32,Out<bool>)),
  GreaterThanVSU8((Arg<u8>,Arg<u8>,Out<bool>)),
  GreaterThanSSU8((Arg<u8>,Arg<u8>,Out<bool>)),
  GreaterThanVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  GreaterThanEqualVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  LessThanSSU8((Arg<u8>,Arg<u8>,Out<bool>)),
  LessThanVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  ParCSGreaterThanVS((Arg<f32>,f32,f32)),

  ParSetVS((Arg<bool>,f32,Out<f32>)),
  ParSetVV((Arg<bool>,Arg<f32>,Out<f32>)),
  SetVVU8((Arg<u8>,Out<u8>)),
  ParCopyVV((Arg<f32>,Out<f32>)),
  ParCopyVVU8((Arg<u8>,Out<u8>)),
  HorizontalConcatenate((Vec<ArgTable>,OutTable)),
  CopySSU8((Arg<u8>,usize,Out<u8>)),
  CopySSString((Arg<MechString>,usize,Out<MechString>)),
  CopyTable((ArgTable,OutTable)),
  CopyVBU8((Arg<u8>, Arg<bool>, OutTable)),
  RangeU8((Arg<u8>,Arg<u8>,OutTable)),
  Null,
}

impl MechFunction for Function {
  fn solve(&mut self) {
    match &*self {
      // MATH
      Function::DivideSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] / (rhs.borrow())[0]; }
      Function::MultiplySSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] * (rhs.borrow())[0]; }
      Function::SubtractSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] - (rhs.borrow())[0]; }
      Function::ExponentSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0].pow((rhs.borrow())[0] as u32); }

      Function::AddSSIPF32((lhs, rhs)) => { ((lhs.borrow_mut())[0]) += (*rhs.borrow())[0] }
      Function::AddVVIPF32((lhs, rhs)) => { lhs.borrow_mut().iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Function::ParAddVVIPF32(args) => { args[0].borrow_mut().par_iter_mut().zip(&(*args[1].borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Function::ParAddVSIPF32(args) => { 
        let rhs = args[1].borrow()[0];
        args[0].borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      Function::ParMultiplyVSF32(args) => { 
        let rhs = args[1].borrow()[0];
        args[2].borrow_mut().par_iter_mut().zip(&(*args[0].borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
      }

      // COMPARE
      Function::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Function::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
      Function::LessThanVVU8((lhs, rhs, out)) => { 
        out.borrow_mut().iter_mut().zip(lhs.borrow().iter()).zip(rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs < *rhs); 
      }
      Function::GreaterThanVVU8((lhs, rhs, out)) => { 
        out.borrow_mut().iter_mut().zip(lhs.borrow().iter()).zip(rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs > *rhs); 
      }  
      Function::GreaterThanEqualVVU8((lhs, rhs, out)) => { 
        out.borrow_mut().iter_mut().zip(lhs.borrow().iter()).zip(rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs >= *rhs); 
      }  
      Function::GreaterThanVSU8((lhs, rhs, out)) => { 
        let rhs_value = rhs.borrow()[0];
        out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > rhs_value); 
      }
      Function::GreaterThanSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] > (rhs.borrow())[0]; }
      Function::LessThanSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] < (rhs.borrow())[0]; }
      Function::ParCSGreaterThanVS((lhs, rhs, swap)) => { 
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| if *lhs > *rhs {
          *lhs = *swap;
        }); 
      }

      // LOGIC
      Function::ParOrVV(args) => { args[2].borrow_mut().par_iter_mut().zip(&(*args[0].borrow())).zip(&(*args[1].borrow())).for_each(|((out, lhs),rhs)| *out = *lhs || *rhs); }
      // SET
      Function::ParSetVS((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).for_each(|(out,ix)| {
          if *ix == true {
            *out = *rhs
          }});          
      }
      Function::ParSetVV((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).zip(&(*rhs.borrow())).for_each(|((out,ix),x)| if *ix == true {
          *out = *x
        });          
      }
      Function::ParCopyVV((rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(out,x)| *out = *x); }
      Function::CopySSU8((rhs, ix, out)) => { (out.borrow_mut())[0] = (rhs.borrow())[*ix] }
      Function::CopySSString((rhs, ix, out)) => { (out.borrow_mut())[0] = (rhs.borrow())[*ix].clone() }
      Function::CopyVBU8((arg, ix, out)) => { 
        let filtered: Vec<u8>  = arg.borrow().iter().zip(ix.borrow().iter()).filter_map(|(x,ix)| if *ix {Some(*x)} else {None}).collect::<Vec<u8>>();
        let mut out_brrw = out.borrow_mut();
        let cols = out_brrw.cols;
        let rows = filtered.len();
        if rows > out_brrw.rows {
          out_brrw.resize(rows,cols)
        }
        out_brrw.set_col_kind(0, ValueKind::U8);
        for row in 0..filtered.len() {
          let value = filtered[row];
          out_brrw.set(row,0,Value::U8(value));
        }
      }
      Function::SetVVU8((src,dest)) => { dest.borrow_mut().iter_mut().zip(&(*src.borrow())).for_each(|(dest,src)| *dest = *src); }
      Function::CopyTable((arg,out)) => {
        let mut out_brrw = out.borrow_mut();
        let arg_brrw = arg.borrow();
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
      Function::RangeU8((start,end,out)) => {
        let start_value = start.borrow()[0];
        let end_value = end.borrow()[0];
        let delta = end_value - start_value + 1;
        let mut out_brrw = out.borrow_mut();
        out_brrw.resize(delta as usize,1);
        out_brrw.set_col_kind(0,ValueKind::U8);
        let mut value = start_value;
        for row in 0..out_brrw.rows {
          out_brrw.set(row,0,Value::U8(value));
          value += 1;
        } 
      }
      x => println!("Not Implemented: {:?}", x),
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}    
}