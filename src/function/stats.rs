use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref COLUMN: u64 = hash_str("column");
  pub static ref ROW: u64 = hash_str("row");
  pub static ref TABLE: u64 = hash_str("table");
  pub static ref STATS_SUM: u64 = hash_str("stats/sum");
}


// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumV<T,U> {
  pub col: (ColumnV<T>,usize,usize),
  pub out: ColumnV<U>
}

impl<T,U> MechFunction for StatsSumV<T,U>
where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send + num_traits::identities::Zero,
      U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send + num_traits::identities::Zero,
{
  fn solve(&self) {
    let (col,six,eix) = &self.col;
    let result = col.borrow()[*six..=*eix].iter().fold(identities::Zero::zero(),|sum, n| sum + *n);
    self.out.borrow_mut()[0] = T::into(result);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


/*
// stats/sum(table: x)
#[derive(Debug)]
pub struct StatsSumTable {
  pub table: ArgTable, pub out: ColumnV<F32>
}

impl MechFunction for StatsSumTable {
  fn solve(&self) {
    let mut sum = 0.0;
    let table_brrw = self.table.borrow();
    let table_els = table_brrw.rows * table_brrw.cols;
    for i in 0..table_els {
      match table_brrw.get_linear(i) {
        Ok(Value::F32(val)) => {
          sum += val
        },
        _ => (),
      }
    }
    (*self.out.borrow_mut())[0] = sum;
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct StatsSumRow {
  pub table: ArgTable, pub out: ColumnV<F32>
}

impl MechFunction for StatsSumRow {
  fn solve(&self) {
    let table_brrw = self.table.borrow();
    for row in 0..table_brrw.rows {
      let mut sum = 0.0;
      for col in 0..table_brrw.cols {
        match table_brrw.get_raw(row,col) {
          Ok(Value::F32(val)) => {
            sum += val
          },
          _ => (),
        }
      }
      (*self.out.borrow_mut())[row] = sum;
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumColVIx<T> {
  pub col: Arg<T>, pub ix: Arg<bool>, pub out: Out<T>
}

impl<T> MechFunction for StatsSumColVIx<T>
where T: std::ops::Add<Output = T> + Debug + Copy + Num {
  fn solve(&self) {
    let result = self.col.borrow()
                         .iter()
                         .zip(self.ix.borrow().iter())
                         .fold(identities::Zero::zero(),|sum, (n,ix)| if *ix {sum + *n} else {sum});
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// stats/sum(column: x{ix})
#[derive(Debug)]
pub struct StatsSumColTIx {
  pub col: ArgTable, pub ix: Arg<bool>, pub out: Out<f32>
}

impl MechFunction for StatsSumColTIx {
  fn solve(&self) {
    let mut sum = 0.0;
    let table_brrw = self.col.borrow();
    let ix_brrw = self.ix.borrow();
    for i in 0..ix_brrw.len() {
      match (table_brrw.get_linear(i),ix_brrw[i]) {
        (Ok(Value::F32(val)),ix_value) => {
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
}*/

pub struct StatsSum{}

impl MechFunctionCompiler for StatsSum {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    if arguments.len() > 1 {
      return Err(MechError::GenericError(6352));
    }
    let (out_table_id, _, _) = out;
    let arg_cols = block.get_whole_table_arg_cols(&arguments[0])?;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.resize(1,arg_cols.len());
    for (col_ix,(arg_name,arg_col,row_index)) in arg_cols.iter().enumerate() {
      if *arg_name == *COLUMN {
        println!("COLUMN!!!!!!!!!!");
        out_brrw.set_col_kind(col_ix,arg_col.kind());
        let mut out_col = out_brrw.get_col_raw(col_ix)?;
        match (arg_col,row_index,out_col) {
          (Column::F32(col),ColumnIndex::All,Column::F32(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          x => {
            println!("{:?}",x);
            return Err(MechError::GenericError(6356));
          }
        }
      }
      else {
        return Err(MechError::GenericError(6357));
      }
    }
    /*else if arg_name == *ROW { 
      let arg_table = block.get_table(&arg_table_id)?;
      out_brrw.resize(arg_table.borrow().rows,1);
      out_brrw.set_kind(ValueKind::F32);
      let out_col = out_brrw.get_column_unchecked(0).get_f32().unwrap();
      block.plan.push(StatsSumRow{table: arg_table.clone(), out: out_col.clone()})
    } 
    else if arg_name == *TABLE {
      let arg_table = block.get_table(&arg_table_id)?;
      out_brrw.resize(1,1);
      out_brrw.set_kind(ValueKind::F32);
      let out_col = out_brrw.get_column_unchecked(0).get_f32().unwrap();
      block.plan.push(StatsSumTable{table: arg_table.clone(), out: out_col.clone()})
    }*/ 
    Ok(())
  }
}