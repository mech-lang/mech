use crate::matrix::Matrix;
use crate::*;
use crate::nodes::Matrix as Mat;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use std::collections::HashMap;

use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
use indexmap::map::*;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use paste::paste;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use serde::de::{self, Deserialize, SeqAccess, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

macro_rules! impl_as_type {
  ($target_type:ty) => {
    paste!{
      pub fn [<as_ $target_type>](&self) -> Option<Ref<$target_type>> {
        match self {
          Value::U8(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::U16(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::U32(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::U64(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::U128(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::I8(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::I16(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::I32(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::I64(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::I128(v) => Some(new_ref(*v.borrow() as $target_type)),
          Value::F32(v) => Some(new_ref((*v.borrow()).0 as $target_type)),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueKind {
  U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64, 
  String, Bool, Id, Index, Empty, Any, 
  Matrix(Box<ValueKind>,Vec<usize>),  Enum(u64),                  Record(Vec<(String,ValueKind)>),
  Map(Box<ValueKind>,Box<ValueKind>), Atom(u64),                  Table(Vec<(String,ValueKind)>, usize), 
  Tuple(Vec<ValueKind>),              Reference(Box<ValueKind>),  Set(Box<ValueKind>, Option<usize>), 
  Option(Box<ValueKind>),
}

impl std::fmt::Display for ValueKind {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
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
      ValueKind::Enum(x) => write!(f, "{:?}",x),
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
  String(Ref<String>),
  Bool(Ref<bool>),
  Atom(u64),
  MatrixIndex(Matrix<usize>),
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
  MatrixString(Matrix<String>),
  MatrixValue(Matrix<Value>),
  Set(MechSet),
  Map(MechMap),
  Record(Ref<MechRecord>),
  Table(Ref<MechTable>),
  Tuple(MechTuple),
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
    self.pretty_print().fmt(f)
  }
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
      Value::Index(x)=> x.borrow().hash(state),
      Value::Bool(x) => x.borrow().hash(state),
      Value::Atom(x) => x.hash(state),
      Value::Set(x)  => x.hash(state),
      Value::Map(x)  => x.hash(state),
      Value::Table(x) => x.borrow().hash(state),
      Value::Tuple(x) => x.hash(state),
      Value::Record(x) => x.borrow().hash(state),
      Value::Enum(x) => x.hash(state),
      Value::String(x) => x.borrow().hash(state),
      Value::MatrixBool(x) => x.hash(state),
      Value::MatrixIndex(x) => x.hash(state),
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
      Value::MatrixString(x) => x.hash(state),
      Value::MatrixValue(x)  => x.hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Empty => Value::Empty.hash(state),
      Value::IndexAll => Value::IndexAll.hash(state),
    }
  }
}

impl Value {

  pub fn size_of(&self) -> usize {
    match self {
      Value::U8(x) => 1,
      Value::U16(x) => 2,
      Value::U32(x) => 4,
      Value::U64(x) => 8,
      Value::U128(x) => 16,
      Value::I8(x) => 1,
      Value::I16(x) => 2,
      Value::I32(x) => 4,
      Value::I64(x) => 8,
      Value::I128(x) => 16,
      Value::F32(x) => 4,
      Value::F64(x) => 8,
      Value::Bool(x) => 1,
      Value::MatrixIndex(x) =>x.size_of(),
      Value::MatrixBool(x) =>x.size_of(),
      Value::MatrixU8(x)   => x.size_of(),
      Value::MatrixU16(x)  => x.size_of(),
      Value::MatrixU32(x)  => x.size_of(),
      Value::MatrixU64(x)  => x.size_of(),
      Value::MatrixU128(x) => x.size_of(),
      Value::MatrixI8(x)   => x.size_of(),
      Value::MatrixI16(x)  => x.size_of(),
      Value::MatrixI32(x)  => x.size_of(),
      Value::MatrixI64(x)  => x.size_of(),
      Value::MatrixI128(x) => x.size_of(),
      Value::MatrixF32(x)  => x.size_of(),
      Value::MatrixF64(x)  => x.size_of(),
      Value::MatrixValue(x)  => x.size_of(),
      Value::MatrixString(x) => x.size_of(),
      Value::String(x) => x.borrow().len(),
      Value::Atom(x) => 8,
      Value::Set(x) => x.size_of(),
      Value::Map(x) => x.size_of(),
      Value::Table(x) => x.borrow().size_of(),
      Value::Record(x) => x.borrow().size_of(),
      Value::Tuple(x) => x.size_of(),
      Value::Enum(x) => x.size_of(),
      Value::MutableReference(x) => x.borrow().size_of(),
      Value::Id(_) => 8,
      Value::Index(x) => 8,
      Value::Kind(_) => 0, // Kind is not a value, so it has no size
      Value::Empty => 0,
      Value::IndexAll => 0, // IndexAll is a special value, so it has no size
    }
  }

  pub fn to_html(&self) -> String {
    match self {
      Value::U8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::U16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::U32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::U64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::F32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::F64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      Value::String(s) => format!("<span class='mech-string'>\"{}\"</span>", s.borrow()),
      Value::Bool(b) => format!("<span class='mech-boolean'>{}</span>", b.borrow()),
      Value::MatrixU8(m) => m.to_html(),
      Value::MatrixU16(m) => m.to_html(),
      Value::MatrixU32(m) => m.to_html(),
      Value::MatrixU64(m) => m.to_html(),
      Value::MatrixU128(m) => m.to_html(),
      Value::MatrixI8(m) => m.to_html(),
      Value::MatrixI16(m) => m.to_html(),
      Value::MatrixI32(m) => m.to_html(),
      Value::MatrixI64(m) => m.to_html(),
      Value::MatrixI128(m) => m.to_html(),
      Value::MatrixF64(m) => m.to_html(),
      Value::MatrixF32(m) => m.to_html(),
      Value::MatrixIndex(m) => m.to_html(),
      Value::MatrixBool(m) => m.to_html(),
      Value::MatrixString(m) => m.to_html(),
      Value::MatrixValue(m) => m.to_html(),
      Value::MutableReference(m) => {
        let inner = m.borrow();
        format!("<span class='mech-reference'>{}</span>", inner.to_html())
      },
      Value::Atom(a) => format!("<span class=\"mech-atom\"><span class=\"mech-atom-grave\">`</span><span class=\"mech-atom-name\">{}</span></span>",a),
      Value::Set(s) => s.to_html(),
      Value::Map(m) => m.to_html(),
      Value::Table(t) => t.borrow().to_html(),
      Value::Record(r) => r.borrow().to_html(),
      Value::Tuple(t) => t.to_html(),
      Value::Enum(e) => e.to_html(),
      _ => "".to_string(),
    }
  }

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      Value::U8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::U16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::U32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::U64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::U128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::I8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::I16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::I32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::I64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::I128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::F32(x)  => {builder.push_record(vec![format!("{}",x.borrow().0)]);},
      Value::F64(x)  => {builder.push_record(vec![format!("{}",x.borrow().0)]);},
      Value::Bool(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::Index(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::Atom(x) => {builder.push_record(vec![format!("{}",x)]);},
      Value::Set(x)  => {return x.pretty_print();}
      Value::Map(x)  => {return x.pretty_print();}
      Value::String(x) => {return format!("\"{}\"",x.borrow().clone());},
      Value::Table(x)  => {return x.borrow().pretty_print();},
      Value::Tuple(x)  => {return x.pretty_print();},
      Value::Record(x) => {return x.borrow().pretty_print();},
      Value::Enum(x) => {return x.pretty_print();},
      Value::MatrixIndex(x) => {return x.pretty_print();}
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
      Value::MatrixValue(x)  => {return x.pretty_print();},
      Value::MatrixString(x)  => {return x.pretty_print();},
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
      Value::Index(x) => vec![1,1],
      Value::String(x) => vec![1,1],
      Value::Bool(x) => vec![1,1],
      Value::Atom(x) => vec![1,1],
      Value::MatrixIndex(x) => x.shape(),
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
      Value::MatrixString(x) => x.shape(),
      Value::MatrixValue(x) => x.shape(),
      Value::Enum(x) => vec![1,1],
      Value::Table(x) => x.borrow().shape(),
      Value::Set(x) => vec![1,x.set.len()],
      Value::Map(x) => vec![1,x.map.len()],
      Value::Record(x) => x.borrow().shape(),
      Value::Tuple(x) => vec![1,x.size()],
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
      Value::MatrixIndex(x) => ValueKind::Matrix(Box::new(ValueKind::Index),x.shape()),
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
      Value::MatrixString(x) => ValueKind::Matrix(Box::new(ValueKind::String),x.shape()),
      Value::MatrixValue(x) => ValueKind::Matrix(Box::new(ValueKind::Any),x.shape()),
      Value::Table(x) => x.borrow().kind(),
      Value::Set(x) => x.kind(),
      Value::Map(x) => x.kind(),
      Value::Record(x) => x.borrow().kind(),
      Value::Tuple(x) => x.kind(),
      Value::Enum(x) => x.kind(),
      Value::MutableReference(x) => ValueKind::Reference(Box::new(x.borrow().kind())),
      Value::Empty => ValueKind::Empty,
      Value::IndexAll => ValueKind::Empty,
      Value::Id(x) => ValueKind::Id,
      Value::Index(x) => ValueKind::Index,
      Value::Kind(x) => x.clone(),
    }
  }

  pub fn is_matrix(&self) -> bool {
    match self {
      Value::MatrixIndex(_) | Value::MatrixBool(_) | Value::MatrixU8(_) | 
      Value::MatrixU16(_) | Value::MatrixU32(_) | Value::MatrixU64(_) | 
      Value::MatrixU128(_) | Value::MatrixI8(_) | Value::MatrixI16(_) | 
      Value::MatrixI32(_) | Value::MatrixI64(_) | Value::MatrixI128(_) | 
      Value::MatrixF32(_) | Value::MatrixF64(_) | Value::MatrixString(_) |
      Value::MatrixValue(_) => true,
      _ => false,
    }
  }

  pub fn is_scalar(&self) -> bool {
    match self {
      Value::U8(_) | Value::U16(_) | Value::U32(_) | 
      Value::U64(_) | Value::U128(_) | Value::I8(_) | 
      Value::I16(_) | Value::I32(_) | Value::I64(_) | 
      Value::I128(_) | Value::F32(_) | Value::F64(_) | 
      Value::Bool(_) | Value::String(_) | 
      Value::Atom(_) | Value::Index(_) => true,
      _ => false,
    }
  }

  pub fn as_bool(&self) -> Option<Ref<bool>> {if let Value::Bool(v) = self { Some(v.clone()) } else if let Value::MutableReference(val) = self { val.borrow().as_bool() } else { None }}
  
  impl_as_type!(i8);
  impl_as_type!(i16);
  impl_as_type!(i32);
  impl_as_type!(i64);
  impl_as_type!(i128);
  impl_as_type!(u8);
  impl_as_type!(u16);
  impl_as_type!(u32);
  impl_as_type!(u64);
  impl_as_type!(u128);

  pub fn as_string(&self) -> Option<Ref<String>> {
    match self {
      Value::String(v) => Some(v.clone()),
      Value::U8(v) => Some(new_ref(v.borrow().to_string())),
      Value::U16(v) => Some(new_ref(v.borrow().to_string())),
      Value::U32(v) => Some(new_ref(v.borrow().to_string())),
      Value::U64(v) => Some(new_ref(v.borrow().to_string())),
      Value::U128(v) => Some(new_ref(v.borrow().to_string())),
      Value::I8(v) => Some(new_ref(v.borrow().to_string())),
      Value::I16(v) => Some(new_ref(v.borrow().to_string())),
      Value::I32(v) => Some(new_ref(v.borrow().to_string())),
      Value::I64(v) => Some(new_ref(v.borrow().to_string())),
      Value::I128(v) => Some(new_ref(v.borrow().to_string())),
      Value::F32(v) => Some(new_ref(format!("{}", v.borrow().0))),
      Value::F64(v) => Some(new_ref(format!("{}", v.borrow().0))),
      Value::Bool(v) => Some(new_ref(format!("{}", v.borrow()))),
      Value::MutableReference(val) => val.borrow().as_string(),
      _ => None,
    }
  }

  pub fn as_f32(&self) -> Option<Ref<F32>> {
    match self {
      Value::U8(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::U16(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::U32(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::U64(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::U128(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::I8(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::I16(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::I32(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::I64(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::I128(v) => Some(new_ref(F32::new(*v.borrow() as f32))),
      Value::F32(v) => Some(new_ref(F32::new((*v.borrow()).0 as f32))),
      Value::F64(v) => Some(new_ref(F32::new((*v.borrow()).0 as f32))),
      Value::MutableReference(val) => val.borrow().as_f32(),
      _ => None,
    }
  }

  pub fn as_f64(&self) -> Option<Ref<F64>> {
    match self {
      Value::U8(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::U16(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::U32(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::U64(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::U128(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::I8(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::I16(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::I32(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::I64(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::I128(v) => Some(new_ref(F64::new(*v.borrow() as f64))),
      Value::F64(v) => Some(new_ref(F64::new((*v.borrow()).0 as f64))),
      Value::F64(v) => Some(new_ref(F64::new((*v.borrow()).0 as f64))),
      Value::MutableReference(val) => val.borrow().as_f64(),
      _ => None,
    }
  }

  pub fn as_vecbool(&self)   -> Option<Vec<bool>>  {if let Value::MatrixBool(v)  = self { Some(v.as_vec()) } else if let Value::Bool(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecbool()  } else { None }}
  
  pub fn as_vecf64(&self) -> Option<Vec<F64>> { if let Value::MatrixF64(v) = self { Some(v.as_vec()) } else if let Value::F64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf64() } else if let Some(v) = self.as_f64() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_vecf32(&self) -> Option<Vec<F32>> { if let Value::MatrixF32(v) = self { Some(v.as_vec()) } else if let Value::F32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf32() } else if let Some(v) = self.as_f32() { Some(vec![v.borrow().clone()]) } else { None } }

  pub fn as_vecu8(&self) -> Option<Vec<u8>> { if let Value::MatrixU8(v) = self { Some(v.as_vec()) } else if let Value::U8(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu8() } else if let Some(v) = self.as_u8() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_vecu16(&self) -> Option<Vec<u16>> { if let Value::MatrixU16(v) = self { Some(v.as_vec()) } else if let Value::U16(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu16() } else if let Some(v) = self.as_u16() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_vecu32(&self) -> Option<Vec<u32>> { if let Value::MatrixU32(v) = self { Some(v.as_vec()) } else if let Value::U32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu32() } else if let Some(v) = self.as_u32() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_vecu64(&self) -> Option<Vec<u64>> { if let Value::MatrixU64(v) = self { Some(v.as_vec()) } else if let Value::U64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu64() } else if let Some(v) = self.as_u64() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_vecu128(&self) -> Option<Vec<u128>> { if let Value::MatrixU128(v) = self { Some(v.as_vec()) } else if let Value::U128(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu128() } else if let Some(v) = self.as_u128() { Some(vec![v.borrow().clone()]) } else { None } }

  pub fn as_veci8(&self) -> Option<Vec<i8>> { if let Value::MatrixI8(v) = self { Some(v.as_vec()) } else if let Value::I8(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci8() } else if let Some(v) = self.as_i8() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_veci16(&self) -> Option<Vec<i16>> { if let Value::MatrixI16(v) = self { Some(v.as_vec()) } else if let Value::I16(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci16() } else if let Some(v) = self.as_i16() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_veci32(&self) -> Option<Vec<i32>> { if let Value::MatrixI32(v) = self { Some(v.as_vec()) } else if let Value::I32(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci32() } else if let Some(v) = self.as_i32() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_veci64(&self) -> Option<Vec<i64>> { if let Value::MatrixI64(v) = self { Some(v.as_vec()) } else if let Value::I64(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci64() } else if let Some(v) = self.as_i64() { Some(vec![v.borrow().clone()]) } else { None } }
  pub fn as_veci128(&self) -> Option<Vec<i128>> { if let Value::MatrixI128(v) = self { Some(v.as_vec()) } else if let Value::I128(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci128() } else if let Some(v) = self.as_i128() { Some(vec![v.borrow().clone()]) } else { None } }

  pub fn as_vecstring(&self)   -> Option<Vec<String>>  {if let Value::MatrixString(v)  = self { Some(v.as_vec()) } else if let Value::String(v) = self { Some(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecstring()  } else { None }}


  pub fn as_vecusize(&self) -> Option<Vec<usize>> {
    match self {
      Value::MatrixIndex(v) => Some(v.as_vec()),
      Value::MatrixI64(v) => Some(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      Value::MatrixF64(v) => Some(v.as_vec().iter().map(|x| (*x).0 as usize).collect::<Vec<usize>>()),
      Value::MutableReference(x) => x.borrow().as_vecusize(),
      Value::MatrixBool(_) => None,
      Value::Bool(_) => None,
      _ => todo!(),
    }
  }

  pub fn as_index(&self) -> MResult<Value> {
    match self.as_usize() {      
      Some(ix) => Ok(Value::Index(new_ref(ix))),
      None => match self.as_vecusize() {
        Some(x) => {
          let shape = self.shape();
          let out = Value::MatrixIndex(usize::to_matrix(x, shape[0] * shape[1],1 ));
          Ok(out)
        },
        None => match self.as_vecbool() {
          Some(x) => {
            let shape = self.shape();
            let out = match (shape[0], shape[1]) {
              (1,1) => Value::Bool(new_ref(x[0])),
              (1,n) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
              (m,1) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
              (m,n) => Value::MatrixBool(Matrix::DVector(new_ref(DVector::from_vec(x)))),
            };
            Ok(out)
          }
          None => match self.as_bool() {
            Some(x) => Ok(Value::Bool(x)),
            None => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledIndexKind}),
          }
        }
      }
    }
  }

  pub fn as_usize(&self) -> Option<usize> {
    match self {      
      Value::Index(v) => Some(*v.borrow()),
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
      Value::MutableReference(v) => v.borrow().as_usize(),
      _ => None,
    }
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
      n => Value::MatrixIndex(Matrix::DVector(new_ref(DVector::from_vec(self.clone())))),
    }
  }
}

impl ToValue for Ref<usize> { fn to_value(&self) -> Value { Value::Index(self.clone()) } }
impl ToValue for Ref<u8>    { fn to_value(&self) -> Value { Value::U8(self.clone())    } }
impl ToValue for Ref<u16>   { fn to_value(&self) -> Value { Value::U16(self.clone())   } }
impl ToValue for Ref<u32>   { fn to_value(&self) -> Value { Value::U32(self.clone())   } }
impl ToValue for Ref<u64>   { fn to_value(&self) -> Value { Value::U64(self.clone())   } }
impl ToValue for Ref<u128>  { fn to_value(&self) -> Value { Value::U128(self.clone())  } }
impl ToValue for Ref<i8>    { fn to_value(&self) -> Value { Value::I8(self.clone())    } }
impl ToValue for Ref<i16>   { fn to_value(&self) -> Value { Value::I16(self.clone())   } }
impl ToValue for Ref<i32>   { fn to_value(&self) -> Value { Value::I32(self.clone())   } }
impl ToValue for Ref<i64>   { fn to_value(&self) -> Value { Value::I64(self.clone())   } }
impl ToValue for Ref<i128>  { fn to_value(&self) -> Value { Value::I128(self.clone())  } }
impl ToValue for Ref<F32>   { fn to_value(&self) -> Value { Value::F32(self.clone())   } }
impl ToValue for Ref<F64>   { fn to_value(&self) -> Value { Value::F64(self.clone())   } }
impl ToValue for Ref<bool>  { fn to_value(&self) -> Value { Value::Bool(self.clone())  } }
impl ToValue for Ref<String>  { fn to_value(&self) -> Value { Value::String(self.clone())  } }

macro_rules! to_value_matrix {
  ($($nd_matrix_kind:ident, $matrix_kind:ident, $base_type:ty),+ $(,)?) => {
    $(
      impl ToValue for Ref<$nd_matrix_kind<$base_type>> {
        fn to_value(&self) -> Value {
          Value::$matrix_kind(Matrix::<$base_type>::$nd_matrix_kind(self.clone()))
        }
      }
    )+
  };}

macro_rules! impl_to_value_matrix {
  ($matrix_kind:ident) => {
    to_value_matrix!(
      $matrix_kind, MatrixIndex,  usize,
      $matrix_kind, MatrixBool,   bool,
      $matrix_kind, MatrixI8,     i8,
      $matrix_kind, MatrixI16,    i16,
      $matrix_kind, MatrixI32,    i32,
      $matrix_kind, MatrixI64,    i64,
      $matrix_kind, MatrixI128,   i128,
      $matrix_kind, MatrixU8,     u8,
      $matrix_kind, MatrixU16,    u16,
      $matrix_kind, MatrixU32,    u32,
      $matrix_kind, MatrixU64,    u64,
      $matrix_kind, MatrixU128,   u128,
      $matrix_kind, MatrixF32,    F32,
      $matrix_kind, MatrixF64,    F64,
      $matrix_kind, MatrixString, String,
    );
  }
}

impl_to_value_matrix!(Matrix2x3);
impl_to_value_matrix!(Matrix3x2);
impl_to_value_matrix!(Matrix1);
impl_to_value_matrix!(Matrix2);
impl_to_value_matrix!(Matrix3);
impl_to_value_matrix!(Matrix4);
impl_to_value_matrix!(Vector2);
impl_to_value_matrix!(Vector3);
impl_to_value_matrix!(Vector4);
impl_to_value_matrix!(RowVector2);
impl_to_value_matrix!(RowVector3);
impl_to_value_matrix!(RowVector4);
impl_to_value_matrix!(RowDVector);
impl_to_value_matrix!(DVector);
impl_to_value_matrix!(DMatrix);

impl From<u8> for Value {
  fn from(val: u8) -> Self {
    Value::U8(new_ref(val))
  }
}

impl From<u16> for Value {
  fn from(val: u16) -> Self {
    Value::U16(new_ref(val))
  }
}

impl From<u32> for Value {
  fn from(val: u32) -> Self {
    Value::U32(new_ref(val))
  }
}

impl From<u64> for Value {
  fn from(val: u64) -> Self {
    Value::U64(new_ref(val))
  }
}

impl From<u128> for Value {
  fn from(val: u128) -> Self {
    Value::U128(new_ref(val))
  }
}

impl From<i8> for Value {
  fn from(val: i8) -> Self {
    Value::I8(new_ref(val))
  }
}

impl From<i16> for Value {
  fn from(val: i16) -> Self {
    Value::I16(new_ref(val))
  }
}

impl From<i32> for Value {
  fn from(val: i32) -> Self {
    Value::I32(new_ref(val))
  }
}

impl From<i64> for Value {
  fn from(val: i64) -> Self {
    Value::I64(new_ref(val))
  }
}

impl From<i128> for Value {
  fn from(val: i128) -> Self {
    Value::I128(new_ref(val))
  }
}

impl From<bool> for Value {
  fn from(val: bool) -> Self {
    Value::Bool(new_ref(val))
  }
}

impl From<String> for Value {
  fn from(val: String) -> Self {
    Value::String(new_ref(val))
  }
}
