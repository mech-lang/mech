use crate::matrix::Matrix;
use crate::*;

use mech_core::nodes::Matrix as Mat;
use mech_core::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};


// Value ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueKind {
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
      Value::Id(x)   => x.hash(state),
      Value::Kind(x) => x.hash(state),
      Value::U8(x)   => x.borrow().hash(state),
      Value::U16(x)  => x.borrow().hash(state),
      Value::U32(x)  => x.borrow().hash(state),
      Value::U64(x)  => x.borrow().hash(state),
      Value::U128(x) => x.borrow().hash(state),
      Value::I8(x)   => x.borrow().hash(state),
      Value::I16(x)  => x.borrow().hash(state),
      Value::I32(x)  => x.borrow().hash(state),
      Value::I64(x)  => x.borrow().hash(state),
      Value::I128(x) => x.borrow().hash(state),
      Value::F32(x)  => x.borrow().hash(state),
      Value::F64(x)  => x.borrow().hash(state),
      Value::Bool(x) => x.borrow().hash(state),
      Value::Atom(x) => x.hash(state),
      Value::Set(x)  => x.hash(state),
      Value::Map(x)  => x.hash(state),
      Value::Table(x) => x.hash(state),
      Value::Tuple(x) => x.hash(state),
      Value::Record(x) => x.hash(state),
      Value::String(x) => x.hash(state),
      Value::MatrixBool(x) => x.hash(state),
      Value::MatrixU8(x)   => x.hash(state),
      Value::MatrixU16(x)  => x.hash(state),
      Value::MatrixU32(x)  => x.hash(state),
      Value::MatrixU64(x)  => x.hash(state),
      Value::MatrixU128(x) => x.hash(state),
      Value::MatrixI8(x)   => x.hash(state),
      Value::MatrixI16(x)  => x.hash(state),
      Value::MatrixI32(x)  => x.hash(state),
      Value::MatrixI64(x)  => x.hash(state),
      Value::MatrixI128(x) => x.hash(state),
      Value::MatrixF32(x)  => x.hash(state),
      Value::MatrixF64(x)  => x.hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Empty => Value::Empty.hash(state),
    }
  }
}

impl Value {

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      Value::U8(x)   => {builder.push_record(vec!["u8"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::U16(x)  => {builder.push_record(vec!["u16"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::U32(x)  => {builder.push_record(vec!["u32"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::U64(x)  => {builder.push_record(vec!["u64"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::U128(x) => {builder.push_record(vec!["u128"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::I8(x)   => {builder.push_record(vec!["i8"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::I16(x)  => {builder.push_record(vec!["i16"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::I32(x)  => {builder.push_record(vec!["i32"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::I64(x)  => {builder.push_record(vec!["i64"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::I128(x) => {builder.push_record(vec!["i128"]); builder.push_record(vec![format!("{:?}",x.borrow())]);},
      Value::F32(x)  => {builder.push_record(vec!["f32"]); builder.push_record(vec![format!("{:?}",x.borrow().0)]);},
      Value::F64(x)  => {builder.push_record(vec!["f64"]); builder.push_record(vec![format!("{:?}",x.borrow().0)]);},
      Value::Bool(x) => {builder.push_record(vec!["bool"]); builder.push_record(vec![format!("{:?}",x)]);},
      Value::Atom(x) => {builder.push_record(vec!["atom"]); builder.push_record(vec![format!("{:?}",x)]);},
      Value::Set(x)  => builder.push_record(vec![format!("{:?}",x)]),
      Value::Map(x)  => builder.push_record(vec![format!("{:?}",x)]),
      Value::String(x) => builder.push_record(vec![x]),
      Value::Table(x)  => builder.push_record(vec![format!("{:?}",x)]),
      Value::Tuple(x)  => builder.push_record(vec![format!("{:?}",x)]),
      Value::Record(x) => builder.push_record(vec![format!("{:?}",x)]),
      Value::MatrixBool(x) => {return x.pretty_print();}
      Value::MatrixU8(x)   => {return x.pretty_print();},
      Value::MatrixU16(x)  => {return x.pretty_print();},
      Value::MatrixU32(x)  => {return x.pretty_print();},
      Value::MatrixU64(x)  => {return x.pretty_print();},
      Value::MatrixU128(x) => {return x.pretty_print();},
      Value::MatrixI8(x)   => {return x.pretty_print();},
      Value::MatrixI16(x)  => {return x.pretty_print();},
      Value::MatrixI32(x)  => {return x.pretty_print();},
      Value::MatrixI64(x)  => {return x.pretty_print();},
      Value::MatrixI128(x) => {return x.pretty_print();},
      Value::MatrixF32(x)  => {return x.pretty_print();},
      Value::MatrixF64(x)  => {return x.pretty_print();},
      Value::MutableReference(x) => {return x.borrow().pretty_print();},
      Value::Empty => builder.push_record(vec!["_"]),
      _ => unreachable!(),
    };
    let mut table = builder.build();
    table.with(Style::modern());
    format!("{table}")
  }

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

  pub fn as_u8(&self) -> Option<Ref<u8>> {if let Value::U8(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u8() } else { None }}
  pub fn as_u16(&self) -> Option<Ref<u16>> {if let Value::U16(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u16() } else { None }}
  pub fn as_u32(&self) -> Option<Ref<u32>> {if let Value::U32(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u32() } else { None }}
  pub fn as_u64(&self) -> Option<Ref<u64>> {if let Value::U64(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u64() } else { None }}
  pub fn as_u128(&self) -> Option<Ref<u128>> {if let Value::U128(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_u128() } else { None }}
  pub fn as_i8(&self) -> Option<Ref<i8>> {if let Value::I8(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i8() } else { None }}
  pub fn as_i16(&self) -> Option<Ref<i16>> {if let Value::I16(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i16() } else { None }}
  pub fn as_i32(&self) -> Option<Ref<i32>> {if let Value::I32(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i32() } else { None }}
  pub fn as_i64(&self) -> Option<Ref<i64>> {if let Value::I64(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i64() } else { None }}
  pub fn as_i128(&self) -> Option<Ref<i128>> {if let Value::I128(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_i128() } else { None }}
  pub fn as_f32(&self) -> Option<Ref<f32>> {if let Value::F32(v) = self { Some(new_ref(v.borrow().0)) } else if let Value::MutableReference(val) = self { val.borrow().as_f32() } else { None }}
  pub fn as_f64(&self) -> Option<Ref<f64>> {if let Value::F64(v) = self { Some(new_ref(v.borrow().0)) } else if let Value::MutableReference(val) = self { val.borrow().as_f64() } else { None }}
  pub fn as_vecf64(&self) -> Option<Vec<F64>> {if let Value::MatrixF64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf64() } else { None }}
  pub fn as_vecf32(&self) -> Option<Vec<F32>> {if let Value::MatrixF32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf32() } else { None }}
  pub fn as_vecbool(&self) -> Option<Vec<bool>> {if let Value::MatrixBool(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecbool() } else { None }}
  pub fn as_vecu8(&self) -> Option<Vec<u8>> {if let Value::MatrixU8(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu8() } else { None }}
  pub fn as_vecu16(&self) -> Option<Vec<u16>> {if let Value::MatrixU16(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu16() } else { None }}
  pub fn as_vecu32(&self) -> Option<Vec<u32>> {if let Value::MatrixU32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu32() } else { None }}
  pub fn as_vecu64(&self) -> Option<Vec<u64>> {if let Value::MatrixU64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu64() } else { None }}
  pub fn as_vecu128(&self) -> Option<Vec<u128>> {if let Value::MatrixU128(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu128() } else { None }}
  pub fn as_veci8(&self) -> Option<Vec<i8>> {if let Value::MatrixI8(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci8() } else { None }}
  pub fn as_veci16(&self) -> Option<Vec<i16>> {if let Value::MatrixI16(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci16() } else { None }}
  pub fn as_veci32(&self) -> Option<Vec<i32>> {if let Value::MatrixI32(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci32() } else { None }}
  pub fn as_veci64(&self) -> Option<Vec<i64>> {if let Value::MatrixI64(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci64() } else { None }}
  pub fn as_veci128(&self) -> Option<Vec<i128>> {if let Value::MatrixI128(v) = self { Some(v.as_vec()) } else if let Value::MutableReference(val) = self { val.borrow().as_veci128() } else { None }}
  
  pub fn as_usize(&self) -> Option<usize> {
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
  };}

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

// Set --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  pub set: IndexSet<Value>,
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

// Map ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechMap {
  pub map: IndexMap<Value,Value>,
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
  pub rows: usize,
  pub cols: usize,
  pub data: IndexMap<Value,Vec<Value>>,
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

// Tuple ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTuple {
  pub elements: Vec<Box<Value>>
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