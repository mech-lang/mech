/* 
In Mech, a table is composed of one or more columns, and each column contains values of a specific data type. The Column enum represents the different types of columns that can be created in Mech, such as floating-point columns (f32, F32, F64, etc.), integer columns (U8, U16, U32, etc.), boolean columns (Bool), and so on. Each variant of the Column enum is associated with a specific type of value.
*/

use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};

use std::collections::VecDeque;
use std::thread;
use crate::*;

use std::fmt::*;
use num_traits::identities::Zero;
use std::ops::*;

#[derive(Clone, Debug)]
pub enum Column {
  f32(ColumnV<f32>),
  F32(ColumnV<F32>),
  F64(ColumnV<F64>),
  U8(ColumnV<U8>),
  U16(ColumnV<U16>),
  U32(ColumnV<U32>),
  U64(ColumnV<U64>),
  U128(ColumnV<U128>),
  Ref(ColumnV<TableId>),
  I8(ColumnV<I8>),
  I16(ColumnV<I16>),
  I32(ColumnV<I32>),
  I64(ColumnV<I64>),
  I128(ColumnV<I128>),
  Index(ColumnV<usize>),
  Bool(ColumnV<bool>),
  String(ColumnV<MechString>),
  Reference((TableRef,(ColumnIndex,ColumnIndex))),
  Time(ColumnV<F32>),
  Length(ColumnV<F32>),
  Angle(ColumnV<F32>),
  Speed(ColumnV<F32>),
  Enum(ColumnV<Enum>),
  Any(ColumnV<Value>),
  Empty,
}

#[derive(Clone, Debug)]
pub enum ColumnIndex {
  All,
  RealIndex(ColumnV<F32>),
  Index(usize),
  IndexCol(ColumnV<usize>),
  IndexColU8(ColumnV<U8>),
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
      Column::f32(col) => col.len(),
      Column::Length(col) | Column::Time(col) | Column::Speed(col) |
      Column::Angle(col) |
      Column::F32(col) => col.len(),
      Column::F64(col) => col.len(),
      Column::Bool(col) => col.len(),
      Column::Index(col) => col.len(),
      Column::String(col) => col.len(),
      Column::Any(col) => col.len(),
      Column::Ref(col) => col.len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Enum(col) => col.len(),
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
      Column::U128(col) => col.borrow_mut().resize(rows,U128(0)),
      Column::I8(col) => col.borrow_mut().resize(rows,I8(0)),
      Column::I16(col) => col.borrow_mut().resize(rows,I16(0)),
      Column::I32(col) => col.borrow_mut().resize(rows,I32(0)),
      Column::I64(col) => col.borrow_mut().resize(rows,I64(0)),
      Column::I128(col) => col.borrow_mut().resize(rows,I128(0)),
      Column::f32(col) => col.borrow_mut().resize(rows,0.0),
      Column::Length(col) | Column::Time(col) | Column::Speed(col) |
      Column::Angle(col) |
      Column::F32(col) => col.borrow_mut().resize(rows,F32(0.0)),
      Column::F64(col) => col.borrow_mut().resize(rows,F64(0.0)),
      Column::Ref(col) => col.borrow_mut().resize(rows,TableId::Local(0)),
      Column::Index(col) => col.borrow_mut().resize(rows,0),
      Column::Any(col) => col.borrow_mut().resize(rows,Value::Empty),
      Column::Bool(col) => col.borrow_mut().resize(rows,false),
      Column::String(col) => col.borrow_mut().resize(rows,MechString::new()),
      Column::Enum(col) => col.borrow_mut().resize(rows,Enum{kind:0, variant:0}),
      Column::Reference(_) |
      Column::Empty => {return Err(MechError{msg: "".to_string(), id: 9430, kind: MechErrorKind::None});}
    }
    Ok(())
  }

  pub fn extend(&self, other: &Column) -> std::result::Result<(),MechError> {
    match (self,other) {
      (Column::U8(col),Column::U8(other)) => col.extend(other),
      (Column::U16(col),Column::U16(other)) => col.extend(other),
      (Column::U32(col),Column::U32(other)) => col.extend(other),
      (Column::U64(col),Column::U64(other)) => col.extend(other),
      (Column::U128(col),Column::U128(other)) => col.extend(other),
      (Column::I8(col),Column::I8(other)) => col.extend(other),
      (Column::I16(col),Column::I16(other)) => col.extend(other),
      (Column::I32(col),Column::I32(other)) => col.extend(other),
      (Column::I64(col),Column::I64(other)) => col.extend(other),
      (Column::I128(col),Column::I128(other)) => col.extend(other),
      (Column::Length(col),Column::Length(other)) |
      (Column::Time(col),Column::Time(other)) |
      (Column::Speed(col),Column::Speed(other)) |
      (Column::Angle(col),Column::Angle(other)) |
      (Column::F32(col),Column::F32(other)) => col.extend(other),
      (Column::F64(col),Column::F64(other)) => col.extend(other),
      (Column::f32(col),Column::f32(other)) => col.extend(other),
      (Column::Index(col),Column::Index(other)) => col.extend(other),
      (Column::Bool(col),Column::Bool(other)) => col.extend(other),
      (Column::String(col),Column::String(other)) => col.extend(other),
      _ => {return Err(MechError{msg: "".to_string(), id: 9431, kind: MechErrorKind::None});}
    }
    Ok(())
  }

  unwrap_column!(unwrap_u8,U8);
  unwrap_column!(unwrap_u16,U16);
  unwrap_column!(unwrap_u32,U32);
  unwrap_column!(unwrap_u64,U64);
  unwrap_column!(unwrap_u128,U128);
  unwrap_column!(unwrap_i8,I8);
  unwrap_column!(unwrap_i16,I16);
  unwrap_column!(unwrap_i32,I32);
  unwrap_column!(unwrap_i64,I64);
  unwrap_column!(unwrap_i128,I128);
  unwrap_column!(unwrap_f32,F32);
  unwrap_column!(unwrap_f64,F64);

  pub fn kind(&self) -> ValueKind {
    match self {
      Column::f32(_) => ValueKind::f32,
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
      Column::Speed(_) => ValueKind::Speed,
      Column::Length(_) => ValueKind::Length,
      Column::Angle(_) => ValueKind::Angle,
      Column::Any(_) => ValueKind::Any,
      Column::Enum(_) => ValueKind::Enum((0,0)),
      Column::Empty => ValueKind::Empty,
    }
  }
}

#[macro_export]
macro_rules! unwrap_column {
  ($function_name:tt,$type:tt) => (
    pub fn $function_name(&self) -> Option<&ColumnV<$type>> {
      match self {
        Column::$type(c) => Some(c),
        _ => None,
      }
    }
  )
}

#[derive(Clone)]
pub struct ColumnV<T>(Rc<RefCell<Vec<T>>>);

impl<T: Clone + Debug> ColumnV<T> {

  pub fn new(vec: Vec<T>) -> ColumnV<T> {
    ColumnV(Rc::new(RefCell::new(vec)))
  }

  pub fn len(&self) -> usize {
    let ColumnV(col) = self;
    col.borrow().len()
  }

  pub fn extend(&self, other: &ColumnV<T>) {
    let mut col_brrw = self.borrow_mut();
    let other_brrw = other.borrow();
    col_brrw.append(&mut other_brrw.clone());
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

impl<T: Clone + Debug> IntoIterator for ColumnV<T> {
  type Item = T;
  type IntoIter = std::vec::IntoIter<Self::Item>;

  fn into_iter(self) -> Self::IntoIter {
    <Vec<T> as Clone>::clone(&self.borrow()).into_iter() 
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
mech_type!(F64,f64);
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
mech_neg!(I8);
mech_neg!(I16);
mech_neg!(I32);
mech_neg!(I64);
mech_neg!(I128);
mech_neg!(F32);
mech_neg!(F64);

impl Zero for U8 {
  fn zero() -> Self {
    U8::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for U16 {
  fn zero() -> Self {
    U16::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for U32 {
  fn zero() -> Self {
    U32::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for U64 {
  fn zero() -> Self {
    U64::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for U128 {
  fn zero() -> Self {
    U128::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for I8 {
  fn zero() -> Self {
    I8::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for I16 {
  fn zero() -> Self {
    I16::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for I32 {
  fn zero() -> Self {
    I32::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for I64 {
  fn zero() -> Self {
    I64::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for I128 {
  fn zero() -> Self {
    I128::new(0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0
  }
}

impl Zero for F32 {
  fn zero() -> Self {
    F32::new(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

impl Zero for F64 {
  fn zero() -> Self {
    F64::new(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

mech_type_conversion!(U8,F32,f32);
mech_type_conversion!(U8,F64,f64);
mech_type_conversion!(U8,U128,u128);
mech_type_conversion!(U8,U64,u64);
mech_type_conversion!(U8,U32,u32);
mech_type_conversion!(U8,U16,u16);
mech_type_conversion!(F32,U8,u8);
mech_type_conversion!(F32,U16,u16);
mech_type_conversion!(F32,U32,u32);
mech_type_conversion!(F32,U64,u64);
mech_type_conversion!(F32,U128,u128);
mech_type_conversion!(F32,F64,f64);
mech_type_conversion!(F32,I8,i8);
mech_type_conversion!(F32,I16,i16);
mech_type_conversion!(F32,I32,i32);
mech_type_conversion!(F32,I64,i64);
mech_type_conversion!(F32,I128,i128);
mech_type_conversion!(I8,I16,i16);
mech_type_conversion!(I8,I32,i32);
mech_type_conversion!(I8,I64,i64);
mech_type_conversion!(I8,I128,i128);
mech_type_conversion!(I8,F32,f32);
mech_type_conversion!(I8,F64,f64);
mech_type_conversion!(I16,I32,i32);
mech_type_conversion!(I16,I64,i64);
mech_type_conversion!(I16,I128,i128);
mech_type_conversion!(I16,F32,f32);
mech_type_conversion!(I16,F64,f64);
mech_type_conversion!(I32,I64,i64);
mech_type_conversion!(I32,I128,i128);
mech_type_conversion!(I32,F32,f32);
mech_type_conversion!(I32,F64,f64);
mech_type_conversion!(I64,F32,f32);
mech_type_conversion!(I64,I128,i128);
mech_type_conversion!(I64,F64,f64);
mech_type_conversion!(I128,F32,f32);
mech_type_conversion!(I128,F64,f64);
mech_type_conversion!(F64,U8,u8);
mech_type_conversion!(F64,U16,u16);
mech_type_conversion!(F64,U32,u32);
mech_type_conversion!(F64,U64,u64);
mech_type_conversion!(F64,U128,u128);
mech_type_conversion!(F64,I8,i8);
mech_type_conversion!(F64,I16,i16);
mech_type_conversion!(F64,I32,i32);
mech_type_conversion!(F64,I64,i64);
mech_type_conversion!(F64,I128,i128);
mech_type_conversion!(F64,F32,f32);
mech_type_conversion!(U16,U32,u32);
mech_type_conversion!(U16,U64,u64);
mech_type_conversion!(U16,U128,u128);
mech_type_conversion!(U16,F32,f32);
mech_type_conversion!(U16,F64,f64);
mech_type_conversion!(U32,U64,u64);
mech_type_conversion!(U32,U128,u128);
mech_type_conversion!(U32,F32,f32);
mech_type_conversion!(U32,F64,f64);
mech_type_conversion!(U64,U8,u8);
mech_type_conversion!(U64,U128,u128);
mech_type_conversion!(U64,F32,f32);
mech_type_conversion!(U64,F64,f64);
mech_type_conversion!(U128,F32,f32);
mech_type_conversion!(U128,F64,f64);
mech_type_conversion_raw!(U8,u8);
mech_type_conversion_raw!(U16,u16);
mech_type_conversion_raw!(U32,u32);
mech_type_conversion_raw!(U64,u32);
mech_type_conversion_raw!(U64,u64);
mech_type_conversion_raw!(U64,i32);
mech_type_conversion_raw!(U64,f32);
mech_type_conversion_raw!(U64,f64);
mech_type_conversion_raw!(U128,u32);
mech_type_conversion_raw!(U128,u128);
mech_type_conversion_raw!(I8,i8);
mech_type_conversion_raw!(I16,i16);
mech_type_conversion_raw!(I32,i32);
mech_type_conversion_raw!(I64,i64);
mech_type_conversion_raw!(I128,u32);
mech_type_conversion_raw!(I128,i128);
mech_type_conversion_raw!(F32,i32);
mech_type_conversion_raw!(F32,f32);
mech_type_conversion_raw!(F32,f64);
mech_type_conversion_raw!(F32,u64);
mech_type_conversion_raw!(F32,usize);

mech_value_conversion!(U8,U8);
mech_value_conversion!(U16,U16);
mech_value_conversion!(U32,U32);
mech_value_conversion!(U64,U64);
mech_value_conversion!(U128,U128);
mech_value_conversion!(I8,I8);
mech_value_conversion!(I16,I16);
mech_value_conversion!(I32,I32);
mech_value_conversion!(I64,I64);
mech_value_conversion!(I128,I128);
mech_value_conversion!(F32,F32);
mech_value_conversion!(F64,F64);
mech_value_conversion!(MechString,String);

#[macro_export]
macro_rules! mech_type {
  ($wrapper:tt,$type:tt) => (
    use std::fmt::*;
    use num_traits::*;
    use std::ops::*;
    #[derive(Copy,Clone,PartialEq,PartialOrd,Serialize,Deserialize)]
    pub struct $wrapper($type);
    impl $wrapper {
      pub fn new(inner: $type) -> $wrapper {
        $wrapper(inner)
      }
      pub fn kind(&self) -> ValueKind {
        ValueKind::$wrapper
      }
      pub fn unwrap(&self) -> $type {
        self.0
      }
    }
    impl Add for $wrapper {
      type Output = $wrapper;
      fn add(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs + rhs)
      }
    }
    impl Sub for $wrapper {
      type Output = $wrapper;
      fn sub(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs - rhs)
      }
    }
    impl Mul for $wrapper {
      type Output = $wrapper;
      fn mul(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs * rhs)
      }
    }
    impl Div for $wrapper {
      type Output = $wrapper;
      fn div(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs / rhs)
      }
    }
    impl AddAssign for $wrapper {
      fn add_assign(&mut self, other: Self) {
        let ($wrapper(lhs),$wrapper(rhs)) = (&self,other);
        *self = $wrapper(*lhs + rhs);
      }
    }
    impl SubAssign for $wrapper {
      fn sub_assign(&mut self, other: Self) {
        let ($wrapper(lhs),$wrapper(rhs)) = (&self,other);
        *self = $wrapper(*lhs - rhs);
      }
    }
    impl MulAssign for $wrapper {
      fn mul_assign(&mut self, other: Self) {
        let ($wrapper(lhs),$wrapper(rhs)) = (&self,other);
        *self = $wrapper(*lhs * rhs);
      }
    }
    impl DivAssign for $wrapper {
      fn div_assign(&mut self, other: Self) {
        let ($wrapper(lhs),$wrapper(rhs)) = (&self,other);
        *self = $wrapper(*lhs / rhs);
      }
    }
    impl fmt::Debug for $wrapper {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let $wrapper(col) = self;
        write!(f,"{}",col)?;
        Ok(())
      }
    }
  )
}

impl From<bool> for MechString {
  fn from(n: bool) -> MechString {
    MechString{chars: format!("{:?}", n).chars().collect()}
  } 
}

impl From<TableId> for Value {
  fn from(n: TableId) -> Value {
    Value::Reference(n)
  } 
}

impl From<bool> for Value {
  fn from(n: bool) -> Value {
    Value::Bool(n)
  } 
}

macro_rules! pow_impl {
  ($t:ty) => {
    pow_impl!($t, u8);
    pow_impl!($t, usize);
  };
  ($t:ty, $rhs:ty) => {
    pow_impl!($t, $rhs, usize, pow);
  };
  ($t:tt, $rhs:tt, $rhs_t:tt, $method:expr) => {
    impl Pow<$rhs> for $t {
      type Output = $t;
      #[inline]
      fn pow(self, rhs: $rhs) -> $t {
        let ($t(lhs),$rhs(rhs)) = (self,rhs);
        $t(($method)(lhs, <u32 as From<$rhs_t>>::from(rhs)))
      }
    }

    impl<'a> Pow<&'a $rhs> for $t {
      type Output = $t;
      #[inline]
      fn pow(self, rhs: &'a $rhs) -> $t {
        let ($t(lhs),$rhs(rhs)) = (self,rhs);
        $t(($method)(lhs, <u32 as From<$rhs_t>>::from(*rhs)))
      }
    }

    impl<'a> Pow<$rhs> for &'a $t {
      type Output = $t;
      #[inline]
      fn pow(self, rhs: $rhs) -> $t {
        let ($t(lhs),$rhs(rhs)) = (self,rhs);
        $t(($method)(*lhs, <u32 as From<$rhs_t>>::from(rhs)))
      }
    }

    impl<'a, 'b> Pow<&'a $rhs> for &'b $t {
      type Output = $t;
      #[inline]
      fn pow(self, rhs: &'a $rhs) -> $t {
        let ($t(lhs),$rhs(rhs)) = (self,rhs);
        $t(($method)(*lhs, <u32 as From<$rhs_t>>::from(*rhs)))
      }
    }
  };
}

pow_impl!(U8, U8, u8, u8::pow);
pow_impl!(U8, U16, u16, u8::pow);
pow_impl!(U8, U32, u32, u8::pow);
pow_impl!(I8, U8, u8, i8::pow);
pow_impl!(I8, U16, u16, i8::pow);
pow_impl!(I8, U32, u32, i8::pow);
pow_impl!(U16, U8, u8, u16::pow);
pow_impl!(U16, U16, u16, u16::pow);
pow_impl!(U16, U32, u32, u16::pow);
pow_impl!(I16, U8, u8, i16::pow);
pow_impl!(I16, U16, u16, i16::pow);
pow_impl!(I16, U32, u32, i16::pow);
pow_impl!(U32, U8, u8, u32::pow);
pow_impl!(U32, U16, u16, u32::pow);
pow_impl!(U32, U32, u32, u32::pow);
pow_impl!(I32, U8, u8, i32::pow);
pow_impl!(I32, U16, u16, i32::pow);
pow_impl!(I32, U32, u32, i32::pow);
pow_impl!(I64, U8, u8, i64::pow);
pow_impl!(I64, U16, u16, i64::pow);
pow_impl!(I64, U32, u32, i64::pow);
pow_impl!(I128, U8, u8, i128::pow);
pow_impl!(I128, U16, u16, i128::pow);
pow_impl!(I128, U32, u32, i128::pow);

mech_powf!(F32,f32);


// These are just to get things compiling. We should
// to a better job implementing these.
mech_pow_dummy!(I8,I8);
mech_pow_dummy!(I16,I16);
mech_pow_dummy!(I32,I32);
mech_pow_dummy!(I64,I64);
mech_pow_dummy!(I128,I128);
mech_pow_dummy!(U128,U128);
mech_pow_dummy!(U64,U64);

// powf64 dummy
impl<T: Into<F64>> Pow<T> for F64 {
  type Output = F64;
  fn pow(self, rhs: T) -> F64 {
    let (F64(lhs),rhs) = (self,F64);
    F64(0.0)
  }
}

#[macro_export]
macro_rules! mech_pow_dummy{
  ($wrapper:tt,$rhs:tt) => (
    impl<T: Into<$rhs>> Pow<T> for $wrapper {
      type Output = $wrapper;
      fn pow(self, rhs: T) -> $wrapper {
        let ($wrapper(lhs),rhs) = (self,rhs);
        $wrapper(0)
      }
    }
  )
}

#[macro_export]
macro_rules! mech_powf{
  ($wrapper:tt,$rhs:tt) => (
    impl<T: Into<$rhs>> Pow<T> for $wrapper {
      type Output = $wrapper;
      fn pow(self, rhs: T) -> $wrapper {
        let ($wrapper(lhs),rhs) = (self,rhs);
        $wrapper(lhs.powf(T::into(rhs)))
      }
    }
  )
}

#[macro_export]
macro_rules! mech_neg {
  ($wrapper:tt) => (
    impl Neg for $wrapper {
      type Output = $wrapper;
      fn neg(self) -> $wrapper {
        let $wrapper(val) = self;
        $wrapper(-val)
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

#[macro_export]
macro_rules! mech_type_conversion_raw {
  ($from_wrapper:tt,$to_type:tt) => (
    impl From<$from_wrapper> for $to_type {
      fn from(n: $from_wrapper) -> $to_type {
        let $from_wrapper(c) = n;
        c as $to_type
      } 
    }
  )
}

#[macro_export]
macro_rules! mech_value_conversion {
  ($from_wrapper:tt,$to_type:tt) => (
    impl From<$from_wrapper> for Value {
      fn from(n: $from_wrapper) -> Value {
        Value::$to_type(n)
      } 
    }
  )
}