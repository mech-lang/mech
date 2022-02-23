#![allow(warnings)]
#![feature(iter_intersperse)]
// New runtime
// requirements:
// pass all tests
// robust units
// all number types
// Unicode
// async blocks
// parallel operators
// rewind capability
// pre-serialized memory layout
// performance target: 10 million updates per 60Hz cycle
// stack allocated tables
// matrix library in std

use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use seahash;

use rayon::prelude::*;
use std::collections::VecDeque;
use std::thread;
use mech_core::*;
use mech_core::function::*;

use std::fmt::*;
use num_traits::*;
use std::ops::*;

// -------------------------
// Column
// -------------------------

#[derive(Clone, Debug)]
pub enum Column {
  F32(ColumnV<F32>),
  F64(ColumnV<f64>),
  U8(ColumnV<U8>),
  U16(ColumnV<U16>),
  U32(ColumnV<U32>),
  U64(ColumnV<U64>),
  U128(ColumnV<u128>),
  Ref(ColumnV<TableId>),
  I8(ColumnV<i8>),
  I16(ColumnV<i16>),
  I32(ColumnV<i32>),
  I64(ColumnV<i64>),
  I128(ColumnV<i128>),
  Index(ColumnV<usize>),
  Bool(ColumnV<bool>),
  //String(ColumnV<MechString>),
  Reference((Reference,(ColumnIndex,ColumnIndex))),
  Time(ColumnV<F32>),
  Length(ColumnV<F32>),
  Empty,
}

#[derive(Clone, Debug)]
pub enum ColumnIndex {
  All,
  Index(usize),
  IndexCol(ColumnV<usize>),
  Bool(ColumnV<bool>),
  ReshapeColumn,
  None,
}

impl Column {
/*
  pub fn copy(&self) -> Column {
    match self {
      Column::U8(col) => Column::U8(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U16(col) => Column::U16(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U32(col) => Column::U32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U64(col) => Column::U64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U128(col) => Column::U128(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I8(col) => Column::I8(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I16(col) => Column::I16(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I32(col) => Column::I32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I64(col) => Column::I64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I128(col) => Column::I128(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::F32(col) => Column::F32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::F64(col) => Column::F64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Bool(col) => Column::Bool(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Index(col) => Column::Index(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::String(col) => Column::String(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Ref(col) => Column::Ref(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Reference(reference) => Column::Reference(reference.clone()),
      Column::Time(col) => Column::Time(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Length(col) => Column::Length(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Empty => Column::Empty,
    } 
  }*/

  pub fn len(&self) -> usize {
    match self {
      Column::U8(col) => col.len(),
      Column::U16(col) => col.len(),
      Column::U32(col) => col.len(),
      Column::U64(col) => col.len(),
      Column::U128(col) => col.len(),
      Column::I8(col) => col.len(),
      Column::I16(col) => col.len(),
      Column::I32(col) => col.len(),
      Column::I64(col) => col.len(),
      Column::I128(col) => col.len(),
      Column::Length(col) | Column::Time(col) |
      Column::F32(col) => col.len(),
      Column::F64(col) => col.len(),
      Column::Bool(col) => col.len(),
      Column::Index(col) => col.len(),
      //Column::String(col) => col.len(),
      Column::Ref(col) => col.len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Empty => 0,
    }
  }
  
  pub fn logical_len(&self) -> usize {
    match self {
      Column::Bool(col) => col.borrow_mut().iter().fold(0, |acc,x| if *x { acc + 1 } else { acc }),
      _ => self.len(),
    }    
  }

  pub fn resize(&self, rows: usize) -> std::result::Result<(),MechError> {
    match self {
      Column::U8(col) => col.borrow_mut().resize(rows,U8(0)),
      Column::U16(col) => col.borrow_mut().resize(rows,U16(0)),
      Column::U32(col) => col.borrow_mut().resize(rows,U32(0)),
      Column::U64(col) => col.borrow_mut().resize(rows,U64(0)),
      Column::U128(col) => col.borrow_mut().resize(rows,0),
      Column::I8(col) => col.borrow_mut().resize(rows,0),
      Column::I16(col) => col.borrow_mut().resize(rows,0),
      Column::I32(col) => col.borrow_mut().resize(rows,0),
      Column::I64(col) => col.borrow_mut().resize(rows,0),
      Column::I128(col) => col.borrow_mut().resize(rows,0),
      Column::Time(col) | Column::Length(col) |
      Column::F32(col) => col.borrow_mut().resize(rows,F32(0.0)),
      Column::F64(col) => col.borrow_mut().resize(rows,0.0),
      Column::Ref(col) => col.borrow_mut().resize(rows,TableId::Local(0)),
      Column::Index(col) => col.borrow_mut().resize(rows,0),
      Column::Bool(col) => col.borrow_mut().resize(rows,false),
      //Column::String(col) => col.borrow_mut().resize(rows,MechString::new()),
      Column::Reference(_) |
      Column::Empty => {return Err(MechError::GenericError(7143));}
    }
    Ok(())
  }

  pub fn kind(&self) -> ValueKind {
    match self {
      Column::F32(_) => ValueKind::F32,
      Column::F64(_) => ValueKind::F64,
      Column::U8(_) => ValueKind::U8,
      Column::U16(_) => ValueKind::U16,
      Column::U32(_) => ValueKind::U32,
      Column::U64(_) => ValueKind::U64,
      Column::U128(_) => ValueKind::U128,
      Column::I8(_) => ValueKind::I8,
      Column::I16(_) => ValueKind::I16,
      Column::I32(_) => ValueKind::I32,
      Column::I64(_) => ValueKind::I64,
      Column::I128(_) => ValueKind::I128,
      Column::Bool(_) => ValueKind::Bool,
      //Column::String(_) => ValueKind::String,
      Column::Index(_) => ValueKind::Index,
      Column::Ref(_) => ValueKind::Reference,
      Column::Reference((table,index)) => table.borrow().kind(),
      Column::Time(_) => ValueKind::Time,
      Column::Length(_) => ValueKind::Length,
      Column::Empty => ValueKind::Empty,
    }
  }

}

#[derive(Clone)]
pub struct ColumnV<T>(Rc<RefCell<Vec<T>>>);

impl<T: Copy> ColumnV<T> {

  pub fn new(vec: Vec<T>) -> ColumnV<T> {
    ColumnV(Rc::new(RefCell::new(vec)))
  }

  pub fn len(&self) -> usize {
    let ColumnV(col) = self;
    col.borrow().len()
  }

  pub fn get_unchecked(&self, row: usize) -> T {
    let ColumnV(col) = self;
    let mut c_brrw = col.borrow();
    c_brrw[row]
  }

  pub fn set_unchecked(&mut self, row: usize, value: T) {
    let ColumnV(col) = self;
    let mut c_brrw = col.borrow_mut();
    c_brrw[row] = value;
  }

  pub fn borrow(&self) -> std::cell::Ref<Vec<T>> {
    let ColumnV(col) = self;
    col.borrow()
  }

  pub fn borrow_mut(&self) -> std::cell::RefMut<Vec<T>> {
    let ColumnV(col) = self;
    col.borrow_mut()
  }
  
}

impl<T: Debug> fmt::Debug for ColumnV<T> {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let ColumnV(col) = self;
    let col_brrw = col.borrow();
    write!(f,"[")?;
    for c in col_brrw.iter().map(|c| format!("{:?}",c)).intersperse(", ".to_string()) {
      write!(f,"{}",c)?;
    }
    write!(f,"]")?;
    Ok(())
  }
}

mech_type!(F32,f32);
mech_type!(U8,u8);
mech_type!(U16,u16);
mech_type!(U32,u32);
mech_type!(U64,u64);
mech_type!(U128,u128);
mech_type!(I8,i8);
mech_type!(I16,i16);
mech_type!(I32,i32);
mech_type!(I64,i64);
mech_type!(I128,i128);

mech_type_conversion!(U8,F32,f32);
mech_type_conversion!(U8,U64,u64);
mech_type_conversion!(U8,U32,u32);
mech_type_conversion!(U8,U16,u16);
mech_type_conversion!(F32,U8,u8);
mech_type_conversion!(U16,U8,u8);
mech_type_conversion!(U32,U8,u8);
mech_type_conversion!(U64,U8,u8);

#[macro_export]
macro_rules! mech_type {
  ($wrapper:tt,$type:tt) => (
    #[derive(Copy,Clone)]
    pub struct $wrapper($type);
    impl Add for $wrapper {
      type Output = $wrapper;
      fn add(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs + rhs)
      }
    }
    impl fmt::Debug for $wrapper {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let $wrapper(col) = self;
        write!(f,"{:?}",col)?;
        Ok(())
      }
    }
  )
}

#[macro_export]
macro_rules! mech_type_conversion {
  ($from_wrapper:tt,$to_wrapper:tt,$to_type:tt) => (
    impl From<$from_wrapper> for $to_wrapper {
      fn from(n: $from_wrapper) -> $to_wrapper {
        let $from_wrapper(c) = n;
        $to_wrapper(c as $to_type)
      } 
    }
  )
}

// -------------------------
// Table
// -------------------------

#[derive(Debug)]
pub struct Table {
  pub id: u64,                           
  pub rows: usize,                       
  pub cols: usize,                       
  pub col_kinds: Vec<ValueKind>,                 
  pub col_map: AliasMap,  
  pub row_map: AliasMap,
  pub data: Vec<Column>,
  pub dictionary: StringDictionary,
}

impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
      rows,
      cols,
      col_kinds: Vec::with_capacity(cols),
      col_map: AliasMap::new(cols),
      row_map: AliasMap::new(rows),
      data: Vec::with_capacity(cols),
      dictionary: Rc::new(RefCell::new(HashMap::new())),
    };
    for col in 0..cols {
      table.data.push(Column::Empty);
      table.col_kinds.push(ValueKind::Empty);
    }
    table
  }

  pub fn resize(&mut self, rows: usize, cols: usize) -> std::result::Result<(),MechError> {
    self.rows = rows;
    self.cols = cols;
    self.col_kinds.resize(cols,ValueKind::Empty);
    self.col_map.resize(cols);
    self.row_map.resize(rows);
    for col in &mut self.data {
      col.resize(rows);
    }
    self.data.resize(cols,Column::Empty);
    Ok(())
  }

  pub fn get_col_raw(&self, col_ix: usize) -> std::result::Result<Column,MechError> {
    if col_ix < self.cols {
      Ok(self.data[col_ix].clone())
    } else {
      Err(MechError::GenericError(6353)) 
    }

  }

  pub fn set_col(&mut self, col_ix: usize, column: Column) -> std::result::Result<(),MechError> {
    if col_ix < self.cols {
      if self.col_kinds[col_ix] == ValueKind::Empty {
        self.col_kinds[col_ix] = column.kind();
        self.data[col_ix] = column;
        Ok(())
      } else {
        Err(MechError::GenericError(6354)) 
      }
    } else {
      Err(MechError::GenericError(6355)) 
    }
  }

  /*
  pub fn set_raw(&mut self, row_ix: usize, col_ix: usize, value: Value) -> std::result::Result<(),MechError> {
    if col_ix < self.cols && row_ix < self.rows {
      let mut col = &mut self.data[col_ix];
      col.set_unchecked(row_ix,value);
      Ok(())
    } else {
      Err(MechError::GenericError(6356))
    }
  }

  pub fn get_raw(&self, row_ix: usize, col_ix: usize) -> std::result::Result<Value,MechError> {
    if col_ix < self.cols && row_ix < self.rows {
      let col = &self.data[col_ix];
      Ok(col.get_unchecked(row_ix))
    } else {
      Err(MechError::GenericError(6357))
    }
  }*/
}

type TableIx = usize;
type Alias = u64;

#[derive(Debug)]
pub struct AliasMap {
  capacity: usize,
  ix_to_alias: Vec<Alias>,  
  alias_to_ix: HashMap<Alias,TableIx>,
}

impl AliasMap {
  pub fn new(capacity: usize) -> Self {
    AliasMap {
      capacity,
      ix_to_alias: vec![0;capacity],
      alias_to_ix: HashMap::new(),
    }
  }

  pub fn resize(&mut self, new_capacity: usize) {
    self.capacity = new_capacity;
    self.ix_to_alias.resize(new_capacity,0);
  }

  pub fn insert(&mut self, ix: TableIx, alias: Alias) -> std::result::Result<(),MechError> {
    if ix < self.capacity {
      self.ix_to_alias[ix] = alias;
      self.alias_to_ix.insert(alias,ix);
      Ok(())
    } else {
      Err(MechError::GenericError(8210))
    }
  }

  pub fn get_index(&self, alias: &Alias) -> std::result::Result<TableIx,MechError> {
    match self.alias_to_ix.get(alias) {
      Some(ix) => Ok(*ix),
      None => Err(MechError::GenericError(8211)),
    }
  }

  pub fn get_alias(&self, ix: &TableIx) -> std::result::Result<Alias,MechError> {
    if ix < &self.capacity {
      Ok(self.ix_to_alias[*ix])
    } else {
      Err(MechError::GenericError(8212))
    }
  }

}

// -------------------------
// Value
// -------------------------

#[derive(Copy,Clone,Debug)]
pub enum Value {
  U8(U8),
  U16(U16),
  U32(U32),
  U64(U64),
  F32(F32),
  Empty,
}

// -------------------------
// Functions
// -------------------------

pub fn par_add<T,U>(lhs: &ColumnV<T>, rhs: &ColumnV<U>, out: &ColumnV<U>) 
  where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send,
        U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send,
{
  let (ColumnV(lhs),ColumnV(rhs),ColumnV(out)) = (lhs,rhs,out);
  out.borrow_mut().par_iter_mut()
     .zip(lhs.borrow().par_iter().map(|x| T::into(*x)))
     .zip(rhs.borrow().par_iter())
     .for_each(|((out, lhs),rhs)| *out = lhs.add(*rhs)); 
}



#[derive(Debug)]
pub struct AddVV<T,U,V> {
  pub lhs: (ColumnV<T>, usize, usize),
  pub rhs: (ColumnV<U>, usize, usize),
  pub out: ColumnV<V>
}
impl<T,U,V> MechFunction for AddVV<T,U,V> 
where T: Copy + Debug + Clone + Add<Output = T> + Into<V> + Sync + Send,
      U: Copy + Debug + Clone + Add<Output = U> + Into<V> + Sync + Send,
      V: Copy + Debug + Clone + Add<Output = V> + Sync + Send,
{
  fn solve(&self) {
    let (lhs,lsix,leix) = &self.lhs;
    let (rhs,rsix,reix) = &self.rhs;
    self.out.borrow_mut().par_iter_mut()
                    .zip(lhs.borrow()[*lsix..=*leix].par_iter().map(|x| T::into(*x)))
                    .zip(rhs.borrow()[*rsix..=*reix].par_iter().map(|x| U::into(*x)))
                    .for_each(|((out, lhs),rhs)| *out = lhs.add(rhs)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub fn copy<T,U>(arg: &ColumnV<T>, out: &ColumnV<U>, start: usize) 
  where T: Copy + Debug + Clone + Into<U>,
        U: Copy + Debug + Clone + Into<T>,
{
  let (ColumnV(arg),ColumnV(out)) = (arg,out);
  let mut o = out.borrow_mut();
  o[start..].iter_mut().zip(arg.borrow().iter().map(|x| T::into(*x))).for_each(|(out, arg)| *out = arg.clone()); 
}



fn main() -> std::result::Result<(),MechError> {

  const n: usize = 1e7 as usize;

  let xx = vec![1.0;n];
  let yy = vec![2.0;n];
  let mut zz = vec![0.0;n];


  let mut f32_1 = ColumnV::<F32>::new(vec![F32(1.0);n]);
  let mut f32_2 = ColumnV::<F32>::new(vec![F32(2.0);n]);
  let mut f32_3 = ColumnV::<F32>::new(vec![F32(0.0);n]);

  let mut u16_1 = ColumnV::<U16>::new(vec![U16(1);n]);
  let mut u16_2 = ColumnV::<U16>::new(vec![U16(4);n]);
  let mut u16_3 = ColumnV::<U16>::new(vec![U16(0);n]);

  let mut u8_1 = ColumnV::<U8>::new(vec![U8(1);n]);
  let mut u8_2 = ColumnV::<U8>::new(vec![U8(4);n]);
  let mut u8_3 = ColumnV::<U8>::new(vec![U8(0);n]);

  let mut table = Table::new(0x1234, 0,0);
  let mut table2 = Table::new(0x12345, 0,0);

  table.resize(n,2);
  table2.resize(n,1);
  
  table.set_col(0,Column::U8(u8_1));
  table.set_col(1,Column::U8(u8_2));
  table2.set_col(0,Column::U8(u8_3));

  let lhs = table.get_col_raw(0)?;
  let rhs = table.get_col_raw(1)?;
  let out = table2.get_col_raw(0)?;

  let mut plan = Plan::new();

  if let (Column::U8(lhs),Column::U8(rhs),Column::U8(out)) = (lhs,rhs,out) {
    plan.push(AddVV{
      lhs: (lhs.clone(),0,n-1),
      rhs: (rhs.clone(),0,n-1),
      out: out.clone()
    });
  }

  let col0 = ColumnV::<Value>::new(vec![Value::F32(F32(1.0));table.rows]);
  let col1 = ColumnV::<Value>::new(vec![Value::U8(U8(2));table.rows]);
  let col3 = ColumnV::<Value>::new(vec![Value::F32(F32(0.0));table.rows]);

  let mut mech_table = mech_core::Table::new(0x0123456789ABCDEF,n,2);
  let mut mech_table2 = mech_core::Table::new(0x0123456789ABCDEA,n,1);

  mech_table2.set_col_kind(0,ValueKind::F32);
  let out = mech_table2.get_column_unchecked(0);


  mech_table.set_col_kind(0,ValueKind::F32);
  mech_table.set_col_kind(1,ValueKind::F32);

  let lhs = mech_table.get_column_unchecked(0);
  let rhs = mech_table.get_column_unchecked(1);


  for i in 0..mech_table.rows {
    mech_table.set_raw(i,0,mech_core::Value::F32(i as f32));
    mech_table.set_raw(i,1,mech_core::Value::F32(i as f32));
  }

  let mut fxn = match (&lhs,&rhs,&out) {
    (mech_core::Column::F32(lhs),mech_core::Column::F32(rhs),mech_core::Column::F32(out)) => {
      mech_core::math::AddVV::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }
    }
    _ => {return Err(MechError::GenericError(8888));}
  };


  //println!("{:#?}", table);
  //println!("{:#?}", table2);

  let i = 4000;

  println!("NEW HOTNESS");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    plan.solve();
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);


  println!("STATUS QUO");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    fxn.solve();
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

/*
  let i = 4000;
  println!("FLOAT");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    par_add(&f32_1,&f32_2,&f32_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("HEAP");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    for i in 0..n {
      zz[i] = xx[i] + yy[i];
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("{:?}", zz[n-1]);
  println!("{:?}", f32_3.get_unchecked(n-1));*/




/*
  table.set_col(0,col0,ValueKind::F32);
  table.set_col(1,col1,ValueKind::U8);
  println!("{:#?}", table);
  table.set_raw(0,1,Value::U8(U8(3)));
  let val = table.get_raw(0,0);
  println!("{:#?}", val);
  println!("{:#?}", table);

  let c1 = table.get_col_raw(0).unwrap();
  println!("{:?}", c1);
  let c2 = table.get_col_raw(1).unwrap();
  println!("{:?}", c2);*/

  //add(&c1,&c2,&col3);

  /*
  let i = 4000;

  println!("FLOAT");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&f32_1,&f32_2,&f32_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("U8");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u8_1,&u8_2,&u8_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("U16");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u16_1,&u16_2,&u16_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("MIXED");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u8_1,&u16_2,&u16_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);*/

  /*let c2 = Column::<U8>::new(vec![U8(0);9]);
  let c3 = Column::<U32>::new(vec![U32(4),U32(5),,U32(5)]);
  let c4 = Column::<U64>::new(vec![U64(6),U64(7),U64(8),U64(9)]);

  copy(&c1,&c2,0);
  copy(&c3,&c2,c1.len());
  copy(&c4,&c2,c1.len() + c3.len());

  add(c1,c2,c2)


  c1.set_unchecked(2,F32(42.0));

  copy(&c1,&c2,0);*/


/* 
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  let mut total_time = VecDeque::new();  
  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let mut time_timer = Table::new(hash_str("time/timer"),1,2);
    time_timer.set_col_kind(0,ValueKind::F32);
    time_timer.set_col_kind(1,ValueKind::F32);
    time_timer.set_raw(0,0,Value::F32(60.0));
    core.insert_table(time_timer.clone());

    // #gravity = 1
    let mut gravity = Table::new(hash_str("gravity"),1,1);
    gravity.set_col_kind(0,ValueKind::F32);
    gravity.set_raw(0,0,Value::F32(1.0));
    core.insert_table(gravity.clone());

    // -80%
    let mut const1 = Table::new(hash_str("-0.8"),1,1);
    const1.set_col_kind(0,ValueKind::F32);
    const1.set_raw(0,0,Value::F32(-0.8));
    core.insert_table(const1.clone());

    // 500
    let mut const2 = Table::new(hash_str("500.0"),1,1);
    const2.set_col_kind(0,ValueKind::F32);
    const2.set_raw(0,0,Value::F32(500.0));
    core.insert_table(const2.clone());

    // 0
    let mut const3 = Table::new(hash_str("0.0"),1,1);
    const3.set_col_kind(0,ValueKind::F32);
    const3.set_raw(0,0,Value::F32(0.0));
    core.insert_table(const3.clone());

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let mut balls = Table::new(hash_str("balls"),n,4);
    balls.set_col_kind(0,ValueKind::F32);
    balls.set_col_kind(1,ValueKind::F32);
    balls.set_col_kind(2,ValueKind::F32);
    balls.set_col_kind(3,ValueKind::F32);
    for i in 0..n {
      balls.set_raw(i,0,Value::F32(i as f32));
      balls.set_raw(i,1,Value::F32(i as f32));
      balls.set_raw(i,2,Value::F32(3.0));
      balls.set_raw(i,3,Value::F32(4.0));
    }
    core.insert_table(balls.clone());
  }

  // Table
  let (x,y,vx,vy) = {
    match core.get_table_by_id(hash_str("balls")) {
      Ok(balls_rc) => {
        let balls = balls_rc.borrow();
        (balls.get_column_unchecked(0),
        balls.get_column_unchecked(1),
        balls.get_column_unchecked(2),
        balls.get_column_unchecked(3))
      }
      _ => std::process::exit(1),
    }
  };

  let g = {
    match core.get_table_by_id(hash_str("gravity")) {
      Ok(gravity_rc) => {
        let gravity = gravity_rc.borrow();
        gravity.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c1 = {
    match core.get_table_by_id(hash_str("-0.8")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c500 = {
    match core.get_table_by_id(hash_str("500.0")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c0 = {
    match core.get_table_by_id(hash_str("0.0")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };
  
  // Temp Vars
  let mut vy2 = Column::F32(Rc::new(RefCell::new(vec![0.0; n])));
  let mut iy = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut iyy = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut iy_or = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ix = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ixx = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ix_or = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut vx2 = Column::F32(Rc::new(RefCell::new(vec![0.0; n])));

  // Update the block positions on each tick of the timer  
  let mut block1 = Block::new();
  block1.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block1")), rows: 1, columns: 1});
  block1.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("gravity")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block1.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&x,&vx,&y,&vy,&g) {
    (Column::F32(x),Column::F32(vx),Column::F32(y),Column::F32(vy),Column::F32(g)) => {
      // #ball.x := #ball.x + #ball.vx
      block1.plan.push(math::ParAddVVIP::<f32>{out: x.clone(), arg: vx.clone()});
      // #ball.y := #ball.y + #ball.vy    
      block1.plan.push(math::ParAddVVIP::<f32>{out: y.clone(), arg: vy.clone()});
      // #ball.vy := #ball.vy + #gravity
      block1.plan.push(math::ParAddVSIP::<f32>{out: vy.clone(), arg: g.clone()});
    }
    _ => (),
  }

  // Keep the balls within the boundary height
  let mut block2 = Block::new();
  block2.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block2")), rows: 1, columns: 1});
  block2.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block2.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block2.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&y,&iy,&iyy,&iy_or,&c1,&vy2,&vy,&c500,&c0) {
    (Column::F32(y),Column::Bool(iy),Column::Bool(iyy),Column::Bool(iy_or),Column::F32(c1),Column::F32(vy2),Column::F32(vy),Column::F32(m500),Column::F32(m0)) => {
      // iy = #ball.y > #boundary.height
      block2.plan.push(compare::ParGreaterVS::<f32>{lhs: y.clone(), rhs: m500.clone(), out: iy.clone()});
      // iyy = #ball.y < 0
      block2.plan.push(compare::ParLessVS::<f32>{lhs: y.clone(), rhs: m0.clone(), out: iyy.clone()});
      // #ball.y{iy} := #boundary.height
      block2.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out:  y.clone(), oix: iy.clone()});
      // #ball.vy{iy | iyy} := #ball.vy * -80%
      block2.plan.push(logic::ParOrVV{lhs: iy.clone(), rhs: iyy.clone(), out: iy_or.clone()});
      block2.plan.push(math::ParMulVS::<f32>{lhs: vy.clone(), rhs: c1.clone(), out: vy2.clone()});
      block2.plan.push(table::ParSetVVB::<f32>{arg: vy2.clone(), out: vy.clone(), oix: iy_or.clone()});
    }
    _ => (),
  }
 
  // Keep the balls within the boundary width
  let mut block3 = Block::new();
  block3.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block3")), rows: 1, columns: 1});
  block3.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block3.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block3.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&x,&ix,&ixx,&ix_or,&vx,&c1,&vx2,&c500,&c0) {
    (Column::F32(x),Column::Bool(ix),Column::Bool(ixx),Column::Bool(ix_or),Column::F32(vx),Column::F32(c1),Column::F32(vx2),Column::F32(m500),Column::F32(m0)) => {
      // ix = #ball.x > #boundary.width
      block3.plan.push(compare::ParGreaterVS::<f32>{lhs: x.clone(), rhs: m500.clone(), out: ix.clone()});
      // ixx = #ball.x < 0
      block3.plan.push(compare::ParLessVS::<f32>{lhs: x.clone(), rhs: m0.clone(), out: ixx.clone()});
      // #ball.x{ix} := #boundary.width
      block3.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out: x.clone(), oix: ix.clone()});
      // #ball.vx{ix | ixx} := #ball.vx * -80%
      block3.plan.push(logic::ParOrVV{lhs: ix.clone(), rhs: ixx.clone(), out: ix_or.clone()});
      block3.plan.push(math::ParMulVS::<f32>{lhs: vx.clone(), rhs: c1.clone(), out: vx2.clone()});
      block3.plan.push(table::ParSetVVB::<f32>{arg: vx2.clone(), out: vx.clone(), oix: ix_or.clone()});
    }
    _ => (),
  }

  //println!("{:?}", block1);
  let block1_ref = Rc::new(RefCell::new(block1));
  core.insert_block(block1_ref.clone());

  //println!("{:?}", block2);
  let block2_ref = Rc::new(RefCell::new(block2));
  core.insert_block(block2_ref.clone());

  //println!("{:?}", block3);
  let block3_ref = Rc::new(RefCell::new(block3));
  core.insert_block(block3_ref.clone());

  core.schedule_blocks();

  //println!("{:?}", core);

  for i in 0..5000 {
    let txn = vec![Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(i as f32))]))];
    
    let start_ns = time::precise_time_ns();
    core.process_transaction(&txn)?;
    let end_ns = time::precise_time_ns();

    let cycle_duration = (end_ns - start_ns) as f32;
    total_time.push_back(cycle_duration);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    //println!("{:?}", core.get_table("balls"));
    //let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
    //println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }
  let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f32;
  println!("{:0.4?} s", time / 1e9);
  println!("{:?}", core);
*/
  Ok(())
}