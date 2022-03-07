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
  pub static ref TABLE_DEFINE: u64 = hash_str("table/define");
  pub static ref TABLE_SET: u64 = hash_str("table/set");
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

#[derive(Debug)]
pub struct CopyVRV<T,U,V> {
  pub arg: ColumnV<T>, pub ix: ColumnV<U>, pub out: ColumnV<V>
}
impl<T,U,V> MechFunction for CopyVRV<T,U,V> 
where T: Clone + Debug + Into<V>,
      U: Clone + Debug + Into<usize>,
      V: Clone + Debug + Into<V>
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
      let urow:usize = U::into(row.clone());
      out_brrw[out_ix] = T::into(arg_brrw[urow - 1].clone());
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


// Copy Vector{Int Ix} : Vector
#[derive(Debug)]
pub struct CopyTIV {
  pub arg: ArgTable, pub ix: ColumnV<F32>, pub out: OutTable
}
impl MechFunction for CopyTIV    
{
  fn solve(&self) {
    let arg_brrw = self.arg.borrow();
    let mut out_brrw = self.out.borrow_mut();
    let arows = self.ix.len();
    let orows = out_brrw.rows;
    let new_rows = orows + arows;
    let ocols = out_brrw.cols;
    out_brrw.resize(new_rows,ocols);
    for ix_col_ix in 0..arows {
      let ix = self.ix.get_unchecked(ix_col_ix);
      let value = arg_brrw.get_linear_raw(ix.into()).unwrap();
      out_brrw.set_raw(orows+ix_col_ix,0,value);
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
impl<T,U> MechFunction for ParSetVS<T,U>
where T: Clone + Debug + Into<U> + Sync + Send,
      U: Clone + Debug + Into<T> + Sync + Send
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
      return Err(MechError{id: 0001, kind: MechErrorKind::None});
    }
    // Check to make sure column types are consistent
    let col_kinds: Vec<ValueKind> = arg_tables[0].borrow().col_kinds.clone();
    let consistent_col_kinds = arg_tables.iter().all(|arg| arg.borrow().col_kinds.iter().zip(&col_kinds).all(|(k1,k2)| *k1 == *k2));
    if consistent_cols == false {
      return Err(MechError{id: 0001, kind: MechErrorKind::None});
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
        Column::Length(out) | Column::Speed(out) | Column::Time(out) |
        Column::F32(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::U128(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U64(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U16(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4888, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::U8(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::U128(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U64(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4889, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::U64(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::U128(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U64(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4890, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::U128(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::U128(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U64(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::F32(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              Column::U8(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4891, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::Bool(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::Bool(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4892, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::String(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::String(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4893, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        Column::Ref(out) => {
          let mut out_ix = 0;
          for arg_col in arg_cols {
            match arg_col {
              Column::Ref(arg) => {block.plan.push(CopyVV{arg:(arg.clone(),0,arg.len()-1), out: (out.clone(),out_ix,out_ix+arg.len()-1)});out_ix += arg.len();},
              x => {return Err(MechError{id: 4894, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
        x => {return Err(MechError{id: 4895, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
      return Err(MechError{id: 0001, kind: MechErrorKind::None});
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
                (Column::F32(arg), ColumnIndex::RealIndex(ix), Column::F32(out)) => {
                  block.plan.push(CopyVRV{arg: arg.clone(), ix: ix.clone(), out: out.clone()});
                },
                (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U16(arg), ColumnIndex::Index(ix), Column::U16(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U32(arg), ColumnIndex::Index(ix), Column::U32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U64(arg), ColumnIndex::Index(ix), Column::U64(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::U128(arg), ColumnIndex::Index(ix), Column::U128(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Speed(arg), ColumnIndex::Index(ix), Column::Speed(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Time(arg), ColumnIndex::Index(ix), Column::Time(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Length(arg), ColumnIndex::Index(ix), Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
                (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySSRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Empty, _, Column::Empty) => (),
                x => {return Err(MechError{id: 4896, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              };
              out_column_ix += 1;
            }
            // The input is scalar but the output is a vector. Copy the scalar into each element of the vector.
            _ => {
              match (&arg_col, &arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::U16(arg), ColumnIndex::Index(ix), Column::U16(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::U32(arg), ColumnIndex::Index(ix), Column::U32(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::U64(arg), ColumnIndex::Index(ix), Column::U64(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::U128(arg), ColumnIndex::Index(ix), Column::U128(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::String(arg), ColumnIndex::Index(ix), Column::String(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Bool(arg), ColumnIndex::Index(ix), Column::Bool(out)) => block.plan.push(CopySV{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Ref(arg), ColumnIndex::Index(ix), Column::Ref(out)) => block.plan.push(CopySVRef{arg: arg.clone(), ix: *ix, out: out.clone()}),
                (Column::Empty, _, Column::Empty) => (),
                x => {return Err(MechError{id: 4897, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
              match (&arg_col, arg_ix, &out_col) {
                (Column::Time(arg), ColumnIndex::All, Column::Time(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::Speed(arg), ColumnIndex::All, Column::Speed(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::Length(arg), ColumnIndex::All, Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::F32(arg), ColumnIndex::All, Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
                (Column::U8(arg), ColumnIndex::All, Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),                
                (Column::U64(arg), ColumnIndex::All, Column::U64(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::U64(arg), ColumnIndex::Bool(bix), Column::U64(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),        
                (Column::String(arg), ColumnIndex::All, Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
                (Column::Ref(arg), ColumnIndex::All, Column::Ref(out)) => block.plan.push(CopyVVRef{arg: arg.clone(), out: out.clone()}),
                x => {return Err(MechError{id: 4898, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              };
              out_column_ix += 1;
            } 
            x => {return Err(MechError{id: 4899, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
              x => {return Err(MechError{id: 4900, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
              x => {return Err(MechError{id: 4901, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            };
            out_column_ix += 1;
          }
        }
        x => {return Err(MechError{id: 4902, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
              x => {return Err(MechError{id: 4903, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
                  x => {return Err(MechError{id: 4904, kind: MechErrorKind::GenericError(format!("{:?}", x))});},                }
              }
            }
            x => {return Err(MechError{id: 4905, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
      x => {return Err(MechError{id: 4906, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    }
    Ok(())
  }
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
      (TableShape::Scalar,TableIndex::All,TableShape::Pending) |
      (TableShape::Scalar,TableIndex::Index(_),TableShape::Column(_)) |
      (TableShape::Scalar,TableIndex::All,TableShape::Scalar) => {
        let arg_col = block.get_arg_column(&arguments[0])?;
        let mut dest_table_brrw = dest_table.borrow_mut();
        let out_rows = dest_table_brrw.rows;
        let new_rows = out_rows + 1;
        let ocols = dest_table_brrw.cols;
        let mut out_col = dest_table_brrw.get_column_unchecked(0);
        match (&arg_col, &out_col) {
          ((_,Column::F32(arg), ColumnIndex::Index(ix)), Column::F32(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },
          ((_,Column::F32(arg), ColumnIndex::Index(ix)), Column::U8(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },   
          ((_,Column::U8(arg), ColumnIndex::Index(ix)), Column::F32(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },
          ((_,Column::Time(arg), ColumnIndex::Index(ix)), Column::Time(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },            
          ((_,Column::Length(arg), ColumnIndex::Index(ix)), Column::Length(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },            
          ((_,Column::Speed(arg), ColumnIndex::Index(ix)), Column::Speed(out)) => { dest_table_brrw.resize(new_rows,ocols); block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),out_rows,out_rows)}) },            
          x => {return Err(MechError{id: 4907, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      (TableShape::Scalar,TableIndex::All,TableShape::Column(rows)) => {
        let arg_col = block.get_arg_column(&arguments[0])?;
        let new_rows = rows + 1;
        let mut dest_table_brrw = dest_table.borrow_mut();
        dest_table_brrw.resize(new_rows,1);
        let mut out_col = dest_table_brrw.get_column_unchecked(0);
        match (&arg_col, &out_col) {
          ((_,Column::Time(arg), ColumnIndex::Index(ix)), Column::Time(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),           
          ((_,Column::Speed(arg), ColumnIndex::Index(ix)), Column::Speed(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),           
          ((_,Column::Length(arg), ColumnIndex::Index(ix)), Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),           
          ((_,Column::F32(arg), ColumnIndex::Index(ix)), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),           
          ((_,Column::F32(arg), ColumnIndex::Index(ix)), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),             
          ((_,Column::U8(arg), ColumnIndex::Index(ix)), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),rows,rows)}),            
          x => {return Err(MechError{id: 4908, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
        }
      }
      (TableShape::Column(src_rows),TableIndex::All,TableShape::Column(dest_rows)) => {
        let arg_col = block.get_arg_column(&arguments[0])?;
        let new_rows = src_rows + dest_rows;
        let mut dest_table_brrw = dest_table.borrow_mut();
        dest_table_brrw.resize(new_rows,1);
        let mut out_col = dest_table_brrw.get_column_unchecked(0);
        match (&arg_col, &out_col) {
          ((_,Column::F32(arg), ColumnIndex::All), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,src_rows-1), out: (out.clone(),dest_rows,new_rows-1)}),           
          ((_,Column::F32(arg), ColumnIndex::All), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,src_rows-1), out: (out.clone(),dest_rows,new_rows-1)}),             
          ((_,Column::U8(arg), ColumnIndex::All), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,src_rows-1), out: (out.clone(),dest_rows,new_rows-1)}),            
          x => {return Err(MechError{id: 4909, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
        }
      }
      x => {
        let arg_col2 = block.get_arg_column(&arguments[0])?;
        match arg_col2 {
          (_,Column::F32(_),ColumnIndex::All) => {
            return Err(MechError{id: 4910, kind: MechErrorKind::Unhandled});  
          }
          (_,Column::Time(_),ColumnIndex::Index(_)) |
          (_,Column::F32(_),ColumnIndex::Index(_)) |
          (_,Column::Reference((_,(ColumnIndex::All,ColumnIndex::All))),ColumnIndex::All) => {
            let (_,arg_table_id,_) = &arguments[0];
            let arg_table = block.get_table(arg_table_id)?;
            let arg_brrw = arg_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            let orows = dest_table_brrw.rows;
            let ocols = if dest_table_brrw.cols == 0 {1} else {dest_table_brrw.cols};
            let arows = arg_brrw.rows;
            let new_rows = orows + arows;
            dest_table_brrw.resize(orows + arows, ocols);
            if arg_brrw.has_col_aliases() {
              for (alias,src_col_ix) in arg_brrw.col_map.iter() {
                let dest_col_ix = match dest_table_brrw.col_map.get_index(&alias) {
                  Ok(col_ix) => col_ix,
                  _ => 0,
                };
                let arg_col = arg_brrw.get_column_unchecked(*src_col_ix);
                let out_col = dest_table_brrw.get_column_unchecked(dest_col_ix);
                match (&arg_col, &out_col) {
                  (Column::F32(arg),    Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),           
                  (Column::F32(arg),    Column::U8(out))     => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),             
                  (Column::U8(arg),     Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::U64(arg),    Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::F32(arg),    Column::U64(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Time(arg),   Column::Time(out))   => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Length(arg), Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Speed(arg),  Column::Speed(out))  => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::String(arg), Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  x => {return Err(MechError{id: 4911, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
                }
              }
            } else {
              for i in 0..arg_brrw.cols {
                let arg_col = arg_brrw.get_column_unchecked(i);
                let mut dest_table_brrw = dest_table.borrow_mut();
                let out_col = dest_table_brrw.get_column_unchecked(i);
                match (&arg_col, &out_col) {
                  (Column::F32(arg),    Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),           
                  (Column::F32(arg),    Column::U8(out))     => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),             
                  (Column::U8(arg),     Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::U64(arg),    Column::F32(out))    => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Time(arg),   Column::Time(out))   => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Length(arg), Column::Length(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::Speed(arg),  Column::Speed(out))  => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  (Column::String(arg), Column::String(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arows-1), out: (out.clone(),orows,new_rows-1)}),            
                  x => {return Err(MechError{id: 4912, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
                }
              }
            }       
          }
          (_,Column::Reference((arg_table,(ColumnIndex::RealIndex(ix_col),ColumnIndex::None))),ColumnIndex::All) => {
            block.plan.push(CopyTIV{arg: arg_table.clone(), ix: ix_col.clone(), out: dest_table.clone()});          
          }
          x => {return Err(MechError{id: 4913, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
        }
      }
      x => {return Err(MechError{id: 4914, kind: MechErrorKind::GenericError(format!("{:?}", x))});},   
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
      return Err(MechError{id: 4915, kind: MechErrorKind::GenericError(format!("Unknown function argument {:?}", arg_name))});
    }
    Ok(())
  }
}


pub struct TableSet{}
impl MechFunctionCompiler for TableSet {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    
    let (_,src_id,src_indices) = &arguments[0];
    let (dest_id,dest_row,dest_col) = out;
    let arg_shapes = block.get_arg_dims(&arguments)?;
    let dest_shape = block.get_arg_dim(&(0,*dest_id,vec![(*dest_row,*dest_col)]))?;
    let src_table = block.get_table(src_id)?;
    let dest_table = block.get_table(dest_id)?;
    let mut arguments = arguments.clone();
    // The destination is pushed into the arguments here in order to use the
    // get_argument_column() machinery later.
    arguments.push((0,*dest_id,vec![(*dest_row,*dest_col)]));
    match (&arg_shapes[0], &dest_shape) {
      (TableShape::Scalar, TableShape::Row(_)) |
      (TableShape::Row(_), TableShape::Row(_)) => {
        let src_table_brrw = src_table.borrow();
        let mut dest_table_brrw = dest_table.borrow_mut();
        // The source table has named columns, so we need to match them
        // up with the destination columns if they are out of order or
        // incomplete.
        if src_table_brrw.has_col_aliases() {
          for alias in src_table_brrw.col_map.aliases() {
            let dest_column = dest_table_brrw.get_column(&TableIndex::Alias(*alias))?;
            let src_column = src_table_brrw.get_column(&TableIndex::Alias(*alias))?;
            match (src_column,dest_column) {
              (Column::U8(src),Column::U8(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              (Column::F32(src),Column::F32(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              x => {return Err(MechError{id: 4916, kind: MechErrorKind::GenericError(format!("{:?}", x))});},            
            }
          }
        // No column aliases, use indices instead
        } else {
          if src_table_brrw.cols > dest_table_brrw.cols {
            return Err(MechError{id: 4917, kind: MechErrorKind::GenericError("src table too big".to_string())});
          }
          // Destination has aliases, need to use them instead 
          if dest_table_brrw.has_col_aliases() {
            return Err(MechError{id: 4918, kind: MechErrorKind::GenericError("Destination has aliases, need to use them instead".to_string())});
          }
          for col_ix in 1..=src_table_brrw.cols {
            let src_column = src_table_brrw.get_column(&TableIndex::Index(col_ix))?;
            dest_table_brrw.set_col_kind(col_ix-1,src_column.kind());
            let dest_column = dest_table_brrw.get_column(&TableIndex::Index(col_ix))?;
            match (src_column,dest_column) {
              (Column::U8(src),Column::U8(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              (Column::F32(src),Column::F32(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              (Column::Bool(src),Column::Bool(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              (Column::Ref(src),Column::Ref(out)) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
              x => {return Err(MechError{id: 4919, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
        }
      }
      (TableShape::Matrix(_,_),TableShape::Matrix(_,_)) |
      (TableShape::Matrix(_,_),TableShape::Row(_)) |
      (TableShape::Matrix(_,_),TableShape::Scalar) => {
        let src_table_brrw = src_table.borrow();
        let mut dest_table_brrw = dest_table.borrow_mut();
        dest_table_brrw.resize(src_table_brrw.rows,src_table_brrw.cols);
        dest_table_brrw.set_kind(src_table_brrw.kind());
        for col_ix in 1..=src_table_brrw.cols {
          let dest_column = dest_table_brrw.get_column(&TableIndex::Index(col_ix))?;
          let src_column = src_table_brrw.get_column(&TableIndex::Index(col_ix))?;
          match (src_column,dest_column) {
            (Column::U8(src),Column::U8(out)) => {block.plan.push(SetVV{arg: src.clone(), out: out.clone()});}
            (Column::F32(src),Column::F32(out)) => {block.plan.push(SetVV{arg: src.clone(), out: out.clone()});}
            (Column::Bool(src),Column::Bool(out)) => {block.plan.push(SetVV{arg: src.clone(), out: out.clone()});}
            x => {return Err(MechError{id: 4920, kind: MechErrorKind::GenericError(format!("{:?}", x))});}      
          }
        }
      }
      (TableShape::Scalar,TableShape::Scalar) => {
        let arg_cols = block.get_arg_columns(&arguments)?;
        match (&arg_cols[0],&arg_cols[1]) {
          ((_,Column::F32(arg),ColumnIndex::Index(ix)),(_,Column::F32(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
          ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
          ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::F32(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
          ((_,Column::U8(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
          ((_,Column::Ref(src),ColumnIndex::Index(in_ix)),(_,Column::Ref(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
          x => {return Err(MechError{id: 4921, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      _ |
      (TableShape::Column(_),TableShape::Column(_)) => {
        let arg_cols = block.get_arg_columns(&arguments)?;
        match (&arg_cols[0], &arg_cols[1]) {
          ((_,Column::U8(arg),ColumnIndex::All),(_,Column::U8(out),ColumnIndex::All)) => block.plan.push(SetVV{arg: arg.clone(), out: out.clone()}),
          ((_,Column::U8(arg),ColumnIndex::Index(ix)),(_,Column::U8(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
          ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::U8(out),ColumnIndex::Index(oix))) => block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix}),
          ((_,Column::U8(arg),ColumnIndex::All), (_,Column::U8(out),ColumnIndex::Bool(oix))) => block.plan.push(SetVVB{arg: arg.clone(), out: out.clone(), oix: oix.clone()}),
          ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::All)) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.resize(1,1);
            dest_table_brrw.set_kind(ValueKind::U8);
            if let Column::U8(out) = dest_table_brrw.get_column_unchecked(0) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: 0});
            }
          }
          ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.set_col_kind(1,ValueKind::U8);
            if let Column::U8(out) = dest_table_brrw.get_column_unchecked(1) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
            }
          }
          ((_,Column::Length(arg),ColumnIndex::All),(_,Column::Length(out),ColumnIndex::All)) => block.plan.push(SetVV{arg: arg.clone(), out: out.clone()}),
          ((_,Column::Length(arg),ColumnIndex::Index(ix)),(_,Column::Length(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
          ((_,Column::Length(arg),ColumnIndex::Index(ix)), (_,Column::Length(out),ColumnIndex::Index(oix))) => block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix}),
          ((_,Column::Length(arg),ColumnIndex::All), (_,Column::Length(out),ColumnIndex::Bool(oix))) => block.plan.push(SetVVB{arg: arg.clone(), out: out.clone(), oix: oix.clone()}),
          ((_,Column::Length(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::All)) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.resize(1,1);
            dest_table_brrw.set_kind(ValueKind::Length);
            if let Column::Length(out) = dest_table_brrw.get_column_unchecked(0) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: 0});
            }
          }
          ((_,Column::Length(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.set_col_kind(1,ValueKind::Length);
            if let Column::Length(out) = dest_table_brrw.get_column_unchecked(1) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
            }
          }
          ((_,Column::F32(arg),ColumnIndex::All),(_,Column::F32(out),ColumnIndex::All)) => block.plan.push(SetVV{arg: arg.clone(), out: out.clone()}),
          ((_,Column::F32(arg),ColumnIndex::Index(ix)),(_,Column::F32(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
          ((_,Column::F32(arg),ColumnIndex::Index(ix)), (_,Column::F32(out),ColumnIndex::Index(oix))) => block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix}),
          ((_,Column::F32(arg),ColumnIndex::All), (_,Column::F32(out),ColumnIndex::Bool(oix))) => block.plan.push(SetVVB{arg: arg.clone(), out: out.clone(), oix: oix.clone()}),
          ((_,Column::F32(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::All)) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.resize(1,1);
            dest_table_brrw.set_kind(ValueKind::F32);
            if let Column::F32(out) = dest_table_brrw.get_column_unchecked(0) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: 0});
            }
          }
          ((_,Column::F32(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.set_col_kind(1,ValueKind::F32);
            if let Column::F32(out) = dest_table_brrw.get_column_unchecked(1) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
            }
          }
          ((_,Column::Ref(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.set_col_kind(1,ValueKind::Reference);
            if let Column::Ref(out) = dest_table_brrw.get_column_unchecked(1) {
              block.plan.push(SetSIxSIx{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
            }
          }
          x => {return Err(MechError{id: 4922, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 4923, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    
    }
    Ok(())
  }
}


pub struct TableDefine{}
impl MechFunctionCompiler for TableDefine {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {

    //Transformation::TableDefine{table_id, indices, out}'
    let (_,table_id,indices) = &arguments[0];
    let (out,_,_) = &out;
    // Iterate through to the last index
    let mut table_id = *table_id;
    for (row,column) in indices.iter().take(indices.len()-1) {
      let argument = (0,table_id,vec![(*row,*column)]);
      match block.get_arg_dim(&argument)? {
        TableShape::Scalar => {
          let arg_col = block.get_arg_column(&argument)?;
          match arg_col {
            (_,Column::Ref(ref_col),_) => {
              table_id = ref_col.borrow()[0].clone();
            }
            x => {return Err(MechError{id: 4924, kind: MechErrorKind::GenericError(format!("{:?}", x))});},          }
        }
        x => {return Err(MechError{id: 4925, kind: MechErrorKind::GenericError(format!("{:?}", x))});},      }
    }
    let src_table = block.get_table(&table_id)?;
    let out_table = block.get_table(out)?;
    let (row, column) = indices.last().unwrap();
    let argument = (0,table_id,vec![(*row,*column)]);
    match (row,column) {
      // Select an entire table
      (TableIndex::All, TableIndex::All) => {
        match out {
          TableId::Global(gid) => {
            block.plan.push(CopyT{arg: src_table.clone(), out: out_table.clone()});
          }
          _ => (),
        }
      }
      // Select a column by row index
      (TableIndex::All, TableIndex::Index(_)) |
      // Select a column by alias
      (TableIndex::All, TableIndex::Alias(_)) => {
        let (_, arg_col,_) = block.get_arg_column(&(0,table_id,vec![(*row,*column)]))?;
        let out_col = block.get_out_column(&(*out,TableIndex::All,TableIndex::All),arg_col.len(),arg_col.kind())?;
        match (&arg_col, &out_col) {
          (Column::U8(arg), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
          (Column::F32(arg), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
          (Column::Bool(arg), Column::Bool(out)) => block.plan.push(CopyVV{arg: (arg.clone(),0,arg.len()-1), out: (out.clone(),0,arg.len()-1)}),
          x => {return Err(MechError{id: 4926, kind: MechErrorKind::GenericError(format!("{:?}", x))});},        }
      }
      // Select a specific element by numberical index
      (TableIndex::Index(ix), TableIndex::None) => {
        let src_brrw = src_table.borrow();
        let (row,col) = src_brrw.index_to_subscript(ix-1)?;
        let mut arg_col = src_brrw.get_column_unchecked(col);
        let out_col = block.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;
        match (&arg_col, &out_col) {
          (Column::U8(arg), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
          (Column::F32(arg), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
          (Column::Ref(arg), Column::Ref(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
          x => {return Err(MechError{id: 4927, kind: MechErrorKind::GenericError(format!("{:?}", x))});},        }
      }
      // Select a number of specific elements by numerical index or lorgical index
      (TableIndex::Table(ix_table_id), TableIndex::None) => {
        let src_brrw = src_table.borrow();
        match src_brrw.shape() {
          TableShape::Row(_) => {
            {
              let mut out_brrw = out_table.borrow_mut();
              out_brrw.set_kind(src_brrw.kind());
            }
            let ix_table = block.get_table(&ix_table_id)?;
            block.plan.push(CopyTB{arg: src_table.clone(), ix: ix_table.clone(), out: out_table.clone()});
          }
          _ => {
            let (_, arg_col,arg_ix) = block.get_arg_column(&argument)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.set_kind(arg_col.kind());
            let out_col = out_brrw.get_column_unchecked(0);    
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::U8(arg), ColumnIndex::IndexCol(ix_col), Column::U8(out)) => block.plan.push(CopyVI{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),(Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
              (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
              (Column::F32(arg), ColumnIndex::IndexCol(ix_col), Column::F32(out)) => block.plan.push(CopyVI{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),
              x => {return Err(MechError{id: 4928, kind: MechErrorKind::GenericError(format!("{:?}", x))});},            }
          }
        }
      }
      (TableIndex::Table(ix_table_id), TableIndex::All) => {
        let src_brrw = src_table.borrow();
        let mut out_brrw = out_table.borrow_mut();
        out_brrw.resize(1,src_brrw.cols);
        out_brrw.set_kind(src_brrw.kind());
        for col in 0..src_brrw.cols {
          let (_, arg_col,arg_ix) = block.get_arg_column(&(0,table_id,vec![(*row,TableIndex::Index(col+1))]))?;
          let mut out_col = out_brrw.get_column_unchecked(col); 
          match (&arg_col, &arg_ix, &out_col) {
            (Column::U8(arg), ColumnIndex::Bool(bix), Column::U8(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
            (Column::U8(arg), ColumnIndex::IndexCol(ix_col), Column::U8(out)) => block.plan.push(CopyVI{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),(Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
            (Column::F32(arg), ColumnIndex::Bool(bix), Column::F32(out)) => block.plan.push(CopyVB{arg: arg.clone(), bix: bix.clone(), out: out.clone()}),
            (Column::F32(arg), ColumnIndex::IndexCol(ix_col), Column::F32(out)) => block.plan.push(CopyVI{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),
            x => {return Err(MechError{id: 4929, kind: MechErrorKind::GenericError(format!("{:?}", x))});},          }
        }
      }
      (TableIndex::Index(row_ix), TableIndex::Alias(column_alias)) => {
        let (_, arg_col,arg_ix) = block.get_arg_column(&(0,table_id,vec![(*row,*column)]))?;
        let out_col = block.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;
        match (&arg_col, &arg_ix, &out_col) {
          (Column::F32(arg), ColumnIndex::Index(ix), Column::F32(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
          (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => block.plan.push(CopyVV{arg: (arg.clone(),*ix,*ix), out: (out.clone(),0,0)}),
          x => {return Err(MechError{id: 4930, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 4931, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}