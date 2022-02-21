// # Value

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
//use crate::quantity::{Quantity, ToQuantity, QuantityMath};
//use errors::{ErrorType};
use crate::*;
use std::fmt;
use std::mem::transmute;
use std::convert::TryInto;

lazy_static! {
  pub static ref F32L: u64 = hash_str("f32-literal");
  pub static ref F32: u64 = hash_str("f32");
  pub static ref U8: u64 = hash_str("u8");
  pub static ref U16: u64 = hash_str("u16");
  pub static ref U32: u64 = hash_str("u32");
  pub static ref U64: u64 = hash_str("u64");
  pub static ref HZ: u64 = hash_str("hz");
  pub static ref MS: u64 = hash_str("ms");
  pub static ref S: u64 = hash_str("s");
  pub static ref M: u64 = hash_str("m");
  pub static ref KM: u64 = hash_str("km");
  pub static ref HEX: u64 = hash_str("hex");
  pub static ref DEC: u64 = hash_str("dec");
}

// ## Value structs and enums

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  U128(u128),
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  I128(i128),
  F32(f32),
  F64(f64),
  Bool(bool),
  Time(f32),
  Length(f32),
  String(MechString),
  Reference(TableId),
  Empty,
}

impl Value {

  pub fn as_table_reference(&self) -> Result<TableId,MechError> {
    match self {
      Value::Reference(table_id) => Ok(*table_id),
      _ => Err(MechError::GenericError(1869)),
    }
  }

  pub fn as_string(&self) -> Result<MechString,MechError> {
    match self {
      Value::String(string) => Ok(string.clone()),
      _ => Err(MechError::GenericError(1870)),
    }
  }

  pub fn from_string(string: &String) -> Value {
    Value::String(MechString::from_string(string.clone()))
  }

  pub fn from_str(string: &str) -> Value {
    Value::String(MechString::from_string(string.to_string()))
  }

}

impl fmt::Debug for Value {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {
      Value::U8(v) => write!(f,"{}u8",v)?,
      Value::U16(v) => write!(f,"{}u16",v)?, 
      Value::U32(v) => write!(f,"{}u32",v)?, 
      Value::U64(v) => write!(f,"{}u64",v)?,
      Value::U128(v) => write!(f,"{}u128",v)?, 
      Value::I8(v) => write!(f,"{}i8",v)?, 
      Value::I16(v) => write!(f,"{}i16",v)?, 
      Value::I32(v) => write!(f,"{}i32",v)?, 
      Value::I64(v) => write!(f,"{}i64",v)?, 
      Value::I128(v) => write!(f,"{}i128",v)?, 
      Value::Time(v) => write!(f,"{}ms",v)?,
      Value::Length(v) => write!(f,"{}m",v)?,
      Value::F32(v) => write!(f,"{}f32",v)?,
      Value::F64(v) => write!(f,"{}f64",v)?, 
      Value::Bool(v) => write!(f,"{}",v)?,
      Value::Reference(v) => write!(f,"{:?}",v)?, 
      Value::String(v) => {
        write!(f,"\"{}\"",v.to_string())?
      }, 
      Value::Empty => write!(f,"_")?,
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValueKind {
  U8,
  U16,
  U32,
  U64,
  U128,
  I8,
  I16,
  I32,
  I64,
  I128,
  F32,
  F64,
  Index,
  Quantity,
  Bool,
  Time,
  Length,
  String,
  Reference,
  NumberLiteral,
  Compound(Vec<ValueKind>), // Note: Not sure of the implications here, doing this to return a ValueKind for a table.
  Empty
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NumberLiteral {
  pub kind: u64,
  pub bytes: Vec<u8>,
}

impl NumberLiteral {

  pub fn new(kind: u64, bytes: Vec<u8>) -> NumberLiteral {
    NumberLiteral{kind,bytes}
  }

  fn is_float(&self) -> bool {
    if self.kind == *F32 || self.kind == *F32L {
      true 
    } else {
      false
    }
  }

  pub fn as_u8(&mut self) -> u8 {
    if self.is_float() {
      self.as_f32() as u8
    } else {
      self.bytes.last().unwrap().clone()
    }
  }

  pub fn as_u16(&mut self) -> u16 {
    if self.is_float() {
      self.as_f32() as u16
    } else {
      while self.bytes.len() < 2 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<u16>());
      let x = u16::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_u32(&mut self) -> u32 {
    if self.is_float() {
      self.as_f32() as u32
    } else {
      while self.bytes.len() < 4 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<u32>());
      let x = u32::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_u64(&mut self) -> u64 {    
    if self.is_float() {
      self.as_f32() as u64
    } else {
      while self.bytes.len() < 8 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<u64>());
      let x = u64::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_u128(&mut self) -> u128 {    
    if self.is_float() {
      self.as_f32() as u128
    } else {
      while self.bytes.len() < 16 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<u128>());
      let x = u128::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_f32(&mut self) -> f32 {    
    while self.bytes.len() < 4 {
      self.bytes.insert(0,0);
    }
    let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<f32>());
    f32::from_be_bytes(fbytes.try_into().unwrap())
  }

  pub fn as_usize(&mut self) -> usize {    
    if self.is_float() {
      self.as_f32() as usize
    } else {
      self.as_u64() as usize
    }
  }

}