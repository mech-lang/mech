#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;

use mech_core::{Transaction, Interner, TableId, Core};

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketClientMessage {
  Listening(Vec<TableId>),
  Control(u8),
  Code(String),
  Table(usize),
  RemoveBlock(usize),
  Transaction(Transaction),
}

// Run loop messages are sent to the run loop from the client

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Stop,
  StepBack,
  StepForward,
  Pause,
  Resume,
  Clear,
  PrintCore(Option<u64>),
  PrintRuntime,
  Listening(Vec<TableId>),
  Table(u64),
  Transaction(Transaction),
  Code((u64,String)),
  EchoCode(String),
  //Core(Core),
}

// ## Watchers

pub trait Watcher {
  fn get_name(& self) -> String;
}

// ## Value

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
  Number(Quantity),
  //Bool(bool),
  //Reference(TableId),
  //Empty,
}

impl Value {

  pub fn from_string(string: String) -> Value {
    Value::Number(0)
  }

  pub fn from_str(string: &str) -> Value {
    Value::Number(0)
  }

  pub fn from_bool(boolean: bool) -> Value {
    Value::Number(0)
  }

  pub fn from_u64(num: u64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn from_quantity(num: Quantity) -> Value {
    Value::Number(num)
  }

  pub fn from_i64(num: i64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn from_f64(num: f64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn as_quantity(&self) -> Option<Quantity> {
    match self {
      Value::Number(n) => Some(*n),
      //Value::Empty => Some(0.to_quantity()),
      _ => None,
    }
  }

  pub fn as_u64(&self) -> Option<u64> {
    match self {
      Value::Number(n) => Some(n.to_float() as u64),
      //Value::Reference(TableId::Local(n)) => Some(*n),
      //Value::Reference(TableId::Global(n)) => Some(*n),
      _ => None,
    }
  }

  pub fn as_float(&self) -> Option<f64> {
    match self {
      Value::Number(n) => Some(n.to_float()),
      _ => None,
    }
  }

  pub fn as_i64(&self) -> Option<i64> {
    match self {
      Value::Number(n) => Some(n.mantissa()),
      _ => None,
    }
  }

  pub fn as_string(&self) -> Option<String> {
    match self {
      //Value::String(n) => Some(n.clone()),
      Value::Number(q) => Some(q.format()),
      //Value::Reference(TableId::Global(r)) |
      //Value::Reference(TableId::Local(r)) => {
      //  Some(format!("{:?}", r))
      //},
      //Value::Empty => Some(String::from("")),
      //Value::Bool(t) => match t {
      //  true => Some(String::from("true")),
      //  false => Some(String::from("false")),
      //},
      _ => None,
    }
  }

  /*pub fn equal(&self, other: &Value) -> Option<bool> {
    match (self, other) {
      (Value::String(ref x), Value::String(ref y)) => {
        Some(x.to_owned() == y.to_owned())
      }
      _ => None,
    }
  }*/

  /*pub fn not_equal(&self, other: &Value) -> Option<bool> {
    match (self, other) {
      (Value::String(ref x), Value::String(ref y)) => {
        Some(x.to_owned() != y.to_owned())
      }
      _ => None,
    }
  }*/

  pub fn less_than(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn less_than_equal(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn greater_than(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn greater_than_equal(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn add(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn sub(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn multiply(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn divide(&self, other: &Value) -> Option<bool> {
    None
  }

}

impl fmt::Debug for Value {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Value::Number(x) => write!(f, "{}", x.to_string()),
      //&Value::String(ref x) => write!(f, "{}", x),
      //&Value::Empty => write!(f, ""),
      //&Value::Bool(ref b) => write!(f, "{}", b),
      //&Value::Reference(ref b) => write!(f, "{:?}", b),
    }
  }
}

