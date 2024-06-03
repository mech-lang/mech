use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::{Zero,zero};
use std::ops::*;
#[cfg(feature = "parallel")]
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
where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send + Zero,
      U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send + Zero,
{
  fn solve(&self) {
    let (col,six,eix) = &self.col;
    let result = col.borrow()[*six..=*eix].iter().fold(zero(),|sum, n| sum + *n);
    self.out.borrow_mut()[0] = T::into(result);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// stats/sum(table)
#[derive(Debug)]
pub struct StatsSumTable<T,U> {
  pub cols: Vec<ColumnV<T>>,
  pub rows: usize,
  pub out: ColumnV<U>
}

impl<T,U> MechFunction for StatsSumTable<T,U>
where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send + Zero,
      U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send + Zero,
{
  fn solve(&self) {
    let mut sum: T = zero();
    for row in 0..self.rows {
      for col in &self.cols {
        let col_brrw = col.borrow();
        sum = sum + col_brrw[row];
      }
    }
    (*self.out.borrow_mut())[0] = T::into(sum);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct StatsSumRow<T,U> {
  pub cols: Vec<ColumnV<T>>,
  pub rows: usize,
  pub out: ColumnV<U>
}

impl<T,U> MechFunction for StatsSumRow<T,U>
where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send + Zero,
      U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send + Zero,
{
  fn solve(&self) {
    for row in 0..self.rows {
      let mut sum: T = zero();
      for col in &self.cols {
        let col_brrw = col.borrow();
        sum = sum + col_brrw[row];
      }
      (*self.out.borrow_mut())[row] = T::into(sum);
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// stats/sum(column: x)
#[derive(Debug)]
pub struct StatsSumVB<T,U> {
  pub col: ColumnV<T>, pub ix: ColumnV<bool>, pub out: ColumnV<U>
}

impl<T,U> MechFunction for StatsSumVB<T,U>
where T: std::ops::Add<Output = T> + Debug + Copy + Into<U> + Zero, 
      U: std::ops::Add<Output = U> + Debug + Copy + Into<T> + Zero,
{
  fn solve(&self) {
    let result = self.col.borrow()
                         .iter()
                         .zip(self.ix.borrow().iter())
                         .fold(zero(),|sum, (n,ix)| if *ix {sum + T::into(*n)} else {sum});
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// stats/sum(column: x{ix})
#[derive(Debug)]
pub struct StatsSumTB {
  pub col: ArgTable, pub ix: Arg<bool>, pub out: ColumnV<F32>
}

impl MechFunction for StatsSumTB {
  fn solve(&self) {
    let mut sum = 0.0;
    let table_brrw = self.col.borrow();
    let ix_brrw = self.ix.borrow();
    for i in 0..ix_brrw.len() {
      match (table_brrw.get_linear(i),ix_brrw[i]) {
        (Ok(Value::F32(val)),ix_value) => {
          if ix_value {
            sum = sum + val.unwrap()
          }
        },
        _ => (),
      }
    }
    (*self.out.borrow_mut())[0] = F32::new(sum);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct StatsSum{}

impl MechFunctionCompiler for StatsSum {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    if arguments.len() > 1 {
      return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3040, kind: MechErrorKind::GenericError("Too many function arguments".to_string())});
    }
    let (out_table_id, _, _) = out;
    let arg_col = block.get_arg_column(&arguments[0])?;
    let arg_cols = vec![arg_col]; // This is a hack for now until it's fixed later
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.resize(1,arg_cols.len());           
    for (col_ix,(arg_name,arg_col,row_index)) in arg_cols.iter().enumerate() {
      if *arg_name == *COLUMN {
        out_brrw.set_col_kind(col_ix,arg_col.kind());
        let mut out_col = out_brrw.get_col_raw(col_ix)?;
        match (arg_col,row_index,out_col) {
          (Column::Length(col),ColumnIndex::All,Column::Length(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::F32(col),ColumnIndex::All,Column::F32(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::F32(col),ColumnIndex::Bool(bix),Column::F32(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::F32(col),ColumnIndex::Index(ix),Column::F32(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::F64(col),ColumnIndex::All,Column::F64(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::F64(col),ColumnIndex::Bool(bix),Column::F64(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::F64(col),ColumnIndex::Index(ix),Column::F64(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::U8(col),ColumnIndex::All,Column::U8(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::U8(col),ColumnIndex::Bool(bix),Column::U8(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::U8(col),ColumnIndex::Index(ix),Column::U8(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::U16(col),ColumnIndex::All,Column::U16(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::U16(col),ColumnIndex::Bool(bix),Column::U16(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::U16(col),ColumnIndex::Index(ix),Column::U16(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::U32(col),ColumnIndex::All,Column::U32(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::U32(col),ColumnIndex::Bool(bix),Column::U32(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::U32(col),ColumnIndex::Index(ix),Column::U32(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::U64(col),ColumnIndex::All,Column::U64(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::U64(col),ColumnIndex::Bool(bix),Column::U64(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::U64(col),ColumnIndex::Index(ix),Column::U64(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::U128(col),ColumnIndex::All,Column::U128(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::U128(col),ColumnIndex::Bool(bix),Column::U128(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::U128(col),ColumnIndex::Index(ix),Column::U128(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::I8(col),ColumnIndex::All,Column::I8(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::I8(col),ColumnIndex::Bool(bix),Column::I8(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::I8(col),ColumnIndex::Index(ix),Column::I8(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::I16(col),ColumnIndex::All,Column::I16(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::I16(col),ColumnIndex::Bool(bix),Column::I16(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::I16(col),ColumnIndex::Index(ix),Column::I16(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::I32(col),ColumnIndex::All,Column::I32(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::I32(col),ColumnIndex::Bool(bix),Column::I32(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::I32(col),ColumnIndex::Index(ix),Column::I32(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::I64(col),ColumnIndex::All,Column::I64(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::I64(col),ColumnIndex::Bool(bix),Column::I64(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::I128(col),ColumnIndex::All,Column::I128(out)) => block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()}),
          (Column::I128(col),ColumnIndex::Bool(bix),Column::I128(out)) => block.plan.push(StatsSumVB{col: col.clone(), ix: bix.clone(), out: out.clone()}),
          (Column::I128(col),ColumnIndex::Index(ix),Column::I128(out)) => block.plan.push(StatsSumV{col: (col.clone(),*ix,*ix), out: out.clone()}),
          (Column::Reference((ref table, (ColumnIndex::All, ColumnIndex::All))),ColumnIndex::All,Column::F32(out)) => {
            let table_brrw = table.borrow();
            out_brrw.resize(1,table_brrw.cols);
            out_brrw.set_kind(table_brrw.kind());
            for i in 0..table_brrw.cols {
              if let (Column::F32(col),Column::F32(out)) = (table_brrw.get_col_raw(i)?, out_brrw.get_col_raw(i)?) {
                block.plan.push(StatsSumV{col: (col.clone(),0,col.len()-1), out: out.clone()});
              }
            }
          }
          (Column::Reference((ref table, (ColumnIndex::Bool(ix_col), ColumnIndex::None))),_,Column::F32(out)) => block.plan.push(StatsSumTB{col: table.clone(), ix: ix_col.clone(), out: out.clone()}),
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3041, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      else if *arg_name == *ROW {
        let (arg_name,arg_table_id,_) = arguments[0];
        let arg_table = block.get_table(&arg_table_id)?;
        let kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        };
        match kind {
          ValueKind::Compound(_) => {
            return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3042, kind: MechErrorKind::GenericError("stat/sum(row) doesn't support compound table kinds.".to_string())});
          }
          k => {
            out_brrw.resize(arg_table.borrow().rows,1);
            out_brrw.set_kind(k);
          }
        }
        match out_brrw.get_column_unchecked(0) {
          Column::U8(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U8>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U8(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U16(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U16>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U16(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U128(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U128>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U128(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I8(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I8>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I8(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I16(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I16>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I16(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I128(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I128>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I128(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::F32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }  
          Column::F64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumRow{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }         
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3043, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      } 
      else if *arg_name == *TABLE {
        let (arg_name,arg_table_id,_) = arguments[0];
        let arg_table = block.get_table(&arg_table_id)?;
        let kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        };
        match kind {
          ValueKind::Compound(_) => {
            return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3044, kind: MechErrorKind::GenericError("stat/sum(row) doesn't support compound table kinds.".to_string())});
          }
          k => {
            out_brrw.resize(1,1);
            out_brrw.set_kind(k);
          }
        }
        match out_brrw.get_column_unchecked(0) {
          Column::U8(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U8>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U8(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U16(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U16>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U16(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::U128(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<U128>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::U128(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I8(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I8>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I8(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I16(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I16>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I16(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::I128(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<I128>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::I128(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }
          Column::F32(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }    
          Column::F64(out_col) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F64>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F64(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(StatsSumTable{cols: cols.clone(), rows: rows, out: out_col.clone()});
          }        
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3045, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      else {  
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 3046, kind: MechErrorKind::UnknownFunctionArgument(*arg_name)});
      }
    } 
    Ok(())
  }
}