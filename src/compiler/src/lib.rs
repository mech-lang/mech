#![cfg_attr(feature = "no_std", no_std)]
#![allow(warnings)]

use mech_core::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use crate::sections::TypeTag;

#[cfg(not(feature = "no_std"))]
use std::collections::HashSet;
#[cfg(feature = "no_std")]
use hashbrown::HashSet;

pub mod sections;
pub mod constants;
pub mod context;
pub mod program;

pub use self::sections::*;
pub use self::constants::*;
pub use self::context::*;
pub use self::program::*;

pub type Register = u32;

pub fn parse_version_to_u16(s: &str) -> Option<u16> {
  let parts: Vec<&str> = s.split('.').collect();
  if parts.len() != 3 { return None; }

  let major = parts[0].parse::<u16>().ok()?;
  let minor = parts[1].parse::<u16>().ok()?;
  let patch = parts[2].parse::<u16>().ok()?;

  if major > 0b111 { return None; }
  if minor > 0b1_1111 { return None; }
  if patch > 0xFF { return None; }

  let encoded = (major << 13) | (minor << 8) | patch;
  Some(encoded as u16)
}

pub fn encode_value_kind(ts: &mut crate::sections::TypeSection, vk: &ValueKind) -> (crate::sections::TypeTag, Vec<u8>) {
  let mut b = Vec::new();
  let tag = match vk {
    ValueKind::Kind(kind) => {
      let kind_id = ts.get_or_intern(kind);
      b.write_u32::<LittleEndian>(kind_id).unwrap();
      crate::sections::TypeTag::Kind
    },
    ValueKind::U8 => crate::sections::TypeTag::U8, ValueKind::U16 => crate::sections::TypeTag::U16, ValueKind::U32 => crate::sections::TypeTag::U32,
    ValueKind::U64 => crate::sections::TypeTag::U64, ValueKind::U128 => crate::sections::TypeTag::U128,
    ValueKind::I8 => crate::sections::TypeTag::I8, ValueKind::I16 => crate::sections::TypeTag::I16, ValueKind::I32 => crate::sections::TypeTag::I32,
    ValueKind::I64 => crate::sections::TypeTag::I64, ValueKind::I128 => crate::sections::TypeTag::I128,
    ValueKind::F32 => crate::sections::TypeTag::F32, ValueKind::F64 => crate::sections::TypeTag::F64,
    ValueKind::C64 => crate::sections::TypeTag::C64,
    ValueKind::R64 => crate::sections::TypeTag::R64,
    ValueKind::String => crate::sections::TypeTag::String,
    ValueKind::Bool => crate::sections::TypeTag::Bool,
    ValueKind::Id => crate::sections::TypeTag::Id,
    ValueKind::Index => crate::sections::TypeTag::Index,
    ValueKind::Empty => crate::sections::TypeTag::Empty,
    ValueKind::Any => crate::sections::TypeTag::Any,
    ValueKind::None => crate::sections::TypeTag::None,

    ValueKind::Matrix(elem, dims) => {
      let elem_id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(elem_id).unwrap();
      b.write_u32::<LittleEndian>(dims.len() as u32).unwrap();
      for &d in dims { b.write_u32::<LittleEndian>(d as u32).unwrap(); }
      match &**elem {
        ValueKind::U8 => crate::sections::TypeTag::MatrixU8,
        ValueKind::U16 => crate::sections::TypeTag::MatrixU16,
        ValueKind::U32 => crate::sections::TypeTag::MatrixU32,
        ValueKind::U64 => crate::sections::TypeTag::MatrixU64,
        ValueKind::U128 => crate::sections::TypeTag::MatrixU128,
        ValueKind::I8 => crate::sections::TypeTag::MatrixI8,
        ValueKind::I16 => crate::sections::TypeTag::MatrixI16,
        ValueKind::I32 => crate::sections::TypeTag::MatrixI32,
        ValueKind::I64 => crate::sections::TypeTag::MatrixI64,
        ValueKind::I128 => crate::sections::TypeTag::MatrixI128,
        ValueKind::F32 => crate::sections::TypeTag::MatrixF32,
        ValueKind::F64 => crate::sections::TypeTag::MatrixF64,
        ValueKind::C64 => crate::sections::TypeTag::MatrixC64,
        ValueKind::R64 => crate::sections::TypeTag::MatrixR64,
        ValueKind::String => crate::sections::TypeTag::MatrixString,
        ValueKind::Bool => crate::sections::TypeTag::MatrixBool,
        ValueKind::Index => crate::sections::TypeTag::MatrixIndex,
        _ => panic!("Unsupported matrix element type {:?}", elem),
      }
    }

    ValueKind::Enum(id, name) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      let name_bytes = name.as_bytes();
      b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
      b.extend_from_slice(name_bytes);
      crate::sections::TypeTag::EnumTag
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
      crate::sections::TypeTag::Record
    }

    ValueKind::Map(k,v) => {
      let kid = ts.get_or_intern(k);
      let vid = ts.get_or_intern(v);
      b.write_u32::<LittleEndian>(kid).unwrap();
      b.write_u32::<LittleEndian>(vid).unwrap();
      crate::sections::TypeTag::Map
    }

    ValueKind::Atom(id, name) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      let name_bytes = name.as_bytes();
      b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
      b.extend_from_slice(name_bytes);
      crate::sections::TypeTag::Atom
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
      crate::sections::TypeTag::Table
    }

    ValueKind::Tuple(elems) => {
      b.write_u32::<LittleEndian>(elems.len() as u32).unwrap();
      for t in elems {
        let tid = ts.get_or_intern(t);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      crate::sections::TypeTag::Tuple
    }

    ValueKind::Reference(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      crate::sections::TypeTag::Reference
    }

    ValueKind::Set(elem, max) => {
      let id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(id).unwrap();
      match max {
        Some(m) => { b.push(1); use byteorder::WriteBytesExt; b.write_u32::<LittleEndian>(*m as u32).unwrap(); }
        None => { b.push(0); }
      }
      crate::sections::TypeTag::Set
    }

    ValueKind::Option(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      crate::sections::TypeTag::OptionT
    }
  };
  (tag, b)
}

