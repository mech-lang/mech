// # Value

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use quantities::{Quantity, ToQuantity, QuantityMath};
use errors::{ErrorType};
use ::{hash_string};

// ## Value structs and enums

pub type Value = u64;

pub enum ValueType {
  Quantity,
  Boolean,
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
  Binary
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NumberLiteral {
  pub kind: NumberLiteralKind,
  pub bytes: Vec<u8>,
}

// ## Value Methods

pub trait ValueMethods {
  fn empty() -> Value;
  fn from_string(string: String) -> Value;
  fn from_str(string: &str) -> Value;
  fn from_bool(boolean: bool) -> Value;
  fn from_u64(num: u64) -> Value;
  fn from_quantity(num: Quantity) -> Value;
  fn from_i64(num: i64) -> Value;
  fn from_f64(num: f64) -> Value;
  fn from_id(id: u64) -> Value;
  fn from_byte_vector(vector: &Vec<u8>) -> Value;
  fn value_type(&self) -> ValueType;
  fn as_quantity(&self) -> Option<Quantity>;
  fn as_u64(&self) -> Option<u64>;
  fn as_i64(&self) -> Option<i64>;
  fn as_float(&self) -> Option<f64>;
  fn as_string(&self) -> Option<u64>;
  fn as_byte_array(&self) -> Option<u64>;
  fn as_bool(&self) -> Option<bool>;
  fn as_reference(&self) -> Option<u64>;
  fn as_raw(&self) -> u64;
  fn is_empty(&self) -> bool;
  fn is_number(&self) -> bool;
  fn is_reference(&self) -> bool;
  fn equal(&self, other: Value) -> Result<Value, ErrorType>;
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
}


// The first byte of a value indicates its domain. We have a couple built-in domains:

// - Empty - 0x10
// - Boolean - x40
// - Reference - 0x20
// - String - 0x80
// - Number Literal - 0xC0

impl ValueMethods for Value {

  fn empty() -> Value {
    0x1000000000000000
  }

  fn from_byte_vector(vector: &Vec<u8>) -> Value {
    let mut vector_hash = hash_string(&format!("byte vector: {:?}",vector));
    vector_hash = vector_hash + 0xC000000000000000;
    vector_hash
  }

  fn from_string(string: String) -> Value {
    let mut string_hash = hash_string(&string);
    string_hash = string_hash + 0x8000000000000000;
    string_hash
  }

  fn from_str(string: &str) -> Value {
    let mut string_hash = hash_string(string);
    string_hash = string_hash + 0x8000000000000000;
    string_hash
  }

  fn from_bool(boolean: bool) -> Value {
    match boolean {
      true => 0x4000000000000001,
      false => 0x4000000000000000,
    }
  }

  fn from_id(id: u64) -> Value {
    id + 0x2000000000000000
  }

  fn from_u64(num: u64) -> Value {
    num.to_quantity()
  }

  fn from_quantity(num: Quantity) -> Value {
    num
  }

  fn from_i64(num: i64) -> Value {
    num.to_quantity()
  }

  fn from_f64(num: f64) -> Value {
    num.to_quantity()
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
                  None => match self.as_byte_array() {
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
    self & 0x00FFFFFFFFFFFFFF
  }

  fn as_quantity(&self) -> Option<Quantity> {
    match self & 0xFF00000000000000 {
      0x1000000000000000 |
      0x2000000000000000 |
      0x4000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 => None,
      _ => Some(*self),
    }
  }

  fn as_reference(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x2000000000000000 => Some(self & 0x00FFFFFFFFFFFFFF),
      _ => None,
    }
  }

  fn as_u64(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x1000000000000000 |
      0x2000000000000000 |
      0x4000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 => None,
      _ => Some(self.to_u64()),
    }
  }

  fn is_number(&self) -> bool {
    match self & 0xFF00000000000000 {
      0x1000000000000000 |
      0x2000000000000000 |
      0x4000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 => false,
      _ => true,
    }
  }

  fn is_reference(&self) -> bool {
    match self & 0xFF00000000000000 {
      0x2000000000000000 => true,
      _ => false,
    }
  }    

  fn as_float(&self) -> Option<f64> {
    match self & 0xFF00000000000000 {
      0x1000000000000000 |
      0x2000000000000000 |
      0x4000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 => None,
      _ => Some(self.to_float()),
    }
  }

  fn as_i64(&self) -> Option<i64> {
    None
  }

  fn as_string(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x8000000000000000 => Some(*self),
      _ => None,
    }
  }

  fn as_bool(&self) -> Option<bool> {
    match self {
      0x4000000000000001 => Some(true),
      0x4000000000000000 => Some(false),
      _ => None,
    }
  }

  fn as_byte_array(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0xC000000000000000 => Some(*self),
      _ => None,
    }
  }

  fn equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.equal(r).unwrap())),
      _ => {
        match (self.as_string(), other.as_string()) {
          (Some(q), Some(r)) => Ok(Value::from_bool(q == r)),
          _ => Err(ErrorType::IncorrectFunctionArgumentType),
        }
      },
    } 
  }

  fn not_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.not_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn less_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn less_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn add(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.add(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn sub(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.sub(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn multiply(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.multiply(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn divide(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.divide(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn power(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.power(r).unwrap())),
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

}