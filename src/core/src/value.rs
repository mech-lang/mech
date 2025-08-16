#[cfg(feature = "matrix")]
use crate::matrix::Matrix;
use crate::*;
use crate::nodes::Matrix as Mat;
#[cfg(feature = "complex")]
use crate::types::ComplexNumber;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use std::collections::HashMap;

#[cfg(feature = "matrix")] 
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::hash::{Hash, Hasher};
#[cfg(feature = "pretty_print")]
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use paste::paste;
#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, SeqAccess, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(feature = "rational")]
use num_rational::Rational64;

macro_rules! impl_as_type {
  ($target_type:ty) => {
    paste!{
      pub fn [<as_ $target_type>](&self) -> Option<Ref<$target_type>> {
        match self {
          #[cfg(feature = "u8")]
          Value::U8(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "u16")]
          Value::U16(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "u32")]
          Value::U32(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "u64")]
          Value::U64(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "u128")]
          Value::U128(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "i8")]
          Value::I8(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "i16")]
          Value::I16(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "i32")]
          Value::I32(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "i64")]
          Value::I64(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "i128")]
          Value::I128(v) => Some(new_ref(*v.borrow() as $target_type)),
          #[cfg(feature = "f32")]
          Value::F32(v) => Some(new_ref((*v.borrow()).0 as $target_type)),
          #[cfg(feature = "f64")]
          Value::F64(v) => Some(new_ref((*v.borrow()).0 as $target_type)),
          Value::Id(v) => Some(new_ref(*v as $target_type)),
          Value::MutableReference(val) => val.borrow().[<as_ $target_type>](),
          _ => None,
        }
      }
    }
  };
}

// Value ----------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ValueKind {
  U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64, ComplexNumber, RationalNumber,
  String, Bool, Id, Index, Empty, Any, 
  Matrix(Box<ValueKind>,Vec<usize>),  Enum(u64),                  Record(Vec<(String,ValueKind)>),
  Map(Box<ValueKind>,Box<ValueKind>), Atom(u64),                  Table(Vec<(String,ValueKind)>, usize), 
  Tuple(Vec<ValueKind>),              Reference(Box<ValueKind>),  Set(Box<ValueKind>, Option<usize>), 
  Option(Box<ValueKind>),
}

impl std::fmt::Display for ValueKind {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      ValueKind::RationalNumber => write!(f, "r64"),
      ValueKind::ComplexNumber => write!(f, "c64"),
      ValueKind::U8 => write!(f, "u8"),
      ValueKind::U16 => write!(f, "u16"),
      ValueKind::U32 => write!(f, "u32"),
      ValueKind::U64 => write!(f, "u64"),
      ValueKind::U128 => write!(f, "u128"),
      ValueKind::I8 => write!(f, "i8"),
      ValueKind::I16 => write!(f, "i16"),
      ValueKind::I32 => write!(f, "i32"),
      ValueKind::I64 => write!(f, "i64"),
      ValueKind::I128 => write!(f, "i128"),
      ValueKind::F32 => write!(f, "f32"),
      ValueKind::F64 => write!(f, "f64"),
      ValueKind::String => write!(f, "string"),
      ValueKind::Bool => write!(f, "bool"),
      ValueKind::Matrix(x,s) => write!(f, "[{}]:{}", x, s.iter().map(|s| s.to_string()).collect::<Vec<String>>().join(",")),
      ValueKind::Enum(x) => write!(f, "{}",x),
      ValueKind::Set(x,el) => write!(f, "{{{}}}{}", x, el.map_or("".to_string(), |e| format!(":{}", e))),
      ValueKind::Map(x,y) => write!(f, "{{{}:{}}}",x,y),
      ValueKind::Record(x) => write!(f, "{{{}}}",x.iter().map(|(i,k)| format!("{}<{}>",i.to_string(),k)).collect::<Vec<String>>().join(" ")),
      ValueKind::Table(x,y) => {
        let size_str = if y > &0 { format!(":{}", y) } else { "".to_string() };
        write!(f, "|{}|{}",x.iter().map(|(i,k)| format!("{}<{}>",i.to_string(),k)).collect::<Vec<String>>().join(" "),size_str)
      }
      ValueKind::Tuple(x) => write!(f, "({})",x.iter().map(|x| format!("{}",x)).collect::<Vec<String>>().join(",")),
      ValueKind::Id => write!(f, "id"),
      ValueKind::Index => write!(f, "ix"),
      ValueKind::Reference(x) => write!(f, "{}",x),
      ValueKind::Atom(x) => write!(f, "`{}",x),
      ValueKind::Empty => write!(f, "_"),
      ValueKind::Any => write!(f, "_"),
      ValueKind::Option(x) => write!(f, "{}?", x),
    }
  }
}

impl ValueKind {

  pub fn collection_kind(&self) -> Option<ValueKind> {
    match self {
      ValueKind::Matrix(x,_) => Some(*x.clone()),
      ValueKind::Set(x,_) => Some(*x.clone()),
      _ => None,
    }
  }

  pub fn deref_kind(&self) -> ValueKind {
    match self {
      ValueKind::Reference(x) => *x.clone(),
      _ => self.clone(),
    }
  }

  pub fn is_convertible_to(&self, other: &ValueKind) -> bool {
    use ValueKind::*;
    match (self, other) {
      // Unsigned widening
      (U8, U16) | (U8, U32) | (U8, U64) | (U8, U128) |
      (U16, U32) | (U16, U64) | (U16, U128) |
      (U32, U64) | (U32, U128) |
      (U64, U128) => true,

      // Signed widening
      (I8, I16) | (I8, I32) | (I8, I64) | (I8, I128) |
      (I16, I32) | (I16, I64) | (I16, I128) |
      (I32, I64) | (I32, I128) |
      (I64, I128) => true,

      // Unsigned -> signed widening
      (U8, I16) | (U8, I32) | (U8, I64) | (U8, I128) |
      (U16, I32) | (U16, I64) | (U16, I128) |
      (U32, I64) | (U32, I128) |
      (U64, I128) => true,

      // Signed -> unsigned widening (runtime safety not enforced here)
      (I8, U16) | (I8, U32) | (I8, U64) | (I8, U128) |
      (I16, U32) | (I16, U64) | (I16, U128) |
      (I32, U64) | (I32, U128) |
      (I64, U128) => true,

      // Integer -> float
      (U8, F32) | (U8, F64) |
      (U16, F32) | (U16, F64) |
      (U32, F32) | (U32, F64) |
      (U64, F32) | (U64, F64) |
      (U128, F32) | (U128, F64) |
      (I8, F32) | (I8, F64) |
      (I16, F32) | (I16, F64) |
      (I32, F32) | (I32, F64) |
      (I64, F32) | (I64, F64) |
      (I128, F32) | (I128, F64) => true,

      // Float widening + narrowing
      (F32, F64) | (F64, F32) => true,

      // Float -> integer (allowed, but lossy)
      (F32, I8) | (F32, I16) | (F32, I32) | (F32, I64) | (F32, I128) |
      (F32, U8) | (F32, U16) | (F32, U32) | (F32, U64) | (F32, U128) |
      (F64, I8) | (F64, I16) | (F64, I32) | (F64, I64) | (F64, I128) |
      (F64, U8) | (F64, U16) | (F64, U32) | (F64, U64) | (F64, U128) => true,

      // Index conversions (both ways)
      (Index, U8) | (Index, U16) | (Index, U32) | (Index, U64) | (Index, U128) |
      (Index, I8) | (Index, I16) | (Index, I32) | (Index, I64) | (Index, I128) |
      (Index, F32) | (Index, F64) |
      (U8, Index) | (U16, Index) | (U32, Index) | (U64, Index) | (U128, Index) |
      (I8, Index) | (I16, Index) | (I32, Index) | (I64, Index) | (I128, Index) => true,

      // Matrix: element type convertible and shape matches
      (Matrix(box a, ashape), Matrix(box b, bshape)) if ashape.into_iter().product::<usize>() == bshape.into_iter().product::<usize>() && a.is_convertible_to(b) => true,

      // Option conversions
      (Option(box a), Option(box b)) if a.is_convertible_to(b) => true,

      // Reference conversions
      (Reference(box a), Reference(box b)) if a.is_convertible_to(b) => true,

      // Tuple conversions (element-wise)
      (Tuple(a), Tuple(b)) if a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.is_convertible_to(y)) => true,

      // Set conversions
      (Set(box a, _), Set(box b, _)) if a.is_convertible_to(b) => true,

      // Map conversions
      (Map(box ak, box av), Map(box bk, box bv)) if ak.is_convertible_to(bk) && av.is_convertible_to(bv) => true,

      // Table conversions: allow source to have extra columns
      (Table(acols, _), Table(bcols, _)) if bcols.iter().all(|(bk, bv)| 
        acols.iter().any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
      ) => true,

      // Record conversions: allow source to have extra fields
      (Record(afields), Record(bfields)) if bfields.iter().all(|(bk, bv)| 
        afields.iter().any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
      ) => true,

      // Direct match
      _ => self == other,
    }
  }

  pub fn is_compatible(k1: ValueKind, k2: ValueKind) -> bool {
    match k1 {
      ValueKind::Reference(x) => {
        ValueKind::is_compatible(*x,k2)
      }
      ValueKind::Matrix(x,_) => {
        *x == k2
      }
      x => x == k2,
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  #[cfg(feature = "u8")]
  U8(Ref<u8>),
  #[cfg(feature = "u16")]
  U16(Ref<u16>),
  #[cfg(feature = "u32")]
  U32(Ref<u32>),
  #[cfg(feature = "u64")]
  U64(Ref<u64>),
  #[cfg(feature = "u128")]
  U128(Ref<u128>),
  #[cfg(feature = "i8")]
  I8(Ref<i8>),
  #[cfg(feature = "i16")]
  I16(Ref<i16>),
  #[cfg(feature = "i32")]
  I32(Ref<i32>),
  #[cfg(feature = "i64")]
  I64(Ref<i64>),
  #[cfg(feature = "i128")]
  I128(Ref<i128>),
  #[cfg(feature = "f32")]
  F32(Ref<F32>),
  #[cfg(feature = "f64")]
  F64(Ref<F64>),
  #[cfg(feature = "string")]
  String(Ref<String>),
  #[cfg(feature = "bool")]
  Bool(Ref<bool>),
  #[cfg(feature = "atom")]
  Atom(u64),
  #[cfg(feature = "matrix")]
  MatrixIndex(Matrix<usize>),
  #[cfg(all(feature = "matrix", feature = "bool"))]
  MatrixBool(Matrix<bool>),
  #[cfg(all(feature = "matrix", feature = "u8"))]
  MatrixU8(Matrix<u8>),
  #[cfg(all(feature = "matrix", feature = "u16"))]
  MatrixU16(Matrix<u16>),
  #[cfg(all(feature = "matrix", feature = "u32"))]
  MatrixU32(Matrix<u32>),
  #[cfg(all(feature = "matrix", feature = "u64"))]
  MatrixU64(Matrix<u64>),
  #[cfg(all(feature = "matrix", feature = "u128"))]
  MatrixU128(Matrix<u128>),
  #[cfg(all(feature = "matrix", feature = "i8"))]
  MatrixI8(Matrix<i8>),
  #[cfg(all(feature = "matrix", feature = "i16"))]
  MatrixI16(Matrix<i16>),
  #[cfg(all(feature = "matrix", feature = "i32"))]
  MatrixI32(Matrix<i32>),
  #[cfg(all(feature = "matrix", feature = "i64"))]
  MatrixI64(Matrix<i64>),
  #[cfg(all(feature = "matrix", feature = "i128"))]
  MatrixI128(Matrix<i128>),
  #[cfg(all(feature = "matrix", feature = "f32"))]
  MatrixF32(Matrix<F32>),
  #[cfg(all(feature = "matrix", feature = "f64"))]
  MatrixF64(Matrix<F64>),
  #[cfg(all(feature = "matrix", feature = "string"))]
  MatrixString(Matrix<String>),
  #[cfg(all(feature = "matrix", feature = "rational"))]
  MatrixRationalNumber(Matrix<RationalNumber>),
  #[cfg(all(feature = "matrix", feature = "complex"))]
  MatrixComplexNumber(Matrix<ComplexNumber>),
  #[cfg(feature = "matrix")]
  MatrixValue(Matrix<Value>),
  #[cfg(feature = "complex")]
  ComplexNumber(Ref<ComplexNumber>),
  #[cfg(feature = "rational")]
  RationalNumber(Ref<RationalNumber>),
  #[cfg(feature = "set")]
  Set(MechSet),
  #[cfg(feature = "map")]
  Map(MechMap),
  #[cfg(feature = "record")]
  Record(Ref<MechRecord>),
  #[cfg(feature = "table")]
  Table(Ref<MechTable>),
  #[cfg(feature = "tuple")]
  Tuple(MechTuple),
  #[cfg(feature = "enum")]
  Enum(Box<MechEnum>),
  Id(u64),
  Index(Ref<usize>),
  MutableReference(MutableReference),
  Kind(ValueKind),
  IndexAll,
  Empty
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if cfg!(feature = "pretty_print") {
      #[cfg(feature = "pretty_print")]
      return self.pretty_print().fmt(f);
      "".to_string().fmt(f) // kind of a hack to assuage the compiler
    } else {
      write!(f, "{:?}", self)
    }
  }
}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => x.borrow().hash(state),
      #[cfg(feature = "u8")]
      Value::U8(x)   => x.borrow().hash(state),
      #[cfg(feature = "u16")]
      Value::U16(x)  => x.borrow().hash(state),
      #[cfg(feature = "u32")]
      Value::U32(x)  => x.borrow().hash(state),
      #[cfg(feature = "u64")]
      Value::U64(x)  => x.borrow().hash(state),
      #[cfg(feature = "u128")]
      Value::U128(x) => x.borrow().hash(state),
      #[cfg(feature = "i8")]
      Value::I8(x)   => x.borrow().hash(state),
      #[cfg(feature = "i16")]
      Value::I16(x)  => x.borrow().hash(state),
      #[cfg(feature = "i32")]
      Value::I32(x)  => x.borrow().hash(state),
      #[cfg(feature = "i64")]
      Value::I64(x)  => x.borrow().hash(state),
      #[cfg(feature = "i128")]
      Value::I128(x) => x.borrow().hash(state),
      #[cfg(feature = "f32")]
      Value::F32(x)  => x.borrow().hash(state),
      #[cfg(feature = "f64")]
      Value::F64(x)  => x.borrow().hash(state),
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => x.borrow().hash(state),
      #[cfg(feature = "bool")]
      Value::Bool(x) => x.borrow().hash(state),
      #[cfg(feature = "atom")]
      Value::Atom(x) => x.hash(state),
      #[cfg(feature = "set")]
      Value::Set(x)  => x.hash(state),
      #[cfg(feature = "map")]
      Value::Map(x)  => x.hash(state),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().hash(state),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.hash(state),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().hash(state),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.hash(state),
      #[cfg(feature = "string")]
      Value::String(x) => x.borrow().hash(state),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.hash(state),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x)   => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x)   => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.hash(state),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(x) => x.hash(state),
      Value::Id(x)   => x.hash(state),
      Value::Kind(x) => x.hash(state),
      Value::Index(x)=> x.borrow().hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Empty => Value::Empty.hash(state),
      Value::IndexAll => Value::IndexAll.hash(state),
    }
  }
}

impl Value {

  pub fn convert_to(&self, other: &ValueKind) -> Option<Value> {

    if self.kind() == *other {
        return Some(self.clone());
    }

    if !self.kind().is_convertible_to(other) {
        return None;
    }

    match (self, other) {
    // ==== Unsigned widening and narrowing ====
    #[cfg(all(feature = "u8", feature = "u16"))]
    (Value::U8(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "u8", feature = "u32"))]
    (Value::U8(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "u8", feature = "u64"))]
    (Value::U8(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "u8", feature = "u128"))]
    (Value::U8(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "u8", feature = "i16"))]
    (Value::U8(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "u8", feature = "i32"))]
    (Value::U8(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "u8", feature = "i64"))]
    (Value::U8(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "u8", feature = "i128"))]
    (Value::U8(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "u8", feature = "f32"))]
    (Value::U8(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "u8", feature = "f64"))]
    (Value::U8(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "u16", feature = "u8"))]
    (Value::U16(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "u16", feature = "u32"))]
    (Value::U16(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "u16", feature = "u64"))]
    (Value::U16(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "u16", feature = "u128"))]
    (Value::U16(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "u16", feature = "i8"))]
    (Value::U16(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "u16", feature = "i32"))]
    (Value::U16(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "u16", feature = "i64"))]
    (Value::U16(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "u16", feature = "i128"))]
    (Value::U16(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "u16", feature = "f32"))]
    (Value::U16(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "u16", feature = "f64"))]
    (Value::U16(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "u32", feature = "u8"))]
    (Value::U32(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "u32", feature = "u16"))]
    (Value::U32(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "u32", feature = "u64"))]
    (Value::U32(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "u32", feature = "u128"))]
    (Value::U32(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "u32", feature = "i8"))]
    (Value::U32(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "u32", feature = "i16"))]
    (Value::U32(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "u32", feature = "i64"))]
    (Value::U32(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "u32", feature = "i128"))]
    (Value::U32(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "u32", feature = "f32"))]
    (Value::U32(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "u32", feature = "f64"))]
    (Value::U32(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "u64", feature = "u8"))]
    (Value::U64(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "u64", feature = "u16"))]
    (Value::U64(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "u64", feature = "u32"))]
    (Value::U64(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "u64", feature = "u128"))]
    (Value::U64(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "u64", feature = "i8"))]
    (Value::U64(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "u64", feature = "i16"))]
    (Value::U64(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "u64", feature = "i32"))]
    (Value::U64(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "u64", feature = "i128"))]
    (Value::U64(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "u64", feature = "f32"))]
    (Value::U64(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "u64", feature = "f64"))]
    (Value::U64(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "u128", feature = "u8"))]
    (Value::U128(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "u128", feature = "u16"))]
    (Value::U128(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "u128", feature = "u32"))]
    (Value::U128(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "u128", feature = "u64"))]
    (Value::U128(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "u128", feature = "i8"))]
    (Value::U128(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "u128", feature = "i16"))]
    (Value::U128(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "u128", feature = "i32"))]
    (Value::U128(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "u128", feature = "i64"))]
    (Value::U128(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "u128", feature = "f32"))]
    (Value::U128(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "u128", feature = "f64"))]
    (Value::U128(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    // ==== Signed widening and narrowing ====
    #[cfg(all(feature = "i8", feature = "i16"))]
    (Value::I8(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "i8", feature = "i32"))]
    (Value::I8(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "i8", feature = "i64"))]
    (Value::I8(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "i8", feature = "i128"))]
    (Value::I8(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "i8", feature = "u16"))]
    (Value::I8(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "i8", feature = "u32"))]
    (Value::I8(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "i8", feature = "u64"))]
    (Value::I8(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "i8", feature = "u128"))]
    (Value::I8(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "i8", feature = "f32"))]
    (Value::I8(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "i8", feature = "f64"))]
    (Value::I8(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "i16", feature = "i8"))]
    (Value::I16(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "i16", feature = "i32"))]
    (Value::I16(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "i16", feature = "i64"))]
    (Value::I16(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "i16", feature = "i128"))]
    (Value::I16(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "i16", feature = "u8"))]
    (Value::I16(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "i16", feature = "u32"))]
    (Value::I16(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "i16", feature = "u64"))]
    (Value::I16(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "i16", feature = "u128"))]
    (Value::I16(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "i16", feature = "f32"))]
    (Value::I16(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "i16", feature = "f64"))]
    (Value::I16(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "i32", feature = "i8"))]
    (Value::I32(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "i32", feature = "i16"))]
    (Value::I32(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "i32", feature = "i64"))]
    (Value::I32(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "i32", feature = "i128"))]
    (Value::I32(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "i32", feature = "u8"))]
    (Value::I32(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "i32", feature = "u16"))]
    (Value::I32(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "i32", feature = "u64"))]
    (Value::I32(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "i32", feature = "u128"))]
    (Value::I32(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "i32", feature = "f32"))]
    (Value::I32(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "i32", feature = "f64"))]
    (Value::I32(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "i64", feature = "i8"))]
    (Value::I64(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "i64", feature = "i16"))]
    (Value::I64(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "i64", feature = "i32"))]
    (Value::I64(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "i64", feature = "i128"))]
    (Value::I64(v), ValueKind::I128) => Some(Value::I128(new_ref((*v.borrow()) as i128))),
    #[cfg(all(feature = "i64", feature = "u8"))]
    (Value::I64(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "i64", feature = "u16"))]
    (Value::I64(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "i64", feature = "u32"))]
    (Value::I64(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "i64", feature = "u128"))]
    (Value::I64(v), ValueKind::U128) => Some(Value::U128(new_ref((*v.borrow()) as u128))),
    #[cfg(all(feature = "i64", feature = "f32"))]
    (Value::I64(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "i64", feature = "f64"))]
    (Value::I64(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    #[cfg(all(feature = "i128", feature = "i8"))]
    (Value::I128(v), ValueKind::I8) => Some(Value::I8(new_ref((*v.borrow()) as i8))),
    #[cfg(all(feature = "i128", feature = "i16"))]
    (Value::I128(v), ValueKind::I16) => Some(Value::I16(new_ref((*v.borrow()) as i16))),
    #[cfg(all(feature = "i128", feature = "i32"))]
    (Value::I128(v), ValueKind::I32) => Some(Value::I32(new_ref((*v.borrow()) as i32))),
    #[cfg(all(feature = "i128", feature = "i64"))]
    (Value::I128(v), ValueKind::I64) => Some(Value::I64(new_ref((*v.borrow()) as i64))),
    #[cfg(all(feature = "i128", feature = "u8"))]
    (Value::I128(v), ValueKind::U8) => Some(Value::U8(new_ref((*v.borrow()) as u8))),
    #[cfg(all(feature = "i128", feature = "u16"))]
    (Value::I128(v), ValueKind::U16) => Some(Value::U16(new_ref((*v.borrow()) as u16))),
    #[cfg(all(feature = "i128", feature = "u32"))]
    (Value::I128(v), ValueKind::U32) => Some(Value::U32(new_ref((*v.borrow()) as u32))),
    #[cfg(all(feature = "i128", feature = "u64"))]
    (Value::I128(v), ValueKind::U64) => Some(Value::U64(new_ref((*v.borrow()) as u64))),
    #[cfg(all(feature = "i128", feature = "f32"))]
    (Value::I128(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(*v.borrow() as f32)))),
    #[cfg(all(feature = "i128", feature = "f64"))]
    (Value::I128(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(*v.borrow() as f64)))),

    // ==== Float widening and narrowing ====
    #[cfg(all(feature = "f32", feature = "f64"))]
    (Value::F32(v), ValueKind::F64) => Some(Value::F64(new_ref(F64::new(v.borrow().0 as f64)))),
    #[cfg(all(feature = "f32", feature = "f64"))]
    (Value::F64(v), ValueKind::F32) => Some(Value::F32(new_ref(F32::new(v.borrow().0 as f32)))),

    // ==== Float to integer conversions (truncate) ====
    #[cfg(all(feature = "f32", feature = "i8"))]
    (Value::F32(v), ValueKind::I8) => Some(Value::I8(new_ref(v.borrow().0 as i8))),
    #[cfg(all(feature = "f32", feature = "i16"))]
    (Value::F32(v), ValueKind::I16) => Some(Value::I16(new_ref(v.borrow().0 as i16))),
    #[cfg(all(feature = "f32", feature = "i32"))]
    (Value::F32(v), ValueKind::I32) => Some(Value::I32(new_ref(v.borrow().0 as i32))),
    #[cfg(all(feature = "f32", feature = "i64"))]
    (Value::F32(v), ValueKind::I64) => Some(Value::I64(new_ref(v.borrow().0 as i64))),
    #[cfg(all(feature = "f32", feature = "i128"))]
    (Value::F32(v), ValueKind::I128) => Some(Value::I128(new_ref(v.borrow().0 as i128))),
    #[cfg(all(feature = "f32", feature = "u8"))]
    (Value::F32(v), ValueKind::U8) => Some(Value::U8(new_ref(v.borrow().0 as u8))),
    #[cfg(all(feature = "f32", feature = "u16"))]
    (Value::F32(v), ValueKind::U16) => Some(Value::U16(new_ref(v.borrow().0 as u16))),
    #[cfg(all(feature = "f32", feature = "u32"))]
    (Value::F32(v), ValueKind::U32) => Some(Value::U32(new_ref(v.borrow().0 as u32))),
    #[cfg(all(feature = "f32", feature = "u64"))]
    (Value::F32(v), ValueKind::U64) => Some(Value::U64(new_ref(v.borrow().0 as u64))),
    #[cfg(all(feature = "f32", feature = "u128"))]
    (Value::F32(v), ValueKind::U128) => Some(Value::U128(new_ref(v.borrow().0 as u128))),

    #[cfg(all(feature = "f64", feature = "i8"))]
    (Value::F64(v), ValueKind::I8) => Some(Value::I8(new_ref(v.borrow().0 as i8))),
    #[cfg(all(feature = "f64", feature = "i16"))]
    (Value::F64(v), ValueKind::I16) => Some(Value::I16(new_ref(v.borrow().0 as i16))),
    #[cfg(all(feature = "f64", feature = "i32"))]
    (Value::F64(v), ValueKind::I32) => Some(Value::I32(new_ref(v.borrow().0 as i32))),
    #[cfg(all(feature = "f64", feature = "i64"))]
    (Value::F64(v), ValueKind::I64) => Some(Value::I64(new_ref(v.borrow().0 as i64))),
    #[cfg(all(feature = "f64", feature = "i128"))]
    (Value::F64(v), ValueKind::I128) => Some(Value::I128(new_ref(v.borrow().0 as i128))),
    #[cfg(all(feature = "f64", feature = "u8"))]
    (Value::F64(v), ValueKind::U8) => Some(Value::U8(new_ref(v.borrow().0 as u8))),
    #[cfg(all(feature = "f64", feature = "u16"))]
    (Value::F64(v), ValueKind::U16) => Some(Value::U16(new_ref(v.borrow().0 as u16))),
    #[cfg(all(feature = "f64", feature = "u32"))]
    (Value::F64(v), ValueKind::U32) => Some(Value::U32(new_ref(v.borrow().0 as u32))),
    #[cfg(all(feature = "f64", feature = "u64"))]
    (Value::F64(v), ValueKind::U64) => Some(Value::U64(new_ref(v.borrow().0 as u64))),
    #[cfg(all(feature = "f64", feature = "u128"))]
    (Value::F64(v), ValueKind::U128) => Some(Value::U128(new_ref(v.borrow().0 as u128))),

      /*
      // ==== INDEX conversions ====
      (Value::Index(i), U32) => Some(Value::U32(new_ref((*i.borrow()) as u32))),
      (Value::U32(v), Index) => Some(Value::Index(new_ref((*v.borrow()) as usize))),


      // ==== MATRIX conversions (element-wise) ====
      (Value::MatrixU8(m), MatrixU16) => Some(Value::MatrixU16(m.map(|x| *x as u16))),
      (Value::MatrixI32(m), MatrixF64) => Some(Value::MatrixF64(m.map(|x| (*x) as f64))),
      // You can expand other matrix conversions similarly...

      // ==== COMPLEX TYPES (stubs) ====
      (Value::Set(set), Set(_)) => Some(Value::Set(set.clone())), // TODO: element-wise convert
      (Value::Map(map), Map(_)) => Some(Value::Map(map.clone())), // TODO: key/value convert
      (Value::Record(r), Record(_)) => Some(Value::Record(r.clone())), // TODO: field convert
      (Value::Table(t), Table(_)) => Some(Value::Table(t.clone())), // TODO: column convert

      // ==== ENUM, KIND ====
      (Value::Enum(e), Enum(_)) => Some(Value::Enum(e.clone())),
      (Value::Kind(k), Kind(_)) => Some(Value::Kind(k.clone())),

      // ==== SPECIAL CASES ====
      (Value::IndexAll, IndexAll) => Some(Value::IndexAll),
      (Value::Empty, Empty) => Some(Value::Empty),
      */
      // ==== FALLBACK ====
      _ => None,
    }
  }

  pub fn size_of(&self) -> usize {
    match self {
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => 16,
      #[cfg(feature = "u8")]
      Value::U8(x) => 1,
      #[cfg(feature = "u16")]
      Value::U16(x) => 2,
      #[cfg(feature = "u32")]
      Value::U32(x) => 4,
      #[cfg(feature = "u64")]
      Value::U64(x) => 8,
      #[cfg(feature = "u128")]
      Value::U128(x) => 16,
      #[cfg(feature = "i8")]
      Value::I8(x) => 1,
      #[cfg(feature = "i16")]
      Value::I16(x) => 2,
      #[cfg(feature = "i32")]
      Value::I32(x) => 4,
      #[cfg(feature = "i64")]
      Value::I64(x) => 8,
      #[cfg(feature = "i128")]
      Value::I128(x) => 16,
      #[cfg(feature = "f32")]
      Value::F32(x) => 4,
      #[cfg(feature = "f64")]
      Value::F64(x) => 8,
      #[cfg(feature = "bool")]
      Value::Bool(x) => 1,
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => 16,
      #[cfg(all(feature = "matrix"))]
      Value::MatrixIndex(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x)   => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x)   => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x)  => x.size_of(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(x) => x.size_of(),
      #[cfg(feature = "string")]
      Value::String(x) => x.borrow().len(),
      #[cfg(feature = "atom")]
      Value::Atom(x) => 8,
      #[cfg(feature = "set")]
      Value::Set(x) => x.size_of(),
      #[cfg(feature = "map")]
      Value::Map(x) => x.size_of(),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().size_of(),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().size_of(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.size_of(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.size_of(),
      Value::MutableReference(x) => x.borrow().size_of(),
      Value::Id(_) => 8,
      Value::Index(x) => 8,
      Value::Kind(_) => 0, // Kind is not a value, so it has no size
      Value::Empty => 0,
      Value::IndexAll => 0, // IndexAll is a special value, so it has no size
    }
  }

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u16")]
      Value::U16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u32")]
      Value::U32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u64")]
      Value::U64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i8")]
      Value::I8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i128")]
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i16")]
      Value::I16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i32")]
      Value::I32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i64")]
      Value::I64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i128")]
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "f32")]
      Value::F32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "f64")]
      Value::F64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "string")]
      Value::String(s) => format!("<span class='mech-string'>\"{}\"</span>", s.borrow()),
      #[cfg(feature = "bool")]
      Value::Bool(b) => format!("<span class='mech-boolean'>{}</span>", b.borrow()),
      #[cfg(feature = "complex")]
      Value::ComplexNumber(c) => c.borrow().to_html(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(m) => m.to_html(),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(m) => m.to_html(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(m) => m.to_html(),
      #[cfg(feature = "atom")]
      Value::Atom(a) => format!("<span class=\"mech-atom\"><span class=\"mech-atom-grave\">`</span><span class=\"mech-atom-name\">{}</span></span>",a),
      #[cfg(feature = "set")]
      Value::Set(s) => s.to_html(),
      #[cfg(feature = "map")]
      Value::Map(m) => m.to_html(),
      #[cfg(feature = "table")]
      Value::Table(t) => t.borrow().to_html(),
      #[cfg(feature = "record")]
      Value::Record(r) => r.borrow().to_html(),
      #[cfg(feature = "tuple")]
      Value::Tuple(t) => t.to_html(),
      #[cfg(feature = "enum")]
      Value::Enum(e) => e.to_html(),
      Value::MutableReference(m) => {
        let inner = m.borrow();
        format!("<span class='mech-reference'>{}</span>", inner.to_html())
      },
      _ => "???".to_string(),
    }
  }

  pub fn shape(&self) -> Vec<usize> {
    match self {
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => vec![1,1],
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => vec![1,1],
      #[cfg(feature = "u8")]
      Value::U8(x) => vec![1,1],
      #[cfg(feature = "u16")]
      Value::U16(x) => vec![1,1],
      #[cfg(feature = "u32")]
      Value::U32(x) => vec![1,1],
      #[cfg(feature = "u64")]
      Value::U64(x) => vec![1,1],
      #[cfg(feature = "u128")]
      Value::U128(x) => vec![1,1],
      #[cfg(feature = "i8")]
      Value::I8(x) => vec![1,1],
      #[cfg(feature = "i16")]
      Value::I16(x) => vec![1,1],
      #[cfg(feature = "i32")]
      Value::I32(x) => vec![1,1],
      #[cfg(feature = "i64")]
      Value::I64(x) => vec![1,1],
      #[cfg(feature = "i128")]
      Value::I128(x) => vec![1,1],
      #[cfg(feature = "f32")]
      Value::F32(x) => vec![1,1],
      #[cfg(feature = "f64")]
      Value::F64(x) => vec![1,1],
      #[cfg(feature = "string")]
      Value::String(x) => vec![1,1],
      #[cfg(feature = "bool")]
      Value::Bool(x) => vec![1,1],
      #[cfg(feature = "atom")]
      Value::Atom(x) => vec![1,1],
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.shape(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(x) => x.shape(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => vec![1,1],
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().shape(),
      #[cfg(feature = "set")]
      Value::Set(x) => vec![1,x.set.len()],
      #[cfg(feature = "map")]
      Value::Map(x) => vec![1,x.map.len()],
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().shape(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => vec![1,x.size()],
      Value::Index(x) => vec![1,1],
      Value::MutableReference(x) => x.borrow().shape(),
      Value::Empty => vec![0,0],
      Value::IndexAll => vec![0,0],
      Value::Kind(_) => vec![0,0],
      Value::Id(x) => vec![0,0],
    }
  }

  pub fn deref_kind(&self) -> ValueKind {
    match self {
      Value::MutableReference(x) => x.borrow().kind(),
      x => x.kind(),
    }
  }

  pub fn kind(&self) -> ValueKind {
    match self {
      #[cfg(feature = "complex")]
      Value::ComplexNumber(_) => ValueKind::ComplexNumber,
      #[cfg(feature = "rational")]
      Value::RationalNumber(_) => ValueKind::RationalNumber,
      #[cfg(feature = "u8")]
      Value::U8(_) => ValueKind::U8,
      #[cfg(feature = "u16")]
      Value::U16(_) => ValueKind::U16,
      #[cfg(feature = "u32")]
      Value::U32(_) => ValueKind::U32,
      #[cfg(feature = "u64")]
      Value::U64(_) => ValueKind::U64,
      #[cfg(feature = "u128")]
      Value::U128(_) => ValueKind::U128,
      #[cfg(feature = "i8")]
      Value::I8(_) => ValueKind::I8,
      #[cfg(feature = "i16")]
      Value::I16(_) => ValueKind::I16,
      #[cfg(feature = "i32")]
      Value::I32(_) => ValueKind::I32,
      #[cfg(feature = "i64")]
      Value::I64(_) => ValueKind::I64,
      #[cfg(feature = "i128")]
      Value::I128(_) => ValueKind::I128,
      #[cfg(feature = "f32")]
      Value::F32(_) => ValueKind::F32,
      #[cfg(feature = "f64")]
      Value::F64(_) => ValueKind::F64,
      #[cfg(feature = "string")]
      Value::String(_) => ValueKind::String,
      #[cfg(feature = "bool")]
      Value::Bool(_) => ValueKind::Bool,
      #[cfg(feature = "atom")]
      Value::Atom(x) => ValueKind::Atom(*x),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x) => ValueKind::Matrix(Box::new(ValueKind::Any),x.shape()),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => ValueKind::Matrix(Box::new(ValueKind::Index),x.shape()),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => ValueKind::Matrix(Box::new(ValueKind::Bool), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x) => ValueKind::Matrix(Box::new(ValueKind::U8), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x) => ValueKind::Matrix(Box::new(ValueKind::U16), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x) => ValueKind::Matrix(Box::new(ValueKind::U32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x) => ValueKind::Matrix(Box::new(ValueKind::U64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => ValueKind::Matrix(Box::new(ValueKind::U128), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x) => ValueKind::Matrix(Box::new(ValueKind::I8), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x) => ValueKind::Matrix(Box::new(ValueKind::I16), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x) => ValueKind::Matrix(Box::new(ValueKind::I32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x) => ValueKind::Matrix(Box::new(ValueKind::I64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => ValueKind::Matrix(Box::new(ValueKind::I128), x.shape()),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x) => ValueKind::Matrix(Box::new(ValueKind::F32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => ValueKind::Matrix(Box::new(ValueKind::F64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => ValueKind::Matrix(Box::new(ValueKind::String), x.shape()),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(x) => ValueKind::Matrix(Box::new(ValueKind::RationalNumber), x.shape()),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(x) => ValueKind::Matrix(Box::new(ValueKind::ComplexNumber), x.shape()),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().kind(),
      #[cfg(feature = "set")]
      Value::Set(x) => x.kind(),
      #[cfg(feature = "map")]
      Value::Map(x) => x.kind(),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().kind(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.kind(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.kind(),
      Value::MutableReference(x) => ValueKind::Reference(Box::new(x.borrow().kind())),
      Value::Empty => ValueKind::Empty,
      Value::IndexAll => ValueKind::Empty,
      Value::Id(x) => ValueKind::Id,
      Value::Index(x) => ValueKind::Index,
      Value::Kind(x) => x.clone(),
    }
  }

  #[cfg(feature = "matrix")]
  pub fn is_matrix(&self) -> bool {
    match self {
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(_) => true,
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(_) => true,
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(_) => true,
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(_) => true,
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(_) => true,
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(_) => true,
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(_) => true,
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(_) => true,
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(_) => true,
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(_) => true,
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(_) => true,
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(_) => true,
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(_) => true,
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(_) => true,
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(_) => true,
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixRationalNumber(_) => true,
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixComplexNumber(_) => true,
      #[cfg(feature = "matrix")]
      Value::MatrixValue(_) => true,
      _ => false,
    }
  }

  pub fn is_scalar(&self) -> bool {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(_) => true,
      #[cfg(feature = "u16")]
      Value::U16(_) => true,
      #[cfg(feature = "u32")]
      Value::U32(_) => true,
      #[cfg(feature = "u64")]
      Value::U64(_) => true,
      #[cfg(feature = "u128")]
      Value::U128(_) => true,
      #[cfg(feature = "i8")]
      Value::I8(_) => true,
      #[cfg(feature = "i16")]
      Value::I16(_) => true,
      #[cfg(feature = "i32")]
      Value::I32(_) => true,
      #[cfg(feature = "i64")]
      Value::I64(_) => true,
      #[cfg(feature = "i128")]
      Value::I128(_) => true,
      #[cfg(feature = "f32")]
      Value::F32(_) => true,
      #[cfg(feature = "f64")]
      Value::F64(_) => true,
      #[cfg(feature = "bool")]
      Value::Bool(_) => true,
      #[cfg(feature = "string")]
      Value::String(_) => true,
      #[cfg(feature = "atom")]
      Value::Atom(_) => true,
      Value::Index(_) => true,
      _ => false,
    }
  }

  #[cfg(feature = "bool")]
  pub fn as_bool(&self) -> Option<Ref<bool>> {if let Value::Bool(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_bool() } else { None }}
  
  #[cfg(feature = "i8")]
  impl_as_type!(i8);
  #[cfg(feature = "i16")]
  impl_as_type!(i16);
  #[cfg(feature = "i32")]
  impl_as_type!(i32);
  #[cfg(feature = "i64")]
  impl_as_type!(i64);
  #[cfg(feature = "i128")]
  impl_as_type!(i128);
  #[cfg(feature = "u8")]
  impl_as_type!(u8);
  #[cfg(feature = "u16")]
  impl_as_type!(u16);
  #[cfg(feature = "u32")]
  impl_as_type!(u32);
  #[cfg(feature = "u64")]
  impl_as_type!(u64);
  #[cfg(feature = "u128")]
  impl_as_type!(u128);

  #[cfg(feature = "string")]
  pub fn as_string(&self) -> Option<Ref<String>> {
    match self {
      #[cfg(feature = "string")]
      Value::String(v) => Some(v.clone()),
      #[cfg(feature = "u8")]
      Value::U8(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "u16")]
      Value::U16(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "u32")]
      Value::U32(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "u64")]
      Value::U64(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "i8")]
      Value::I8(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "i16")]
      Value::I16(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "i32")]
      Value::I32(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "i64")]
      Value::I64(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "f32")]
      Value::F32(v) => Some(new_ref(format!("{}", v.borrow().0))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Some(new_ref(format!("{}", v.borrow().0))),
      #[cfg(feature = "bool")]
      Value::Bool(v) => Some(new_ref(format!("{}", v.borrow()))),
      #[cfg(feature = "rational")]
      Value::RationalNumber(v) => Some(new_ref(v.borrow().to_string())),
      #[cfg(feature = "complex")]
      Value::ComplexNumber(v) => Some(new_ref(v.borrow().to_string())),
      Value::MutableReference(val) => val.borrow().as_string(),
      _ => None,
    }
  }

  #[cfg(feature = "rational")]
  pub fn as_rationalnumber(&self) -> Option<Ref<RationalNumber>> {
    match self {
      Value::RationalNumber(v) => Some(v.clone()),
      #[cfg(feature = "f32")]
      Value::F32(v) => Some(new_ref(RationalNumber::new(v.borrow().0 as i64, 1))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Some(new_ref(RationalNumber::new(v.borrow().0 as i64, 1))),
      #[cfg(feature = "u8")]
      Value::U8(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u16")]
      Value::U16(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u32")]
      Value::U32(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u64")]
      Value::U64(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i8")]
      Value::I8(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i16")]
      Value::I16(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i32")]
      Value::I32(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i64")]
      Value::I64(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(new_ref(RationalNumber::new(*v.borrow() as i64, 1))),
      Value::MutableReference(val) => val.borrow().as_rationalnumber(),
      _ => None,
    }
  }

  #[cfg(feature = "complex")]
  pub fn as_complexnumber(&self) -> Option<Ref<ComplexNumber>> {
    match self {
      Value::ComplexNumber(v) => Some(v.clone()),
      #[cfg(feature = "f32")]
      Value::F32(v) =>  Some(new_ref(ComplexNumber::new(v.borrow().0 as f64, 0.0))),
      #[cfg(feature = "f64")]
      Value::F64(v) =>  Some(new_ref(ComplexNumber::new(v.borrow().0, 0.0))),
      #[cfg(feature = "u8")]
      Value::U8(v) =>   Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u16")]
      Value::U16(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u32")]
      Value::U32(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u64")]
      Value::U64(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i8")]
      Value::I8(v) =>   Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i16")]
      Value::I16(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i32")]
      Value::I32(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i64")]
      Value::I64(v) =>  Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(new_ref(ComplexNumber::new(*v.borrow() as f64, 0.0))),
      Value::MutableReference(val) => val.borrow().as_complexnumber(),
      _ => None,
    }
  }

  #[cfg(feature = "f32")]
  pub fn as_f32(&self) -> Option<Ref<F32>> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "u16")]
      Value::U16(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "u32")]
      Value::U32(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "u64")]
      Value::U64(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "i8")]
      Value::I8(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "i16")]
      Value::I16(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "i32")]
      Value::I32(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "i64")]
      Value::I64(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      #[cfg(feature = "f32")]
      Value::F32(v) => Some(new_ref(F32::new((*v.borrow()).0 as f32))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Some(new_ref(F32::new((*v.borrow()).0 as f32))),
      Value::MutableReference(val) => val.borrow().as_f32(),
      _ => None,
    }
  }

  #[cfg(feature = "f64")]
  pub fn as_f64(&self) -> Option<Ref<F64>> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "u16")]
      Value::U16(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "u32")]
      Value::U32(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "u64")]
      Value::U64(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "i8")]
      Value::I8(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "i16")]
      Value::I16(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "i32")]
      Value::I32(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "i64")]
      Value::I64(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::F64(v) => Some(new_ref(F64::new((*v.borrow()).0 as f64))),
      Value::MutableReference(val) => val.borrow().as_f64(),
      _ => None,
    }
  }

  #[cfg(all(feature = "matrix", feature = "bool"))]
  pub fn as_vecbool(&self) -> Option<Vec<bool>> {if let Value::MatrixBool(v)  = self { Some(v.as_vec()) } else if let Value::Bool(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecbool()  } else { None }}
  
  #[cfg(all(feature = "matrix", feature = "f64"))]
  pub fn as_vecf64(&self) -> Option<Vec<F64>> { if let Value::MatrixF64(v) = self { Some(v.as_vec()) } else if let Value::F64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf64() } else if let Some(v) = self.as_f64() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "f32"))]
  pub fn as_vecf32(&self) -> Option<Vec<F32>> { if let Value::MatrixF32(v) = self { Some(v.as_vec()) } else if let Value::F32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf32() } else if let Some(v) = self.as_f32() { Some(vec![v.borrow().clone()]) } else { None } }

  #[cfg(all(feature = "matrix", feature = "u8"))]
  pub fn as_vecu8(&self) -> Option<Vec<u8>> { if let Value::MatrixU8(v) = self { Some(v.as_vec()) } else if let Value::U8(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu8() } else if let Some(v) = self.as_u8() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "u16"))]
  pub fn as_vecu16(&self) -> Option<Vec<u16>> { if let Value::MatrixU16(v) = self { Some(v.as_vec()) } else if let Value::U16(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu16() } else if let Some(v) = self.as_u16() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "u32"))]
  pub fn as_vecu32(&self) -> Option<Vec<u32>> { if let Value::MatrixU32(v) = self { Some(v.as_vec()) } else if let Value::U32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu32() } else if let Some(v) = self.as_u32() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "u64"))]
  pub fn as_vecu64(&self) -> Option<Vec<u64>> { if let Value::MatrixU64(v) = self { Some(v.as_vec()) } else if let Value::U64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu64() } else if let Some(v) = self.as_u64() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "u128"))]
  pub fn as_vecu128(&self) -> Option<Vec<u128>> { if let Value::MatrixU128(v) = self { Some(v.as_vec()) } else if let Value::U128(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu128() } else if let Some(v) = self.as_u128() { Some(vec![v.borrow().clone()]) } else { None } }

  #[cfg(all(feature = "matrix", feature = "i8"))]
  pub fn as_veci8(&self) -> Option<Vec<i8>> { if let Value::MatrixI8(v) = self { Some(v.as_vec()) } else if let Value::I8(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci8() } else if let Some(v) = self.as_i8() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "i16"))]
  pub fn as_veci16(&self) -> Option<Vec<i16>> { if let Value::MatrixI16(v) = self { Some(v.as_vec()) } else if let Value::I16(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci16() } else if let Some(v) = self.as_i16() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "i32"))]
  pub fn as_veci32(&self) -> Option<Vec<i32>> { if let Value::MatrixI32(v) = self { Some(v.as_vec()) } else if let Value::I32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci32() } else if let Some(v) = self.as_i32() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "i64"))]
  pub fn as_veci64(&self) -> Option<Vec<i64>> { if let Value::MatrixI64(v) = self { Some(v.as_vec()) } else if let Value::I64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci64() } else if let Some(v) = self.as_i64() { Some(vec![v.borrow().clone()]) } else { None } }
  #[cfg(all(feature = "matrix", feature = "i128"))]
  pub fn as_veci128(&self) -> Option<Vec<i128>> { if let Value::MatrixI128(v) = self { Some(v.as_vec()) } else if let Value::I128(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci128() } else if let Some(v) = self.as_i128() { Some(vec![v.borrow().clone()]) } else { None } }

  #[cfg(all(feature = "matrix", feature = "string"))]
  pub fn as_vecstring(&self) -> Option<Vec<String>> {if let Value::MatrixString(v)  = self { Some(v.as_vec()) } else if let Value::String(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecstring()  } else { None }}

  #[cfg(all(feature = "matrix", feature = "rational"))]
  pub fn as_vecrationalnumber(&self) -> Option<Vec<RationalNumber>> {if let Value::MatrixRationalNumber(v)  = self { Some(v.as_vec()) } else if let Value::RationalNumber(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecrationalnumber()  } else { None }}
  #[cfg(all(feature = "matrix", feature = "complex"))]
  pub fn as_veccomplexnumber(&self) -> Option<Vec<ComplexNumber>> {if let Value::MatrixComplexNumber(v)  = self { Some(v.as_vec()) } else if let Value::ComplexNumber(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veccomplexnumber()  } else { None }}

  pub fn as_vecusize(&self) -> Option<Vec<usize>> {
    match self {
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(v) => Some(v.as_vec()),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(v) => Some(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(v) => Some(v.as_vec().iter().map(|x| (*x).0 as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(v) => Some(v.as_vec().iter().map(|x| (*x).0 as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(_) => None,
      #[cfg(feature = "bool")]
      Value::Bool(_) => None,
      Value::MutableReference(x) => x.borrow().as_vecusize(),
      _ => todo!(),
    }
  }

  pub fn as_index(&self) -> MResult<Value> {
    match self.as_usize() {      
      Some(ix) => Ok(Value::Index(new_ref(ix))),
      #[cfg(feature = "matrix")]
      None => match self.as_vecusize() {
        #[cfg(feature = "matrix")]
        Some(x) => {
          let shape = self.shape();
          let out = Value::MatrixIndex(usize::to_matrix(x, shape[0] * shape[1],1 ));
          Ok(out)
        },
        #[cfg(all(feature = "matrix", feature = "bool"))]
        None => match self.as_vecbool() {
          Some(x) => {
            let shape = self.shape();
            let out = match (shape[0], shape[1]) {
              (1,1) => Value::Bool(new_ref(x[0])),
              (1,n) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
              (m,1) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
              (m,n) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
              _ => todo!(),
            };
            Ok(out)
          }
          None => match self.as_bool() {
            Some(x) => Ok(Value::Bool(x)),
            None => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledIndexKind}),
          }
        }
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
      }
      _ => todo!(),
    }
  }

  pub fn as_usize(&self) -> Option<usize> {
    match self {      
      Value::Index(v) => Some(*v.borrow()),
      #[cfg(feature = "u8")]
      Value::U8(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "u16")]
      Value::U16(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "u32")]
      Value::U32(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "u64")]
      Value::U64(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "u128")]
      Value::U128(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "i8")]
      Value::I8(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "i16")]
      Value::I16(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "i32")]
      Value::I32(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "i64")]
      Value::I64(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "i128")]
      Value::I128(v) => Some(*v.borrow() as usize),
      #[cfg(feature = "f32")]
      Value::F32(v) => Some((*v.borrow()).0 as usize),
      #[cfg(feature = "f64")]
      Value::F64(v) => Some((*v.borrow()).0 as usize),
      Value::MutableReference(v) => v.borrow().as_usize(),
      _ => None,
    }
  }

}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Value {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      #[cfg(feature = "u8")]
      Value::U8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u16")]
      Value::U16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u32")]
      Value::U32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u64")]
      Value::U64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u128")]
      Value::U128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i8")]
      Value::I8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i16")]
      Value::I16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i32")]
      Value::I32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i64")]
      Value::I64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i128")]
      Value::I128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "f32")]
      Value::F32(x)  => {builder.push_record(vec![format!("{}",x.borrow().0)]);},
      #[cfg(feature = "f64")]
      Value::F64(x)  => {builder.push_record(vec![format!("{}",x.borrow().0)]);},
      #[cfg(feature = "bool")]
      Value::Bool(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => {builder.push_record(vec![x.borrow().pretty_print()]);},
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => {builder.push_record(vec![format!("{}",x.borrow().pretty_print())]);},
      #[cfg(feature = "atom")]
      Value::Atom(x) => {builder.push_record(vec![format!("{}",x)]);},
      #[cfg(feature = "set")]
      Value::Set(x)  => {return x.pretty_print();}
      #[cfg(feature = "map")]
      Value::Map(x)  => {return x.pretty_print();}
      #[cfg(feature = "string")]
      Value::String(x) => {return format!("\"{}\"",x.borrow().clone());},
      #[cfg(feature = "table")]
      Value::Table(x)  => {return x.borrow().pretty_print();},
      #[cfg(feature = "tuple")]
      Value::Tuple(x)  => {return x.pretty_print();},
      #[cfg(feature = "record")]
      Value::Record(x) => {return x.borrow().pretty_print();},
      #[cfg(feature = "enum")]
      Value::Enum(x) => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => {return x.pretty_print();}
      #[cfg(feature = "matrix")]
      Value::MatrixBool(x) => {return x.pretty_print();}
      #[cfg(feature = "matrix")]
      Value::MatrixU8(x)   => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixU16(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixU32(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixU64(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixU128(x) => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixI8(x)   => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixI16(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixI32(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixI64(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixI128(x) => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixF32(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixF64(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixString(x)  => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixRationalNumber(x) => {return x.pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixComplexNumber(x) => {return x.pretty_print();},
      Value::Index(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::MutableReference(x) => {return x.borrow().pretty_print();},
      Value::Empty => builder.push_record(vec!["_"]),
      Value::IndexAll => builder.push_record(vec![":"]),
      Value::Id(x) => builder.push_record(vec![format!("{}",humanize(x))]),
      Value::Kind(x) => builder.push_record(vec![format!("{}",x)]),
    };
    let value_style = Style::empty()
      .top(' ')
      .left(' ')
      .right(' ')
      .bottom(' ')
      .vertical(' ')
      .intersection_bottom(' ')
      .corner_top_left(' ')
      .corner_top_right(' ')
      .corner_bottom_left(' ')
      .corner_bottom_right(' ');
    let mut table = builder.build();
    table.with(value_style);
    format!("{table}")
  }
}


pub trait ToIndex {
  fn to_index(&self) -> Value;
}

impl ToIndex for Ref<Vec<i64>> { fn to_index(&self) -> Value { (*self.borrow()).iter().map(|x| *x as usize).collect::<Vec<usize>>().to_value() } }

pub trait ToValue {
  fn to_value(&self) -> Value;
}

impl ToValue for Vec<usize> {
  fn to_value(&self) -> Value {
    match self.len() {
      1 => Value::Index(new_ref(self[0].clone())),
      //2 => Value::MatrixIndex(Matrix::RowVector2(new_ref(RowVector2::from_vec(self.clone())))),
      //3 => Value::MatrixIndex(Matrix::RowVector3(new_ref(RowVector3::from_vec(self.clone())))),
      //4 => Value::MatrixIndex(Matrix::RowVector4(new_ref(RowVector4::from_vec(self.clone())))),
      #[cfg(feature = "matrix")]
      n => Value::MatrixIndex(Matrix::DVector(new_ref(DVector::from_vec(self.clone())))),
      _ => todo!(),
    }
  }
}

impl ToValue for Ref<usize>  { fn to_value(&self) -> Value { Value::Index(self.clone())  } }
#[cfg(feature = "u8")]
impl ToValue for Ref<u8>     { fn to_value(&self) -> Value { Value::U8(self.clone())     } }
#[cfg(feature = "u16")]
impl ToValue for Ref<u16>    { fn to_value(&self) -> Value { Value::U16(self.clone())    } }
#[cfg(feature = "u32")]
impl ToValue for Ref<u32>    { fn to_value(&self) -> Value { Value::U32(self.clone())    } }
#[cfg(feature = "u64")]
impl ToValue for Ref<u64>    { fn to_value(&self) -> Value { Value::U64(self.clone())    } }
#[cfg(feature = "u128")]
impl ToValue for Ref<u128>   { fn to_value(&self) -> Value { Value::U128(self.clone())   } }
#[cfg(feature = "i8")]
impl ToValue for Ref<i8>     { fn to_value(&self) -> Value { Value::I8(self.clone())     } }
#[cfg(feature = "i16")]
impl ToValue for Ref<i16>    { fn to_value(&self) -> Value { Value::I16(self.clone())    } }
#[cfg(feature = "i32")]
impl ToValue for Ref<i32>    { fn to_value(&self) -> Value { Value::I32(self.clone())    } }
#[cfg(feature = "i64")]
impl ToValue for Ref<i64>    { fn to_value(&self) -> Value { Value::I64(self.clone())    } }
#[cfg(feature = "i128")]
impl ToValue for Ref<i128>   { fn to_value(&self) -> Value { Value::I128(self.clone())   } }
#[cfg(feature = "f32")]
impl ToValue for Ref<F32>    { fn to_value(&self) -> Value { Value::F32(self.clone())    } }
#[cfg(feature = "f64")]
impl ToValue for Ref<F64>    { fn to_value(&self) -> Value { Value::F64(self.clone())    } }
#[cfg(feature = "bool")]
impl ToValue for Ref<bool>   { fn to_value(&self) -> Value { Value::Bool(self.clone())   } }
#[cfg(feature = "string")]
impl ToValue for Ref<String> { fn to_value(&self) -> Value { Value::String(self.clone()) } }
#[cfg(feature = "rational")]
impl ToValue for Ref<RationalNumber> { fn to_value(&self) -> Value { Value::RationalNumber(self.clone()) } }
#[cfg(feature = "complex")]
impl ToValue for Ref<ComplexNumber> { fn to_value(&self) -> Value { Value::ComplexNumber(self.clone()) } }

macro_rules! to_value_ndmatrix {
  ($($nd_matrix_kind:ident, $matrix_kind:ident, $base_type:ty, $type_string:tt),+ $(,)?) => {
    $(
      #[cfg(all(feature = "matrix", feature = $type_string))]
      impl ToValue for Ref<$nd_matrix_kind<$base_type>> {
        fn to_value(&self) -> Value {
          Value::$matrix_kind(Matrix::<$base_type>::$nd_matrix_kind(self.clone()))
        }
      }
    )+
  };}

#[cfg(feature = "matrix")]
macro_rules! impl_to_value_matrix {
  ($matrix_kind:ident) => {
    to_value_ndmatrix!(
      $matrix_kind, MatrixIndex,  usize, "matrix",
      $matrix_kind, MatrixBool,   bool, "bool",
      $matrix_kind, MatrixI8,     i8, "i8",
      $matrix_kind, MatrixI16,    i16, "i16",
      $matrix_kind, MatrixI32,    i32, "i32",
      $matrix_kind, MatrixI64,    i64, "i64",
      $matrix_kind, MatrixI128,   i128, "i128",
      $matrix_kind, MatrixU8,     u8, "u8",
      $matrix_kind, MatrixU16,    u16, "u16",
      $matrix_kind, MatrixU32,    u32, "u32",
      $matrix_kind, MatrixU64,    u64, "u64",
      $matrix_kind, MatrixU128,   u128, "u128",
      $matrix_kind, MatrixF32,    F32, "f32",
      $matrix_kind, MatrixF64,    F64, "f64",
      $matrix_kind, MatrixString, String, "string",
      $matrix_kind, MatrixRationalNumber, RationalNumber, "rational",
      $matrix_kind, MatrixComplexNumber, ComplexNumber, "complex",
    );
  }
}

#[cfg(feature = "matrix2x3")]
impl_to_value_matrix!(Matrix2x3);
#[cfg(feature = "matrix3x2")]
impl_to_value_matrix!(Matrix3x2);
#[cfg(feature = "matrix1")]
impl_to_value_matrix!(Matrix1);
#[cfg(feature = "matrix2")]
impl_to_value_matrix!(Matrix2);
#[cfg(feature = "matrix3")]
impl_to_value_matrix!(Matrix3);
#[cfg(feature = "matrix4")]
impl_to_value_matrix!(Matrix4);
#[cfg(feature = "vector2")]
impl_to_value_matrix!(Vector2);
#[cfg(feature = "vector3")]
impl_to_value_matrix!(Vector3);
#[cfg(feature = "vector4")]
impl_to_value_matrix!(Vector4);
#[cfg(feature = "row_vector2")]
impl_to_value_matrix!(RowVector2);
#[cfg(feature = "row_vector3")]
impl_to_value_matrix!(RowVector3);
#[cfg(feature = "row_vector4")]
impl_to_value_matrix!(RowVector4);
#[cfg(feature = "row_vectord")]
impl_to_value_matrix!(RowDVector);
#[cfg(feature = "vectord")]
impl_to_value_matrix!(DVector);
#[cfg(feature = "matrixd")]
impl_to_value_matrix!(DMatrix);

#[cfg(feature = "u8")]
impl From<u8> for Value {
  fn from(val: u8) -> Self {
    Value::U8(new_ref(val))
  }
}

#[cfg(feature = "u16")]
impl From<u16> for Value {
  fn from(val: u16) -> Self {
    Value::U16(new_ref(val))
  }
}

#[cfg(feature = "u32")]
impl From<u32> for Value {
  fn from(val: u32) -> Self {
    Value::U32(new_ref(val))
  }
}

#[cfg(feature = "u64")]
impl From<u64> for Value {
  fn from(val: u64) -> Self {
    Value::U64(new_ref(val))
  }
}

#[cfg(feature = "u128")]
impl From<u128> for Value {
  fn from(val: u128) -> Self {
    Value::U128(new_ref(val))
  }
}

#[cfg(feature = "i8")]
impl From<i8> for Value {
  fn from(val: i8) -> Self {
    Value::I8(new_ref(val))
  }
}

#[cfg(feature = "i16")]
impl From<i16> for Value {
  fn from(val: i16) -> Self {
    Value::I16(new_ref(val))
  }
}

#[cfg(feature = "i32")]
impl From<i32> for Value {
  fn from(val: i32) -> Self {
    Value::I32(new_ref(val))
  }
}

#[cfg(feature = "i64")]
impl From<i64> for Value {
  fn from(val: i64) -> Self {
    Value::I64(new_ref(val))
  }
}

#[cfg(feature = "i128")]
impl From<i128> for Value {
  fn from(val: i128) -> Self {
    Value::I128(new_ref(val))
  }
}

#[cfg(feature = "bool")]
impl From<bool> for Value {
  fn from(val: bool) -> Self {
    Value::Bool(new_ref(val))
  }
}

#[cfg(feature = "string")]
impl From<String> for Value {
  fn from(val: String) -> Self {
    Value::String(new_ref(val))
  }
}

#[cfg(feature = "rational")]
impl From<RationalNumber> for Value {
  fn from(val: RationalNumber) -> Self {
    Value::RationalNumber(new_ref(val))
  }
}


pub trait ToUsize {
  fn to_usize(&self) -> usize;
}

macro_rules! impl_to_usize_for {
  ($t:ty) => {
    impl ToUsize for $t {
      fn to_usize(&self) -> usize {
        #[allow(unused_comparisons)]
        if *self < 0 as $t {
          panic!("Cannot convert negative number to usize");
        }
        *self as usize
      }
    }
  };
}

#[cfg(feature = "u8")]
impl_to_usize_for!(u8);
#[cfg(feature = "u16")]
impl_to_usize_for!(u16);
#[cfg(feature = "u32")]
impl_to_usize_for!(u32);
#[cfg(feature = "u64")]
impl_to_usize_for!(u64);
#[cfg(feature = "u128")]
impl_to_usize_for!(u128);
impl_to_usize_for!(usize);

#[cfg(feature = "i8")]
impl_to_usize_for!(i8);
#[cfg(feature = "i16")]
impl_to_usize_for!(i16);
#[cfg(feature = "i32")]
impl_to_usize_for!(i32);
#[cfg(feature = "i64")]
impl_to_usize_for!(i64);
#[cfg(feature = "i128")]
impl_to_usize_for!(i128);