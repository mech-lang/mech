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
pub enum Function {
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
  GreaterThanVSU8((ArgU8,ArgU8,OutBool)),
  GreaterThanSSU8((ArgU8,ArgU8,OutBool)),
  GreaterThanVVU8((ArgU8,ArgU8,OutBool)),
  GreaterThanEqualVVU8((ArgU8,ArgU8,OutBool)),

  LessThanSSU8((ArgU8,ArgU8,OutBool)),
  LessThanVVU8((ArgU8,ArgU8,OutBool)),
  ParCSGreaterThanVS((ArgF32,f32,f32)),
  ParSetVS((ArgBool,f32,OutF32)),
  ParSetVV((ArgBool,ArgF32,OutF32)),
  SetVVU8((ArgU8,OutU8)),
  StatsSumColU8((ArgU8,OutU8)),
  ParCopyVV((ArgF32,OutF32)),
  ParCopyVVU8((ArgU8,OutU8)),
  HorizontalConcatenate((Vec<ArgTable>,OutTable)),
  CopySSU8((ArgU8,usize,OutU8)),
  CopySSString((ArgString,usize,OutString)),
  ConcatVU8((Vec<ArgU8>,OutU8)),
  CopyTable((ArgTable,OutTable)),
  CopyVBU8((ArgU8, ArgBool, OutTable)),
  RangeU8((ArgU8,ArgU8,OutTable)),
  Null,
}

#[derive(Clone)]
pub enum Transformation {
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
      _ => write!(f,"Tfm Print Not Implemented")?
    }
    Ok(())
  }
}
      
      
impl fmt::Debug for Function {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {      
      Function::AddSSU8(args) => write!(f,"AddSSU8(args: \n{:?}\n{:?}\n{:?}\n)",args[0].borrow(),args[1].borrow(),args[2].borrow())?,
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

impl Function {
  pub fn solve(&mut self) {
    match &*self {
      // MATH
      // f32 arithmetic
      Function::AddSSF32((lhs, rhs, out)) => { (out.borrow_mut())[0] = (lhs.borrow())[0] + (rhs.borrow())[0]; }

      // u8 arithmetic
      Function::AddSSU8(args) => { (args[2].borrow_mut())[0] = (args[0].borrow())[0] + (args[1].borrow())[0]; }
      Function::AddSSU8(args) => { (args[2].borrow_mut())[0] = (args[0].borrow())[0] + (args[1].borrow())[0]; }

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
      Function::AddVVU8(args) => { 
        let lhs = args[0].borrow();
        let rhs = args[1].borrow();
        args[2].borrow_mut().iter_mut().zip(lhs.iter()).zip(rhs.iter()).for_each(|((out, lhs), rhs)| *out = *lhs + rhs); 
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
        println!("{:?}", filtered);
        for row in 0..filtered.len() {
          println!("{:?}", row);
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