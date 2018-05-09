// # Entity, Attribute, Value

// ## Prelude

use core::fmt;
use indexes::Hasher;
use alloc::{Vec, String};
use hashmap_core::map::HashMap;

// ## Row and Column

#[derive(Clone, PartialEq)]
pub enum Value {
  Number(i64),
  String(String),
  Table(u64),
  Bool(bool),
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
    Value::Number(num as i64)
  }

  pub fn from_i64(num: i64) -> Value {
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
        &Value::Table(ref x) => write!(f, "{}", x),
        &Value::Bool(ref b) => write!(f, "{}", b),
      }
    }
}

// ## Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

#[derive(Clone, PartialEq)]
pub struct Table {
  pub id: u64,
  pub rows: usize,
  pub columns: usize,
  pub data: Vec<Vec<Value>>,
  pub attributes: HashMap<u64, usize>,
  pub entities: HashMap<u64, usize>,
}

impl Table {

  pub fn new(tag: u64, rows: usize, columns: usize) -> Table {
    Table {
      id: tag,
      rows: 0,
      columns: 0,
      data: vec![vec![Value::Empty; rows]; columns], 
      entities: HashMap::with_capacity(rows),
      attributes: HashMap::with_capacity(columns),
    }
  }

  pub fn get_row_index(&mut self, entity: u64) -> Option<&usize> {
    self.entities.get(&entity)
  }

  pub fn get_column_index(&mut self, attribute: u64) -> Option<&usize> {
    self.attributes.get(&attribute)
  }

  pub fn set_cell(&mut self, row_ix: usize, column_ix: usize, value: Value) -> Result<(), &str> {
    if row_ix > 0 && column_ix > 0 &&
       self.rows > 0 && row_ix <= self.rows &&
       self.columns > 0 && column_ix <= self.columns {
      self.data[column_ix - 1][row_ix - 1] = value;
      Ok(())
    } else {
      Err("Index out of table bounds.")
    }
  }

  /*
  pub fn add_row(&mut self) {
    let rows = self.rows + 1;
    let cols = self.columns;
    self.grow_to_fit(rows, cols);
  }

  pub fn add_column(&mut self, attribute: u64) {
    if !self.attributes.contains_key(&attribute) {
      self.columns = self.columns + 1;
      self.attributes.insert(attribute.clone(), self.columns.clone());
    };
  }*/

  pub fn get_columns(&self, column_ixes: Vec<usize>) -> Vec<Option<&Vec<Value>>> {
    let mut columns: Vec<Option<&Vec<Value>>> = vec![];
    for ix in column_ixes{
      let column = self.get_column(ix);
      columns.push(column);
    }
    columns
  }

  pub fn get_column(&self, column_ix: usize) -> Option<&Vec<Value>> {
    if column_ix - 1 < self.columns {
      let mut column = &self.data[column_ix - 1];      
      //column.truncate(self.rows);
      Some(column)
    } else {
      None
    }
  }

  pub fn grow_to_fit(&mut self, rows: usize, columns: usize) {
    if columns > self.columns {
      // The new row is larger than the underlying column structure
      if columns > self.data.len() {
        self.data.resize(columns, Vec::new());
      }
      self.columns = columns;
    }

    if rows > self.rows {
      // The new row is larger than the underlying row structure
      if rows > self.data[0].len() {
        for column in &mut self.data {
          column.resize(rows, Value::Empty);
        }
      }
      self.rows = rows;
    }    
  }
  
  pub fn get_rows(&self, row_ixes: Vec<usize>) -> Vec<Option<Vec<Value>>> {
    let mut rows: Vec<Option<Vec<Value>>> = vec![];
    for ix in row_ixes {
      let row = self.get_row(ix);
      rows.push(row);
    }
    rows
  }

  pub fn get_row(&self, row_ix: usize) -> Option<Vec<Value>> {
    if row_ix - 1 < self.rows {
      let mut row: Vec<Value> = vec![];
      // Get the index for the given attribute
      for column_ix in 0 .. self.columns {
        let cell = self.data[column_ix][row_ix - 1].clone();
        row.push(cell);
      }
      Some(row)
    } else {
      None
    }
  }

  // Index into a cell without having to access the data member directly
  pub fn index(&self, row_ix: usize, column_ix: usize) -> Option<&Value> {
    if column_ix <= self.columns && row_ix <= self.rows {
      Some(&self.data[column_ix - 1][row_ix - 1])
    } else {
      None
    }
  }

  // Clear a cell, setting it's value to Value::Empty
  pub fn clear_cell(&mut self, row_ix: usize, column_ix: usize) -> Result<(), &str> {
    self.set_cell(row_ix, column_ix, Value::Empty)
  }



}

// ### Pretty Printing Tables

impl fmt::Debug for Table {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      let cell_width = 15;
      let mut table_width = cell_width * self.columns + self.columns * 2;
      if table_width < 20 {
        table_width = 20;
      }
      let header_width = table_width - self.columns - 1;

      // Print table header
      write!(f, "╔").unwrap();
      print_repeated_char("═", header_width, f);
      write!(f, "╗\n").unwrap();

      let table_name = format!("#{:X}", self.id);
      write!(f, "║").unwrap();
      print_cell_contents(table_name, header_width, f);
      write!(f, "║\n").unwrap();

      let table_dimensions = format!("{:?} x {:?}", self.rows, self.columns);
      write!(f, "║").unwrap();
      print_cell_contents(table_dimensions, header_width, f);
      write!(f, "║\n").unwrap();

      write!(f, "╚").unwrap();
      print_repeated_char("═", header_width, f);
      write!(f, "╝\n").unwrap();

      // Print table body
      if self.columns > 0 {
        print_top_border(self.columns, cell_width, f);
        let max_rows = if self.rows > 10 {
          10
        } else {
          self.rows
        };
        for m in 1 .. max_rows + 1 {
          print_row(self.get_row(m).unwrap(), self.columns, cell_width, f);
        }
        print_bottom_border(self.columns, cell_width,  f);
      }
      Ok(())
    }
}

fn print_repeated_char(to_print: &str, n: usize, f: &mut fmt::Formatter) {
  for _ in 0..n {
    write!(f, "{}", to_print).unwrap();
  }
}

fn print_top_border(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "┌").unwrap();
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f);
    write!(f, "┬").unwrap();
  }
  print_repeated_char("─", m, f);
  write!(f, "┐\n").unwrap();
}

fn print_row(row: Vec<Value>, n: usize, cell_width: usize, f: &mut fmt::Formatter) {
  write!(f, "│").unwrap();
  for i in 0 .. n {
    let content_string = format!("{:?}", row[i]);
    print_cell_contents(content_string, cell_width, f);
    write!(f, "│").unwrap();
  }
  write!(f, "\n").unwrap();
}

fn print_cell_contents(content_string: String, cell_width: usize, f: &mut fmt::Formatter) {
    // If the contents exceed the cell width, truncate it and add ellipsis
    if content_string.len() > cell_width {
      let mut truncated_content_string = content_string.clone();
      let content_width = cell_width - 3; 
      truncated_content_string.truncate(content_width);
      truncated_content_string.insert_str(content_width, "...");
      write!(f, "{}", truncated_content_string.clone()).unwrap();
    } else {
      write!(f, "{}", content_string.clone()).unwrap();
      let cell_padding = cell_width - content_string.len();
      for _ in 0 .. cell_padding {
        write!(f, " ").unwrap();
      }
    }
}

fn print_bottom_border(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "└").unwrap();
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f);
    write!(f, "┴").unwrap();
  }
  print_repeated_char("─", m, f);
  write!(f, "┘\n").unwrap();
}