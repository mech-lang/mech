use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};

use rayon::prelude::*;
use std::collections::VecDeque;
use std::thread;
use crate::*;

use std::fmt::*;
use num_traits::*;
use std::ops::*;


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
  String(ColumnV<MechString>),
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
      Column::String(col) => col.len(),
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
      Column::String(col) => col.borrow_mut().resize(rows,MechString::new()),
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
      Column::String(_) => ValueKind::String,
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

impl<T: Clone> ColumnV<T> {

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
    c_brrw[row].clone()
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
    #[derive(Copy, Clone)]
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

pub type Reference = Rc<RefCell<Table>>;


/*



pub type ColumnV<T> = Rc<RefCell<Vec<T>>>;

#[derive(Clone, Debug)]
pub enum Column {
  F32(ColumnV<f32>),
  F64(ColumnV<f64>),
  U8(ColumnV<u8>),
  U16(ColumnV<u16>),
  U32(ColumnV<u32>),
  U64(ColumnV<u64>),
  U128(ColumnV<u128>),
  Ref(ColumnV<TableId>),
  I8(ColumnV<i8>),
  I16(ColumnV<i16>),
  I32(ColumnV<i32>),
  I64(ColumnV<i64>),
  I128(ColumnV<i128>),
  Index(ColumnV<usize>),
  Bool(ColumnV<bool>),
  String(ColumnV<MechString>),
  Reference((Reference,(ColumnIndex,ColumnIndex))),
  Time(ColumnV<f32>),
  Length(ColumnV<f32>),
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
  }

  pub fn get_u8(&self) -> Result<ColumnV<u8>,MechError> {
    match self {
      Column::U8(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8172));},
    }
  }

  pub fn get_u16(&self) -> Result<ColumnV<u16>,MechError> {
    match self {
      Column::U16(col) => Ok(col.clone()),
      Column::U8(col) => {
        let out_col = col.borrow().iter().map(|x| *x as u16).collect();
        Ok(Rc::new(RefCell::new(out_col)))
      }
      x => {return Err(MechError::GenericError(8182));},
    }
  }

  
  pub fn get_f32(&self) -> Result<ColumnV<f32>,MechError> {
    match self {
      Column::F32(col) => Ok(col.clone()),
      x => {
        println!("{:?}", x);
        return Err(MechError::GenericError(8189));
      },
    }
  }


  pub fn get_bool(&self) -> Result<ColumnV<bool>,MechError> {
    match self {
      Column::Bool(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8170));},
    }
  }

  pub fn get_string(&self) -> Result<ColumnV<MechString>,MechError> {
    match self {
      Column::String(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8171));},
    }
  }

  pub fn get_reference(&self) -> Result<ColumnV<TableId>,MechError> {
    match self {
      Column::Ref(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8175));},
    }
  }

  pub fn get_u64(&self) -> Result<ColumnV<u64>,MechError> {
    match self {
      Column::U64(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8173));},
    }
  }

  pub fn to_index(&mut self) -> Result<Column,MechError> {
    match self {
      Column::U64(col) => {
        let mut new_column: Vec<usize> = Vec::new();
        for value in col.borrow().iter() {
          new_column.push(*value as usize);
        }
        Ok(Column::Index(Rc::new(RefCell::new(new_column))))
      }
      _ => Err(MechError::GenericError(8174)),
    }
  }

  pub fn len(&self) -> usize {
    match self {
      Column::U8(col) => col.borrow().len(),
      Column::U16(col) => col.borrow().len(),
      Column::U32(col) => col.borrow().len(),
      Column::U64(col) => col.borrow().len(),
      Column::U128(col) => col.borrow().len(),
      Column::I8(col) => col.borrow().len(),
      Column::I16(col) => col.borrow().len(),
      Column::I32(col) => col.borrow().len(),
      Column::I64(col) => col.borrow().len(),
      Column::I128(col) => col.borrow().len(),
      Column::Length(col) | Column::Time(col) |
      Column::F32(col) => col.borrow().len(),
      Column::F64(col) => col.borrow().len(),
      Column::Bool(col) => col.borrow().len(),
      Column::String(col) => col.borrow().len(),
      Column::Index(col) => col.borrow().len(),
      Column::Ref(col) => col.borrow().len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Empty => 0,
    }
  }

  pub fn logical_len(&self) -> usize {
    match self {
      Column::U8(col) => col.borrow().len(),
      Column::U16(col) => col.borrow().len(),
      Column::U32(col) => col.borrow().len(),
      Column::U64(col) => col.borrow().len(),
      Column::U128(col) => col.borrow().len(),
      Column::I8(col) => col.borrow().len(),
      Column::I16(col) => col.borrow().len(),
      Column::I32(col) => col.borrow().len(),
      Column::I64(col) => col.borrow().len(),
      Column::I128(col) => col.borrow().len(),
      Column::Ref(col) => col.borrow().len(),
      Column::Time(col) | Column::Length(col) |
      Column::F32(col) => col.borrow().len(),
      Column::F64(col) => col.borrow().len(),
      Column::Bool(col) => col.borrow().iter().fold(0, |acc,x| if *x { acc + 1 } else { acc }),
      Column::String(col) => col.borrow().len(),
      Column::Index(col) => col.borrow().len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Empty => 0,
    }    
  }

  pub fn resize(&self, rows: usize) -> Result<(),MechError> {
    match self {
      Column::U8(col) => col.borrow_mut().resize(rows,0),
      Column::U16(col) => col.borrow_mut().resize(rows,0),
      Column::U32(col) => col.borrow_mut().resize(rows,0),
      Column::U64(col) => col.borrow_mut().resize(rows,0),
      Column::U128(col) => col.borrow_mut().resize(rows,0),
      Column::I8(col) => col.borrow_mut().resize(rows,0),
      Column::I16(col) => col.borrow_mut().resize(rows,0),
      Column::I32(col) => col.borrow_mut().resize(rows,0),
      Column::I64(col) => col.borrow_mut().resize(rows,0),
      Column::I128(col) => col.borrow_mut().resize(rows,0),
      Column::Time(col) | Column::Length(col) |
      Column::F32(col) => col.borrow_mut().resize(rows,0.0),
      Column::F64(col) => col.borrow_mut().resize(rows,0.0),
      Column::Ref(col) => col.borrow_mut().resize(rows,TableId::Local(0)),
      Column::Index(col) => col.borrow_mut().resize(rows,0),
      Column::Bool(col) => col.borrow_mut().resize(rows,false),
      Column::String(col) => col.borrow_mut().resize(rows,MechString::new()),
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
      Column::String(_) => ValueKind::String,
      Column::Index(_) => ValueKind::Index,
      Column::Ref(_) => ValueKind::Reference,
      Column::Reference((table,index)) => table.borrow().kind(),
      Column::Time(_) => ValueKind::Time,
      Column::Length(_) => ValueKind::Length,
      Column::Empty => ValueKind::Empty,
    }
  }

}


*/