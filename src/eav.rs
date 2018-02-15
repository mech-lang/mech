// # Entity, Attribute, Value

// ## Prelude

use core::fmt;
use indexes::Hasher;
use database::{Change, ChangeType};
use alloc::{Vec, String};
use hashmap_core::map::HashMap;

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
        &Value::Empty => write!(f, "Empty"),
      }
    }
}

// ## Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents a record.

pub struct Table {
  pub id: u64,
  pub rows: usize,
  pub cols: usize,
  pub data: Vec<Vec<Value>>,
  pub attributes: HashMap<u64, usize>,
  pub entities: HashMap<u64, usize>,
}

impl Table {

  // m x attributes and n x records
  pub fn new(tag: &str, m: usize, n: usize) -> Table {
    let id = Hasher::hash_str(tag);
    Table {
      id: id,
      rows: 0,
      cols: 0,
      data: vec![vec![Value::Empty; n]; m], 
      entities: HashMap::with_capacity(n),
      attributes: HashMap::with_capacity(m),
    }
  }
  
  pub fn add_value(&mut self, entity: &u64, attribute: &u64, value: Value) {

    // Check if the row
    let row = if self.entities.contains_key(&entity) {
      self.entities.get(&entity).unwrap()
    } else {
      self.rows = self.rows + 1;
      self.entities.insert(entity.clone(), self.rows.clone());
      &self.rows
    };

    // Get the column
    let col = if self.attributes.contains_key(&attribute) {
      self.attributes.get(&attribute).unwrap()
    } else {
      self.cols = self.cols + 1;
      self.attributes.insert(attribute.clone(), self.cols.clone());
      &self.cols
    };
    self.data[*col - 1][*row - 1] = value;
  }

}

impl fmt::Debug for Table {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "---------------------------------------\n");
      write!(f, "{:?}\n", self.id);
      write!(f, "{:?} x {:?}\n", self.cols, self.rows);
      write!(f, "---------------------------------------\n");
      write!(f, "\n");
      for m in 0 .. self.rows {
        for n in 0 .. self.cols {
          write!(f, "{:?} ", self.data[n][m]);
        }
        write!(f, "\n");
      }
      Ok(())
    }
}