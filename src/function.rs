use crate::{Column, ColumnV, humanize, ValueKind, MechString, Table, TableId, TableIndex, Value, Register, NumberLiteralKind};
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;

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
}

// ParMul Vector : Scalar
#[derive(Debug)]
pub struct ParMultiplyVS<T>
where T: std::ops::Mul<Output = T> + Copy + Sync + Send
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for ParMultiplyVS<T> 
where T: std::ops::Mul<Output = T> + Copy + Sync + Send
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
  }
}

// Add Vector : Vector
#[derive(Debug)]
pub struct AddVV<T> 
where T: std::ops::Add<Output = T> + Copy
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for AddVV<T> 
where T: std::ops::Add<Output = T> + Copy
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs + *rhs); 
  }
}

// Add Scalar : Scalar
#[derive(Debug)]
pub struct AddSS<T> 
where T: std::ops::Add<Output = T> + Copy
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for AddSS<T> 
where T: std::ops::Add<Output = T> + Copy
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] + (self.rhs.borrow())[0];
  }
}

// ParMul Vector : Scalar
#[derive(Debug)]
pub struct ParAddVS<T>
where T: std::ops::Add<Output = T> + Copy + Sync + Send
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for ParAddVS<T> 
where T: std::ops::Add<Output = T> + Copy + Sync + Send
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = *lhs + rhs); 
  }
}


#[derive(Clone)]
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
  StatsSumColU8((Arg<u8>,Out<u8>)),
  ParCopyVV((Arg<f32>,Out<f32>)),
  ParCopyVVU8((Arg<u8>,Out<u8>)),
  HorizontalConcatenate((Vec<ArgTable>,OutTable)),
  CopySSU8((Arg<u8>,usize,Out<u8>)),
  CopySSString((Arg<MechString>,usize,Out<MechString>)),
  ConcatVU8((Vec<Arg<u8>>,Out<u8>)),
  CopyTable((ArgTable,OutTable)),
  CopyVBU8((Arg<u8>, Arg<bool>, OutTable)),
  RangeU8((Arg<u8>,Arg<u8>,OutTable)),
  Null,
}
      
impl fmt::Debug for Function {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {      
      Function::RangeU8((start,end, out)) => write!(f,"RangeU8(start: {:?}, end: {:?}, out: {:?})",start.borrow(), end.borrow(), out.borrow())?,

      Function::GreaterThanVSU8((lhs,rhs,out)) => write!(f,"GreaterThanVSU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,
      Function::GreaterThanSSU8((lhs,rhs,out)) => write!(f,"GreaterThanSSU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,
      Function::GreaterThanVVU8((lhs,rhs,out)) => write!(f,"GreaterThanVVU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,
      Function::LessThanSSU8((lhs,rhs,out)) => write!(f,"LessThanSSU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,
      Function::GreaterThanEqualVVU8((lhs,rhs,out)) => write!(f,"GreaterThanEqualVVU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,

      Function::LessThanVVU8((lhs,rhs,out)) => write!(f,"LessThanVVU8(lhs: {:?}, rhs: {:?}, out: {:?})",lhs.borrow(), rhs.borrow(), out.borrow())?,
      
      Function::CopyVBU8((arg, ix, out)) => write!(f,"CopyVBU8(arg:\n{:?}\nix:\n{:?}\nout:\n{:?})",arg.borrow(),ix,out.borrow())?,
      Function::CopySSU8((arg,ix,out)) => write!(f,"CopySSU8(arg: {:?}, ix: {}, out: {:?})",arg.borrow(),ix,out.borrow())?,
      Function::CopyTable((arg,out)) => write!(f,"CopyTable(arg: \n{:?}\nout: \n{:?}\n)",arg.borrow(),out.borrow())?,
     
      Function::StatsSumColU8((arg, out)) => write!(f,"StatsSumColU8(arg: {:?} out: {:?})",arg, out)?,
      
      _ => write!(f,"Tfm Print Not Implemented")?
    }
    Ok(())
  }
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
      Function::ConcatVU8((args, out)) => {
        let mut out_brrw = out.borrow_mut();
        let mut arg_ix = 0;
        let mut ix = 0;
        let mut arg_brrw = args[arg_ix].borrow();
        for r in 0..out_brrw.len() {
          out_brrw[r] = arg_brrw[ix];
          ix += 1;
          if ix == arg_brrw.len() {
            ix = 0;
            arg_ix += 1;
            if arg_ix == args.len() {
              return;
            } else {
              arg_brrw = args[arg_ix].borrow();
            }
          } 
        }
      }
      Function::SetVVU8((src,dest)) => { dest.borrow_mut().iter_mut().zip(&(*src.borrow())).for_each(|(dest,src)| *dest = *src); }
      Function::CopyTable((arg,out)) => {
        let mut out_brrw = out.borrow_mut();
        let arg_brrw = arg.borrow();
        out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
        for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
          out_brrw.set_col_kind(col, *kind);
        }
        for col in 0..arg_brrw.cols {
          for row in 0..arg_brrw.rows {
            let value = arg_brrw.get(row,col).unwrap();
            out_brrw.set(row,col,value);
          }
        }
      }
      Function::StatsSumColU8((arg,out)) => {
        let result = arg.borrow().iter().fold(0,|sum, n| sum + n);
        out.borrow_mut()[0] = result;
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
}