#![cfg_attr(feature = "no_std", no_std)]
#![allow(warnings)]

pub use mech_core::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

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

pub fn encode_value_kind(ts: &mut TypeSection, vk: &ValueKind) -> (TypeTag, Vec<u8>) {
  let mut b = Vec::new();
  let tag = match vk {
    ValueKind::Kind(kind) => {
      let kind_id = ts.get_or_intern(kind);
      b.write_u32::<LittleEndian>(kind_id).unwrap();
      TypeTag::Kind
    },
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
    _ => return mech_core::compiler::encode_value_kind(ts, vk),
  };
  (tag, b)
}
