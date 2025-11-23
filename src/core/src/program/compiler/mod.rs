use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::Cursor;
#[cfg(not(feature = "no_std"))]
use std::collections::HashSet;
#[cfg(feature = "no_std")]
use hashbrown::HashSet;

pub mod sections;
pub mod constants;
pub mod context;

pub use self::sections::*;
pub use self::constants::*;
pub use self::context::*;

pub type Register = u32;

pub fn encode_value_kind(ts: &mut TypeSection, vk: &ValueKind) -> (TypeTag, Vec<u8>) {
  let mut b = Vec::new();
  let tag = match vk {
    ValueKind::U8 => TypeTag::U8, ValueKind::U16 => TypeTag::U16, ValueKind::U32 => TypeTag::U32,
    ValueKind::U64 => TypeTag::U64, ValueKind::U128 => TypeTag::U128,
    ValueKind::I8 => TypeTag::I8, ValueKind::I16 => TypeTag::I16, ValueKind::I32 => TypeTag::I32,
    ValueKind::I64 => TypeTag::I64, ValueKind::I128 => TypeTag::I128,
    ValueKind::F32 => TypeTag::F32, ValueKind::F64 => TypeTag::F64,
    ValueKind::C64 => TypeTag::C64,
    ValueKind::R64 => TypeTag::R64,
    ValueKind::String => TypeTag::String,
    ValueKind::Bool => TypeTag::Bool,
    ValueKind::Id => TypeTag::Id,
    ValueKind::Index => TypeTag::Index,
    ValueKind::Empty => TypeTag::Empty,
    ValueKind::Any => TypeTag::Any,
    ValueKind::None => TypeTag::None,

    ValueKind::Matrix(elem, dims) => {
      let elem_id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(elem_id).unwrap();
      b.write_u32::<LittleEndian>(dims.len() as u32).unwrap();
      for &d in dims { b.write_u32::<LittleEndian>(d as u32).unwrap(); }
      match &**elem {
        ValueKind::U8 => TypeTag::MatrixU8,
        ValueKind::U16 => TypeTag::MatrixU16,
        ValueKind::U32 => TypeTag::MatrixU32,
        ValueKind::U64 => TypeTag::MatrixU64,
        ValueKind::U128 => TypeTag::MatrixU128,
        ValueKind::I8 => TypeTag::MatrixI8,
        ValueKind::I16 => TypeTag::MatrixI16,
        ValueKind::I32 => TypeTag::MatrixI32,
        ValueKind::I64 => TypeTag::MatrixI64,
        ValueKind::I128 => TypeTag::MatrixI128,
        ValueKind::F32 => TypeTag::MatrixF32,
        ValueKind::F64 => TypeTag::MatrixF64,
        ValueKind::C64 => TypeTag::MatrixC64,
        ValueKind::R64 => TypeTag::MatrixR64,
        ValueKind::String => TypeTag::MatrixString,
        ValueKind::Bool => TypeTag::MatrixBool,
        ValueKind::Index => TypeTag::MatrixIndex,
        _ => panic!("Unsupported matrix element type {:?}", elem),
      }
    }

    ValueKind::Enum(space) => {
      b.write_u64::<LittleEndian>(*space).unwrap();
      TypeTag::EnumTag
    }

    ValueKind::Record(fields) => {
      b.write_u32::<LittleEndian>(fields.len() as u32).unwrap();
      for (name, ty) in fields {
        let name_bytes = name.as_bytes();
        b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
        b.extend_from_slice(name_bytes);
        let tid = ts.get_or_intern(ty);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      TypeTag::Record
    }

    ValueKind::Map(k,v) => {
      let kid = ts.get_or_intern(k);
      let vid = ts.get_or_intern(v);
      b.write_u32::<LittleEndian>(kid).unwrap();
      b.write_u32::<LittleEndian>(vid).unwrap();
      TypeTag::Map
    }

    ValueKind::Atom(id) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      TypeTag::Atom
    }

    ValueKind::Table(cols, pk_col) => {
      b.write_u32::<LittleEndian>(cols.len() as u32).unwrap();
      for (name, ty) in cols {
        let name_b = name.as_bytes();
        b.write_u32::<LittleEndian>(name_b.len() as u32).unwrap();
        b.extend_from_slice(name_b);
        let tid = ts.get_or_intern(ty);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      b.write_u32::<LittleEndian>(*pk_col as u32).unwrap();
      TypeTag::Table
    }

    ValueKind::Tuple(elems) => {
      b.write_u32::<LittleEndian>(elems.len() as u32).unwrap();
      for t in elems {
        let tid = ts.get_or_intern(t);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      TypeTag::Tuple
    }

    ValueKind::Reference(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      TypeTag::Reference
    }

    ValueKind::Set(elem, max) => {
      let id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(id).unwrap();
      match max {
        Some(m) => { b.push(1); use byteorder::WriteBytesExt; b.write_u32::<LittleEndian>(*m as u32).unwrap(); }
        None => { b.push(0); }
      }
      TypeTag::Set
    }

    ValueKind::Option(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      TypeTag::OptionT
    }
  };
  (tag, b)
}