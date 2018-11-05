// # Entity, Attribute, Value

// ## Prelude

use core::fmt;
use alloc::string::String;
use alloc::vec::Vec;
use hashmap_core::map::{HashMap, Entry};

// ## Row and Column

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Number(i64),
  String(String),
  Table(u64),
  Bool(bool),
  Reference((u64,Vec<u64>,Vec<u64>)),
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

  pub fn as_u64(&self) -> Option<u64> {
    match self {
      Value::Number(n) => Some(*n as u64),
      _ => None,
    }
  }

  pub fn as_i64(&self) -> Option<i64> {
    match self {
      Value::Number(n) => Some(*n),
      _ => None,
    }
  }

  pub fn as_string(&self) -> Option<&String> {
    match self {
      Value::String(n) => Some(n),
      _ => None,
    }
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
      &Value::Reference(ref b) => write!(f, "{:?}", b),
    }
  }
}

// ## Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum TableId {
  Local(u64),
  Global(u64)
}

impl TableId {
  pub fn unwrap(&self) -> &u64 {
    match self {
      TableId::Local(id) => id,
      TableId::Global(id) => id,
    }
  }
}

impl fmt::Debug for TableId {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &TableId::Local(ref id) => write!(f, "Local({:#x})", id),
      &TableId::Global(ref id) => write!(f, "Global({:#x})", id),
    }
  }
}


#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Index {
  Index(u64),
  Alias(u64)
}

impl Index {
  pub fn unwrap(&self) -> &u64 {
    match self {
      Index::Index(ix) => ix,
      Index::Alias(alias) => alias,
    }
  }
}

impl fmt::Debug for Index {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Index::Index(ref ix) => write!(f, "Ix({:#x})", ix),
      &Index::Alias(ref alias) => write!(f, "Alias({:#x})", alias),
    }
  }
}

#[derive(Clone, PartialEq)]
pub struct Table {
  pub id: u64,
  pub rows: u64,
  pub columns: u64,
  pub column_aliases: HashMap<u64, u64>,
  pub row_aliases: HashMap<u64, u64>,
  pub data: Vec<Vec<Value>>,
}

impl Table {

  pub fn new(tag: u64, rows: u64, columns: u64) -> Table {
    Table {
      id: tag,
      rows: rows,
      columns: columns,
      column_aliases: HashMap::with_capacity(columns as usize),
      row_aliases: HashMap::with_capacity(rows as usize),
      data: vec![vec![Value::Empty; rows as usize]; columns as usize], 
    }
  }

  pub fn clear(&mut self) {
    self.rows = 0;
    self.columns = 0;
    self.row_aliases.clear();
    self.column_aliases.clear();
    self.data.clear();
  }

  pub fn get_row_index(&self, row: &Index) -> Option<u64> {
    match row {
      Index::Index(ix) => Some(*ix),
      Index::Alias(alias) => match self.row_aliases.get(&alias) {
        Some(ix) => Some(ix.clone()),
        None => None,
      },
    }
  }

  pub fn get_column_index(&self, column: &Index) -> Option<u64> {
    match column {
      Index::Index(ix) => Some(*ix),
      Index::Alias(alias) => match self.column_aliases.get(&alias) {
        Some(ix) => Some(ix.clone()),
        None => None,
      },
    }
  }

  pub fn set_cell(&mut self, row: &Index, column: &Index, value: Value) -> Value {
    let row_ix = self.get_row_index(row).unwrap() as usize;
    let column_ix = self.get_column_index(column).unwrap() as usize;
    let old_value = self.data[column_ix - 1][row_ix - 1].clone();
    self.data[column_ix - 1][row_ix - 1] = value;
    old_value
  }

  pub fn set_column_alias(&mut self, alias: u64, ix: u64) {
    match self.column_aliases.entry(alias) {
      Entry::Occupied(_) => {
        ()
      },
      Entry::Vacant(v) => {
        v.insert(ix);
      },
    }
  }
  /*
  pub fn add_row(&mut self) {
    let rows = self.rows + 1;
    let columns = self.columns;
    self.grow_to_fit(rows, columns);
  }

  pub fn add_column(&mut self, attribute: u64) {
    if !self.column_aliases.contains_key(&attribute) {
      let columns = self.columns + 1;
      let rows = self.rows;
      self.grow_to_fit(rows, columns);
      self.column_aliases.insert(attribute.clone(), columns);
      if self.column_ids[columns - 1] == None {
        self.column_ids[columns - 1] = Some(attribute.clone());
      }
    };
  }

  pub fn get_columns(&self, column_ids: Vec<usize>) -> Vec<Option<&Vec<Value>>> {
    let mut columns: Vec<Option<&Vec<Value>>> = vec![];
    for id in column_ids{
      let column = self.get_column_by_id(id);
      columns.push(column);
    }
    columns
  }

  pub fn get_column_by_id(&self, column_id: usize) -> Option<&Vec<Value>> {
    match self.column_aliases.get(&(column_id as u64)) {
      Some(column_ix) => {
        let column_data = &self.data[*column_ix - 1];      
        Some(column_data)
      },
      None => None,
    }
  }

  pub fn get_columns_by_ixes(&self, column_ixes: Vec<usize>) -> Vec<Option<&Vec<Value>>> {
    let mut columns: Vec<Option<&Vec<Value>>> = vec![];
    for ix in column_ixes{
      let column = self.get_column_by_ix(ix);
      columns.push(column);
    }
    columns
  }

  pub fn get_column_by_ix(&self, column_ix: usize) -> Option<&Vec<Value>> {
    if self.columns > 0 && self.columns >= column_ix {
      let column_data = &self.data[column_ix - 1];      
      Some(column_data)
    } else {
      None
    }
  }

  pub fn get_column_mut_by_ix(&mut self, column_ix: usize) -> Option<&mut Vec<Value>> {
    if self.columns > 0 && self.columns >= column_ix {
      let column_data = &mut self.data[column_ix - 1];      
      Some(column_data)
    } else {
      None
    }
  }

  pub fn get_column_mut(&mut self, column_id: usize) -> Option<&mut Vec<Value>> {
    match self.column_aliases.get_mut(&(column_id as u64)) {
      Some(column_ix) => {
        let column_data = &mut self.data[*column_ix - 1];      
        Some(column_data)
      },
      None => None,
    }
  }*/

  pub fn grow_to_fit(&mut self, rows: u64, columns: u64) {
    if columns > self.columns {
      // The new row is larger than the underlying column structure
      if columns > self.data.len() as u64 {
        let new_column = vec![Value::Empty; self.rows as usize];
        self.data.resize(columns as usize, new_column);
      }
      self.columns = columns;
    }
    if rows > self.rows {
      for column in &mut self.data {
        column.resize(rows as usize, Value::Empty);
      }
      self.rows = rows;
    }    

  }/*
  
  pub fn get_rows(&self, row_ixes: Vec<usize>) -> Vec<Option<Vec<Value>>> {
    let mut rows: Vec<Option<Vec<Value>>> = vec![];
    for ix in row_ixes {
      let row = self.get_row(ix);
      rows.push(row);
    }
    rows
  }
  */
  pub fn get_row(&self, row: &Index) -> Option<Vec<Value>> {
    match self.get_row_index(row) {
      Some(row_ix) => {
        let mut row: Vec<Value> = vec![];
        // Get the index for the given attribute
        for column_ix in 0 .. self.columns as usize {
          let cell = self.data[column_ix][row_ix as usize - 1].clone();
          row.push(cell);
        }
        Some(row)
      }
      None => None,
    }
  }/*

  // Index into a cell without having to access the data member directly
  pub fn index(&self, row_ix: usize, column_ix: usize) -> Option<&Value> {
    if column_ix <= self.columns && row_ix <= self.rows {
      Some(&self.data[column_ix - 1][row_ix - 1])
    } else {
      None
    }
  }

  pub fn index_by_alias(&self, row_ix: usize, column_alias: &u64) -> Option<&Value> {
    let column_ix = self.column_aliases.get(column_alias).unwrap();
    if *column_ix <= self.columns && row_ix <= self.rows {
      Some(&self.data[column_ix - 1][row_ix - 1])
    } else {
      None
    }
  }

  // Clear a cell, setting it's value to Value::Empty
  pub fn clear_cell(&mut self, row_ix: usize, column_ix: usize) -> Result<Value, &str> {
    self.set_cell(row_ix, column_ix, Value::Empty)
  }*/

}

// ### Pretty Printing Tables
impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let cell_width: usize = 15;
    let columns = self.columns as usize;
    let mut table_width: usize = cell_width * columns + columns * 2;
    if table_width < 20 {
      table_width = 20;
    }
    let header_width: usize = table_width - columns - 1 ;

    // Print table header
    write!(f, "╔").unwrap();
    print_repeated_char("═", header_width, f);
    write!(f, "╗\n").unwrap();

    let table_name = format!("#{:#x}", self.id);
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
      //print_top_border(self.columns, cell_width, f);
      let max_rows = if self.rows > 10 {
        10
      } else {
        self.rows
      };
      let mut column_labels: Vec<Value> = Vec::new();
      /*for (ix, id) in self.column_ids.iter().enumerate() {
        match id {
          Some(column_id) => column_labels.push(Value::from_string(format!("{:?} ({:#x})", ix + 1, *column_id))),
          None => column_labels.push(Value::from_u64(ix as u64 + 1)),
        }
      }*/
      print_row(column_labels, cell_width as usize, f);
      print_inner_border(self.columns as usize, cell_width as usize,  f);
      for m in 1 .. max_rows + 1 {
        print_row(self.get_row(&Index::Index(m)).unwrap(), cell_width as usize, f);
      }
      print_bottom_border(self.columns as usize, cell_width as usize,  f);
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

fn print_row(row: Vec<Value>, cell_width: usize, f: &mut fmt::Formatter) {
  write!(f, "│").unwrap();
  for value in row {
    let content_string = format!("{:?}", value);
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

fn print_inner_border(n: usize, m: usize, f: &mut fmt::Formatter) {
  write!(f, "├").unwrap();
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f);
    write!(f, "┼").unwrap();
  }
  print_repeated_char("─", m, f);
  write!(f, "┤\n").unwrap();
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