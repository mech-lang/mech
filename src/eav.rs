// # Entity, Attribute, Value

// ## Prelude

use core::fmt;

// ## Entity

pub struct Entity(u64);

// ## Attribute

pub struct Attribute(String);

// ## Value

#[derive(Clone)]
pub enum Value {
  Any,
  Number(u64),
  String(String),
}

impl Value {

  pub fn from_string(string: String) -> Value {
    Value::String(string)
  }

  pub fn from_str(string: &str) -> Value {
    Value::String(String::from(string))
  }

  pub fn from_int(int: u64) -> Value {
    Value::Number(int)
  }

}

impl fmt::Debug for Value {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        &Value::Number(ref x) => write!(f, "{}", x),
        &Value::String(ref x) => write!(f, "{}", x),
        &Value::Any => write!(f, "Any"),
      }
    }
}