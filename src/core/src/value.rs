// # Value

/*
Implementation for the Value data type in Mech, The Value type contains various primitive types, such as integers, floats, and booleans, and can also represent strings and references to tables. Provides methods for converting between the Value type and other data types, as well as for retrieving information about the type of a given Value. By unifying all these values into an enum, Mech can treat them as a single type that can be used in a variety of contexts.
*/


// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use crate::*;
use std::mem::transmute;
use std::convert::TryInto;

// ## Value structs and enums

#[derive(Clone,PartialEq,PartialOrd,Serialize,Deserialize)]
pub enum Value {
  U8(U8),
  U16(U16),
  U32(U32),
  U64(U64),
  U128(U128),
  I8(I8),
  I16(I16),
  I32(I32),
  I64(I64),
  I128(I128),
  f32(f32),
  F32(F32),
  F64(F64),
  Bool(bool),
  Time(F32),
  Length(F32),
  Speed(F32),
  Angle(F32),
  String(MechString),
  Reference(TableId),
  Enum(Enum),
  Empty,
}

impl Value {

  pub fn as_table_reference(&self) -> Result<TableId,MechError> {
    match self {
      Value::Reference(table_id) => Ok(*table_id),
      _ => Err(MechError{msg: "".to_string(), id: 4020, kind: MechErrorKind::None}),
    }
  }

  pub fn as_string(&self) -> Result<MechString,MechError> {
    match self {
      Value::String(string) => Ok(string.clone()),
      _ => Err(MechError{msg: "".to_string(), id: 4021, kind: MechErrorKind::None}),
    }
  }

  pub fn from_u64(number: u64) -> Value {
    Value::U64(U64::new(number))
  }

  pub fn from_string(string: &String) -> Value {
    Value::String(MechString::from_string(string.clone()))
  }

  pub fn from_str(string: &str) -> Value {
    Value::String(MechString::from_string(string.to_string()))
  }

  pub fn kind(&self) -> ValueKind {
    match &self {
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
      Value::Time(_) => ValueKind::Time,
      Value::Length(_) => ValueKind::Length,
      Value::Speed(_) => ValueKind::Speed,
      Value::Angle(_) => ValueKind::Angle,
      Value::F32(_) => ValueKind::F32,
      Value::F64(_) => ValueKind::F64,
      Value::f32(_) => ValueKind::f32,
      Value::Bool(_) => ValueKind::Bool,
      Value::Reference(_) => ValueKind::Reference,
      Value::String(_) => ValueKind::String,
      Value::Enum(Enum{kind,variant}) => ValueKind::Enum((*kind,*variant)),
      Value::Empty => ValueKind::Empty,
    }
  }
}

impl fmt::Debug for Value {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {
      Value::U8(v) => write!(f,"{:?}",v)?,
      Value::U16(v) => write!(f,"{:?}",v)?, 
      Value::U32(v) => write!(f,"{:?}",v)?, 
      Value::U64(v) => write!(f,"{:?}",v)?,
      Value::U128(v) => write!(f,"{:?}",v)?, 
      Value::I8(v) => write!(f,"{:?}",v)?, 
      Value::I16(v) => write!(f,"{:?}",v)?, 
      Value::I32(v) => write!(f,"{:?}",v)?, 
      Value::I64(v) => write!(f,"{:?}",v)?, 
      Value::I128(v) => write!(f,"{:?}",v)?, 
      Value::Time(v) => write!(f,"{:?}",v)?,
      Value::Length(v) => write!(f,"{:?}",v)?,
      Value::Speed(v) => write!(f,"{:?}",v)?,
      Value::Angle(v) => write!(f,"{:?}",v)?,
      Value::f32(v) => write!(f,"{:?}",v)?,
      Value::F32(v) => write!(f,"{:?}",v)?,
      Value::F64(v) => write!(f,"{:?}",v)?, 
      Value::Bool(v) => write!(f,"{}",v)?,
      Value::Reference(v) => write!(f,"{:?}",v)?, 
      Value::String(v) => write!(f,"\"{}\"",v.to_string())?,
      Value::Enum(v) => write!(f,"{:?}",v)?, 
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
  f32,
  F64,
  Index,
  Quantity,
  Bool,
  Time,
  Length,
  Angle,
  Speed,
  String,
  Reference,
  NumberLiteral,
  Enum((u64,u64)),
  Any,
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
    if self.kind == *cF32 || self.kind == *cF32L {
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

  pub fn as_i8(&mut self) -> i8 {
    if self.is_float() {
      self.as_f32() as i8
    } else {
      self.bytes.last().unwrap().clone() as i8
    }
  }

  pub fn as_i16(&mut self) -> i16 {
    if self.is_float() {
      self.as_f32() as i16
    } else {
      while self.bytes.len() < 2 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<i16>());
      let x = i16::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_i32(&mut self) -> i32 {
    if self.is_float() {
      self.as_f32() as i32
    } else {
      while self.bytes.len() < 4 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<i32>());
      let x = i32::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_i64(&mut self) -> i64 {    
    if self.is_float() {
      self.as_f32() as i64
    } else {
      while self.bytes.len() < 8 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<i64>());
      let x = i64::from_be_bytes(fbytes.try_into().unwrap());
      x
    }
  }

  pub fn as_i128(&mut self) -> i128 {    
    if self.is_float() {
      self.as_f32() as i128
    } else {
      while self.bytes.len() < 16 {
        self.bytes.insert(0,0);
      }
      let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<i128>());
      let x = i128::from_be_bytes(fbytes.try_into().unwrap());
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

  pub fn as_f64(&mut self) -> f64 {    
    while self.bytes.len() < 8 {
      self.bytes.insert(0,0);
    }
    let (fbytes, rest) = self.bytes.split_at(std::mem::size_of::<f64>());
    f64::from_be_bytes(fbytes.try_into().unwrap())
  }

  pub fn as_usize(&mut self) -> usize {    
    if self.is_float() {
      self.as_f32() as usize
    } else {
      self.as_u64() as usize
    }
  }

}