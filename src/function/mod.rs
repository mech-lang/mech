use crate::*;
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

pub trait MechNumArithmetic<T>: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + num_traits::Pow<T, Output = T> + Sized {}

pub trait MechFunctionCompiler {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError>;
}

pub trait MechFunction {
  fn solve(&mut self);
  fn to_string(&self) -> String;
}

#[derive(Clone, Debug)]
pub enum Function {
  AddSSIPF32((Out<f32>, Arg<f32>)),
  AddVVIPF32((Out<f32>, Arg<f32>)),
  ParAddVVIPF32(Vec<ColumnV<f32>>),  
  ParAddVSIPF32(Vec<ColumnV<f32>>),
  ParOrVV(Vec<ColumnV<bool>>),
  ParLessThanVS((Arg<f32>,f32,Out<bool>)),
  ParGreaterThanVS((Arg<f32>,f32,Out<bool>)),
  ParCSGreaterThanVS((Arg<f32>,f32,f32)),
  ParSetVS((Arg<bool>,f32,Out<f32>)),
  ParSetVV((Arg<bool>,Arg<f32>,Out<f32>)),
  ParCopyVV((Arg<f32>,Out<f32>)),
  ParCopyVVU8((Arg<u8>,Out<u8>)),
  Null,
}

impl MechFunction for Function {
  fn solve(&mut self) {
    match &*self {
      // MATH
      Function::AddSSIPF32((lhs, rhs)) => { ((lhs.borrow_mut())[0]) += (*rhs.borrow())[0] }
      Function::AddVVIPF32((lhs, rhs)) => { lhs.borrow_mut().iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += *rhs); }
      Function::ParAddVVIPF32(args) => { args[0].borrow_mut().par_iter_mut().zip(&(*args[1].borrow())).for_each(|(lhs, rhs)| *lhs += *rhs); }
      Function::ParAddVSIPF32(args) => { 
        let rhs = args[1].borrow()[0];
        args[0].borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      // COMPARE
      Function::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Function::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
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
      x => println!("Not Implemented: {:?}", x),
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}    
}