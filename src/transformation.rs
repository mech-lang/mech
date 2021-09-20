use crate::{Column, ColumnF32, ColumnU8, ColumnBool, ValueKind, TableId, TableIndex, Value, Register, NumberLiteralKind};
use std::cell::RefCell;
use std::rc::Rc;

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

pub type ArgF32 = ColumnF32;
pub type ArgU8 = ColumnU8;
pub type ArgBool = ColumnBool;
pub type OutF32 = ColumnF32;
pub type OutU8 = ColumnU8;
pub type OutBool = ColumnBool;

#[derive(Debug, Clone)]
pub enum Transformation {
  AddSS((ArgF32, ArgF32, OutF32)),
  AddSSU8((ArgU8, ArgU8, OutU8)),
  AddSSIP((OutF32, ArgF32)),
  AddVVIP((OutF32, ArgF32)),
  ParAddVVIP((OutF32, ArgF32)),  
  ParAddVSIP((OutF32, ArgF32)),
  ParMultiplyVS((ArgF32, ArgF32, OutF32)),
  ParOrVV((ArgBool,ArgBool,OutBool)),
  ParLessThanVS((ArgF32,f32,OutBool)),
  ParGreaterThanVS((ArgF32,f32,OutBool)),
  ParCSGreaterThanVS((ArgF32,f32,f32)),
  ParSetVS((ArgBool,f32,OutF32)),
  ParSetVV((ArgBool,ArgF32,OutF32)),
  ParCopyVV((ArgF32,OutF32)),
  ParCopyVVU8((ArgU8,OutU8)),
  CopySSU8((ArgU8,OutU8)),
  
  Identifier{ name: Vec<char>, id: u64 },
  NumberLiteral{kind: NumberLiteralKind, bytes: Vec<u8>, table_id: TableId, row: usize, column: usize },
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value, unit: u64},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  ColumnKind{table_id: TableId, column_ix: usize, column_kind: ValueKind},
  Set{table_id: TableId, row: TableIndex, column: TableIndex},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: TableId, row: TableIndex, column: TableIndex, registers: Vec<Register>},
  Function{name: u64, arguments: Vec<(u64, TableId, TableIndex, TableIndex)>, out: (TableId, TableIndex, TableIndex)},
  Select{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>, out: TableId},
}

impl Transformation {
  pub fn solve(&mut self) {
    match &*self {
      // MATH
      Transformation::AddSS((lhs, rhs, out)) => { 
        (out.borrow_mut())[0] = (lhs.borrow())[0] + (rhs.borrow())[0]
      }
      Transformation::AddSSU8((lhs, rhs, out)) => { 
        (out.borrow_mut())[0] = (lhs.borrow())[0] + (rhs.borrow())[0]
      }
      Transformation::AddSSIP((lhs, rhs)) => { ((lhs.borrow_mut())[0]) += (*rhs.borrow())[0] }
      Transformation::AddVVIP((lhs, rhs)) => { lhs.borrow_mut().iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVVIP((lhs, rhs)) => { lhs.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVSIP((lhs, rhs)) => { 
        let rhs = rhs.borrow()[0];
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      Transformation::ParMultiplyVS((lhs, rhs, out)) => { 
        let rhs = rhs.borrow()[0];
        out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
      }
      // COMPARE
      Transformation::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Transformation::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
      Transformation::ParCSGreaterThanVS((lhs, rhs, swap)) => { 
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| if *lhs > *rhs {
          *lhs = *swap;
        }); 
      }

      // LOGIC
      Transformation::ParOrVV((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).zip(&(*rhs.borrow())).for_each(|((out, lhs),rhs)| *out = *lhs || *rhs); }
      // SET
      Transformation::ParSetVS((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).for_each(|(out,ix)| {
          if *ix == true {
            *out = *rhs
          }});          
      }
      Transformation::ParSetVV((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).zip(&(*rhs.borrow())).for_each(|((out,ix),x)| if *ix == true {
          *out = *x
        });          
      }
      Transformation::ParCopyVV((rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(out,x)| *out = *x); }
      Transformation::CopySSU8((rhs, out)) => { (out.borrow_mut())[0] = (rhs.borrow())[0] }
      x => println!("Not Implemented: {:?}", x),
    }
  }
}