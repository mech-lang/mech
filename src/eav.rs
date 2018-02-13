// # Entity, Attribute, Value

// ## Prelude

use core::fmt;
use indexes::Hasher;
use runtime::{Change, ChangeType};
use alloc::{Vec,String};

// ## Entity

pub struct Entity {
  pub id: u64,
  pub pairs: Vec<(Attribute, Value)>,
}

impl Entity {

  pub fn new() -> Entity {
    Entity {
      id: 0,
      pairs: Vec::new(),
    }
  }

  // Transform a vector of raw string/value pairs into
  // an entity. The entity ID is computed as a hash of the
  // pairs.

  pub fn from_raw(pairs: Vec<(&str, Value)>) -> Entity {
    let mut entity = Entity::new();
    let mut entity_id = Hasher::new();
    let mut attribute_id = Hasher::new();
    for (attribute, value) in pairs {
      entity_id.write(attribute);
      entity_id.write_value(&value);
      let attribute = Attribute::from_str(attribute);
      entity.pairs.push((attribute, value));
    } 
    entity.id = entity_id.finish();
    entity
  }

  // Convert an Entity to a set of changes. These changes
  // will be appleid to the DB.

  pub fn make_changeset(&self, kind: ChangeType) -> Vec<Change> {
    let mut changes: Vec<Change> = Vec::with_capacity(self.pairs.len());
    for &(ref attribute, ref value) in &self.pairs {
      let change = Change::from_eav(self, attribute, value, kind.clone());
      changes.push(change);
    }
    changes
  }


}

impl fmt::Debug for Entity {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f,"{} -> [ ", self.id);
      for &(ref attribute, ref value) in &self.pairs {
        write!(f,"{:?}: {:?} ", attribute, value);
      }
      write!(f,"]")
    }
}

// ## Attribute

#[derive(Clone)]
pub struct Attribute {
  pub id: u64,
  pub display: String,
}

impl Attribute {

  pub fn new() -> Attribute {
    Attribute {
      id: 0,
      display: String::from(""),
    }
  }

  pub fn from_str(string: &str) -> Attribute {
    let mut attribute_id = Hasher::new();
    attribute_id.write(string);
    let mut attribute = Attribute::new();
    attribute.id = attribute_id.finish();
    attribute.display = String::from(string);
    attribute
  }
}

impl fmt::Debug for Attribute {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f,"{}", self.display)
    }
}

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

  pub fn from_u64(num: u64) -> Value {
    Value::Number(num)
  }

}

impl fmt::Debug for Value {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        &Value::Number(ref x) => write!(f, "{}", x),
        &Value::String(ref x) => write!(f, "\"{}\"", x),
        &Value::Any => write!(f, "Any"),
      }
    }
}