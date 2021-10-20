use crate::{Column, ColumnF32, humanize, ColumnU8, ColumnString, ColumnBool, ValueKind, Table, TableId, TableIndex, Value, Register, NumberLiteralKind};
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

pub type ArgF32 = ColumnF32;
pub type ArgU8 = ColumnU8;
pub type ArgBool = ColumnBool;
pub type ArgString = ColumnString;
pub type OutF32 = ColumnF32;
pub type OutU8 = ColumnU8;
pub type OutBool = ColumnBool;
pub type OutString = ColumnString;
pub type ArgTable = Rc<RefCell<Table>>;
pub type OutTable = Rc<RefCell<Table>>;

trait MechFunction<T> {
  fn solve(args: Vec<T>);
}

#[derive(Clone)]
pub enum Transformation {
  AddSSF32((ArgF32, ArgF32, OutF32)),
  AddSSU8(Vec<ColumnU8>),
  AddVVU8(Vec<ColumnU8>),
  DivideSSU8((ArgU8, ArgU8, OutU8)),
  MultiplySSU8((ArgU8, ArgU8, OutU8)),
  SubtractSSU8((ArgU8, ArgU8, OutU8)),
  ExponentSSU8((ArgU8, ArgU8, OutU8)),
  AddSSIPF32((OutF32, ArgF32)),
  AddVVIPF32((OutF32, ArgF32)),
  ParAddVVIPF32(Vec<ColumnF32>),  
  ParAddVSIPF32(Vec<ColumnF32>),
  ParMultiplyVSF32(Vec<ColumnF32>),
  ParOrVV(Vec<ColumnBool>),
  ParLessThanVS((ArgF32,f32,OutBool)),
  ParGreaterThanVS((ArgF32,f32,OutBool)),
  ParCSGreaterThanVS((ArgF32,f32,f32)),
  ParSetVS((ArgBool,f32,OutF32)),
  ParSetVV((ArgBool,ArgF32,OutF32)),
  SetVVU8((ArgU8,OutU8)),
  ParCopyVV((ArgF32,OutF32)),
  ParCopyVVU8((ArgU8,OutU8)),
  HorizontalConcatenate((Vec<ArgTable>,OutTable)),
  CopySSU8((ArgU8,usize,OutU8)),
  CopySSString((ArgString,usize,OutString)),
  ConcatVU8((Vec<ArgU8>,OutU8)),
  CopyTable((ArgTable,OutTable)),
  
  Identifier{ name: Vec<char>, id: u64 },
  NumberLiteral{kind: NumberLiteralKind, bytes: Vec<u8>},
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  ColumnKind{table_id: TableId, column_ix: usize, column_kind: ValueKind},
  Set{src_id: TableId, src_indices: Vec<(TableIndex, TableIndex)>, dest_id: TableId, dest_indices: Vec<(TableIndex, TableIndex)>},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: TableId, row: TableIndex, column: TableIndex, registers: Vec<Register>},
  Function{name: u64, arguments: Vec<(u64, TableId, TableIndex, TableIndex)>, out: (TableId, TableIndex, TableIndex)},
  Select{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>, out: TableId},
  Null,
}

impl fmt::Debug for Transformation {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {
      Transformation::NewTable{table_id, rows, columns} =>  write!(f,"NewTable(table_id: {:?}, rows: {} cols: {})",table_id,rows,columns)?,
      Transformation::Identifier{name,id} => write!(f,"Identifier(name: {:?}, id: {})",name,humanize(id))?,
      Transformation::NumberLiteral{kind,bytes} => write!(f,"NumberLiteral(kind: {:?}, bytes: {:?})",kind,bytes)?,
      Transformation::TableAlias{table_id,alias} => write!(f,"Alias(table_id: {:?}, alias: {})",table_id,humanize(alias))?,
      Transformation::Select{table_id,indices,out} => write!(f,"Select(table_id: {:?}, indices: {:?}, out: {:?})",table_id,indices,out)?,
      Transformation::Set{src_id, src_indices,dest_id,dest_indices} => write!(f,"Set(src_id: {:?}, src_indices: {:?},\n    dest_id: {:?}, dest_indices: {:?})",src_id,src_indices,dest_id,dest_indices)?,
      Transformation::Function{name,arguments,out} => {
        write!(f,"Function(name: {}, args: {:#?}, out: {:#?})",humanize(name),arguments,out)?
      },
      Transformation::Constant{table_id, value} => write!(f,"Constant(table_id: {:?}, value: {:?})",table_id, value)?,
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => write!(f,"ColumnAlias(table_id: {:?}, column_ix: {}, column_alias: {})",table_id,column_ix,humanize(column_alias))?,
      Transformation::CopySSU8((arg,ix,out)) => write!(f,"CopySSU8(arg: {:?}, ix: {}, out: {:?})",arg.borrow(),ix,out.borrow())?,
      Transformation::CopyTable((arg,out)) => write!(f,"CopyTable(arg: \n{:?}\nout: \n{:?}\n)",arg.borrow(),out.borrow())?,
      Transformation::AddSSU8(args) => write!(f,"AddSSU8(args: \n{:?}\n{:?}\n{:?}\n)",args[0].borrow(),args[1].borrow(),args[2].borrow())?,
      Transformation::AddVVU8(args) => write!(f,"AddVVU8(args: \n{:?}\n{:?}\n{:?}\n)",args[0].borrow(),args[1].borrow(),args[2].borrow())?,
      _ => write!(f,"Tfm Print Not Implemented")?
    }
    Ok(())
  }
}

impl Transformation {
  pub fn solve(&mut self) {
    match &*self {
      // MATH
      // f32 arithmetic
      Transformation::AddSSF32((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] + (rhs.borrow())[0]; }

      // u8 arithmetic
      Transformation::AddSSU8(args) => { (args[2].borrow_mut())[0] = (args[0].borrow())[0] + (args[1].borrow())[0]; }
      Transformation::AddSSU8(args) => { (args[2].borrow_mut())[0] = (args[0].borrow())[0] + (args[1].borrow())[0]; }

      Transformation::DivideSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] / (rhs.borrow())[0]; }
      Transformation::MultiplySSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] * (rhs.borrow())[0]; }
      Transformation::SubtractSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] - (rhs.borrow())[0]; }
      Transformation::ExponentSSU8((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0].pow((rhs.borrow())[0] as u32); }

      Transformation::AddSSIPF32((lhs, rhs)) => { ((lhs.borrow_mut())[0]) += (*rhs.borrow())[0] }
      Transformation::AddVVIPF32((lhs, rhs)) => { lhs.borrow_mut().iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVVIPF32(args) => { args[0].borrow_mut().par_iter_mut().zip(&(*args[1].borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVSIPF32(args) => { 
        let rhs = args[1].borrow()[0];
        args[0].borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      Transformation::AddVVU8(args) => { 
        let lhs = args[0].borrow();
        let rhs = args[1].borrow();
        args[2].borrow_mut().iter_mut().zip(lhs.iter()).zip(rhs.iter()).for_each(|((out, lhs), rhs)| *out = *lhs + rhs); 
      }
      Transformation::ParMultiplyVSF32(args) => { 
        let rhs = args[1].borrow()[0];
        args[2].borrow_mut().par_iter_mut().zip(&(*args[0].borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
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
      Transformation::ParOrVV(args) => { args[2].borrow_mut().par_iter_mut().zip(&(*args[0].borrow())).zip(&(*args[1].borrow())).for_each(|((out, lhs),rhs)| *out = *lhs || *rhs); }
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
      Transformation::CopySSU8((rhs, ix, out)) => { (out.borrow_mut())[0] = (rhs.borrow())[*ix] }
      Transformation::CopySSString((rhs, ix, out)) => { (out.borrow_mut())[0] = (rhs.borrow())[*ix].clone() }
      Transformation::ConcatVU8((args, out)) => {
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
      Transformation::SetVVU8((src,dest)) => { dest.borrow_mut().iter_mut().zip(&(*src.borrow())).for_each(|(dest,src)| *dest = *src); }
      Transformation::CopyTable((arg,out)) => {
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
      x => println!("Not Implemented: {:?}", x),
    }
  }
}