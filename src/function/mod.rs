use crate::{Column, ColumnV, humanize, ValueKind, MechString, Table, TableId, TableIndex, Value, Register, NumberLiteralKind};
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

pub mod compare;
pub mod math;
pub mod stats;
pub mod table;
pub mod set;
pub mod logic;

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

pub trait MechNum<T>: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + num_traits::Pow<T, Output = T> + Sized {}

pub trait MechFunction {
  fn solve(&mut self);
  fn to_string(&self) -> String;
}

#[derive(Clone, Debug)]
pub enum Function {
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
  GreaterThanVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  GreaterThanEqualVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  LessThanVVU8((Arg<u8>,Arg<u8>,Out<bool>)),
  ParCSGreaterThanVS((Arg<f32>,f32,f32)),

  ParSetVS((Arg<bool>,f32,Out<f32>)),
  ParSetVV((Arg<bool>,Arg<f32>,Out<f32>)),
  SetVVU8((Arg<u8>,Out<u8>)),
  ParCopyVV((Arg<f32>,Out<f32>)),
  ParCopyVVU8((Arg<u8>,Out<u8>)),
  HorizontalConcatenate((Vec<ArgTable>,OutTable)),
  RangeU8((Arg<u8>,Arg<u8>,OutTable)),
  Null,
}

impl MechFunction for Function {
  fn solve(&mut self) {
    match &*self {
      // MATH
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
      Function::SetVVU8((src,dest)) => { dest.borrow_mut().iter_mut().zip(&(*src.borrow())).for_each(|(dest,src)| *dest = *src); }
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