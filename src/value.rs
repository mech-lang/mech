use quantity::{Quantity, to_quantity};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Number(Quantity),
  String(String),
  Bool(bool),
  Reference(u64),
  Empty,
}

impl Value {

  pub fn from_string(string: String) -> Value {
    Value::String(string)
  }

  pub fn from_str(string: &str) -> Value {
    Value::String(String::from(string))
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
      Value::Empty => Some(0.to_quantity()),
      _ => None,
    }
  }

  pub fn as_u64(&self) -> Option<u64> {
    match self {
      Value::Number(n) => Some(n.to_float() as u64),
      Value::Reference(n) => Some(*n),
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
      Value::String(n) => Some(n.clone()),
      Value::Number(q) => Some(q.format()),
      Value::Reference(r) => Some(format!("{:?}", r)),
      Value::Empty => Some(String::from("")),
      Value::Bool(t) => match t {
        true => Some(String::from("true")),
        false => Some(String::from("false")),
      },
      _ => None,
    }
  }
}

impl fmt::Debug for Value {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Value::Number(x) => write!(f, "{}", x.to_string()),
      &Value::String(ref x) => write!(f, "{}", x),
      &Value::Empty => write!(f, ""),
      &Value::Bool(ref b) => write!(f, "{}", b),
      &Value::Reference(ref b) => write!(f, "@{:#x}", b),
    }
  }
}