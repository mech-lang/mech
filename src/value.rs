// # Value

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
//use crate::quantity::{Quantity, ToQuantity, QuantityMath};
//use errors::{ErrorType};
use crate::*;
use std::fmt;

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

  pub fn as_u8(&self) -> u8 {
    self.bytes.last().unwrap().clone()
  }

  pub fn as_u32(&self) -> u32 {
    let mut container: u32 = 0;
    for (i,byte) in self.bytes.iter().rev().take(4).enumerate() {
      container = container | (*byte as u32) << (8 * i) ;
    }
    container
  }

  pub fn as_u64(&self) -> u64 {    
    let mut container: u64 = 0;
    for (i,byte) in self.bytes.iter().rev().take(8).enumerate() {
      container = container | (*byte as u64) << (8 * i) ;
    }
    container
  }

  pub fn as_usize(&self) -> usize {    
    let mut container: usize = 0;
    let usize_bytes = usize::BITS as usize / 8 ;
    for (i,byte) in self.bytes.iter().rev().take(usize_bytes).enumerate() {
      container = container | (*byte as usize) << (usize_bytes * i) ;
    }
    container
  }

}