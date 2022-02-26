use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use rust_core::iter::FromIterator;
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


// Copy Vector : Vector
#[derive(Debug)]
pub struct CopyVV<T,U> {
  pub arg: (ColumnV<T>, usize, usize),
  pub out: (ColumnV<U>, usize, usize),
}
impl<T,U> MechFunction for CopyVV<T,U> 
where T: Debug + Clone + Into<U> + Sync + Send,
      U: Debug + Clone + Into<T> + Sync + Send,
{
  fn solve(&self) {
    let (arg,asix,aeix) = &self.arg;
    let (out,osix,oeix) = &self.out;
    out.borrow_mut()[*osix..=*oeix]
       .iter_mut()
       .zip(arg.borrow()[*asix..=*aeix].iter())
       .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Parallel Copy Vector : Vector
#[derive(Debug)]
pub struct ParCopyVV<T,U> {
  pub arg: (ColumnV<T>, usize, usize),
  pub out: (ColumnV<U>, usize, usize),
}
impl<T,U> MechFunction for ParCopyVV<T,U> 
where T: Debug + Clone + Into<U> + Sync + Send,
      U: Debug + Clone + Into<T> + Sync + Send,
{
  fn solve(&self) {
    let (arg,asix,aeix) = &self.arg;
    let (out,osix,oeix) = &self.out;
    out.borrow_mut()[*osix..=*oeix]
       .par_iter_mut()
       .zip(arg.borrow()[*asix..=*aeix].par_iter())
       .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Scalar : Vector
#[derive(Debug)]
pub struct CopySV<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>
}
impl<T,U> MechFunction for CopySV<T,U>  
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    let arg = self.arg.borrow()[self.ix].clone();
    self.out.borrow_mut().iter_mut().for_each(|out| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector : Vector Ref
#[derive(Debug)]
pub struct CopyVVRef {
  pub arg: ColumnV<TableId>, pub out: ColumnV<TableId>
}
impl MechFunction for CopyVVRef {
  fn solve(&self) {
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
  pub arg: ColumnV<TableId>, pub ix: usize , pub out: ColumnV<TableId>
}
impl MechFunction for CopySVRef 
{
  fn solve(&self) {
    let id = TableId::Global(*self.arg.borrow()[self.ix].unwrap());
    self.out.borrow_mut().iter_mut().for_each(|out| *out = id.clone()); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Reference
#[derive(Debug)]
pub struct CopySSRef {
  pub arg: ColumnV<TableId>, pub ix: usize , pub out: ColumnV<TableId>
}
impl MechFunction for CopySSRef 
{
  fn solve(&self) {
    (self.out.borrow_mut())[0] = TableId::Global(*self.arg.borrow()[self.ix].unwrap())
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector{Bool Ix} : Vector
#[derive(Debug)]
pub struct CopyVB<T,U> {
  pub arg: ColumnV<T>,
  pub bix: ColumnV<bool>,
  pub out: ColumnV<U>,
}
impl<T,U> MechFunction for CopyVB<T,U> 
where T: Debug + Clone + Into<U> + Sync + Send,
      U: Debug + Clone + Into<T> + Sync + Send,
      Vec<U>: FromIterator<T>,
{
  fn solve(&self) {
    // Filter the column to include only elements with a "true" index
    let filtered: Vec<U>  = 
      self.arg.borrow()
         .iter()
         .zip(self.bix.borrow().iter())
         .filter_map(|(x,ix)| if *ix {Some(x.clone())} else {None})
         .collect::<Vec<U>>();
    let mut out_brrw = self.out.borrow_mut();
    let rows = filtered.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,filtered[0].clone());
    }
    for row in 0..filtered.len() {
      out_brrw[row] = filtered[row].clone();
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector{Int Ix} : Vector
#[derive(Debug)]
pub struct CopyVI<T,U> {
  pub arg: ColumnV<T>, pub ix: ColumnV<usize>, pub out: ColumnV<U>
}

impl<T,U> MechFunction for CopyVI<T,U> 
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let ix_brrw = self.ix.borrow();

    let rows = ix_brrw.len();
    if rows > out_brrw.len() {
      out_brrw.resize(rows,T::into(arg_brrw[0].clone()));
    }
    for (out_ix, row) in ix_brrw.iter().enumerate() {
      out_brrw[out_ix] = T::into(arg_brrw[*row as usize - 1].clone());
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Set Scalar : Scalar
#[derive(Debug)]
pub struct SetSIxSIx<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: usize
}
impl<T,U> MechFunction for SetSIxSIx<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    (self.out.borrow_mut())[self.oix] = T::into((self.arg.borrow())[self.ix].clone());
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Set Scalar : Vector {Bool}
#[derive(Debug)]
pub struct SetSIxVB<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: ColumnV<bool>
}
impl<T,U> MechFunction for SetSIxVB<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    let oix_brrw = self.oix.borrow();
    for row in 0..oix_brrw.len() {
      if oix_brrw[row] {                
        (self.out.borrow_mut())[row] = T::into((self.arg.borrow())[self.ix].clone()); 
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
  fn solve(&self) {
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
pub struct SetVVB<T,U> {
  pub arg: ColumnV<T>, pub out: ColumnV<U>, pub oix: ColumnV<bool>
}

impl<T,U> MechFunction for SetVVB<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    self.out.borrow_mut()
            .iter_mut()
            .zip(self.oix.borrow().iter())
            .zip(self.arg.borrow().iter())
            .for_each(|((out,oix),x)| if *oix == true {
      *out = T::into(x.clone())
    });
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[derive(Debug)]
pub struct ParSetVVB<T,U> {
  pub arg: ColumnV<T>, pub out: ColumnV<U>, pub oix: ColumnV<bool>
}

impl<T,U> MechFunction for ParSetVVB<T,U>
where T: Clone + Debug + Into<U> + Sync + Send,
      U: Clone + Debug + Into<T> + Sync + Send
{
  fn solve(&self) {
    self.out.borrow_mut()
            .par_iter_mut()
            .zip(self.oix.borrow().par_iter())
            .zip(self.arg.borrow().par_iter())
            .for_each(|((out,oix),x)| if *oix == true {
      *out = T::into(x.clone())
    });
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Set Vector : Vector
#[derive(Debug)]
pub struct SetVV<T,U> {
  pub arg: ColumnV<T>, pub out: ColumnV<U>
}

impl<T,U> MechFunction for SetVV<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    self.out.borrow_mut()
            .iter_mut()
            .zip(self.arg.borrow().iter())
            .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Set Vector : Vector
#[derive(Debug)]
pub struct ParSetVV<T,U> {
  pub arg: ColumnV<T>, pub out: ColumnV<U>
}

impl<T,U> MechFunction for ParSetVV<T,U>
where T: Clone + Debug + Into<U> + Sync + Send,
      U: Clone + Debug + Into<T> + Sync + Send
{
  fn solve(&self) {
    self.out.borrow_mut()
            .par_iter_mut()
            .zip(self.arg.borrow().par_iter())
            .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Set Vector : Scalar
#[derive(Debug)]
pub struct SetVS<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>
}

impl<T,U> MechFunction for SetVS<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    let arg = &self.arg.borrow()[self.ix];
    self.out.borrow_mut()
            .iter_mut()
            .zip(self.arg.borrow().iter())
            .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[derive(Debug)]
pub struct ParSetVS<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>
}

impl<T,U> MechFunction for SetVS<T,U>
where T: Clone + Debug + Into<U>,
      U: Clone + Debug + Into<T>
{
  fn solve(&self) {
    let arg = &self.arg.borrow()[self.ix];
    self.out.borrow_mut()
            .par_iter_mut()
            .zip(self.arg.borrow().par_iter())
            .for_each(|(out, arg)| *out = T::into(arg.clone())); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[derive(Debug)]
pub struct ParSetVSB<T,U>  {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: ColumnV<bool>
}

impl<T,U>  MechFunction for ParSetVSB<T,U> 
where T: Clone + Debug + Into<U> + Sync + Send,
      U: Clone + Debug + Into<T> + Sync + Send
{
  fn solve(&self) {
    let arg = &self.arg.borrow()[self.ix];
    self.out.borrow_mut()
            .par_iter_mut()
            .zip(self.oix.borrow().par_iter())
            .for_each(|(out, oix)| if *oix {*out = T::into(arg.clone())}); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct SetVSB<T,U>  {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: ColumnV<bool>
}

impl<T,U>  MechFunction for SetVSB<T,U> 
where T: Clone + Debug + Into<U> + Sync + Send,
      U: Clone + Debug + Into<T> + Sync + Send
{
  fn solve(&self) {
    let arg = &self.arg.borrow()[self.ix];
    self.out.borrow_mut()
            .iter_mut()
            .zip(self.oix.borrow().iter())
            .for_each(|(out, oix)| if *oix {*out = T::into(arg.clone())}); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Table : Table
#[derive(Debug)]
pub struct CopyT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for CopyT {
  fn solve(&self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();

    out_brrw.resize(arg_brrw.rows, arg_brrw.cols);
    for (col, kind) in arg_brrw.col_kinds.iter().enumerate() {
      out_brrw.set_col_kind(col, kind.clone());
    }
    out_brrw.col_map = arg_brrw.col_map.clone();
    out_brrw.row_map = arg_brrw.row_map.clone();
    for col in 0..arg_brrw.cols {
      for row in 0..arg_brrw.rows {
        let value = arg_brrw.get_raw(row,col).unwrap();
        out_brrw.set_raw(row,col,value);
      }
    }
  }
  fn to_string(&self) -> String { 
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_header("CopyT");
    box_drawing.add_header("arg");
    box_drawing.add_line(format!("{:#?}", &self.arg.borrow()));
    box_drawing.add_header("out");
    box_drawing.add_line(format!("{:#?}", &self.out.borrow()));
    box_drawing.print()
  }
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
      let mut arg_cols = vec![];       
      for table in &arg_tables {
        let table_brrw = table.borrow();
        let column = table_brrw.get_column(&TableIndex::Index(ix+1))?;
        arg_cols.push(column.clone());
      }
      match out_col {
        Column::F32(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => (),
            }
          }
        }
        Column::U8(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => (),
            }
          }
        }
        Column::Bool(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::Bool(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => (),
            }
          }
        }
        Column::String(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::String(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => (),
            }
          }
        }
        Column::Ref(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::Ref(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => (),
            }
          }
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
            // The input is scalar and the output is scalar
            1 => {
              match (&arg_col, &arg_ix, &out_col) {
                (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U16(arg), ColumnIndex::Index(ix), Column::U16(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U32(arg), ColumnIndex::Index(ix), Column::U32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U64(arg), ColumnIndex::Index(ix), Column::U64(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U128(arg), ColumnIndex::Index(ix), Column::U128(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Time(arg), ColumnIndex::Index(ix), Column::Time(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Length(arg), ColumnIndex::Index(ix), Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Empty, _, Column::Empty) => (),
                x => {
                  println!("{:?}", x);
                  return Err(MechError::GenericError(6366));
                },
              };
              out_column_ix += 1;
            }
            // The input is scalar but the output is a vector. Copy the scalar into each element of the vector.
            _ => {
              match (&arg_col, &arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
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
              println!("{:?}", (&arg_col, &arg_ix, &out_col));
              let fxn = match (&arg_col, arg_ix, &out_col) {
                (Column::F32(arg), ColumnIndex::All, Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
                (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
                (Column::U64(arg), ColumnIndex::All, Column::U64(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::U64(arg), ColumnIndex::Bool(bix), Column::U64(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),        
                (Column::String(arg), ColumnIndex::All, Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopyVVRef{arg: arg.clone(), out: out.clone()}),
                x => {
                  println!("{:?}", x);
                  return Err(MechError::GenericError(6367));
                },
              };
              out_column_ix += 1;
            } 
            x => {
              println!("{:?}",x);
              return Err(MechError::GenericError(6967));
            }
          }
        }
        TableShape::Row(_) => {
          for (_, arg_col,arg_ix) in block.get_whole_table_arg_cols(&argument)? {
            o.set_col_kind(out_column_ix, arg_col.kind());
            let mut out_col = o.get_column_unchecked(out_column_ix);
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
              (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
              (Column::F32(arg), ColumnIndex::All, Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::String(arg), ColumnIndex::Bool(bix), Column::String(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
              (Column::String(arg), ColumnIndex::All, Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::Bool(arg), ColumnIndex::Bool(bix), Column::Bool(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
              (Column::Bool(arg), ColumnIndex::All, Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: 0, out: out.clone()}),
              (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
              (Column::Empty, _, Column::Empty) => (),
              x => {
                println!("{:?}",x);
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
              (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::F32(arg), ColumnIndex::All, Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::String(arg), ColumnIndex::Bool(bix), Column::String(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::String(arg), ColumnIndex::All, Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::Bool(arg), ColumnIndex::Bool(bix), Column::Bool(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::Bool(arg), ColumnIndex::All, Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
              (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopyVVRef{arg: arg.clone(), out: out.clone()}),
              x => {
                println!("{:?}", x);
                return Err(MechError::GenericError(6881));},
            };
            out_column_ix += 1;
          }
        }
        x => {
          println!("{:?}", x);
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
    out_brrw.resize(1,1);
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
              (_,Column::F32(_),_) => { dest_table.set_col_kind(col,ValueKind::F32); }
              _ => {return Err(MechError::GenericError(6095));},
            }
          }
          block.global_database.borrow_mut().insert_table(dest_table);
          out_brrw.set_raw(row,0,Value::Reference(TableId::Global(split_id)));
        }
        // Write functions
        for (col_ix,arg_col) in arg_cols.iter().enumerate() {
          match arg_col {
            (_,Column::F32(src_col),ColumnIndex::All) => {
              for row in 0..rows {
                // get the destination table
                let split_id = hash_str(&format!("{:?}{:?}", out_table_id, row));
                let dest_table = block.get_table(&TableId::Global(split_id))?;
                let dest_col = dest_table.borrow().get_column(&TableIndex::Index(col_ix+1))?;
                match dest_col {
                  Column::F32(dest_col) => { block.plan.push(SetSIxSIx{arg: src_col.clone(), ix: row, out: dest_col.clone(), oix: 0}); }
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


// A range of values from start to end
#[derive(Debug)]
pub struct Range  {
  pub start: ColumnV<F32>, pub end: ColumnV<F32>, pub out: OutTable
}

impl MechFunction for Range
{
  fn solve(&self) {
    let start_value = self.start.borrow()[0];
    let end_value = self.end.borrow()[0];
    let delta = end_value.unwrap() - start_value.unwrap() + 1.0;
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.resize(delta as usize,1);
    out_brrw.set_col_kind(0,ValueKind::F32);
    let mut value = start_value.unwrap();
    for row in 0..out_brrw.rows {
      out_brrw.set_raw(row,0,Value::F32(F32::new(value)));
      value += 1.0;
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
      ((_,Column::F32(start),_), (_,Column::F32(end),_)) => {  
        let fxn = Range{start: start.clone(), end: end.clone(), out: out_table.clone()};
        block.plan.push(fxn);
      }
      _ => {return Err(MechError::GenericError(6349));},
    }
    Ok(())
  }
}


// AppendRow Table : Table
#[derive(Debug)]
pub struct AppendRowT {
  pub arg: ArgTable, pub out: OutTable
}

impl MechFunction for AppendRowT {
  fn solve(&self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    let ocols = if out_brrw.cols == 0 {1} else {out_brrw.cols};
    let arows = arg_brrw.rows;
    out_brrw.resize(orows + arows, ocols);
    if arg_brrw.has_col_aliases() {
      for (alias,ix) in arg_brrw.col_map.iter() {
        for row in 0..arows {
          let value = arg_brrw.get_raw(row,*ix).unwrap();
          let col_ix = match out_brrw.col_map.get_index(&alias) {
            Ok(col_ix) => col_ix,
            _ => 0,
          };
          out_brrw.set_col_kind(col_ix,value.kind());
          out_brrw.set_raw(orows + row,col_ix,value);
        }
      }
    } else {
      for col in 0..arg_brrw.cols {
        for row in 0..arows {
          let value = arg_brrw.get_raw(row,col).unwrap();
          out_brrw.set_col_kind(col,value.kind());
          out_brrw.set_raw(orows + row,col,value);
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
  fn solve(&self) {
    let mut out_brrw = self.out.borrow_mut();
    let arg_brrw = self.arg.borrow();
    let orows = out_brrw.rows;
    out_brrw.resize(orows + 1, 1);
    let value = arg_brrw.get_linear(self.ix).unwrap();
    out_brrw.set_col_kind(0,value.kind());
    out_brrw.set_raw(orows,0,value);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
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
  fn solve(&self) {
    let arg_brrw = self.arg.borrow();
    let rows = arg_brrw.rows;
    let cols = arg_brrw.cols;
    let mut out_brrw = self.out.borrow_mut();
    out_brrw.set_raw(0,0,Value::U64(U64::new(rows as u64)));
    out_brrw.set_raw(0,1,Value::U64(U64::new(cols as u64)));
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
