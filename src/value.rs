// # Value

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
//use crate::quantity::{Quantity, ToQuantity, QuantityMath};
//use errors::{ErrorType};
use crate::{hash_string};
use std::fmt;

// ## Value structs and enums

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  F32(f32),
  Bool(bool),
  String(String),
  Empty,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValueKind {
  U8,
  U16,
  U32,
  U64,
  F32,
  Quantity,
  Bool,
  String,
  Reference,
  NumberLiteral,
  Empty
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NumberLiteralKind {
  Decimal,
  Hexadecimal,
  Octal,
  Binary,
  U8,U16,U32,U64,U128,
  I8,I16,I32,I64,I128,
  F32,F64
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NumberLiteral {
  pub kind: NumberLiteralKind,
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

// ## Value Methods
/*
pub trait ValueMethods {
  fn empty() -> Value;
  fn from_string(string: &String) -> Value;
  fn from_str(string: &str) -> Value;
  fn from_bool(boolean: bool) -> Value;
  fn from_quantity(num: Quantity) -> Value;
  fn from_id(id: u64) -> Value;
  fn from_number_literal(number_literal: &NumberLiteral) -> Value;
  fn value_type(&self) -> ValueType;
  fn as_quantity(&self) -> Option<Quantity>;
  fn as_u64(&self) -> Option<u64>;
  fn as_i64(&self) -> Option<i64>;
  fn as_f64(&self) -> Option<f64>;
  fn as_f32(&self) -> Option<f32>;
  fn from_u64(num: u64) -> Quantity;
  fn from_f32(num: f32) -> Quantity;
  fn from_i32(num: i32) -> Quantity;
  fn from_u32(num: u32) -> Quantity;
  fn as_string(&self) -> Option<u64>;
  fn as_number_literal(&self) -> Option<u64>;
  fn as_bool(&self) -> Option<bool>;
  fn as_reference(&self) -> Option<u64>;
  fn as_raw(&self) -> u64;
  fn get_tag(&self) -> u64;
  fn is_empty(&self) -> bool;
  fn is_number(&self) -> bool;
  fn is_reference(&self) -> bool;
  fn is_number_literal(&self) -> bool;
  fn is_number_literal_interned(&self) -> bool;
  fn len(&self) -> Option<usize>;
  /*fn equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn not_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn less_than(&self, other: Value) -> Result<Value, ErrorType>;
  fn less_than_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn greater_than(&self, other: Value) -> Result<Value, ErrorType>;
  fn greater_than_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn add(&self, other: Value) -> Result<Value, ErrorType>;
  fn sub(&self, other: Value) -> Result<Value, ErrorType>;
  fn multiply(&self, other: Value) -> Result<Value, ErrorType>;
  fn divide(&self, other: Value) -> Result<Value, ErrorType>;
  fn power(&self, other: Value) -> Result<Value, ErrorType>;
  fn and(&self, other: Value) -> Result<Value, ErrorType>;
  fn or(&self, other: Value) -> Result<Value, ErrorType>;
  fn xor(&self, other: Value) -> Result<Value, ErrorType>;*/
}


// The first byte of a value indicates its domain. We have a couple built-in domains:

// - Empty - 0x10
// - Boolean - x40
// - Reference - 0x20
// - String - 0x80
// - Number Literal - 0xC0

const EMPTY: u64 = 0x1000000000000000;
const TRUE: u64 = 0x4000000000000001;
const FALSE: u64 = 0x4000000000000000;
const REFERENCE: u64 = 0x2000000000000000;
const STRING: u64 = 0x8000000000000000;
const NUMBER_LITERAL: u64 = 0xC000000000000000;
const NUMBER_LITERAL_INTERNED: u64 = 0xD000000000000000;

impl ValueMethods for Value {

  fn empty() -> Value {
    EMPTY
  }

  fn from_number_literal(number_literal: &NumberLiteral) -> Value {
    if number_literal.bytes.len() <= 7 {
      let mut number: u64 = 0;
      for (ix, byte) in number_literal.bytes.iter().enumerate() {
        let shift = (number_literal.bytes.len() - ix - 1) * 8;
        number = number | ((*byte as u64) << shift);
      }
      let len = (number_literal.bytes.len() as u64) << (14 * 4);
      number = number + NUMBER_LITERAL;
      number = number | len;
      number
    } else {
      hash_string(&format!("byte vector: {:?}",number_literal)) + NUMBER_LITERAL_INTERNED
    }
  }

  fn from_string(string: &String) -> Value {
    let mut string_hash = hash_string(string);
    string_hash = string_hash + STRING;
    string_hash
  }

  fn from_str(string: &str) -> Value {
    let mut string_hash = hash_string(string);
    string_hash = string_hash + STRING;
    string_hash
  }

  fn from_bool(boolean: bool) -> Value {
    match boolean {
      true => TRUE,
      false => FALSE,
    }
  }

  fn from_id(id: u64) -> Value {
    id + REFERENCE
  }

  fn from_quantity(num: Quantity) -> Value {
    num
  }

  fn is_empty(&self) -> bool {
    if *self == Value::empty() {
      true
    } else {
      false
    }
  }

  fn value_type(&self) -> ValueType {
    match self.as_quantity() {
      Some(_) => ValueType::Quantity,
      None => {
        match self.as_string() {
          Some(_) => ValueType::String,
          None => {
            match self.as_reference() {
              Some(_) => ValueType::Reference,
              None => {
                match self.as_bool() {
                  Some(_) => ValueType::Boolean,
                  None => match self.as_number_literal() {
                    Some(_) => ValueType::NumberLiteral,
                    None => ValueType::Empty,
                  },
                }
              }
            }
          }
        }
      }
    }
  }

  fn as_raw(&self) -> u64 {
    self & 0x0FFFFFFFFFFFFFFF
  }

  fn get_tag(&self) -> u64 {
    self & 0xF000000000000000
  }

  fn as_quantity(&self) -> Option<Quantity> {
    match self.is_number() {
      true => Some(*self),
      false => None,
    }
  }

  fn as_reference(&self) -> Option<u64> {
    match self.get_tag() {
      REFERENCE => Some(self.as_raw()),
      _ => None,
    }
  }

  fn as_u64(&self) -> Option<u64> {
    match self.is_number() {
      true => Some(self.to_u64()),
      false => None,
    }
  }

  fn is_number(&self) -> bool {
    match self.get_tag() {
      EMPTY | REFERENCE | TRUE | FALSE | STRING | NUMBER_LITERAL | NUMBER_LITERAL_INTERNED => false,
      _ => true,
    }
  }

  fn is_reference(&self) -> bool {
    match self.get_tag() {
      REFERENCE => true,
      _ => false,
    }
  }   
  
  fn is_number_literal(&self) -> bool {
    match self.get_tag() {
      NUMBER_LITERAL => true,
      _ => false,
    }
  } 

  fn len(&self) -> Option<usize> {
    match self.is_number_literal() {
      true => {
        Some(((self & 0x0F00000000000000) >> (14 * 4)) as usize)
      },
      _ => None,
    }
  } 

  fn is_number_literal_interned(&self) -> bool {
    match self.get_tag() {
      NUMBER_LITERAL_INTERNED => true,
      _ => false,
    }
  } 

  fn as_f64(&self) -> Option<f64> {
    match self.is_number() {
      true => Some(self.to_f32() as f64),
      false => None,
    }
  }

  fn as_f32(&self) -> Option<f32> {
    match self.is_number() {
      true => Some(self.to_f32()),
      false => None,
    }
  }

  fn from_u64(num: u64) -> Value {
    num.to_quantity()
  }

  fn from_i32(num: i32) -> Value {
    num.to_quantity()
  }

  fn from_u32(num: u32) -> Value {
    num.to_quantity()
  }

  fn from_f32(num: f32) -> Value {
    num.to_quantity()
  }

  fn as_i64(&self) -> Option<i64> {
    match self.is_number() {
      true => Some(self.to_f32() as i64),
      false => None,
    }
  }

  fn as_string(&self) -> Option<u64> {
    match self.get_tag() {
      STRING => Some(*self),
      _ => None,
    }
  }

  fn as_bool(&self) -> Option<bool> {
    match *self {
      TRUE => Some(true),
      FALSE => Some(false),
      _ => None,
    }
  }

  fn as_number_literal(&self) -> Option<u64> {
    match self.get_tag() {
      NUMBER_LITERAL | NUMBER_LITERAL_INTERNED => Some(*self),
      _ => None,
    }
  }
/*
  fn equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.value_type(), other.value_type()) {
      (ValueType::Boolean, ValueType::Boolean) => {
        Ok(Value::from_bool(self.as_bool().unwrap() == other.as_bool().unwrap()))
      }
      (ValueType::String, ValueType::String) => {
        Ok(Value::from_bool(self.as_string().unwrap() == other.as_string().unwrap()))
      }
      (ValueType::Quantity, ValueType::Quantity) => {
        Ok(Value::from_bool(self.as_quantity().unwrap().equal(other.as_quantity().unwrap())))
      }
      _ => Err(ErrorType::IncorrectFunctionArgumentType)
    }
  }

  fn not_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.value_type(), other.value_type()) {
      (ValueType::Boolean, ValueType::Boolean) => {
        Ok(Value::from_bool(self.as_bool().unwrap() != other.as_bool().unwrap()))
      }
      (ValueType::String, ValueType::String) => {
        Ok(Value::from_bool(self.as_string().unwrap() != other.as_string().unwrap()))
      }
      (ValueType::Quantity, ValueType::Quantity) => {
        Ok(Value::from_bool(self.as_quantity().unwrap().not_equal(other.as_quantity().unwrap())))
      }
      _ => Err(ErrorType::IncorrectFunctionArgumentType)
    }
  }

  fn less_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn less_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than_equal(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than_equal(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn add(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.add(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn sub(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.sub(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn multiply(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.multiply(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn divide(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.divide(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn power(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.power(r))),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn or(&self, other: Value) -> Result<Value, ErrorType>{
    match (self.as_bool(), other.as_bool()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q || r)),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn and(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_bool(), other.as_bool()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q && r)),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn xor(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_bool(), other.as_bool()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q ^ r)),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }*/

}*/