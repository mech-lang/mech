// # Entity, Attribute, Value

// ## Prelude

use core::fmt;
use indexes::Hasher;
use database::{Change, ChangeType};
use alloc::{Vec, String};
use hashmap_core::map::HashMap;

// ## Entity

pub struct Entity {
  pub table: u64,
  pub id: u64,
  pub pairs: Vec<(Attribute, Value)>,
}

impl Entity {

  pub fn new(table: &str) -> Entity {
    let table_id = Hasher::hash_str(table);
    Entity {
      table: table_id,
      id: 0,
      pairs: Vec::new(),
    }
  }

  // Transform a vector of raw string/value pairs into
  // an entity. The entity ID is computed as a hash of the
  // pairs.

  pub fn from_raw(pairs: Vec<(&str, Value)>) -> Entity {
    let mut entity = Entity::new("");
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
  // will be applied to the DB.

  pub fn make_changeset(&self, kind: ChangeType) -> Vec<Change> {
    let mut changes: Vec<Change> = Vec::with_capacity(self.pairs.len());
    for &(ref attribute, ref value) in &self.pairs {
      let change = Change::new(0, self, attribute, value, kind.clone());
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
        &Value::String(ref x) => write!(f, "{}", x),
        &Value::Empty => write!(f, ""),
      }
    }
}

// ## Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents a record.

pub struct Table {
  pub name: String,
  pub id: u64,
  pub rows: usize,
  pub cols: usize,
  pub data: Vec<Vec<Value>>,
  pub attributes: HashMap<u64, usize>,
  pub entities: HashMap<u64, usize>,
}

impl Table {

  // m x attributes and n x records. nxm is the capacity of the table
  // while the actual size starts at 0x0 (since it is empty)
  pub fn new(tag: &str, m: usize, n: usize) -> Table {
    let id = Hasher::hash_str(tag);
    Table {
      name: String::from(tag),
      id: id,
      rows: 0,
      cols: 0,
      data: vec![vec![Value::Empty; n]; m], 
      entities: HashMap::with_capacity(n),
      attributes: HashMap::with_capacity(m),
    }
  }

  pub fn add_value(&mut self, entity: u64, attribute: u64, value: Value) {

    // Check if the row is already in the table. If it is, return it.
    let row = if self.entities.contains_key(&entity) {
      self.entities.get(&entity).unwrap()
    // If the row doesn't exist yet, create it at the end.
    } else {
      self.rows = self.rows + 1;
      self.entities.insert(entity.clone(), self.rows.clone());
      &self.rows
    };

    // Get the column indicated by the attribute
    let col = if self.attributes.contains_key(&attribute) {
      self.attributes.get(&attribute).unwrap()
    // If it doesn't exist yet, create it at the end
    } else {
      self.cols = self.cols + 1;
      self.attributes.insert(attribute.clone(), self.cols.clone());
      &self.cols
    };
    // Add the value at the indicated location
    self.data[*row - 1][*col - 1] = value;
  }

  pub fn get_row(&mut self, entity: u64) -> Option<&Vec<Value>> {
    // Get the index for the given entity
    match self.entities.get(&entity) {
      Some(x) => Some(&self.data[x - 1]),
      None => None,
    }
  }

}

impl fmt::Debug for Table {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      let cell_width = 15;
      write!(f, "╔");
      print_rep_char("═", (cell_width + 3) * self.cols - 1, f);
      write!(f, "╗\n");
      write!(f, "║ #{} ({:?})\n", self.name, self.id);
      write!(f, "║ {:?} × {:?}\n", self.rows, self.cols);
      write!(f, "╚");
      print_rep_char("═", (cell_width + 3) * self.cols - 1, f);
      write!(f, "╝\n");
      write!(f, "\n");
      print_header(self.cols, cell_width, f);
      for m in 0 .. self.rows {
        print_row(self.data[m].clone(), self.cols, cell_width, f);
      }
      print_footer(self.cols, cell_width,  f);
      
      Ok(())
    }
}

fn print_rep_char(to_print: &str, n: usize, f: &mut fmt::Formatter) {
  for i in 0..n {
    write!(f, "{}", to_print);
  }
}

fn print_header(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "┌");
  for i in 0 .. n - 1 {
    write!(f, "─");
    print_rep_char("─", m, f);
    write!(f, "─┬");
  }
  write!(f, "─");
  print_rep_char("─", m, f);
  write!(f, "─┐\n");
}

fn print_row(row: Vec<Value>, n: usize, cell_size: usize, f: &mut fmt::Formatter) {
  write!(f, "│");
  for i in 0 .. n {
    let mut s = format!("{:?}", row[i]);
    let mut ell = "";
    if s.len() > cell_size {
      s.truncate(cell_size - 3);
      ell = "..."
    }
    
    write!(f, " {}{}", s.clone(), ell);
    for j in 0 .. cell_size - (s.len() + ell.len()) {
      write!(f, " ");
    }
    write!(f, " │");
  }
  write!(f, "\n");
}

fn print_footer(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "└");
  for i in 0 .. n - 1 {
    write!(f, "─");
    print_rep_char("─", m, f);
    write!(f, "─┴");
  }
  write!(f, "─");
  print_rep_char("─", m, f);
  write!(f, "─┘\n");
}