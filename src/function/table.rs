use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref TABLE_RANGE: u64 = hash_str("table/range");
  pub static ref TABLE_SPLIT: u64 = hash_str("table/split");
  pub static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_str("table/horizontal-concatenate");
  pub static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_str("table/vertical-concatenate");
  pub static ref TABLE_APPEND: u64 = hash_str("table/append");
  pub static ref TABLE_SIZE: u64 = hash_str("table/size");
  pub static ref TABLE: u64 = hash_str("table");
}

// Concat Vectors
#[derive(Debug)]
pub struct ConcatV<T> 
where T: Clone + Debug
{
  pub args: Vec<Arg<T>>, 
  pub out: Out<T>,
}

impl<T> MechFunction for ConcatV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let mut arg_ix = 0;
    let mut ix = 0;
    let mut arg_brrw = self.args[arg_ix].borrow();
    for r in 0..out_brrw.len() {
      out_brrw[r] = arg_brrw[ix].clone();
      ix += 1;
      if ix == arg_brrw.len() {
        ix = 0;
        arg_ix += 1;
        if arg_ix == self.args.len() {
          return;
        } else {
          arg_brrw = self.args[arg_ix].borrow();
        }
      } 
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector : Vector
#[derive(Debug)]
pub struct CopyVV<T> 
where T: Clone + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for CopyVV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = arg.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Parallel Copy Vector : Vector
#[derive(Debug)]
pub struct ParCopyVV<T> 
where T: Clone + Debug + Sync + Send
{
  pub arg: Arg<T>, pub out: Out<T>
}
impl<T> MechFunction for ParCopyVV<T> 
where T: Clone + Debug + Sync + Send
{
  fn solve(&mut self) {
    self.out.borrow_mut().par_iter_mut().zip(self.arg.borrow().par_iter()).for_each(|(out, arg)| *out = arg.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Scalar : Vector
#[derive(Debug)]
pub struct CopySV<T> 
where T: Clone + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Out<T>
}
impl<T> MechFunction for CopySV<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    let arg = self.arg.borrow()[self.ix].clone();
    self.out.borrow_mut().iter_mut().for_each(|out| *out = arg.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector : Vector Ref
#[derive(Debug)]
pub struct CopyVVRef {
  pub arg: Arg<TableId>, pub out: Out<TableId>
}
impl MechFunction for CopyVVRef {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| {
      let id = TableId::Global(*arg.unwrap());
      *out = id;
    }); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Reference
#[derive(Debug)]
pub struct CopySVRef {
  pub arg: Arg<TableId>, pub ix: usize , pub out: Out<TableId>
}
impl MechFunction for CopySVRef 
{
  fn solve(&mut self) {
    let id = TableId::Global(*self.arg.borrow()[self.ix].unwrap());
    self.out.borrow_mut().iter_mut().for_each(|out| *out = id.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Scalar : Scalar
#[derive(Debug)]
pub struct CopySS<T> {
  pub arg: Arg<T>, pub ix: usize , pub out: Out<T>
}
impl<T> MechFunction for CopySS<T> 
where T: Clone + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.arg.borrow())[self.ix].clone()
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Reference
#[derive(Debug)]
pub struct CopySSRef {
  pub arg: Arg<TableId>, pub ix: usize , pub out: Out<TableId>
}
impl MechFunction for CopySSRef 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = TableId::Global(*self.arg.borrow()[self.ix].unwrap())
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector{Bool Ix} : Vector
#[derive(Debug)]
pub struct CopyVB<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: Arg<bool>, pub out: Out<T>
}

impl<T> MechFunction for CopyVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    // Filter the column to include only elements with a "true" index
    let filtered: Vec<T>  = 
      self.arg.borrow()
         .iter()
         .zip(self.ix.borrow().iter())
         .filter_map(|(x,ix)| if *ix {Some(*x)} else {None})
         .collect::<Vec<T>>();
    let mut out_brrw = self.out.borrow_mut();
    let rows = filtered.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,filtered[0]);
    }
    for row in 0..filtered.len() {
      out_brrw[row] = filtered[row];
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Vector{Int Ix} : Vector
#[derive(Debug)]
pub struct CopyVI<T>  {
  pub arg: Arg<T>, pub ix: Arg<usize>, pub out: Out<T>
}

impl<T> MechFunction for CopyVI<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let ix_brrw = self.ix.borrow();

    let rows = ix_brrw.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,arg_brrw[0]);
    }
    for (out_ix, row) in ix_brrw.iter().enumerate() {
      out_brrw[out_ix] = arg_brrw[*row as usize - 1];
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Scalar : Scalar
#[derive(Debug)]
pub struct SetSIxSIx<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>, pub oix: usize
}
impl<T> MechFunction for SetSIxSIx<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[self.oix] = (self.arg.borrow())[self.ix];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Scalar : Vector {Bool}
#[derive(Debug)]
pub struct SetSIxVB<T> 
where T: Copy + Debug
{
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>, pub oix: Arg<bool>
}
impl<T> MechFunction for SetSIxVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let oix_brrw = self.oix.borrow();
    for row in 0..oix_brrw.len() {
      if oix_brrw[row] {
        (self.out.borrow_mut())[row] = (self.arg.borrow())[self.ix];
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Scalar : Vector {Bool}
#[derive(Debug)]
pub struct CopyTB {
  pub arg: ArgTable, pub ix: ArgTable, pub out: OutTable, 
}
impl MechFunction for CopyTB
{
  fn solve(&mut self) {
    let ix_brrw = self.ix.borrow();
    let rows = ix_brrw.logical_len();

    let src_brrw = self.arg.borrow();
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.resize(rows,1);
    let mut i = 0;
    for j in 0..ix_brrw.len() {
      match ix_brrw.get_linear(j) {
        Ok(Value::Bool(false)) => continue,
        Ok(Value::Bool(true)) => {
          let value = src_brrw.get_linear(j).unwrap();
          out_brrw.set_linear(i,value).unwrap();
          i+=1;
        }
        _ => (),
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Vector : Vector {Bool}
#[derive(Debug)]
pub struct SetVVB<T> {
  pub arg: Arg<T>, pub out: Arg<T>, pub oix: Arg<bool>
}

impl<T> MechFunction for SetVVB<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.oix.borrow().iter()).zip(self.arg.borrow().iter()).for_each(|((out,oix),x)| if *oix == true {
      *out = *x
    });
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct ParSetVVB<T> {
  pub arg: Arg<T>, pub out: Arg<T>, pub oix: Arg<bool>
}

impl<T> MechFunction for ParSetVVB<T> 
where T: Copy + Debug + Sync + Send
{
  fn solve(&mut self) {
    self.out.borrow_mut().par_iter_mut().zip(self.oix.borrow().par_iter()).zip(self.arg.borrow().par_iter()).for_each(|((out,oix),x)| if *oix == true {
      *out = *x
    });
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Set Vector : Vector
#[derive(Debug)]
pub struct SetVV<T> {
  pub arg: Arg<T>, pub out: Arg<T>
}

impl<T> MechFunction for SetVV<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = *arg); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct ParSetVV<T> {
  pub arg: Arg<T>, pub out: Arg<T>
}

impl<T> MechFunction for ParSetVV<T> 
where T: Copy + Debug + Sync + Send
{
  fn solve(&mut self) {
    self.out.borrow_mut().par_iter_mut().zip(self.arg.borrow().par_iter()).for_each(|(out, arg)| *out = *arg); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct SetVS<T> {
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>
}

impl<T> MechFunction for SetVS<T> 
where T: Copy + Debug
{
  fn solve(&mut self) {
    let arg = self.arg.borrow()[self.ix];
    self.out.borrow_mut().iter_mut().for_each(|out| *out = arg); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[derive(Debug)]
pub struct ParSetVS<T> {
  pub arg: Arg<T>, pub ix: usize, pub out: Arg<T>
}

impl<T> MechFunction for ParSetVS<T> 
where T: Copy + Debug + Sync + Send
{
  fn solve(&mut self) {
    let arg = self.arg.borrow()[self.ix];
    self.out.borrow_mut().par_iter_mut().for_each(|out| *out = arg); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Copy Table : Table
#[derive(Debug)]
pub struct CopyT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for CopyT {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();

    out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
    for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
      out_brrw.set_col_kind(col, kind.clone());
    }
    out_brrw.column_ix_to_alias = arg_brrw.column_ix_to_alias.clone();
    out_brrw.column_alias_to_ix = arg_brrw.column_alias_to_ix.clone();
    for col in 0..arg_brrw.cols {
      for row in 0..arg_brrw.rows {
        let value = arg_brrw.get(row,col).unwrap();
        out_brrw.set(row,col,value);
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// AppendRow Table : Table
#[derive(Debug)]
pub struct AppendRowT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for AppendRowT {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    let ocols = out_brrw.cols;
    let arows = arg_brrw.rows;
    out_brrw.resize(orows + arows, ocols);
    if arg_brrw.has_col_aliases() {
      for col in 0..arg_brrw.cols {
        for row in 0..arows {
          let value = arg_brrw.get(row,col).unwrap();
          let alias = arg_brrw.column_ix_to_alias[col];
          match out_brrw.column_alias_to_ix.get(&alias) {
            Some(col_ix) => {out_brrw.set(orows + row,*col_ix,value);}
            None => (), // TODO Error
          }
        }
      }
    } else {
      for col in 0..arg_brrw.cols {
        for row in 0..arows {
          let value = arg_brrw.get(row,col).unwrap();
          out_brrw.set(orows + row,col,value);
        }
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// AppendRow Table : Table
#[derive(Debug)]
pub struct AppendRowSV {
  pub arg: ArgTable, pub ix: usize,  pub out: OutTable
}

impl MechFunction for AppendRowSV {
  fn solve(&mut self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    out_brrw.resize(orows + 1, 1);
    let value = arg_brrw.get_linear(self.ix).unwrap();
    out_brrw.set(orows,0,value);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct TableVerticalConcatenate{}
impl MechFunctionCompiler for TableVerticalConcatenate {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {

    // Get all of the tables
    let mut arg_tables = vec![];
    let mut rows = 0;
    let mut cols = 0;
    for (_,table_id,_) in arguments {
      let table = block.get_table(table_id)?;
      arg_tables.push(table);
    }

    // Each table should have the same number of columns
    let cols = arg_tables[0].borrow().cols;
    let consistent_cols = arg_tables.iter().all(|arg| {arg.borrow().cols == cols});
    if consistent_cols == false {
      return Err(MechError::GenericError(1243));
    }
    
    // Check to make sure column types are consistent
    let col_kinds: Vec<ValueKind> = arg_tables[0].borrow().col_kinds.clone();
    let consistent_col_kinds = arg_tables.iter().all(|arg| arg.borrow().col_kinds.iter().zip(&col_kinds).all(|(k1,k2)| *k1 == *k2));
    if consistent_cols == false {
      return Err(MechError::GenericError(1244));
    }

    // Add up the rows
    let rows = arg_tables.iter().fold(0, |acc, table| acc + table.borrow().rows);
    
    // Resize out table to match dimensions 
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.resize(rows,cols);

    // Set out column kind and push a concat function
    for (ix, kind) in (0..cols).zip(col_kinds.clone()) {
      out_brrw.set_col_kind(ix, kind);
      let out_col = out_brrw.get_column_unchecked(ix).clone();
      let mut argument_columns = vec![];       
      for table in &arg_tables {
        let table_brrw = table.borrow();
        let column = table_brrw.get_column(&TableIndex::Index(ix+1))?;
        argument_columns.push(column.clone());
      }

      match out_col {
        Column::U8(ref out_c) => {
          let mut u8_cols:Vec<ColumnV<u8>> = vec![];
          for colv in argument_columns {
            u8_cols.push(colv.get_u8()?.clone());
          }
          let fxn = ConcatV::<u8>{args: u8_cols, out: out_c.clone()};
          block.plan.push(fxn);
        }
        Column::U16(ref out_c) => {
          let mut u16_cols:Vec<ColumnV<u16>> = vec![];
          for colv in argument_columns {
            u16_cols.push(colv.get_u16()?.clone());
          }
          let fxn = ConcatV::<u16>{args: u16_cols, out: out_c.clone()};
          block.plan.push(fxn);
        }
        Column::Bool(ref out_c) => {
          let mut bool_cols:Vec<ColumnV<bool>> = vec![];
          for colv in argument_columns {
            bool_cols.push(colv.get_bool()?.clone());
          }
          let fxn = ConcatV::<bool>{args: bool_cols, out: out_c.clone()};
          block.plan.push(fxn);
        }
        Column::String(ref out_c) => {
          let mut cols:Vec<ColumnV<MechString>> = vec![];
          for colv in argument_columns {
            cols.push(colv.get_string()?.clone());
          }
          let fxn = ConcatV::<MechString>{args: cols, out: out_c.clone()};
          block.plan.push(fxn);
        }
        Column::Ref(ref out_c) => {
          let mut cols:Vec<ColumnV<TableId>> = vec![];
          for colv in argument_columns {
            cols.push(colv.get_reference()?.clone());
          }
          let fxn = ConcatV::<TableId>{args: cols, out: out_c.clone()};
          block.plan.push(fxn);
        }
        x => {
          return Err(MechError::GenericError(6361));
        },
      }
    }
    Ok(())
  }
}

pub struct TableHorizontalConcatenate{}
impl MechFunctionCompiler for TableHorizontalConcatenate {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    // Get all of the tables
    let mut rows = 0;
    let mut cols = 0;
    let arg_shapes = block.get_arg_dims(&arguments)?;
    // Each table should have the same number of rows or be scalar
    let arg_dims: Vec<(usize,usize)> = arg_shapes.iter().map(|shape| match shape {
      TableShape::Scalar => (1,1),
      TableShape::Column(rows) => (*rows,1),
      TableShape::Row(cols) => (1,*cols),
      TableShape::Matrix(rows,cols) => (*rows,*cols),
      _ => (0,0),
    }).collect();

    let max_rows = arg_dims.iter().map(|(rows,_)| rows).max().unwrap();

    let consistent_rows = arg_dims.iter().all(|(rows,_)| {
      max_rows == rows || *rows == 1
    });

    if consistent_rows == false {
      return Err(MechError::GenericError(1245));
    }

    // Add up the columns
    let cols = arg_dims.iter().fold(0, |acc, (_,cols)| acc + cols);
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?.clone();
    let mut o = out_table.borrow_mut();
    o.resize(*max_rows,cols);
    let mut out_column_ix = 0;
    for (argument, shape) in arguments.iter().zip(arg_shapes) {
      match shape {
        TableShape::Scalar => {
          let (_, arg_col,arg_ix) = block.get_arg_column(&argument)?;
          o.set_col_kind(out_column_ix, arg_col.kind());
          let mut out_col = o.get_column_unchecked(out_column_ix);
          match out_col.len() {
            1 => {
              match (&arg_col, &arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopySS::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U16(out)) => {
                  let arg_16 = arg.borrow().iter().map(|a| *a as u16).collect();
                  block.plan.push(CopySS::<u16>{arg: Rc::new(RefCell::new(arg_16)), ix: *ix, out: out.clone()})
                },
                (Column::U16(arg), ColumnIndex::Index(ix), Column::U16(out)) => block.plan.push(CopySS::<u16>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopySS::<MechString>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopySS::<bool>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Empty, _, Column::Empty) => (),
                x => {
                  println!("{:?}", x);
                  return Err(MechError::GenericError(6366));
                },
              };
              out_column_ix += 1;
            }
            _ => {
              match (&arg_col, &arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopySV::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopySV::<MechString>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopySV::<bool>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySVRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Empty, _, Column::Empty) => (),
                x => {return Err(MechError::GenericError(6368));},
              };
              out_column_ix += 1;
            }
          }

        }
        TableShape::Column(rows) => {
          match block.get_arg_column(&argument) {
            // The usual case where we just have a regular column
            Ok((_, arg_col,arg_ix)) => {
              o.set_col_kind(out_column_ix, arg_col.kind());
              let mut out_col = o.get_column_unchecked(out_column_ix);
              let fxn = match (&arg_col, arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV::<u8>{arg: arg.clone(), out: out.clone()}),
                (Column::U64(arg), ColumnIndex::All, Column::U64(out)) => block.plan.push(CopyVV::<u64>{arg: arg.clone(), out: out.clone()}),
                (Column::String(arg), ColumnIndex::All, Column::String(out)) => block.plan.push(CopyVV::<MechString>{arg: arg.clone(), out: out.clone()}),
                (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopyVVRef{arg: arg.clone(), out: out.clone()}),
                (Column::U8(arg), ColumnIndex::Bool(ix), Column::U8(out)) => block.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
                x => {
                  return Err(MechError::GenericError(6367));
                },
              };
              out_column_ix += 1;
            } 
            _ => {
              for (_, arg_col,arg_ix) in block.get_whole_table_arg_cols(&argument)? {
                println!("{:?} {:?}", arg_col, arg_ix);
              }
            }
          }
        }
        TableShape::Row(_) => {
          for (_, arg_col,arg_ix) in block.get_whole_table_arg_cols(&argument)? {
            o.set_col_kind(out_column_ix, arg_col.kind());
            let mut out_col = o.get_column_unchecked(out_column_ix);
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Bool(ix), Column::U8(out)) => block.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
              (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopySS::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
              (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopySS::<u8>{arg: arg.clone(), ix: 0, out: out.clone()}),
              (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopySS::<MechString>{arg: arg.clone(), ix: *ix, out: out.clone()}),
              (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopySS::<bool>{arg: arg.clone(), ix: *ix, out: out.clone()}),
              (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: 0, out: out.clone()}),
              (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
              (Column::Empty, _, Column::Empty) => (),
              x => {
                return Err(MechError::GenericError(6369));},
            };
            out_column_ix += 1;
          }
        }
        TableShape::Matrix(_,_) => {
          for (_, arg_col,arg_ix) in block.get_whole_table_arg_cols(&argument)? {
            o.set_col_kind(out_column_ix, arg_col.kind());
            let mut out_col = o.get_column_unchecked(out_column_ix);
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Bool(ix), Column::U8(out)) => block.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
              (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV::<u8>{arg: arg.clone(), out: out.clone()}),
              (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopyVVRef{arg: arg.clone(), out: out.clone()}),
              x => {
                return Err(MechError::GenericError(6379));},
            };
            out_column_ix += 1;
          }
        }
        x => {
          return Err(MechError::GenericError(6364));
        },
      }
    }
    Ok(())
  }
}

pub struct TableSplit{}
impl MechFunctionCompiler for TableSplit {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {

    let arg_shapes = block.get_arg_dims(&arguments)?;
    let arg_cols = block.get_whole_table_arg_cols(&arguments[0])?;

    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    out_brrw.set_col_kind(0,ValueKind::Reference);
    match arg_shapes[0] {
      TableShape::Matrix(rows,cols) => {
        out_brrw.resize(rows,1);
        // Initialize table
        for row in 0..rows {
          let split_id = hash_str(&format!("{:?}{:?}", out_table_id, row));
          let mut dest_table = Table::new(split_id,1,cols);
          for (col,arg_col) in arg_cols.iter().enumerate() {
            match arg_col {
              (_,Column::U8(_),_) => {
                dest_table.set_col_kind(col,ValueKind::U8);
              }
              _ => {return Err(MechError::GenericError(6095));},
            }
          }
          block.global_database.borrow_mut().insert_table(dest_table);
          out_brrw.set(row,0,Value::Reference(TableId::Global(split_id)));
        }
        // Write functions
        for (col_ix,arg_col) in arg_cols.iter().enumerate() {
          match arg_col {
            (_,Column::U8(src_col),ColumnIndex::All) => {
              for row in 0..rows {
                // get the destination table
                let split_id = hash_str(&format!("{:?}{:?}", out_table_id, row));
                let dest_table = block.get_table(&TableId::Global(split_id))?;
                let dest_col = dest_table.borrow().get_column(&TableIndex::Index(col_ix+1))?;
                match dest_col {
                  Column::U8(dest_col) => {
                    block.plan.push(SetSIxSIx::<u8>{arg: src_col.clone(), ix: row, out: dest_col.clone(), oix: 0});
                  }
                  _ => {return Err(MechError::GenericError(6097));},
                }
              }
            }
            _ => {return Err(MechError::GenericError(5995));},
          }
        }
      }
      _ => (),
    }     
    Ok(())
  }
}


// Copy Vector{Int Ix} : Vector
#[derive(Debug)]
pub struct Range  {
  pub start: Arg<u8>, pub end: Arg<u8>, pub out: OutTable
}

impl MechFunction for Range
{
  fn solve(&mut self) {
    let start_value = self.start.borrow()[0];
    let end_value = self.end.borrow()[0];
    let delta = end_value - start_value + 1;
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.resize(delta as usize,1);
    out_brrw.set_col_kind(0,ValueKind::U8);
    let mut value = start_value;
    for row in 0..out_brrw.rows {
      out_brrw.set(row,0,Value::U8(value));
      value += 1;
    } 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct TableRange{}
impl MechFunctionCompiler for TableRange {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {

    let mut argument_columns = block.get_arg_columns(arguments)?;
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    match (&argument_columns[0], &argument_columns[1]) {
      ((_,Column::U8(start),_), (_,Column::U8(end),_)) => {  
        let fxn = Range{start: start.clone(), end: end.clone(), out: out_table.clone()};
        block.plan.push(fxn);
      }
      _ => {return Err(MechError::GenericError(6349));},
    }
    Ok(())
  }
}

pub struct TableAppend{}
impl MechFunctionCompiler for TableAppend {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {

    let arg_shape = block.get_arg_dim(&arguments[0])?;
    let (_,_,indices) = &arguments[0];
    let (arow_ix,_) = indices[0];

    let (_,src_table_id,src_indices) = &arguments[0];
    let (src_rows,src_cols) = src_indices[0];
    let (dest_table_id, _, _) = out;

    let src_table = block.get_table(&src_table_id)?;
    let dest_table = block.get_table(dest_table_id)?;

    {
      let mut src_table_brrw = src_table.borrow_mut();
      let mut dest_table_brrw = dest_table.borrow_mut();
      match dest_table_brrw.kind() {
        ValueKind::Empty => {
          dest_table_brrw.resize(src_table_brrw.rows,src_table_brrw.cols);
          dest_table_brrw.set_kind(src_table_brrw.kind());
          dest_table_brrw.rows = 0;
        },
        x => {
        }
      }
    }

    let dest_shape = {dest_table.borrow().shape()};
    match (arg_shape,arow_ix,dest_shape) {
      (TableShape::Scalar,TableIndex::Index(ix),TableShape::Column(_)) => {
        block.plan.push(AppendRowSV{arg: src_table.clone(), ix: ix-1, out: dest_table.clone()});
      }
      x => {
        block.plan.push(AppendRowT{arg: src_table.clone(), out: dest_table.clone()});
      },
    }
    Ok(())
  }
}


#[derive(Debug)]
pub struct Size  {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for Size
{
  fn solve(&mut self) {
    let arg_brrw = self.arg.borrow();
    let rows = arg_brrw.rows;
    let cols = arg_brrw.cols;
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.set(0,0,Value::U64(rows as u64));
    out_brrw.set(0,1,Value::U64(cols as u64));
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


pub struct TableSize{}

impl MechFunctionCompiler for TableSize {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let (arg_name,arg_table_id,_) = arguments[0];
    if arg_name == *TABLE {
      let (out_table_id, _, _) = out;
      let arg_table = block.get_table(&arg_table_id)?;
      let out_table = block.get_table(out_table_id)?;
      {
        let mut out_brrw = out_table.borrow_mut();
        out_brrw.resize(1,2);
        out_brrw.set_kind(ValueKind::U64);
      }
      block.plan.push(Size{arg: arg_table.clone(), out: out_table.clone()});
    } else {
      return Err(MechError::GenericError(7352));
    }
    Ok(())
  }
}