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

#[derive(Clone, PartialEq)]
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
// where each column represents an attribute, and each row represents an entity.

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

  // m x attributes and n x entities. n x m is the capacity of the table
  // while the actual size starts at 0 x 0 (since it is empty)
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

  // Supply a list of entities (rows), get them back in a vector.
  pub fn get_rows(&mut self, entities: Vec<u64>) -> Vec<Option<Vec<Value>>> {
    let mut rows: Vec<Option<Vec<Value>>> = vec![];
    for entity in entities {
      // Get the index for the given entity
      match self.entities.get(&entity) {
        Some(x) => {
          let mut row = self.data[x - 1].clone();
          row.truncate(self.cols);
          rows.push(Some(row));
        },
        None => rows.push(None),
      };
    }
    rows
  }

  // Supply a list of entities (rows), get them back in a vector.
  pub fn get_cols(&mut self, attributes: Vec<u64>) -> Vec<Option<Vec<Value>>> {
    let mut columns: Vec<Option<Vec<Value>>> = vec![];
    for attribute in attributes {
      let mut column: Vec<Value> = vec![];
      // Get the index for the given attribute
      match self.attributes.get(&attribute) {
        Some(x) => {
          // get the column from each row
          for i in 0 .. self.rows {
            let cell = self.data[i][*x - 1].clone();
            column.push(cell);
          }
          columns.push(Some(column));
        },
        None => columns.push(None),
      };
    }
    columns
  }

  pub fn index(&mut self, entity: u64, attribute: u64) -> Option<Value> {
    match self.entities.get(&entity) {
      Some(x) => {
        match self.attributes.get(&attribute) {
          Some(y) => Some(self.data[*x - 1][*y - 1].clone()),
          None => None,
        }
      },
      None => None,
    }
    //self.data[entity][attribute]
  }

}

// ### Pretty Printing Tables

impl fmt::Debug for Table {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      let cell_width = 15;
      let table_width = cell_width * self.cols + self.cols * 2;
      let header_width = table_width - self.cols - 1;

      // Print table header
      write!(f, "╔");
      print_repeated_char("═", header_width, f);
      write!(f, "╗\n");

      let table_name = format!("#{} ({:?})", self.name, self.id);
      write!(f, "║");
      print_cell_contents(table_name, header_width, f);
      write!(f, "║\n");

      let table_dimensions = format!("{:?} x {:?}", self.rows, self.cols);
      write!(f, "║");
      print_cell_contents(table_dimensions, header_width, f);
      write!(f, "║\n");

      write!(f, "╚");
      print_repeated_char("═", header_width, f);
      write!(f, "╝\n");

      write!(f, "\n");

      // Print table body
      print_top_border(self.cols, cell_width, f);
      for m in 0 .. self.rows {
        print_row(self.data[m].clone(), self.cols, cell_width, f);
      }
      print_bottom_border(self.cols, cell_width,  f);
      
      Ok(())
    }
}

fn print_repeated_char(to_print: &str, n: usize, f: &mut fmt::Formatter) {
  for i in 0..n {
    write!(f, "{}", to_print);
  }
}

fn print_top_border(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "┌");
  for i in 0 .. n - 1 {
    print_repeated_char("─", m, f);
    write!(f, "┬");
  }
  print_repeated_char("─", m, f);
  write!(f, "┐\n");
}

fn print_row(row: Vec<Value>, n: usize, cell_width: usize, f: &mut fmt::Formatter) {
  write!(f, "│");
  for i in 0 .. n {
    let content_string = format!("{:?}", row[i]);
    print_cell_contents(content_string, cell_width, f);
    write!(f, "│");
  }
  write!(f, "\n");
}

fn print_cell_contents(content_string: String, cell_width: usize, f: &mut fmt::Formatter) {
    // If the contents exceed the cell width, truncate it and add ellipsis
    if content_string.len() > cell_width {
      let mut truncated_content_string = content_string.clone();
      let content_width = cell_width - 3; 
      truncated_content_string.truncate(content_width);
      truncated_content_string.insert_str(content_width, "...");
      write!(f, "{}", truncated_content_string.clone());
    } else {
      write!(f, "{}", content_string.clone());
      let cell_padding = cell_width - content_string.len();
      for _ in 0 .. cell_padding {
        write!(f, " ");
      }
    }
}

fn print_bottom_border(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "└");
  for i in 0 .. n - 1 {
    print_repeated_char("─", m, f);
    write!(f, "┴");
  }
  print_repeated_char("─", m, f);
  write!(f, "┘\n");
}