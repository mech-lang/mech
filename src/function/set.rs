use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref COLUMN: u64 = hash_str("column");
  pub static ref ROW: u64 = hash_str("row");
  pub static ref TABLE: u64 = hash_str("table");
  pub static ref SET_ANY: u64 = hash_str("set/any");
  pub static ref SET_ALL: u64 = hash_str("set/all");  
}

// set/any(column: x)
#[derive(Debug)]
pub struct SetAnyCol {
  pub col: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for SetAnyCol {
  fn solve(&mut self) {
    let result = self.col.borrow().iter().any(|x| *x == true);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// set/all(column: x)
#[derive(Debug)]
pub struct SetAllCol {
  pub col: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for SetAllCol {
  fn solve(&mut self) {
    let result = self.col.borrow().iter().all(|x| *x == true);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct SetAll{}
impl MechFunctionCompiler for SetAll {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let (arg_name, mut arg_column,_) = block.get_arg_columns(arguments)?[0].clone();
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.set_col_kind(0,ValueKind::Bool);
    out_brrw.resize(1,1);
    let out_col = out_brrw.get_column_unchecked(0).get_bool().unwrap();
    if arg_name == *COLUMN {
      match arg_column {
        Column::Bool(col) => block.plan.push(SetAllCol{col: col.clone(), out: out_col.clone()}),
        _ => {return Err(MechError::GenericError(6595));},
      }
    } 
    Ok(())
  }
}

pub struct SetAny{}
impl MechFunctionCompiler for SetAny {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let (arg_name, mut arg_column,_) = block.get_arg_columns(arguments)?[0].clone();
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.set_col_kind(0,ValueKind::Bool);
    out_brrw.resize(1,1);
    let out_col = out_brrw.get_column_unchecked(0).get_bool().unwrap();
    if arg_name == *COLUMN {
      match arg_column {
        Column::Bool(col) => block.plan.push(SetAnyCol{col: col.clone(), out: out_col.clone()}),
        _ => {return Err(MechError::GenericError(6391));},
      }
    }
    Ok(())
  }
}
