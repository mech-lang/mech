use crate::{Column, TableId, TableIndex, Value, Register, NumberLiteralKind};
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

pub type ArgF64 = Column;
pub type ArgBool = Rc<RefCell<Vec<bool>>>;
pub type OutF64 = Column;
pub type OutBool = Rc<RefCell<Vec<bool>>>;

#[derive(Debug)]
pub enum Transformation {
  ParAddVVIP((OutF64, ArgF64)),  
  ParAddVSIP((OutF64, ArgF64)),
  ParMultiplyVS((ArgF64, ArgF64, OutF64)),
  ParOrVV((ArgBool,ArgBool,OutBool)),
  ParLessThanVS((ArgF64,f32,OutBool)),
  ParGreaterThanVS((ArgF64,f32,OutBool)),
  ParCSGreaterThanVS((ArgF64,f32,f32)),
  ParSetVS((ArgBool,f32,OutF64)),
  ParSetVV((ArgBool,ArgF64,OutF64)),
  Identifier{ name: Vec<char>, id: u64 },
  NumberLiteral{kind: NumberLiteralKind, bytes: Vec<u8> },
  
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value, unit: u64},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
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
      _ => (),
    }
  }
}