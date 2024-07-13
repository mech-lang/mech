use mech_core::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use crate::parser2::*;
use mech_core::nodes::Matrix as Mat;
use serde_derive::*;
use std::any::Any;
use hashbrown::{HashMap, HashSet};
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign, Neg};
use num_traits::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;
use std::fmt;
use std::cmp::PartialOrd;
use simba::scalar::ClosedNeg;
use tabled::{
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use tabled::{settings::style::LineText};
use std::fmt::Debug;
use paste::paste;

type Ref<T> = Rc<RefCell<T>>;
pub fn new_ref<T>(item: T) -> Rc<RefCell<T>> {
  Rc::new(RefCell::new(item))
}

type MResult<T> = Result<T,MechError>;

// Value ----------------------------------------------------------------------

#[derive(PartialEq, Debug, Clone, Copy, PartialOrd)]
pub struct F64(f64);
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}
impl Eq for F64 {}
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}
impl Add for F64 {
  type Output = F64;
  fn add(self, other: F64) -> F64 {
    F64(self.0 + other.0)
  }
}
impl AddAssign for F64 {
  fn add_assign(&mut self, other: F64) {
    self.0 += other.0;
  }
}
impl Sub for F64 {
  type Output = F64;
  fn sub(self, other: F64) -> F64 {
    F64(self.0 - other.0)
  }
}
impl SubAssign for F64 {
  fn sub_assign(&mut self, other: F64) {
    self.0 -= other.0;
  }
}
impl Mul for F64 {
  type Output = F64;
  fn mul(self, other: F64) -> F64 {
    F64(self.0 * other.0)
  }
}
impl MulAssign for F64 {
  fn mul_assign(&mut self, other: F64) {
    self.0 *= other.0;
  }
}
impl Div for F64 {
  type Output = F64;
  fn div(self, other: F64) -> F64 {
    F64(self.0 / other.0)
  }
}
impl DivAssign for F64 {
  fn div_assign(&mut self, other: F64) {
    self.0 /= other.0;
  }
}
impl Zero for F64 {
  fn zero() -> Self {
    F64(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}
impl One for F64 {
  fn one() -> Self {
    F64(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}
impl Neg for F64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F64(-self.0)
  }
}

#[derive(PartialEq, Debug, Clone, Copy, PartialOrd)]
pub struct F32(f32);
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
  }
}
impl Eq for F32 {}
impl Hash for F32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}
impl Add for F32 {
  type Output = F32;
  fn add(self, other: F32) -> F32 {
    F32(self.0 + other.0)
  }
}
impl AddAssign for F32 {
  fn add_assign(&mut self, other: F32) {
    self.0 += other.0;
  }
}
impl Zero for F32 {
  fn zero() -> Self {
    F32(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}
impl One for F32 {
  fn one() -> Self {
    F32(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}
impl Sub for F32 {
  type Output = F32;
  fn sub(self, other: F32) -> F32 {
    F32(self.0 - other.0)
  }
}
impl SubAssign for F32 {
  fn sub_assign(&mut self, other: F32) {
    self.0 -= other.0;
  }
}
impl Mul for F32 {
  type Output = F32;
  fn mul(self, other: F32) -> F32 {
    F32(self.0 * other.0)
  }
}
impl MulAssign for F32 {
  fn mul_assign(&mut self, other: F32) {
    self.0 *= other.0;
  }
}
impl Div for F32 {
  type Output = F32;
  fn div(self, other: F32) -> F32 {
    F32(self.0 / other.0)
  }
}
impl DivAssign for F32 {
  fn div_assign(&mut self, other: F32) {
    self.0 /= other.0;
  }
}
impl Neg for F32 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F32(-self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ValueKind {
  U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64, 
  String, Bool, Matrix(Box<ValueKind>,Vec<usize>), Set, Map, Record, Table, Tuple, Id, Reference, Atom(u64), Empty
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  U8(Ref<u8>),
  U16(Ref<u16>),
  U32(Ref<u32>),
  U64(Ref<u64>),
  U128(Ref<u128>),
  I8(Ref<i8>),
  I16(Ref<i16>),
  I32(Ref<i32>),
  I64(Ref<i64>),
  I128(Ref<i128>),
  F32(Ref<F32>),
  F64(Ref<F64>),
  String(String),
  Bool(Ref<bool>),
  Atom(u64),
  MatrixBool(Matrix<bool>),
  MatrixU8(Matrix<u8>),
  MatrixU16(Matrix<u16>),
  MatrixU32(Matrix<u32>),
  MatrixU64(Matrix<u64>),
  MatrixU128(Matrix<u128>),
  MatrixI8(Matrix<i8>),
  MatrixI16(Matrix<i16>),
  MatrixI32(Matrix<i32>),
  MatrixI64(Matrix<i64>),
  MatrixI128(Matrix<i128>),
  MatrixF32(Matrix<F32>),
  MatrixF64(Matrix<F64>),
  Set(MechSet),
  Map(MechMap),
  Record(MechMap),
  Table(MechTable),
  Tuple(MechTuple),
  Id(u64),
  MutableReference(MutableReference),
  Kind(ValueKind),
  Empty
}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Value::U8(x) => x.borrow().hash(state),
      Value::U16(x) => x.borrow().hash(state),
      Value::U32(x) => x.borrow().hash(state),
      Value::U64(x) => x.borrow().hash(state),
      Value::U128(x) => x.borrow().hash(state),
      Value::I8(x) => x.borrow().hash(state),
      Value::I16(x) => x.borrow().hash(state),
      Value::I32(x) => x.borrow().hash(state),
      Value::I64(x) => x.borrow().hash(state),
      Value::I128(x) => x.borrow().hash(state),
      Value::F32(x) => x.borrow().hash(state),
      Value::F64(x) => x.borrow().hash(state),
      Value::String(x) => x.hash(state),
      Value::Bool(x) => x.borrow().hash(state),
      Value::Atom(x) => x.hash(state),
      Value::MatrixBool(x) => x.hash(state),
      Value::MatrixU8(x) => x.hash(state),
      Value::MatrixU16(x) => x.hash(state),
      Value::MatrixU32(x) => x.hash(state),
      Value::MatrixU64(x) => x.hash(state),
      Value::MatrixU128(x) => x.hash(state),
      Value::MatrixI8(x) => x.hash(state),
      Value::MatrixI16(x) => x.hash(state),
      Value::MatrixI32(x) => x.hash(state),
      Value::MatrixI64(x) => x.hash(state),
      Value::MatrixI128(x) => x.hash(state),
      Value::MatrixF32(x) => x.hash(state),
      Value::MatrixF64(x) => x.hash(state),
      Value::Set(x) => x.hash(state),
      Value::Map(x) => x.hash(state),
      Value::Record(x) => x.hash(state),
      Value::Table(x) => x.hash(state),
      Value::Tuple(x) => x.hash(state),
      Value::Id(x) => x.hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Kind(x) => x.hash(state),
      Value::Empty => Value::Empty.hash(state),
    }
  }
}

impl Value {
  pub fn shape(&self) -> Vec<usize> {
    match self {
      Value::U8(x) => vec![1,1],
      Value::U16(x) => vec![1,1],
      Value::U32(x) => vec![1,1],
      Value::U64(x) => vec![1,1],
      Value::U128(x) => vec![1,1],
      Value::I8(x) => vec![1,1],
      Value::I16(x) => vec![1,1],
      Value::I32(x) => vec![1,1],
      Value::I64(x) => vec![1,1],
      Value::I128(x) => vec![1,1],
      Value::F32(x) => vec![1,1],
      Value::F64(x) => vec![1,1],
      Value::String(x) => vec![1,1],
      Value::Bool(x) => vec![1,1],
      Value::Atom(x) => vec![1,1],
      Value::MatrixBool(x) => x.shape(),
      Value::MatrixU8(x) => x.shape(),
      Value::MatrixU16(x) => x.shape(),
      Value::MatrixU32(x) => x.shape(),
      Value::MatrixU64(x) => x.shape(),
      Value::MatrixU128(x) => x.shape(),
      Value::MatrixI8(x) => x.shape(),
      Value::MatrixI16(x) => x.shape(),
      Value::MatrixI32(x) => x.shape(),
      Value::MatrixI64(x) => x.shape(),
      Value::MatrixI128(x) => x.shape(),
      Value::MatrixF32(x) => x.shape(),
      Value::MatrixF64(x) => x.shape(),
      Value::Table(x) => x.shape(),
      Value::Set(x) => vec![1,x.set.len()],
      Value::Map(x) => vec![1,x.map.len()],
      Value::Record(x) => vec![1,x.map.len()],
      Value::Tuple(x) => vec![1,x.size()],
      Value::MutableReference(x) => vec![1,1],
      Value::Empty => vec![0,0],
      Value::Kind(_) => vec![0,0],
      Value::Id(x) => vec![0,0],
    }
  }

  pub fn kind(&self) -> ValueKind {
    match self {
      Value::U8(_) => ValueKind::U8,
      Value::U16(_) => ValueKind::U16,
      Value::U32(_) => ValueKind::U32,
      Value::U64(_) => ValueKind::U64,
      Value::U128(_) => ValueKind::U128,
      Value::I8(_) => ValueKind::I8,
      Value::I16(_) => ValueKind::I16,
      Value::I32(_) => ValueKind::I32,
      Value::I64(_) => ValueKind::I64,
      Value::I128(_) => ValueKind::I128,
      Value::F32(_) => ValueKind::F32,
      Value::F64(_) => ValueKind::F64,
      Value::String(_) => ValueKind::String,
      Value::Bool(_) => ValueKind::Bool,
      Value::Atom(x) => ValueKind::Atom(*x),
      Value::MatrixBool(x) => ValueKind::Matrix(Box::new(ValueKind::Bool),x.shape()),
      Value::MatrixU8(x) => ValueKind::Matrix(Box::new(ValueKind::U8),x.shape()),
      Value::MatrixU16(x) => ValueKind::Matrix(Box::new(ValueKind::U16),x.shape()),
      Value::MatrixU32(x) => ValueKind::Matrix(Box::new(ValueKind::U32),x.shape()),
      Value::MatrixU64(x) => ValueKind::Matrix(Box::new(ValueKind::U64),x.shape()),
      Value::MatrixU128(x) => ValueKind::Matrix(Box::new(ValueKind::U128),x.shape()),
      Value::MatrixI8(x) => ValueKind::Matrix(Box::new(ValueKind::I8),x.shape()),
      Value::MatrixI16(x) => ValueKind::Matrix(Box::new(ValueKind::I16),x.shape()),
      Value::MatrixI32(x) => ValueKind::Matrix(Box::new(ValueKind::I32),x.shape()),
      Value::MatrixI64(x) => ValueKind::Matrix(Box::new(ValueKind::I64),x.shape()),
      Value::MatrixI128(x) => ValueKind::Matrix(Box::new(ValueKind::U128,),x.shape()),
      Value::MatrixF32(x) => ValueKind::Matrix(Box::new(ValueKind::F32),x.shape()),
      Value::MatrixF64(x) => ValueKind::Matrix(Box::new(ValueKind::F64),x.shape()),
      Value::Table(x) => ValueKind::Table,
      Value::Set(x) => ValueKind::Set,
      Value::Map(x) => ValueKind::Map,
      Value::Record(x) => ValueKind::Record,
      Value::Tuple(x) => ValueKind::Tuple,
      Value::MutableReference(x) => ValueKind::Reference,
      Value::Empty => ValueKind::Empty,
      Value::Id(x) => ValueKind::Id,
      Value::Kind(x) => x.clone(),
    }
  }

  fn as_u8(&self) -> Option<Ref<u8>> {if let Value::U8(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u8() } else { None }}
  fn as_u16(&self) -> Option<Ref<u16>> {if let Value::U16(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u16() } else { None }}
  fn as_u32(&self) -> Option<Ref<u32>> {if let Value::U32(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u32() } else { None }}
  fn as_u64(&self) -> Option<Ref<u64>> {if let Value::U64(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u64() } else { None }}
  fn as_u128(&self) -> Option<Ref<u128>> {if let Value::U128(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u128() } else { None }}
  fn as_i8(&self) -> Option<Ref<i8>> {if let Value::I8(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i8() } else { None }}
  fn as_i16(&self) -> Option<Ref<i16>> {if let Value::I16(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i16() } else { None }}
  fn as_i32(&self) -> Option<Ref<i32>> {if let Value::I32(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i32() } else { None }}
  fn as_i64(&self) -> Option<Ref<i64>> {if let Value::I64(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i64() } else { None }}
  fn as_i128(&self) -> Option<Ref<i128>> {if let Value::I128(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i128() } else { None }}
  fn as_f32(&self) -> Option<Ref<f32>> {if let Value::F32(v) = self { Some(new_ref(v.borrow().0)) } else if let Value::MutableReference(val) = self { val.borrow().as_f32() } else { None }}
  fn as_f64(&self) -> Option<Ref<f64>> {if let Value::F64(v) = self { Some(new_ref(v.borrow().0)) } else if let Value::MutableReference(val) = self { val.borrow().as_f64() } else { None }}
  fn as_vecf64(&self) -> Option<Vec<F64>> {if let Value::MatrixF64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf64() } else { None }}
  fn as_vecf32(&self) -> Option<Vec<F32>> {if let Value::MatrixF32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf32() } else { None }}
  fn as_vecbool(&self) -> Option<Vec<bool>> {if let Value::MatrixBool(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecbool() } else { None }}
  fn as_vecu8(&self) -> Option<Vec<u8>> {if let Value::MatrixU8(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu8() } else { None }}
  fn as_vecu16(&self) -> Option<Vec<u16>> {if let Value::MatrixU16(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu16() } else { None }}
  fn as_vecu32(&self) -> Option<Vec<u32>> {if let Value::MatrixU32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu32() } else { None }}
  fn as_vecu64(&self) -> Option<Vec<u64>> {if let Value::MatrixU64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu64() } else { None }}
  fn as_vecu128(&self) -> Option<Vec<u128>> {if let Value::MatrixU128(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu128() } else { None }}
  fn as_veci8(&self) -> Option<Vec<i8>> {if let Value::MatrixI8(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci8() } else { None }}
  fn as_veci16(&self) -> Option<Vec<i16>> {if let Value::MatrixI16(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci16() } else { None }}
  fn as_veci32(&self) -> Option<Vec<i32>> {if let Value::MatrixI32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci32() } else { None }}
  fn as_veci64(&self) -> Option<Vec<i64>> {if let Value::MatrixI64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci64() } else { None }}
  fn as_veci128(&self) -> Option<Vec<i128>> {if let Value::MatrixI128(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci128() } else { None }}
  
  fn as_usize(&self) -> Option<usize> {
    match self {
      Value::U8(v) => Some(*v.borrow() as usize),
      Value::U16(v) => Some(*v.borrow() as usize),
      Value::U32(v) => Some(*v.borrow() as usize),
      Value::U64(v) => Some(*v.borrow() as usize),
      Value::U128(v) => Some(*v.borrow() as usize),
      Value::I8(v) => Some(*v.borrow() as usize),
      Value::I16(v) => Some(*v.borrow() as usize),
      Value::I32(v) => Some(*v.borrow() as usize),
      Value::I64(v) => Some(*v.borrow() as usize),
      Value::I128(v) => Some(*v.borrow() as usize),
      Value::F32(v) => Some((*v.borrow()).0 as usize),
      Value::F64(v) => Some((*v.borrow()).0 as usize),
      _ => None,
    }
  }

}

pub trait ToValue {
  fn to_value(&self) -> Value;
}

impl ToValue for Ref<u8> { fn to_value(&self) -> Value { Value::U8(self.clone()) } }
impl ToValue for Ref<u16> { fn to_value(&self) -> Value { Value::U16(self.clone()) } }
impl ToValue for Ref<u32> { fn to_value(&self) -> Value { Value::U32(self.clone()) } }
impl ToValue for Ref<u64> { fn to_value(&self) -> Value { Value::U64(self.clone()) } }
impl ToValue for Ref<u128> { fn to_value(&self) -> Value { Value::U128(self.clone()) } }
impl ToValue for Ref<i8> { fn to_value(&self) -> Value { Value::I8(self.clone()) } }
impl ToValue for Ref<i16> { fn to_value(&self) -> Value { Value::I16(self.clone()) } }
impl ToValue for Ref<i32> { fn to_value(&self) -> Value { Value::I32(self.clone()) } }
impl ToValue for Ref<i64> { fn to_value(&self) -> Value { Value::I64(self.clone()) } }
impl ToValue for Ref<i128> { fn to_value(&self) -> Value { Value::I128(self.clone()) } }
impl ToValue for Ref<F32> { fn to_value(&self) -> Value { Value::F32(self.clone()) } }
impl ToValue for Ref<F64> { fn to_value(&self) -> Value { Value::F64(self.clone()) } }
impl ToValue for Ref<bool> { fn to_value(&self) -> Value { Value::Bool(self.clone()) } }

macro_rules! impl_to_value_matrix {
  ($($nd_matrix_kind:ident, $matrix_kind:ident, $base_type:ty),+ $(,)?) => {
    $(
      impl ToValue for Ref<$nd_matrix_kind<$base_type>> {
        fn to_value(&self) -> Value {
          Value::$matrix_kind(Matrix::<$base_type>::$nd_matrix_kind(self.clone()))
        }
      }
    )+
  };
}

impl_to_value_matrix!(

  Matrix2x3, MatrixBool, bool,
  Matrix2x3, MatrixI8, i8,
  Matrix2x3, MatrixI16, i16,
  Matrix2x3, MatrixI32, i32,
  Matrix2x3, MatrixI64, i64,
  Matrix2x3, MatrixI128, i128,
  Matrix2x3, MatrixU8, u8,
  Matrix2x3, MatrixU16, u16,
  Matrix2x3, MatrixU32, u32,
  Matrix2x3, MatrixU64, u64,
  Matrix2x3, MatrixU128, u128,
  Matrix2x3, MatrixF32, F32,
  Matrix2x3, MatrixF64, F64,

  Matrix3x2, MatrixBool, bool,
  Matrix3x2, MatrixI8, i8,
  Matrix3x2, MatrixI16, i16,
  Matrix3x2, MatrixI32, i32,
  Matrix3x2, MatrixI64, i64,
  Matrix3x2, MatrixI128, i128,
  Matrix3x2, MatrixU8, u8,
  Matrix3x2, MatrixU16, u16,
  Matrix3x2, MatrixU32, u32,
  Matrix3x2, MatrixU64, u64,
  Matrix3x2, MatrixU128, u128,
  Matrix3x2, MatrixF32, F32,
  Matrix3x2, MatrixF64, F64,

  Matrix1, MatrixBool, bool,
  Matrix1, MatrixI8, i8,
  Matrix1, MatrixI16, i16,
  Matrix1, MatrixI32, i32,
  Matrix1, MatrixI64, i64,
  Matrix1, MatrixI128, i128,
  Matrix1, MatrixU8, u8,
  Matrix1, MatrixU16, u16,
  Matrix1, MatrixU32, u32,
  Matrix1, MatrixU64, u64,
  Matrix1, MatrixU128, u128,
  Matrix1, MatrixF32, F32,
  Matrix1, MatrixF64, F64,

  Matrix2, MatrixBool, bool,
  Matrix2, MatrixI8, i8,
  Matrix2, MatrixI16, i16,
  Matrix2, MatrixI32, i32,
  Matrix2, MatrixI64, i64,
  Matrix2, MatrixI128, i128,
  Matrix2, MatrixU8, u8,
  Matrix2, MatrixU16, u16,
  Matrix2, MatrixU32, u32,
  Matrix2, MatrixU64, u64,
  Matrix2, MatrixU128, u128,
  Matrix2, MatrixF32, F32,
  Matrix2, MatrixF64, F64,

  Matrix3, MatrixBool, bool,
  Matrix3, MatrixI8, i8,
  Matrix3, MatrixI16, i16,
  Matrix3, MatrixI32, i32,
  Matrix3, MatrixI64, i64,
  Matrix3, MatrixI128, i128,
  Matrix3, MatrixU8, u8,
  Matrix3, MatrixU16, u16,
  Matrix3, MatrixU32, u32,
  Matrix3, MatrixU64, u64,
  Matrix3, MatrixU128, u128,
  Matrix3, MatrixF32, F32,
  Matrix3, MatrixF64, F64,

  Matrix4, MatrixBool, bool,
  Matrix4, MatrixI8, i8,
  Matrix4, MatrixI16, i16,
  Matrix4, MatrixI32, i32,
  Matrix4, MatrixI64, i64,
  Matrix4, MatrixI128, i128,
  Matrix4, MatrixU8, u8,
  Matrix4, MatrixU16, u16,
  Matrix4, MatrixU32, u32,
  Matrix4, MatrixU64, u64,
  Matrix4, MatrixU128, u128,
  Matrix4, MatrixF32, F32,
  Matrix4, MatrixF64, F64,

  Vector2, MatrixBool, bool,
  Vector2, MatrixI8, i8,
  Vector2, MatrixI16, i16,
  Vector2, MatrixI32, i32,
  Vector2, MatrixI64, i64,
  Vector2, MatrixI128, i128,
  Vector2, MatrixU8, u8,
  Vector2, MatrixU16, u16,
  Vector2, MatrixU32, u32,
  Vector2, MatrixU64, u64,
  Vector2, MatrixU128, u128,
  Vector2, MatrixF32, F32,
  Vector2, MatrixF64, F64,

  Vector3, MatrixBool, bool,
  Vector3, MatrixI8, i8,
  Vector3, MatrixI16, i16,
  Vector3, MatrixI32, i32,
  Vector3, MatrixI64, i64,
  Vector3, MatrixI128, i128,
  Vector3, MatrixU8, u8,
  Vector3, MatrixU16, u16,
  Vector3, MatrixU32, u32,
  Vector3, MatrixU64, u64,
  Vector3, MatrixU128, u128,
  Vector3, MatrixF32, F32,
  Vector3, MatrixF64, F64,

  Vector4, MatrixBool, bool,
  Vector4, MatrixI8, i8,
  Vector4, MatrixI16, i16,
  Vector4, MatrixI32, i32,
  Vector4, MatrixI64, i64,
  Vector4, MatrixI128, i128,
  Vector4, MatrixU8, u8,
  Vector4, MatrixU16, u16,
  Vector4, MatrixU32, u32,
  Vector4, MatrixU64, u64,
  Vector4, MatrixU128, u128,
  Vector4, MatrixF32, F32,
  Vector4, MatrixF64, F64,

  RowVector2, MatrixBool, bool,
  RowVector2, MatrixI8, i8,
  RowVector2, MatrixI16, i16,
  RowVector2, MatrixI32, i32,
  RowVector2, MatrixI64, i64,
  RowVector2, MatrixI128, i128,
  RowVector2, MatrixU8, u8,
  RowVector2, MatrixU16, u16,
  RowVector2, MatrixU32, u32,
  RowVector2, MatrixU64, u64,
  RowVector2, MatrixU128, u128,
  RowVector2, MatrixF32, F32,
  RowVector2, MatrixF64, F64,
  
  RowVector3, MatrixBool, bool,
  RowVector3, MatrixI8, i8,
  RowVector3, MatrixI16, i16,
  RowVector3, MatrixI32, i32,
  RowVector3, MatrixI64, i64,
  RowVector3, MatrixI128, i128,
  RowVector3, MatrixU8, u8,
  RowVector3, MatrixU16, u16,
  RowVector3, MatrixU32, u32,
  RowVector3, MatrixU64, u64,
  RowVector3, MatrixU128, u128,
  RowVector3, MatrixF32, F32,
  RowVector3, MatrixF64, F64,

  RowVector4, MatrixBool, bool,
  RowVector4, MatrixI8, i8,
  RowVector4, MatrixI16, i16,
  RowVector4, MatrixI32, i32,
  RowVector4, MatrixI64, i64,
  RowVector4, MatrixI128, i128,
  RowVector4, MatrixU8, u8,
  RowVector4, MatrixU16, u16,
  RowVector4, MatrixU32, u32,
  RowVector4, MatrixU64, u64,
  RowVector4, MatrixU128, u128,
  RowVector4, MatrixF32, F32,
  RowVector4, MatrixF64, F64,

  RowDVector, MatrixBool, bool,
  RowDVector, MatrixI8, i8,
  RowDVector, MatrixI16, i16,
  RowDVector, MatrixI32, i32,
  RowDVector, MatrixI64, i64,
  RowDVector, MatrixI128, i128,
  RowDVector, MatrixU8, u8,
  RowDVector, MatrixU16, u16,
  RowDVector, MatrixU32, u32,
  RowDVector, MatrixU64, u64,
  RowDVector, MatrixU128, u128,
  RowDVector, MatrixF32, F32,
  RowDVector, MatrixF64, F64,

  DVector, MatrixBool, bool,
  DVector, MatrixI8, i8,
  DVector, MatrixI16, i16,
  DVector, MatrixI32, i32,
  DVector, MatrixI64, i64,
  DVector, MatrixI128, i128,
  DVector, MatrixU8, u8,
  DVector, MatrixU16, u16,
  DVector, MatrixU32, u32,
  DVector, MatrixU64, u64,
  DVector, MatrixU128, u128,
  DVector, MatrixF32, F32,
  DVector, MatrixF64, F64,

  DMatrix, MatrixBool, bool,
  DMatrix, MatrixI8, i8,
  DMatrix, MatrixI16, i16,
  DMatrix, MatrixI32, i32,
  DMatrix, MatrixI64, i64,
  DMatrix, MatrixI128, i128,
  DMatrix, MatrixU8, u8,
  DMatrix, MatrixU16, u16,
  DMatrix, MatrixU32, u32,
  DMatrix, MatrixU64, u64,
  DMatrix, MatrixU128, u128,
  DMatrix, MatrixF32, F32,
  DMatrix, MatrixF64, F64,
);

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Scalar(u64),
  Matrix(Box<Kind>,Vec<usize>),
  Tuple,
  Brace,
  Map,
  Atom,
  Function,
  Fsm,
  Empty,
}

impl Kind {

  fn to_value_kind(&self, functions: FunctionsRef) -> MResult<ValueKind> {
    match self {
      Kind::Scalar(id) => {
        match functions.borrow().kinds.get(id).cloned() {
          Some(val_knd) => Ok(val_knd),
          None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(*id)}),
        }
      },
      Kind::Matrix(knd,size) => {
        let val_knd = knd.to_value_kind(functions.clone())?;
        Ok(ValueKind::Matrix(Box::new(val_knd),size.clone()))
      },
      Kind::Tuple => todo!(),
      Kind::Brace => todo!(),
      Kind::Map => todo!(),
      Kind::Atom => todo!(),
      Kind::Function => todo!(),
      Kind::Fsm => todo!(),
      Kind::Empty => Ok(ValueKind::Empty),
    }
  }
}

//-----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  set: IndexSet<Value>,
}

impl MechSet {
  pub fn from_vec(vec: Vec<Value>) -> MechSet {
    let mut set = IndexSet::new();
    for v in vec {
      set.insert(v);
    }
    MechSet{set}
  }
}

impl Hash for MechSet {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.set.iter() {
      x.hash(state)
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechMap {
  map: IndexMap<Value,Value>,
}

impl MechMap {
  pub fn from_vec(vec: Vec<(Value,Value)>) -> MechMap {
    let mut map = IndexMap::new();
    for (k,v) in vec {
      map.insert(k,v);
    }
    MechMap{map}
  }
}

impl Hash for MechMap {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.map.iter() {
      x.hash(state)
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTable {
  rows: usize,
  cols: usize,
  data: IndexMap<Value,Vec<Value>>,
}

impl MechTable {
  pub fn shape(&self) -> Vec<usize> {
    vec![self.rows,self.cols]
  }
}

impl Hash for MechTable {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,v) in self.data.iter() {
      k.hash(state);
      v.hash(state);
    }
  }
}

pub struct Functions {
  pub functions: HashMap<u64,FunctionDefinition>,
  pub function_compilers: HashMap<u64, Box<dyn NativeFunctionCompiler>>,
  pub kinds: HashMap<u64,ValueKind>,
}

impl Functions {
  pub fn new() -> Self {
    Self {functions: HashMap::new(), function_compilers: HashMap::new(), kinds: HashMap::new()}
  }
}


type FunctionsRef = Ref<Functions>;
type Plan = Ref<Vec<Box<dyn MechFunction>>>;
type MutableReference = Ref<Value>;
type SymbolTableRef= Ref<SymbolTable>;
type ValRef = Ref<Value>;

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub reverse_lookup: HashMap<*const RefCell<Value>, u64>,
}

impl SymbolTable {

  pub fn new() -> SymbolTable {
    Self {
      symbols: HashMap::new(),
      reverse_lookup: HashMap::new(),
    }
  }

  pub fn get(&self, key: u64) -> Option<ValRef> {
    self.symbols.get(&key).cloned()
  }

  pub fn insert(&mut self, key: u64, value: Value) -> ValRef {
    let cell = new_ref(value);
    self.reverse_lookup.insert(Rc::as_ptr(&cell), key);
    self.symbols.insert(key,cell.clone());
    cell.clone()
  }
}


#[derive(Clone)]
pub struct FunctionDefinition {
  pub code: FunctionDefine,
  pub id: u64,
  pub name: String,
  pub input: IndexMap<u64, KindAnnotation>,
  pub output: IndexMap<u64, KindAnnotation>,
  pub symbols: SymbolTableRef,
  pub out: Ref<Value>,
  pub plan: Plan,
}

impl fmt::Debug for FunctionDefinition {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let input_str = format!("{:#?}", self.input);
    let output_str = format!("{:#?}", self.output);
    let symbols_str = format!("{:#?}", self.symbols);
    let mut plan_str = "".to_string();
    for step in self.plan.borrow().iter() {
      plan_str = format!("{}  - {}\n",plan_str,step.to_string());
    }
    let data = vec!["📥 Input", &input_str, 
                    "📤 Output", &output_str, 
                    "🔣 Symbols",   &symbols_str,
                    "📋 Plan", &plan_str];
    let mut table = tabled::Table::new(data);
    table
        .with(Style::modern())
        .with(Panel::header(format!("📈 UserFxn::{}\n({})", self.name, humanize(&self.id))))
        .with(Alignment::left());
    println!("{table}");
    Ok(())
  }
}

impl FunctionDefinition {

  pub fn new(id: u64, name: String, code: FunctionDefine) -> Self {
    Self {
      id,
      name,
      code,
      input: IndexMap::new(),
      output: IndexMap::new(),
      out: new_ref(Value::Empty),
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
    }
  }

  pub fn recompile(&self, functions: FunctionsRef) -> MResult<FunctionDefinition> {
    function_define(&self.code, functions.clone())
  }

  pub fn solve(&self) -> ValRef {
    let plan_brrw = self.plan.borrow();
    for step in plan_brrw.iter() {
      let result = step.solve();
    }
    self.out.clone()
  }

  pub fn out(&self) -> ValRef {
    self.out.clone()
  }


}


// Matrix ---------------------------------------------------------------------

trait ToMatrix: Clone {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self>;
}

macro_rules! impl_to_matrix {
  ($t:ty) => {
    impl ToMatrix for $t {
      fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
        match (rows,cols) {
          (1,1) => Matrix::Matrix1(new_ref(Matrix1::from_element(elements[0].clone()))),
          (2,2) => Matrix::Matrix2(new_ref(Matrix2::from_vec(elements))),
          (3,4) => Matrix::Matrix3(new_ref(Matrix3::from_vec(elements))),
          (4,2) => Matrix::Matrix4(new_ref(Matrix4::from_vec(elements))),
          (2,3) => Matrix::Matrix2x3(new_ref(Matrix2x3::from_vec(elements))),
          (3,2) => Matrix::Matrix3x2(new_ref(Matrix3x2::from_vec(elements))),
          (1,2) => Matrix::RowVector2(new_ref(RowVector2::from_vec(elements))),
          (1,3) => Matrix::RowVector3(new_ref(RowVector3::from_vec(elements))),
          (1,4) => Matrix::RowVector4(new_ref(RowVector4::from_vec(elements))),
          (2,1) => Matrix::Vector2(new_ref(Vector2::from_vec(elements))),
          (3,1) => Matrix::Vector3(new_ref(Vector3::from_vec(elements))),
          (4,1) => Matrix::Vector4(new_ref(Vector4::from_vec(elements))),
          (1,n) => Matrix::RowDVector(new_ref(RowDVector::from_vec(elements))),
          (m,1) => Matrix::DVector(new_ref(DVector::from_vec(elements))),
          (m,n) => Matrix::DMatrix(new_ref(DMatrix::from_vec(m,n,elements))),
        }
      }
    }
  };
}

impl_to_matrix!(u8);
impl_to_matrix!(u16);
impl_to_matrix!(u32);
impl_to_matrix!(u64);
impl_to_matrix!(u128);
impl_to_matrix!(i8);
impl_to_matrix!(i16);
impl_to_matrix!(i32);
impl_to_matrix!(i64);
impl_to_matrix!(i128);
impl_to_matrix!(F32);
impl_to_matrix!(F64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix<T> {
  RowVector2(Ref<RowVector2<T>>),
  RowVector3(Ref<RowVector3<T>>),
  RowVector4(Ref<RowVector4<T>>),
  Vector2(Ref<Vector2<T>>),
  Vector3(Ref<Vector3<T>>),
  Vector4(Ref<Vector4<T>>),  
  Matrix1(Ref<Matrix1<T>>),
  Matrix2(Ref<Matrix2<T>>),
  Matrix3(Ref<Matrix3<T>>),
  Matrix4(Ref<Matrix4<T>>),
  Matrix2x3(Ref<Matrix2x3<T>>),
  Matrix3x2(Ref<Matrix3x2<T>>),
  DMatrix(Ref<DMatrix<T>>),
  DVector(Ref<DVector<T>>),
  RowDVector(Ref<RowDVector<T>>),
}

impl<T> Hash for Matrix<T> 
where T: Hash + na::Scalar
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Matrix::RowVector2(x) => x.borrow().hash(state),
      Matrix::RowVector3(x) => x.borrow().hash(state),
      Matrix::RowVector4(x) => x.borrow().hash(state),
      Matrix::Vector2(x) => x.borrow().hash(state),
      Matrix::Vector3(x) => x.borrow().hash(state),
      Matrix::Vector4(x) => x.borrow().hash(state),
      Matrix::Matrix1(x) => x.borrow().hash(state),
      Matrix::Matrix2(x) => x.borrow().hash(state),
      Matrix::Matrix3(x) => x.borrow().hash(state),
      Matrix::Matrix4(x) => x.borrow().hash(state),
      Matrix::Matrix2x3(x) => x.borrow().hash(state),
      Matrix::Matrix3x2(x) => x.borrow().hash(state),
      Matrix::DMatrix(x) => x.borrow().hash(state),
      Matrix::RowDVector(x) => x.borrow().hash(state),
      Matrix::DVector(x) => x.borrow().hash(state),
    }
  }
}

impl<T> Matrix<T> 
where T: Debug + Clone + Copy + PartialEq + 'static
{

  pub fn shape(&self) -> Vec<usize> {
    let shape = match self {
      Matrix::RowVector2(x) => x.borrow().shape(),
      Matrix::RowVector3(x) => x.borrow().shape(),
      Matrix::RowVector4(x) => x.borrow().shape(),
      Matrix::Vector2(x) => x.borrow().shape(),
      Matrix::Vector3(x) => x.borrow().shape(),
      Matrix::Vector4(x) => x.borrow().shape(),
      Matrix::Matrix1(x) => x.borrow().shape(),
      Matrix::Matrix2(x) => x.borrow().shape(),
      Matrix::Matrix3(x) => x.borrow().shape(),
      Matrix::Matrix4(x) => x.borrow().shape(),
      Matrix::Matrix2x3(x) => x.borrow().shape(),
      Matrix::Matrix3x2(x) => x.borrow().shape(),
      Matrix::DMatrix(x) => x.borrow().shape(),
      Matrix::RowDVector(x) => x.borrow().shape(),
      Matrix::DVector(x) => x.borrow().shape(),
    };
    vec![shape.0, shape.1]
  }

  pub fn index1d(&self, ix: usize) -> T {
    match self {
      Matrix::RowVector2(x) => *x.borrow().index(ix-1),
      Matrix::RowVector3(x) => *x.borrow().index(ix-1),
      Matrix::RowVector4(x) => *x.borrow().index(ix-1),
      Matrix::Vector2(x) => *x.borrow().index(ix-1),
      Matrix::Vector3(x) => *x.borrow().index(ix-1),
      Matrix::Vector4(x) => *x.borrow().index(ix-1),
      Matrix::Matrix1(x) => *x.borrow().index(ix-1),
      Matrix::Matrix2(x) => *x.borrow().index(ix-1),
      Matrix::Matrix3(x) => *x.borrow().index(ix-1),
      Matrix::Matrix4(x) => *x.borrow().index(ix-1),
      Matrix::Matrix2x3(x) => *x.borrow().index(ix-1),
      Matrix::Matrix3x2(x) => *x.borrow().index(ix-1),
      Matrix::DMatrix(x) => *x.borrow().index(ix-1),
      Matrix::RowDVector(x) => *x.borrow().index(ix-1),
      Matrix::DVector(x) => *x.borrow().index(ix-1),
    }
  }

  pub fn index2d(&self, row: usize, col: usize) -> T {
    match self {
      Matrix::RowVector2(x) => *x.borrow().index((row-1,col-1)),
      Matrix::RowVector3(x) => *x.borrow().index((row-1,col-1)),
      Matrix::RowVector4(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Vector2(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Vector3(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Vector4(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix1(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix2(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix3(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix4(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix2x3(x) => *x.borrow().index((row-1,col-1)),
      Matrix::Matrix3x2(x) => *x.borrow().index((row-1,col-1)),
      Matrix::DMatrix(x) => *x.borrow().index((row-1,col-1)),
      Matrix::RowDVector(x) => *x.borrow().index((row-1,col-1)),
      Matrix::DVector(x) => *x.borrow().index((row-1,col-1)),
    }
  }

  pub fn as_vec(&self) -> Vec<T> {
    match self {
      Matrix::RowVector2(x) => x.borrow().as_slice().to_vec(),
      Matrix::RowVector3(x) => x.borrow().as_slice().to_vec(),
      Matrix::RowVector4(x) => x.borrow().as_slice().to_vec(),
      Matrix::Vector2(x) => x.borrow().as_slice().to_vec(),
      Matrix::Vector3(x) => x.borrow().as_slice().to_vec(),
      Matrix::Vector4(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix1(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix2(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix3(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix4(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix2x3(x) => x.borrow().as_slice().to_vec(),
      Matrix::Matrix3x2(x) => x.borrow().as_slice().to_vec(),
      Matrix::DMatrix(x) => x.borrow().as_slice().to_vec(),
      Matrix::RowDVector(x) => x.borrow().as_slice().to_vec(),
      Matrix::DVector(x) => x.borrow().as_slice().to_vec(),
    }
  }

}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTuple {
  elements: Vec<Box<Value>>
}

impl MechTuple {

  pub fn from_vec(elements: Vec<Value>) -> Self {
    MechTuple{elements: elements.iter().map(|m| Box::new(m.clone())).collect::<Vec<Box<Value>>>()}
  }

  pub fn size(&self) -> usize {
    self.elements.len()
  }

}

impl Hash for MechTuple {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.elements.iter() {
        x.hash(state)
    }
  }
}


// Functions
// ------------------------------------------------------------------------

// The naming scheme will be OP LHS RHS
// The abbreviations are:
// Rv - row vector
// Cv - col vector
// MXY  - Matrix size X Y, or just X if it's square

pub trait MechFunction {
  fn solve(&self);
  fn out(&self) -> Value;
  fn to_string(&self) -> String;
}

// User Function --------------------------------------------------------------

#[derive(Debug)]
struct UserFunction {
  fxn: FunctionDefinition,
}

impl MechFunction for UserFunction {
  fn solve(&self) {
    self.fxn.solve();
  }
  fn out(&self) -> Value {
    Value::MutableReference(self.fxn.out.clone())
  }
  fn to_string(&self) -> String { format!("UserFxn::{:?}", self.fxn.name)}
}

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub symbols: SymbolTableRef,
  pub plan: Plan,
  pub functions: FunctionsRef,
}

impl Interpreter {
  pub fn new() -> Interpreter {
    
    // Preload functions
    let mut fxns = Functions::new();
    fxns.function_compilers.insert(hash_str("math/sin"),Box::new(MathSin{}));
    fxns.function_compilers.insert(hash_str("math/cos"),Box::new(MathCos{}));

    // Preload kinds
    fxns.kinds.insert(hash_str("u8"),ValueKind::U8);
    fxns.kinds.insert(hash_str("u16"),ValueKind::U16);
    fxns.kinds.insert(hash_str("u32"),ValueKind::U32);
    fxns.kinds.insert(hash_str("u64"),ValueKind::U64);
    fxns.kinds.insert(hash_str("u128"),ValueKind::U128);
    fxns.kinds.insert(hash_str("i8"),ValueKind::I8);
    fxns.kinds.insert(hash_str("i16"),ValueKind::I16);
    fxns.kinds.insert(hash_str("i32"),ValueKind::I32);
    fxns.kinds.insert(hash_str("i64"),ValueKind::I64);
    fxns.kinds.insert(hash_str("i128"),ValueKind::I128);
    fxns.kinds.insert(hash_str("f32"),ValueKind::F32);
    fxns.kinds.insert(hash_str("f64"),ValueKind::F64);
    fxns.kinds.insert(hash_str("string"),ValueKind::String);
    fxns.kinds.insert(hash_str("bool"),ValueKind::Bool);

    Interpreter {
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
      functions: new_ref(fxns),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    program(tree, self.plan.clone(), self.symbols.clone(), self.functions.clone())
  }
}

pub trait NativeFunctionCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>>;
}

//-----------------------------------------------------------------------------

fn program(program: &Program, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  body(&program.body, plan.clone(), symbols.clone(), functions.clone())
}

fn body(body: &Body, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for sec in &body.sections {
    result = Some(section(&sec, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section(section: &Section, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for el in &section.elements {
    result = Some(section_element(&el, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section_element(element: &SectionElement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let out = match element {
    SectionElement::MechCode(code) => {mech_code(&code, plan.clone(), symbols.clone(), functions.clone())?},
    SectionElement::Section(sctn) => todo!(),
    SectionElement::Comment(cmmnt) => Value::Empty,
    SectionElement::Paragraph(p) => Value::Empty,
    SectionElement::MechCode(code) => todo!(),
    SectionElement::UnorderedList(ul) => todo!(),
    SectionElement::CodeBlock => todo!(),
    SectionElement::OrderedList => todo!(),
    SectionElement::BlockQuote => todo!(),
    SectionElement::ThematicBreak => todo!(),
    SectionElement::Image => todo!(),
  };
  Ok(out)
}

fn mech_code(code: &MechCode, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &code {
    MechCode::Expression(expr) => expression(&expr, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::Statement(stmt) => statement(&stmt, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(fxn_def) => {
      let usr_fxn = function_define(&fxn_def, functions.clone())?;
      let mut fxns_brrw = functions.borrow_mut();
      fxns_brrw.functions.insert(usr_fxn.id, usr_fxn);
      Ok(Value::Empty)
    },
  }
}


fn function_define(fxn_def: &FunctionDefine, functions: FunctionsRef) -> MResult<FunctionDefinition> {
  let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id,fxn_def.name.to_string(), fxn_def.clone());
  for input_arg in &fxn_def.input {
    let arg_id = input_arg.name.hash();
    new_fxn.input.insert(arg_id,input_arg.kind.clone());
    let in_arg = Value::I64(new_ref(0));
    new_fxn.symbols.borrow_mut().insert(arg_id, in_arg);
  }
  let output_arg_ids = fxn_def.output.iter().map(|output_arg| {
    let arg_id = output_arg.name.hash();
    new_fxn.output.insert(arg_id,output_arg.kind.clone());
    arg_id
  }).collect::<Vec<u64>>();
  
  for stmnt in &fxn_def.statements {
    let result = statement(stmnt, new_fxn.plan.clone(), new_fxn.symbols.clone(), functions.clone());
  }    
  // get the output cell
  {
    let symbol_brrw = new_fxn.symbols.borrow();
    for arg_id in output_arg_ids {
      match symbol_brrw.get(arg_id) {
        Some(cell) => new_fxn.out = cell.clone(),
        None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::OutputUndefinedInFunctionBody(arg_id)});} 
      }
    }
  }
  Ok(new_fxn)
}

fn statement(stmt: &Statement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match stmt {
    Statement::VariableDefine(var_def) => variable_define(&var_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::VariableAssign(_) => todo!(),
    Statement::KindDefine(_) => todo!(),
    Statement::EnumDefine(_) => todo!(),
    Statement::FsmDeclare(_) => todo!(),
    Statement::SplitTable => todo!(),
    Statement::FlattenTable => todo!(),
  }
}

fn variable_define(var_def: &VariableDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = var_def.var.name.hash();
  let mut result = expression(&var_def.expression, plan.clone(), symbols.clone(), functions.clone())?;
  if let Some(knd_atn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_atn.kind,functions.clone())?;
    let target_knd = knd.to_value_kind(functions.clone())?;
    let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_knd)])?;
    convert_fxn.solve();
    let converted_result = convert_fxn.out();
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(convert_fxn);
    result = converted_result;
  };
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(id,result.clone());
  Ok(result)
}

fn kind_annotation(knd: &NodeKind, functions: FunctionsRef) -> MResult<Kind> {
  match knd {
    NodeKind::Scalar(id) => {
      let kind_id = id.hash();
      Ok(Kind::Scalar(kind_id))
    }
    NodeKind::Bracket((el_knds, size)) => {
      let mut knds = vec![];
      for knd in el_knds {
        let knd = kind_annotation(knd, functions.clone())?;
        knds.push(knd);
      }
      let mut dims = vec![];
      for dim in size {
        let dim_val = literal(dim, functions.clone())?;
        match dim_val.as_usize() {
          Some(size_val) => dims.push(size_val.clone()),
          None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::ExpectedNumericForSize});} 
        }
      }
      if knds.len() != 1 {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::MatrixMustHaveHomogenousKind});
      }
      Ok(Kind::Matrix(Box::new(knds[0].clone()),dims))
    }
    _ => todo!(),
  }
}

fn expression(expr: &Expression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &expr {
    Expression::Var(v) => var(&v, symbols.clone()),
    Expression::Range(rng) => range(&rng, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Slice(slc) => slice(&slc, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Formula(fctr) => factor(fctr, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Structure(strct) => structure(strct, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Literal(ltrl) => literal(&ltrl, functions.clone()),
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, plan.clone(), symbols.clone(), functions.clone()),
    Expression::FsmPipe(_) => todo!(),
  }
}

fn function_call(fxn_call: &FunctionCall, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let fxn_name_id = fxn_call.name.hash();
  let fxns_brrw = functions.borrow();
  match fxns_brrw.functions.get(&fxn_name_id) {
    Some(fxn) => {
      let mut new_fxn = fxn.recompile(functions.clone())?; // This just calles function_define again, it should be smarter.
      for (ix,(arg_name, arg_expr)) in fxn_call.args.iter().enumerate() {
        // Get the value
        let value_ref: ValRef = match arg_name {
          // Arg is called with a name
          Some(arg_id) => {
            match new_fxn.input.get(&arg_id.hash()) {
              // Arg name matches expected name
              Some(kind) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(arg_id.hash()).unwrap().clone()
              }
              // The argument name doesn't match
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnknownFunctionArgument(arg_id.hash())});}
            }
          }
          // Arg is called positionally (no arg name supplied)
          None => {
            match &new_fxn.input.iter().nth(ix) {
              Some((arg_id,kind)) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(**arg_id).unwrap().clone()
              }
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::TooManyInputArguments(ix+1,new_fxn.input.len())});} 
            }
          }
        };
        let result = expression(&arg_expr, plan.clone(), symbols.clone(), functions.clone())?;
        let mut ref_brrw = value_ref.borrow_mut();
        // TODO check types
        match (&mut *ref_brrw, &result) {
          (Value::I64(arg_ref), Value::I64(i64_ref)) => {
            *arg_ref.borrow_mut() = i64_ref.borrow().clone();
          }
          _ => todo!(),
        }
      }
      // schedule function
      let mut plan_brrw = plan.borrow_mut();
      let result = new_fxn.solve();
      let result_brrw = result.borrow();
      plan_brrw.push(Box::new(UserFunction{fxn: new_fxn.clone()}));
      return Ok(result_brrw.clone())
    }
    None => { 
      match fxns_brrw.function_compilers.get(&fxn_name_id) {
        Some(fxn_compiler) => {
          let mut input_arg_values = vec![];
          for (arg_name, arg_expr) in fxn_call.args.iter() {
            let result = expression(&arg_expr, plan.clone(), symbols.clone(), functions.clone())?;
            input_arg_values.push(result);
          }
          match fxn_compiler.compile(&input_arg_values) {
            Ok(new_fxn) => {
              let mut plan_brrw = plan.borrow_mut();
              new_fxn.solve();
              let result = new_fxn.out();
              plan_brrw.push(new_fxn);
              return Ok(result)
            }
            Err(x) => {return Err(x);}
          }
        }
        None => {return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::MissingFunction(fxn_name_id)});}
      }
    }
  }   
  unreachable!()
}

fn range(rng: &RangeExpression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let start = factor(&rng.start, plan.clone(),symbols.clone(), functions.clone())?;
  let terminal = factor(&rng.terminal, plan.clone(),symbols.clone(), functions.clone())?;
  let new_fxn = match &rng.operator {
    RangeOp::Exclusive => RangeExclusive{}.compile(&vec![start,terminal])?,
    RangeOp::Inclusive => RangeInclusive{}.compile(&vec![start,terminal])?,
    x => unreachable!(),
  };
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  let step = plan_brrw.last().unwrap();
  step.solve();
  let res = step.out();
  Ok(res)
}

fn slice(slc: &Slice, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let val: Value = match symbols_brrw.get(name) {
    Some(val) => Value::MutableReference(val.clone()),
    None => {return Err(MechError{tokens: slc.name.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  for s in &slc.subscript {
    let s_result = subscript(&s, &val, plan.clone(), symbols.clone(), functions.clone())?;
    return Ok(s_result);
  }
  unreachable!() // subscript should have through an error if we can't access an element
}

fn subscript(sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      match val {
        Value::Record(rcrd) => {
          match rcrd.map.get(&Value::Id(key)) {
            Some(value) => return Ok(value.clone()),
            None => { return Err(MechError{tokens: x.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(key)});}
          }
        }
        Value::MutableReference(r) => match &*r.borrow() {
          Value::Record(rcrd) => {
            match rcrd.map.get(&Value::Id(key)) {
              Some(value) => return Ok(value.clone()),
              None => { return Err(MechError{tokens: x.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(key)});}
            }
          }
          _ => todo!(),
        }
        _ => todo!(),
      }
    },
    Subscript::Range(x) => todo!(),
    Subscript::Swizzle(x) => todo!(),
    Subscript::Formula(fctr) => {return factor(fctr,plan.clone(), symbols.clone(), functions.clone());},
    Subscript::Bracket(subs) => {
      let mut resolved_subs = vec![];
      for s in subs {
        let result = subscript(&s, val, plan.clone(), symbols.clone(), functions.clone())?;
        resolved_subs.push(result);
      }
      match val {
        Value::MatrixI64(mat) => {
          let result = match &resolved_subs[..] {
            [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
            [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
            _ => todo!(),
          };
          return Ok(Value::I64(new_ref(result)));
        }
        Value::MutableReference(x) => match &*x.borrow() {
          Value::MatrixI64(mat) => {
            let result = match &resolved_subs[..] {
              [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
              [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
              _ => todo!(),
            };
            return Ok(Value::I64(new_ref(result)));
          }
          _ => todo!(),
        }
        x => {
          println!("{:?}",x);
          todo!()
        },
      }
    },
    Subscript::Brace(x) => todo!(),
    Subscript::All => todo!(),
  }
  return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::None});
}

fn structure(strct: &Structure, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match strct {
    Structure::Empty => Ok(Value::Empty),
    Structure::Record(x) => record(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Matrix(x) => matrix(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Table(x) => table(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Tuple(x) => tuple(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::TupleStruct(x) => todo!(),
    Structure::Set(x) => set(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Map(x) => map(&x, plan.clone(), symbols.clone(), functions.clone()),
  }
}

fn tuple(tpl: &Tuple, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,plan.clone(),symbols.clone(), functions.clone())?;
    elements.push(Box::new(result));
  }
  let mech_tuple = MechTuple{elements};
  Ok(Value::Tuple(mech_tuple))
}

fn map(mp: &Map, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut m = IndexMap::new();
  for b in &mp.elements {
    let key = expression(&b.key, plan.clone(), symbols.clone(), functions.clone())?;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(key,val);
  }
  Ok(Value::Map(MechMap{map: m}))
}

fn record(rcrd: &Record, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut m = IndexMap::new();
  for b in &rcrd.bindings {
    let name = b.name.hash();
    let kind = &b.kind;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(Value::Id(name),val);
  }
  Ok(Value::Record(MechMap{map: m}))
}

fn set(m: &Set, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  let mut out = IndexSet::new();
  for el in &m.elements {
    let result = expression(el, plan.clone(), symbols.clone(), functions.clone())?;
    out.insert(result);
  }
  Ok(Value::Set(MechSet{set: out}))
}

fn table(t: &Table, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  let mut rows = vec![];
  let header = table_header(&t.header)?;
  let mut cols = 0;
  // Interpret the rows
  for row in &t.rows {
    let result = table_row(row, plan.clone(), symbols.clone(), functions.clone())?;
    cols = result.len();
    rows.push(result);
  }
  // Provision columns
  let mut data = Vec::new();
  for i in 0..cols {
    data.push(vec![])
  }
  // Populate columns with data from rows
  for row in rows {
    for (ix,el) in row.iter().enumerate() {
      data[ix].push(el.clone());
    }
  }
  // Build the table
  let mut data_map = IndexMap::new();
  for (field_label,column) in header.iter().zip(data.iter()) {
    data_map.insert(field_label.clone(),column.clone());
  }
  let tbl = MechTable{rows: t.rows.len(), cols, data: data_map.clone()  };
  Ok(Value::Table(tbl))
}

fn table_header(fields: &Vec<Field>) -> MResult<Vec<Value>> {
  let mut row: Vec<Value> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = &f.kind;
    row.push(Value::Id(id));
  }
  Ok(row)
}

fn table_row(r: &TableRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Vec<Value>> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = table_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    row.push(result);
  }
  Ok(row)
}

fn table_column(r: &TableColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn matrix(m: &Mat, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut out = vec![];
  for row in &m.rows {
    let result = matrix_row(row, plan.clone(), symbols.clone(), functions.clone())?;
    out.push(result);
  }

  if out.is_empty() {
    return Ok(Value::MatrixF64(Matrix::<F64>::DMatrix(new_ref(DMatrix::from_vec(0, 0, vec![])))));
  }

  let shape = out[0].shape();
  let col_n = shape[1];
  let row_n = out.len();

  // Function to put element vector into column-major ordering so it can be reconstituted into a matrix
  fn to_column_major<T: Clone>(out: &[Value], row_n: usize, col_n: usize, extract_fn: impl Fn(&Value) -> Option<Vec<T>> + Clone) -> Vec<T> {
    (0..col_n).flat_map(|col| out.iter().map({let value = extract_fn.clone();move |row| value(row).unwrap()[col].clone()})).collect()
  }

  let mat = match &out[0] {
    Value::MatrixU8(_) => Value::MatrixU8(u8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu8()), row_n, col_n)),
    Value::MatrixU16(_) => Value::MatrixU16(u16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu16()), row_n, col_n)),
    Value::MatrixU32(_) => Value::MatrixU32(u32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu32()), row_n, col_n)),
    Value::MatrixU64(_) => Value::MatrixU64(u64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu64()), row_n, col_n)),
    Value::MatrixU128(_) => Value::MatrixU128(u128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu128()), row_n, col_n)),
    Value::MatrixI8(_) => Value::MatrixI8(i8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci8()), row_n, col_n)),
    Value::MatrixI16(_) => Value::MatrixI16(i16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci16()), row_n, col_n)),
    Value::MatrixI32(_) => Value::MatrixI32(i32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci32()), row_n, col_n)),
    Value::MatrixI64(_) => Value::MatrixI64(i64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci64()), row_n, col_n)),
    Value::MatrixI128(_) => Value::MatrixI128(i128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci128()), row_n, col_n)),
    Value::MatrixF32(_) => Value::MatrixF32(F32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf32()), row_n, col_n)),
    Value::MatrixF64(_) => Value::MatrixF64(F64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf64()), row_n, col_n)),
    _ => todo!(),
  };

  Ok(mat)
}

fn matrix_row(r: &MatrixRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = matrix_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    row.push(result);
  }
  let mat = match &row[0] {
    Value::U8(_) => {Value::MatrixU8(u8::to_matrix(row.iter().map(|v| v.as_u8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U16(_) => {Value::MatrixU16(u16::to_matrix(row.iter().map(|v| v.as_u16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U32(_) => {Value::MatrixU32(u32::to_matrix(row.iter().map(|v| v.as_u32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U64(_) => {Value::MatrixU64(u64::to_matrix(row.iter().map(|v| v.as_u64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U128(_) => {Value::MatrixU128(u128::to_matrix(row.iter().map(|v| v.as_u128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I8(_) => {Value::MatrixI8(i8::to_matrix(row.iter().map(|v| v.as_i8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I16(_) => {Value::MatrixI16(i16::to_matrix(row.iter().map(|v| v.as_i16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I32(_) => {Value::MatrixI32(i32::to_matrix(row.iter().map(|v| v.as_i32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I64(_) => {Value::MatrixI64(i64::to_matrix(row.iter().map(|v| v.as_i64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I128(_) => {Value::MatrixI128(i128::to_matrix(row.iter().map(|v| v.as_i128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::F32(_) => {Value::MatrixF32(F32::to_matrix(row.iter().map(|v| F32::new(v.as_f32().unwrap().borrow().clone())).collect(),1,row.len()))},
    Value::F64(_) => {Value::MatrixF64(F64::to_matrix(row.iter().map(|v| F64::new(v.as_f64().unwrap().borrow().clone())).collect(),1,row.len()))},
    _ => todo!(),
  };
  Ok(mat)
}

fn matrix_column(r: &MatrixColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn var(v: &Var, symbols: SymbolTableRef) -> MResult<Value> {
  let id = v.name.hash();
  let symbols_brrw = symbols.borrow();
  match symbols_brrw.get(id) {
    Some(value) => {
      return Ok(Value::MutableReference(value.clone()))
    }
    None => {
      return Err(MechError{tokens: v.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(id)});
    }
  }
}

fn factor(fctr: &Factor, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, plan.clone(), symbols.clone(), functions.clone())?;
      Ok(result)
    },
    Factor::Expression(expr) => expression(expr, plan.clone(), symbols.clone(), functions.clone()),
    Factor::Negated(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Transpose(fctr) => {
      let value = factor(fctr, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MatrixTranspose{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
  }
}

fn term(trm: &Term, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut lhs = factor(&trm.lhs, plan.clone(), symbols.clone(), functions.clone())?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, plan.clone(), symbols.clone(), functions.clone())?;
    let new_fxn = match op {
      FormulaOperator::AddSub(AddSubOp::Add) => MathAdd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Exponent(ExponentOp::Exp) => MathExp{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Or) => LogicOr{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Logic(LogicOp::Not) => LogicNot{}.compile(&vec![lhs,rhs])?,
      //FormulaOperator::Logic(LogicOp::Xor) => LogicXor{}.compile(&vec![lhs,rhs])?,
      x => todo!(),
    };
    new_fxn.solve();
    let res = new_fxn.out();
    term_plan.push(new_fxn);
    lhs = res;
  }
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.append(&mut term_plan);
  return Ok(lhs);
}

fn literal(ltrl: &Literal, functions: FunctionsRef) -> MResult<Value> {
  match &ltrl {
    Literal::Empty(_) => Ok(empty()),
    Literal::Boolean(bln) => Ok(boolean(bln)),
    Literal::Number(num) => Ok(number(num)),
    Literal::String(strng) => Ok(string(strng)),
    Literal::Atom(atm) => Ok(atom(atm)),
    Literal::TypedLiteral((ltrl,kind)) => typed_literal(ltrl,kind,functions),
  }
}

fn typed_literal(ltrl: &Literal, knd_attn: &KindAnnotation, functions: FunctionsRef) -> MResult<Value> {
  let value = literal(ltrl,functions.clone())?;
  let kind = kind_annotation(&knd_attn.kind, functions.clone())?;
  match (&value,kind) {
    (Value::I64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8) => Ok(Value::I8(new_ref(*num.borrow() as i8))),
        Some(ValueKind::I16) => Ok(Value::I16(new_ref(*num.borrow() as i16))),
        Some(ValueKind::I32) => Ok(Value::I32(new_ref(*num.borrow() as i32))),
        Some(ValueKind::I64) => Ok(value),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref(*num.borrow() as i128))),
        Some(ValueKind::U8) => Ok(Value::U8(new_ref(*num.borrow() as u8))),
        Some(ValueKind::U16) => Ok(Value::U16(new_ref(*num.borrow() as u16))),
        Some(ValueKind::U32) => Ok(Value::U32(new_ref(*num.borrow() as u32))),
        Some(ValueKind::U64) => Ok(Value::U64(new_ref(*num.borrow() as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref(*num.borrow() as u128))),
        Some(ValueKind::F32) => Ok(Value::F32(new_ref(F32::new(*num.borrow() as f32)))),
        Some(ValueKind::F64) => Ok(Value::F64(new_ref(F64::new(*num.borrow() as f64)))),
        None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    _ => todo!(),
  }
}

fn atom(atm: &Atom) -> Value {
  let id = atm.name.hash();
  Value::Atom(id)
}

fn number(num: &Number) -> Value {
  match num {
    Number::Real(num) => real(num),
    Number::Imaginary(num) => todo!(),
  }
}

fn real(rl: &RealNumber) -> Value {
  match rl {
    RealNumber::Negated(num) => todo!(),
    RealNumber::Integer(num) => integer(num),
    RealNumber::Float(num) => float(num),
    RealNumber::Decimal(num) => todo!(),
    RealNumber::Hexadecimal(num) => todo!(),
    RealNumber::Octal(num) => todo!(),
    RealNumber::Binary(num) => todo!(),
    RealNumber::Scientific(num) => todo!(),
    RealNumber::Rational(num) => todo!(),
  }
}

fn float(flt: &(Token,Token)) -> Value {
  let a = flt.0.chars.iter().collect::<String>();
  let b = flt.1.chars.iter().collect::<String>();
  let num: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  Value::F64(new_ref(F64(num)))
}

fn integer(int: &Token) -> Value {
  let num: i64 = int.chars.iter().collect::<String>().parse::<i64>().unwrap();
  Value::I64(new_ref(num))
}

fn string(tkn: &MechString) -> Value {
  let strng: String = tkn.text.chars.iter().collect::<String>();
  Value::String(strng)
}

fn empty() -> Value {
  Value::Empty
}

fn boolean(tkn: &Token) -> Value {
  let strng: String = tkn.chars.iter().collect::<String>();
  let val = match strng.as_str() {
    "true" => true,
    "false" => false,
    _ => unreachable!(),
  };
  Value::Bool(new_ref(val))
}

// ============================================================================
// The Standard Library!
// ============================================================================

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

macro_rules! addto_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).add_to(&*$rhs,&mut *$out) }
  };
}

macro_rules! subto_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).sub_to(&*$rhs,&mut *$out) }
  };
}

macro_rules! component_mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).component_mul(&*$rhs); }
  };
}

macro_rules! component_div_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).component_div(&*$rhs); }
  };
}

macro_rules! add_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs + *$rhs; }
  };
}

macro_rules! sub_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs - *$rhs; }
  };
}

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }
  };
}

macro_rules! div_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs / *$rhs; }
  };
}

macro_rules! mul_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).clone() * *$rhs; }
  };
}
macro_rules! mul_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$rhs).clone() * *$lhs;}
  };
}

macro_rules! gt_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] > (*$rhs)[i];
      }
    }
  };
}

macro_rules! gt_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) > (*$rhs);
    }
  };
}

macro_rules! lt_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] < (*$rhs)[i];
      }
    }
  };
}

macro_rules! lt_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) < (*$rhs);
    }
  };
}

macro_rules! matmul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).mul_to(&*$rhs,&mut *$out); }
  };
}

macro_rules! impl_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Zero + One,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

macro_rules! generate_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => {
              Ok(Box::new([<$lib Scalar>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($default) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
              Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => {
              Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector2(rhs))) => {
              Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector2::from_element($default)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector3(rhs))) => {
              Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector3::from_element($default)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector4(rhs))) => {
              Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector4::from_element($default)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))}))
            },          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib RDRD>]{lhs, rhs, out: new_ref(RowDVector::from_element(length,$default))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib VDVD>]{lhs, rhs, out: new_ref(DVector::from_element(length,$default))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
            },
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

// Cos ------------------------------------------------------------------------

use libm::cos;

#[derive(Debug)]
pub struct MathCosScalar {
  val: Ref<F64>,
  out: Ref<F64>,
}

impl MechFunction for MathCosScalar {
  fn solve(&self) {
    let val_ptr = self.val.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{(*out_ptr).0 = cos((*val_ptr).0);}
  }
  fn out(&self) -> Value { Value::F64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MathCos {}

impl NativeFunctionCompiler for MathCos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match &arguments[0] {
      Value::F64(val) =>
        Ok(Box::new(MathCosScalar{val: val.clone(), out: new_ref(F64(0.0))})),
      Value::MutableReference(val) => match &*val.borrow() {
        Value::F64(val) => Ok(Box::new(MathCosScalar{val: val.clone(), out: new_ref(F64(0.0))})),
        x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})  
      }
      x =>Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Sin ------------------------------------------------------------------------

use libm::sin;

#[derive(Debug)]
pub struct MathSinScalar {
  val: Ref<F64>,
  out: Ref<F64>,
}

impl MechFunction for MathSinScalar {
  fn solve(&self) {
    let val_ptr = self.val.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{(*out_ptr).0 = sin((*val_ptr).0);}
  }
  fn out(&self) -> Value { Value::F64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MathSin {}

impl NativeFunctionCompiler for MathSin {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match &arguments[0] {
      Value::F64(val) =>
        Ok(Box::new(MathSinScalar{val: val.clone(), out: new_ref(F64(0.0))})),
      Value::MutableReference(val) => match &*val.borrow() {
        Value::F64(val) => Ok(Box::new(MathSinScalar{val: val.clone(), out: new_ref(F64(0.0))})),
        x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})  
      }
      x =>Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Add ------------------------------------------------------------------------

impl_binop!(AddScalar, T,T,T, add_op);
impl_binop!(AddM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, add_op);
impl_binop!(AddM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, add_op);
impl_binop!(AddM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, add_op);
impl_binop!(AddR2R2, RowVector2<T>, RowVector2<T>, RowVector2<T>, add_op);
impl_binop!(AddR3R3, RowVector3<T>, RowVector3<T>, RowVector3<T>, add_op);
impl_binop!(AddR4R4, RowVector4<T>, RowVector4<T>, RowVector4<T>, add_op);
impl_binop!(AddRDRD, RowDVector<T>, RowDVector<T>, RowDVector<T>, addto_op);
impl_binop!(AddVDVD, DVector<T>,DVector<T>,DVector<T>, addto_op);
impl_binop!(AddMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, addto_op);

fn generate_add_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Add,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   i8::zero();
    I16,  I16  => MatrixI16,  i16,  i16::zero();
    I32,  I32  => MatrixI32,  i32,  i32::zero();
    I64,  I64  => MatrixI64,  i64,  i64::zero();
    I128, I128 => MatrixI128, i128, i128::zero();
    U8,   U8   => MatrixU8,   u8,   u8::zero();
    U16,  U16  => MatrixU16,  u16,  u16::zero();
    U32,  U32  => MatrixU32,  u32,  u32::zero();
    U64,  U64  => MatrixU64,  u64,  u64::zero();
    U128, U128 => MatrixU128, u128, u128::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

pub struct MathAdd {}

impl NativeFunctionCompiler for MathAdd {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_add_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_add_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_add_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_add_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Sub ------------------------------------------------------------------------

impl_binop!(SubScalar, T,T,T, sub_op);
impl_binop!(SubM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, sub_op);
impl_binop!(SubM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, sub_op);
impl_binop!(SubM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, sub_op);
impl_binop!(SubR2R2, RowVector2<T>,RowVector2<T>,RowVector2<T>, sub_op);
impl_binop!(SubR3R3, RowVector3<T>,RowVector3<T>,RowVector3<T>, sub_op);
impl_binop!(SubR4R4, RowVector4<T>,RowVector4<T>,RowVector4<T>, sub_op);
impl_binop!(SubRDRD, RowDVector<T>,RowDVector<T>,RowDVector<T>, subto_op);
impl_binop!(SubVDVD, DVector<T>,DVector<T>,DVector<T>, subto_op);
impl_binop!(SubMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, subto_op);

fn generate_sub_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Sub,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   i8::zero();
    I16,  I16  => MatrixI16,  i16,  i16::zero();
    I32,  I32  => MatrixI32,  i32,  i32::zero();
    I64,  I64  => MatrixI64,  i64,  i64::zero();
    I128, I128 => MatrixI128, i128, i128::zero();
    U8,   U8   => MatrixU8,   u8,   u8::zero();
    U16,  U16  => MatrixU16,  u16,  u16::zero();
    U32,  U32  => MatrixU32,  u32,  u32::zero();
    U64,  U64  => MatrixU64,  u64,  u64::zero();
    U128, U128 => MatrixU128, u128, u128::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

pub struct MathSub {}

impl NativeFunctionCompiler for MathSub {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_sub_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_sub_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_sub_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_sub_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// add ------------------------------------------------------------------------

impl_binop!(MulScalar, T,T,T, mul_op);
impl_binop!(MulSM2x3, T, Matrix2x3<T>, Matrix2x3<T>,mul_scalar_rhs_op);
impl_binop!(MulSM2, T, Matrix2<T>, Matrix2<T>,mul_scalar_rhs_op);
impl_binop!(MulSM3, T, Matrix3<T>, Matrix3<T>,mul_scalar_rhs_op);
impl_binop!(MulSRv2, T, RowVector2<T>, RowVector2<T>,mul_scalar_rhs_op);
impl_binop!(MulSRv3, T, RowVector3<T>, RowVector3<T>,mul_scalar_rhs_op);
impl_binop!(MulSRv4, T, RowVector4<T>, RowVector4<T>,mul_scalar_rhs_op);
impl_binop!(MulSRvD, T, RowDVector<T>, RowDVector<T>,mul_scalar_rhs_op);
impl_binop!(MulSVD, T, DVector<T>, DVector<T>,mul_scalar_rhs_op);
impl_binop!(MulSMD, T, DMatrix<T>, DMatrix<T>,mul_scalar_rhs_op);
impl_binop!(MulM2x3S, Matrix2x3<T>, T, Matrix2x3<T>,mul_scalar_lhs_op);
impl_binop!(MulM2S, Matrix2<T>, T, Matrix2<T>,mul_scalar_lhs_op);
impl_binop!(MulM3S, Matrix3<T>, T, Matrix3<T>,mul_scalar_lhs_op);
impl_binop!(MulRv2S, RowVector2<T>, T, RowVector2<T>,mul_scalar_lhs_op);
impl_binop!(MulRv3S, RowVector3<T>, T, RowVector3<T>,mul_scalar_lhs_op);
impl_binop!(MulRv4S, RowVector4<T>, T, RowVector4<T>,mul_scalar_lhs_op);
impl_binop!(MulRvDS, RowDVector<T>, T, RowDVector<T>,mul_scalar_lhs_op);
impl_binop!(MulVDS, DVector<T>, T, DVector<T>,mul_scalar_lhs_op);
impl_binop!(MulMDS, DMatrix<T>, T, DMatrix<T>,mul_scalar_lhs_op);
impl_binop!(MulM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>, component_mul_op);
impl_binop!(MulM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>, component_mul_op);
impl_binop!(MulM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>, component_mul_op);
impl_binop!(MulR2R2, RowVector2<T>,RowVector2<T>,RowVector2<T>, component_mul_op);
impl_binop!(MulR3R3, RowVector3<T>,RowVector3<T>,RowVector3<T>, component_mul_op);
impl_binop!(MulR4R4, RowVector4<T>,RowVector4<T>,RowVector4<T>, component_mul_op);
impl_binop!(MulRDRD, RowDVector<T>,RowDVector<T>,RowDVector<T>, component_mul_op);
impl_binop!(MulVDVD, DVector<T>,DVector<T>,DVector<T>, component_mul_op);
impl_binop!(MulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>, component_mul_op);

fn generate_mul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Mul,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   i8::zero();
    I16,  I16  => MatrixI16,  i16,  i16::zero();
    I32,  I32  => MatrixI32,  i32,  i32::zero();
    I64,  I64  => MatrixI64,  i64,  i64::zero();
    I128, I128 => MatrixI128, i128, i128::zero();
    U8,   U8   => MatrixU8,   u8,   u8::zero();
    U16,  U16  => MatrixU16,  u16,  u16::zero();
    U32,  U32  => MatrixU32,  u32,  u32::zero();
    U64,  U64  => MatrixU64,  u64,  u64::zero();
    U128, U128 => MatrixU128, u128, u128::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

pub struct MathMul {}

impl NativeFunctionCompiler for MathMul {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_mul_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_mul_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_mul_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_mul_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Div ------------------------------------------------------------------------

impl_binop!(DivScalar, T, T, T, div_op);
impl_binop!(DivM2x3M2x3, Matrix2x3<T>,Matrix2x3<T>,Matrix2x3<T>,component_div_op);
impl_binop!(DivM2M2, Matrix2<T>,Matrix2<T>,Matrix2<T>,component_div_op);
impl_binop!(DivM3M3, Matrix3<T>,Matrix3<T>,Matrix3<T>,component_div_op);
impl_binop!(DivR2R2, RowVector2<T>,RowVector2<T>,RowVector2<T>,component_div_op);
impl_binop!(DivR3R3, RowVector3<T>,RowVector3<T>,RowVector3<T>,component_div_op);
impl_binop!(DivR4R4, RowVector4<T>,RowVector4<T>,RowVector4<T>,component_div_op);
impl_binop!(DivRDRD, RowDVector<T>,RowDVector<T>,RowDVector<T>,component_div_op);
impl_binop!(DivVDVD, DVector<T>,DVector<T>,DVector<T>,component_div_op);
impl_binop!(DivMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,component_div_op);

fn generate_div_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Div,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   i8::zero();
    I16,  I16  => MatrixI16,  i16,  i16::zero();
    I32,  I32  => MatrixI32,  i32,  i32::zero();
    I64,  I64  => MatrixI64,  i64,  i64::zero();
    I128, I128 => MatrixI128, i128, i128::zero();
    U8,   U8   => MatrixU8,   u8,   u8::zero();
    U16,  U16  => MatrixU16,  u16,  u16::zero();
    U32,  U32  => MatrixU32,  u32,  u32::zero();
    U64,  U64  => MatrixU64,  u64,  u64::zero();
    U128, U128 => MatrixU128, u128, u128::zero();
    F32,  F32  => MatrixF32,  F32,  F32::zero();
    F64,  F64  => MatrixF64,  F64,  F64::zero();
  )
}

pub struct MathDiv {}

impl NativeFunctionCompiler for MathDiv {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_div_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_div_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_div_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_div_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Exp ------------------------------------------------------------------------

#[derive(Debug)] 
struct ExpScalar {
  lhs: Ref<i64>,
  rhs: Ref<i64>,
  out: Ref<i64>,
}

impl MechFunction for ExpScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = (*lhs_ptr).pow(*rhs_ptr as u32);}
  }
  fn out(&self) -> Value {
    Value::I64(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct MathExp {}

impl NativeFunctionCompiler for MathExp {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(ExpScalar{lhs, rhs, out: new_ref(0)})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Negate ---------------------------------------------------------------------

macro_rules! impl_neg_fxn {
  ($struct_name:ident, $arg_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      input: Ref<$arg_type>,
      out: Ref<$arg_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + Neg<Output = T> + PartialEq + 'static,
      Ref<$arg_type>: ToValue
    {
      fn solve(&self) {
        let input_ptr = self.input.as_ptr();
        let output_ptr = self.out.as_ptr();
        unsafe { *output_ptr = -*input_ptr; }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

impl_neg_fxn!(NegateScalar, T);
impl_neg_fxn!(NegateM2, Matrix2<T>);
impl_neg_fxn!(NegateM3, Matrix3<T>);
impl_neg_fxn!(NegateM2x3, Matrix2x3<T>);
impl_neg_fxn!(NegateRv2, RowVector2<T>);
impl_neg_fxn!(NegateRv3, RowVector3<T>);
impl_neg_fxn!(NegateRv4, RowVector4<T>);

macro_rules! impl_neg_fxn_dynamic {
  ($struct_name:ident, $arg_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      input: Ref<$arg_type>,
      out: Ref<$arg_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + Neg + ClosedNeg + PartialEq + 'static,
      Ref<$arg_type>: ToValue
    {
      fn solve(&self) {
        let input_ptr = self.input.borrow();
        let output_ptr = self.out.as_ptr();
        unsafe { *output_ptr = input_ptr.clone().neg(); }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

impl_neg_fxn_dynamic!(NegateRvD, RowDVector<T>);
impl_neg_fxn_dynamic!(NegateVD, DVector<T>);
impl_neg_fxn_dynamic!(NegateMD, DMatrix<T>);

macro_rules! generate_neg_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          Value::$input_type(input) => {
            Ok(Box::new(NegateScalar{ input: input.clone(), out: new_ref($target_type::zero()) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)) => {
            Ok(Box::new(NegateRv4{ input: input.clone(), out: new_ref(RowVector4::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)) => {
            Ok(Box::new(NegateRv3{ input: input.clone(), out: new_ref(RowVector3::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)) => {
            Ok(Box::new(NegateRv2{ input: input.clone(), out: new_ref(RowVector2::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)) => {
            Ok(Box::new(NegateM2{input, out: new_ref(Matrix2::from_element($target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)) => {
            Ok(Box::new(NegateM3{input, out: new_ref(Matrix3::from_element($target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)) => {
            Ok(Box::new(NegateM2x3{input, out: new_ref(Matrix2x3::from_element($target_type::zero()))}))
          },          
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)) => {
            let length = {input.borrow().len()};
            Ok(Box::new(NegateRvD{input, out: new_ref(RowDVector::from_element(length,$target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DVector(input)) => {
            let length = {input.borrow().len()};
            Ok(Box::new(NegateVD{input, out: new_ref(DVector::from_element(length,$target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)) => {
            let (rows,cols) = {input.borrow().shape()};
            Ok(Box::new(NegateMD{input, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn generate_neg_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_neg_match_arms!(
    (lhs_value),
    I8 => MatrixI8, i8;
    I16 => MatrixI16, i16;
    I32 => MatrixI32, i32;
    I64 => MatrixI64, i64;
    I128 => MatrixI128, i128;
    F32 => MatrixF32, F32;
    F64 => MatrixF64, F64;
  )
}

pub struct MathNegate {}

impl NativeFunctionCompiler for MathNegate {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match generate_neg_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {generate_neg_fxn(input.borrow().clone())}
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

// And ------------------------------------------------------------------------

#[derive(Debug)]
struct AndScalar {
  lhs: Ref<bool>,
  rhs: Ref<bool>,
  out: Ref<bool>,
}

impl MechFunction for AndScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr && *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicAnd {}

impl NativeFunctionCompiler for LogicAnd {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(AndScalar{lhs, rhs, out: new_ref(false)})),
      (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {
        match (&*lhs.borrow(), &*rhs.borrow()) {
          (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Box::new(AndScalar{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::Bool(lhs),Value::MutableReference(rhs)) => {
        match (&*rhs.borrow()) {
          (Value::Bool(rhs)) => Ok(Box::new(AndScalar{lhs, rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::MutableReference(lhs),Value::Bool(rhs)) => {
        match (&*lhs.borrow()) {
          (Value::Bool(lhs)) => Ok(Box::new(AndScalar{lhs: lhs.clone(), rhs, out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Or ------------------------------------------------------------------------

#[derive(Debug)]
struct OrScalar {
  lhs: Ref<bool>,
  rhs: Ref<bool>,
  out: Ref<bool>,
}

impl MechFunction for OrScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr || *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicOr {}

impl NativeFunctionCompiler for LogicOr {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(OrScalar{lhs, rhs, out: new_ref(false)})),
      (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {
        match (&*lhs.borrow(), &*rhs.borrow()) {
          (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Box::new(OrScalar{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::Bool(lhs),Value::MutableReference(rhs)) => {
        match (&*rhs.borrow()) {
          (Value::Bool(rhs)) => Ok(Box::new(OrScalar{lhs, rhs: rhs.clone(), out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      (Value::MutableReference(lhs),Value::Bool(rhs)) => {
        match (&*lhs.borrow()) {
          (Value::Bool(lhs)) => Ok(Box::new(OrScalar{lhs: lhs.clone(), rhs, out: new_ref(false)})),
          _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
        }
      }
      x => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------


macro_rules! generate_compare_match_arms {
  ($lib: ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => {
              Ok(Box::new([<$lib Scalar>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(false) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element(false))}))
            },   
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector4(rhs))) => {
              Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector4::from_element(false)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector3(rhs))) => {
              Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector3::from_element(false)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector2(rhs))) => {
              Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector2::from_element(false)) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
              Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element(false))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => {
              Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element(false))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element(false))}))
            },          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib RDRD>]{lhs, rhs, out: new_ref(RowDVector::from_element(length,false))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib VDVD>]{lhs, rhs, out: new_ref(DVector::from_element(length,false))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,false))}))
            },
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

// Greater Than ---------------------------------------------------------------

impl_binop!(GTScalar, T, T, bool, gt_op);
impl_binop!(GTM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, gt_vec_op);
impl_binop!(GTM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, gt_vec_op);
impl_binop!(GTM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, gt_vec_op);
impl_binop!(GTR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, gt_vec_op);
impl_binop!(GTR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, gt_vec_op);
impl_binop!(GTR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, gt_vec_op);
impl_binop!(GTRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, gt_vec_op);
impl_binop!(GTVDVD, DVector<T>, DVector<T>, DVector<bool>, gt_vec_op);
impl_binop!(GTMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, gt_vec_op);

fn generate_gt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    GT,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

pub struct CompareGreaterThan {}

impl NativeFunctionCompiler for CompareGreaterThan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_gt_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_gt_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_gt_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_gt_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Less Than ------------------------------------------------------------------

impl_binop!(LTScalar, T, T, bool, lt_op);
impl_binop!(LTM2x3M2x3, Matrix2x3<T>, Matrix2x3<T>, Matrix2x3<bool>, lt_vec_op);
impl_binop!(LTM2M2, Matrix2<T>, Matrix2<T>, Matrix2<bool>, lt_vec_op);
impl_binop!(LTM3M3, Matrix3<T>,Matrix3<T>, Matrix3<bool>, lt_vec_op);
impl_binop!(LTR2R2, RowVector2<T>, RowVector2<T>, RowVector2<bool>, lt_vec_op);
impl_binop!(LTR3R3, RowVector3<T>, RowVector3<T>, RowVector3<bool>, lt_vec_op);
impl_binop!(LTR4R4, RowVector4<T>, RowVector4<T>, RowVector4<bool>, lt_vec_op);
impl_binop!(LTRDRD, RowDVector<T>, RowDVector<T>, RowDVector<bool>, lt_vec_op);
impl_binop!(LTVDVD, DVector<T>, DVector<T>, DVector<bool>, lt_vec_op);
impl_binop!(LTMDMD, DMatrix<T>, DMatrix<T>, DMatrix<bool>, lt_vec_op);

fn generate_lt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    LT,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

pub struct CompareLessThan {}

impl NativeFunctionCompiler for CompareLessThan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_lt_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_lt_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_lt_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_lt_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

// MatMul ---------------------------------------------------------------------

impl_binop!(MatMulScalar, T,T,T,mul_op);
impl_binop!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulRv2V2, RowVector2<T>,Vector2<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulRv3V3, RowVector3<T>,Vector3<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulRv4V4, RowVector4<T>,Vector4<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulV2Rv2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulV3Rv3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulV4Rv4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op);
impl_binop!(MatMulRvDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op);
impl_binop!(MatMulVDRvD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);

macro_rules! generate_matmul_match_arms {
  ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => {
            Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector4(rhs))) => {
            Ok(Box::new(MatMulRv4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector3(rhs))) => {
            Ok(Box::new(MatMulRv3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector2(rhs))) => {
            Ok(Box::new(MatMulRv2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
            Ok(Box::new(MatMulM2M2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))}))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => {
            Ok(Box::new(MatMulM3M3{lhs, rhs, out: new_ref(Matrix3::from_element($target_type::zero()))}))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(rhs))) => {
            Ok(Box::new(MatMulM2x3M3x2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))}))
          },          
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
            let length = {lhs.borrow().len()};
            Ok(Box::new(MatMulRvDVD{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))}))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
            let rows = {lhs.borrow().len()};
            let cols = {rhs.borrow().len()};
            Ok(Box::new(MatMulVDRvD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
            let (rows,_) = {lhs.borrow().shape()};
            let (_,cols) = {rhs.borrow().shape()};
            Ok(Box::new(MatMulMDMD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn generate_matmul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_matmul_match_arms!(
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8;
    I16,  I16  => MatrixI16,  i16;
    I32,  I32  => MatrixI32,  i32;
    I64,  I64  => MatrixI64,  i64;
    I128, I128 => MatrixI128, i128;
    U8,   U8   => MatrixU8,   u8;
    U16,  U16  => MatrixU16,  u16;
    U32,  U32  => MatrixU32,  u32;
    U64,  U64  => MatrixU64,  u64;
    U128, U128 => MatrixU128, u128;
    F32,  F32  => MatrixF32,  F32;
    F64,  F64  => MatrixF64,  F64;
  )
}

pub struct MatrixMatMul {}

impl NativeFunctionCompiler for MatrixMatMul {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_matmul_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_matmul_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_matmul_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_matmul_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Transpose ------------------------------------------------------------------

macro_rules! impl_transpose_fxn {
  ($struct_name:ident, $arg_type:ty, $out_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      input: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + Neg<Output = T> + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let input_ptr = self.input.as_ptr();
        let output_ptr = self.out.as_ptr();
        unsafe { *output_ptr = (*input_ptr).transpose(); }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

impl_transpose_fxn!(TransposeM2, Matrix2<T>, Matrix2<T>);
impl_transpose_fxn!(TransposeM3, Matrix3<T>, Matrix3<T>);
impl_transpose_fxn!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>);
impl_transpose_fxn!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>);
impl_transpose_fxn!(TransposeRv2, RowVector2<T>, Vector2<T>);
impl_transpose_fxn!(TransposeRv3, RowVector3<T>, Vector3<T>);
impl_transpose_fxn!(TransposeRv4, RowVector4<T>, Vector4<T>);

macro_rules! impl_transpose_fxn_dynamic {
  ($struct_name:ident, $arg_type:ty, $out_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      input: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + ClosedNeg + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let input_ptr = self.input.borrow();
        let output_ptr = self.out.as_ptr();
        unsafe { *output_ptr = input_ptr.clone().transpose(); }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

impl_transpose_fxn_dynamic!(TransposeRvD, RowDVector<T>, DVector<T>);
impl_transpose_fxn_dynamic!(TransposeVD, DVector<T>, RowDVector<T>);
impl_transpose_fxn_dynamic!(TransposeMD, DMatrix<T>, DMatrix<T>);

macro_rules! generate_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)) => {
            Ok(Box::new(TransposeRv4{input: input.clone(), out: new_ref(Vector4::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)) => {
            Ok(Box::new(TransposeRv3{input: input.clone(), out: new_ref(Vector3::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)) => {
            Ok(Box::new(TransposeRv2{input: input.clone(), out: new_ref(Vector2::from_element($target_type::zero())) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)) => {
            Ok(Box::new(TransposeM2{input, out: new_ref(Matrix2::from_element($target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)) => {
            Ok(Box::new(TransposeM3{input, out: new_ref(Matrix3::from_element($target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)) => {
            Ok(Box::new(TransposeM2x3{input, out: new_ref(Matrix3x2::from_element($target_type::zero()))}))
          },          
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)) => {
            Ok(Box::new(TransposeM3x2{input, out: new_ref(Matrix2x3::from_element($target_type::zero()))}))
          },          
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)) => {
            let length = {input.borrow().len()};
            Ok(Box::new(TransposeRvD{input, out: new_ref(DVector::from_element(length,$target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DVector(input)) => {
            let length = {input.borrow().len()};
            Ok(Box::new(TransposeVD{input, out: new_ref(RowDVector::from_element(length,$target_type::zero()))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)) => {
            let (rows,cols) = {input.borrow().shape()};
            Ok(Box::new(TransposeMD{input, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn generate_transpose_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_transpose_match_arms!(
    (lhs_value),
    I8 => MatrixI8, i8;
    I16 => MatrixI16, i16;
    I32 => MatrixI32, i32;
    I64 => MatrixI64, i64;
    I128 => MatrixI128, i128;
    F32 => MatrixF32, F32;
    F64 => MatrixF64, F64;
  )
}

pub struct MatrixTranspose {}

impl NativeFunctionCompiler for MatrixTranspose {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match generate_transpose_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {generate_transpose_fxn(input.borrow().clone())}
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeExclusiveScalar {
  max: Ref<i64>,
  min: Ref<i64>,
  out: Ref<RowDVector<i64>>,
}

impl MechFunction for RangeExclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    
    unsafe {
      let rng = (*min_ptr..*max_ptr).collect::<Vec<i64>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::MatrixI64(Matrix::<i64>::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeExclusive {}

impl NativeFunctionCompiler for RangeExclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeExclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}


// Inclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeInclusiveScalar {
  max: Ref<i64>,
  min: Ref<i64>,
  out: Ref<RowDVector<i64>>,
}

impl MechFunction for RangeInclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let rng = (*min_ptr..=*max_ptr).collect::<Vec<i64>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::MatrixI64(Matrix::<i64>::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeInclusiveScalar{max,min, out: new_ref(RowDVector::from_element(1,0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Type Conversion Library
// ----------------------------------------------------------------------------

// Convert --------------------------------------------------------------------

#[derive(Debug)]
struct ConvertScalar<T, U> {
  input: Ref<T>,
  out: Ref<U>,
}

impl<T, U> MechFunction for ConvertScalar<T, U>
where
  T: Copy + std::fmt::Debug,
  U: Copy + std::fmt::Debug,
  Ref<U>: ToValue
{
  fn solve(&self) {
    let in_value = self.input.borrow();
    let mut out_value = self.out.borrow_mut();
    unsafe {
      *out_value = *(&*in_value as *const T as *const U);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! generate_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($value_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$input_type(arg), ValueKind::$value_kind) => {Ok(Box::new(ConvertScalar {input: arg.clone(),out: new_ref(0 as $target_type)}))},
        )+
      )+
      x => Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}

fn generate_conversion_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  generate_conversion_match_arms!(
    (source_value, target_kind),
    I8 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I16 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I32 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I64 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I128 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U8 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U16 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U32 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U64 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U128 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
  )
}

pub struct ConvertKind {}

impl NativeFunctionCompiler for ConvertKind {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let target_kind = arguments[1].kind();
    match generate_conversion_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(lhs) => {
            generate_conversion_fxn(lhs.borrow().clone(), target_kind.clone())
          }
          _ => unreachable!(),
        }
      }
    }
  }
}