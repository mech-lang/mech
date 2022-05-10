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
  pub static ref SET_CARTESIAN: u64 = hash_str("set/cartesian");  
}

// set/any(column: x)
#[derive(Debug)]
pub struct SetAnyCol {
  pub col: ColumnV<bool>, pub out: ColumnV<bool>
}

impl MechFunction for SetAnyCol {
  fn solve(&self) {
    let result = self.col.borrow().iter().any(|x| *x == true);
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// set/all(column: x)
#[derive(Debug)]
pub struct SetAllCol {
  pub col: ColumnV<bool>, pub out: ColumnV<bool>
}

impl MechFunction for SetAllCol {
  fn solve(&self) {
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
    out_brrw.resize(1,1);
    out_brrw.set_col_kind(0,ValueKind::Bool);
    let out_col = out_brrw.get_column_unchecked(0);
    if arg_name == *COLUMN {
      match (arg_column,out_col) {
        (Column::Bool(col),Column::Bool(out)) => block.plan.push(SetAllCol{col: col.clone(), out: out.clone()}),
        x => {return Err(MechError{id: 4687, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
    out_brrw.resize(1,1);
    out_brrw.set_col_kind(0,ValueKind::Bool);
    let out_col = out_brrw.get_column_unchecked(0);
    if arg_name == *COLUMN {
      match (arg_column,out_col) {
        (Column::Bool(col),Column::Bool(out)) => block.plan.push(SetAnyCol{col: col.clone(), out: out.clone()}),
        x => {return Err(MechError{id: 4688, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    } 
    Ok(())
  }
}

// set/cartesian(column: x)
#[derive(Debug)]
pub struct SetCartLeftV<T> {
  pub col: (ColumnV<T>,usize),
  pub out: ColumnV<T>
}

impl<T> MechFunction for SetCartLeftV<T>
where T: Copy + Debug + Clone + Sync + Send,
{
  fn solve(&self) {
    let (col,len) = &self.col;
    let col_brrw = col.borrow();
    col_brrw.iter().flat_map(|n| std::iter::repeat(n).take(*len))
              .zip(self.out.borrow_mut().iter_mut())
              .for_each(|(c,o)| *o = *c);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct SetCartRightV<T> {
  pub col: ColumnV<T>,
  pub out: ColumnV<T>
}

impl<T> MechFunction for SetCartRightV<T>
where T: Copy + Debug + Clone + Sync + Send,
{
  fn solve(&self) {
    self.out.borrow_mut()
            .iter_mut()
            .zip(self.col.borrow().iter().cycle())
            .for_each(|(out,col)| *out = *col);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


pub struct SetCartesian{}
impl MechFunctionCompiler for SetCartesian {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let (_, mut lhs_arg_column,_) = block.get_arg_columns(arguments)?[0].clone();
    let (_, mut rhs_arg_column,_) = block.get_arg_columns(arguments)?[1].clone();
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    let arg_dims = block.get_arg_dims(&arguments)?;
    match (&arg_dims[0],&arg_dims[1]) {
      (TableShape::Column(rows_left), TableShape::Column(rows_right)) => {
        out_brrw.resize(rows_left * rows_right, 2);
        out_brrw.set_col_kind(0,lhs_arg_column.kind());
        out_brrw.set_col_kind(1,rhs_arg_column.kind());
        let out_left_col = out_brrw.get_column_unchecked(0);
        match (lhs_arg_column,out_left_col) {
          (Column::F32(col),Column::F32(out)) => block.plan.push(SetCartLeftV{col: (col.clone(), *rows_right), out: out.clone()}),
          x => {return Err(MechError{id: 4689, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
        let out_right_col = out_brrw.get_column_unchecked(1);
        match (rhs_arg_column,out_right_col) {
          (Column::F32(col),Column::F32(out)) => block.plan.push(SetCartRightV{col: col.clone(), out: out.clone()}),
          x => {return Err(MechError{id: 4690, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 4691, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}
