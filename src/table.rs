// # Entity, Attribute, Value

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use quantities::{Quantity, ToQuantity, QuantityMath};
use database::Store;
use hashbrown::hash_map::{HashMap, Entry};
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap, SerializeStruct};
use std::rc::Rc;
use std::cell::RefCell;


// ## Row and Column

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
  Number(Quantity),
  //Bool(bool),
  //Reference(TableId),
  //Empty,
}

impl Value {

  /*pub fn from_string(string: String) -> Value {
    Value::String(string)
  }*/

  /*pub fn from_str(string: &str) -> Value {
    Value::String(String::from(string))
  }*/

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


// ## Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

// ### Table Id

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum TableId {
  Local(u64),
  Global(u64),
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

// ### Row or Column Index

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Index {
  Index(usize),
  Alias(u64),
  All,
}

impl Index {
  pub fn unwrap(&self) -> usize {
    match self {
      Index::Index(ix) => *ix,
      Index::Alias(alias) => {
        alias.clone() as usize
      },
      Index::All => 0,
    }
  }
}

impl fmt::Debug for Index {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Index::Index(ref ix) => write!(f, "Ix({:#x})", ix),
      &Index::Alias(ref alias) => write!(f, "Alias({:#x})", alias),
      &Index::All => write!(f, "All"),
    }
  }
}

// ## Table

// A 2D table of values.
pub struct Table {
  pub id: u64,
  pub store: Rc<Store>,
  pub rows: usize,
  pub columns: usize,
  pub data: Vec<usize>, // Each entry is a memory address into the store
}

impl Table {

  pub fn new(table_id: u64, rows: usize, columns: usize, store: Rc<Store>) -> Table {
    Table {
      id: table_id,
      store,
      rows,
      columns,
      data: vec![0; rows*columns], // Initialize with zeros, indicating Value::Empty (always the zeroth element of the store)
    }
  }

  // Transform a (row, column) into a linear address into the data. If it's out of range, return None
  pub fn index(&self, row: &Index, column: &Index) -> Option<usize> {
    let rix = match row {
      &Index::Index(ix) => ix,
      _ => 0, // TODO aliases and all
    };
    let cix = match column {
      &Index::Index(ix) => ix,
      &Index::Alias(alias) => *self.store.column_alias_to_index.get(&(self.id,alias)).unwrap(),
      _ => 0, // TODO all
    };
    if rix <= self.rows && cix <= self.columns && rix > 0 && cix > 0 {
      Some((rix - 1) * self.columns + (cix - 1))
    } else {
      None
    }
  }

  // Transform a (row, column) into a linear address into the data. If it's out of range, return None
  pub fn index_unchecked(&self, row: usize, column: usize) -> usize {
    (row - 1) * self.columns + (column - 1)
  }

  // Get the memory address into the store at a (row, column)
  pub fn get_address(&self, row: &Index, column: &Index) -> Option<usize> {
    match self.index(row, column) {
      Some(ix) => Some(self.data[ix]),
      None => None,
    }
  }

  // Get the memory address into the store at a (row, column)
  pub fn get_address_unchecked(&self, row: usize, column: usize) -> usize {
    let ix = self.index_unchecked(row, column);
    self.data[ix]
  }
  

  // Get the value in the store at memory address (row, column)
  pub fn get(&self, row: &Index, column: &Index) -> Option<Value> {
    match self.index(row, column) {
      Some(ix) => {
        let address = self.data[ix];
        Some(self.store.data[address])
      },
      None => None,
    }
  }

  // Set the value of at a (row, column). This will decrement the reference count of the value
  // at the old address, and insert the new value into the store while pointing the cell to the
  // new address.
  pub fn set(&mut self, row: &Index, column: &Index, value: Value) {
    let ix = self.index(row, column).unwrap();
    let old_address = self.data[ix];
    let store = unsafe{&mut *Rc::get_mut_unchecked(&mut self.store)};
    store.dereference(old_address);
    let new_address = store.intern(value);
    self.data[ix] = new_address;
  }

  pub fn set_unchecked(&mut self, row: usize, column: usize, value: Value) {
    let ix = self.index_unchecked(row, column);
    let old_address = self.data[ix];
    let store = unsafe{&mut *Rc::get_mut_unchecked(&mut self.store)};
    store.dereference(old_address);
    let new_address = store.intern(value);
    self.data[ix] = new_address;
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let cell_width = 7;
    let rows = if self.rows > 15 {
      10
    } else {
      self.rows
    };
    
    
    let table_name = match self.store.identifiers.get(&self.id) {
      Some(name) => name.to_string(),
      None => format!("0x{:x}", self.id),
    };
    
    let table_header = format!("#{} ({} x {})", table_name, self.rows, self.columns);
    let header_width = table_header.len()+2;
    let aggregate_cell_width = (cell_width+2) * self.columns + self.columns-1;
    let cell_width = if header_width > aggregate_cell_width {
      header_width / self.columns - 2
    } else {
      cell_width
    };
    let table_width = if header_width > aggregate_cell_width {
      header_width
    } else  {
      aggregate_cell_width
    };
    print_top_span_border(self.columns, cell_width+2, f)?;
    write!(f,"│ ")?;
    print_cell_contents(&table_header, table_width-2, f)?;
    write!(f," │\n")?;
    print_inner_span_border(self.columns, cell_width+2, f)?;
    write!(f, "│ ", )?;
    for i in 1..=self.columns {
      let column_header = match self.store.column_index_to_alias.get(&(self.id,i)) {
        Some(alias) => self.store.identifiers.get(alias).unwrap().to_string(),
        None => format!("{}", i),
      };
      print_cell_contents(&column_header, cell_width, f)?;
      write!(f, " │ ", )?;
    }
    write!(f, "\n")?;
    print_inner_border(self.columns, cell_width + 2, f)?;
    for i in 0..rows {
      write!(f, "│ ", )?;
      for j in 0..self.columns {
        match self.get_address(&Index::Index(i+1),&Index::Index(j+1)) {
          Some(x) => {
            let value = &self.store.data[x];
            let text = format!("{:?}", value);
            write!(f, "{:?}", value)?;
            if text.len() < cell_width {
              for _ in 0..cell_width-text.len() {
                write!(f, " ")?;
              }
            }
            write!(f, " │ ")?;
          },
          _ => (),
        }
        
      }
      write!(f, "\n")?;
    }
    if self.rows > 10 {
      write!(f, "│ ")?;
      for j in 0..self.columns {
        print_cell_contents(&"...".to_string(), cell_width, f)?;
        write!(f, " │ ")?;
      }
      write!(f, "\n")?;
      for i in self.rows-3..self.rows {
        write!(f, "│ ", )?;
        for j in 0..self.columns {
          match self.get_address(&Index::Index(i+1),&Index::Index(j+1)) {
            Some(x) => {
              let value = &self.store.data[x];
              let text = format!("{:?}", value);
              print_cell_contents(&text, cell_width, f)?;
              write!(f, " │ ")?;
            },
            _ => (),
          }
          
        }
        write!(f, "\n")?;
      }
    }
    print_bottom_border(self.columns, cell_width + 2, f)?;
    
    Ok(())
  }
}

fn print_top_span_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "┌")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "─")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┐\n")?;
  Ok(())
}

fn print_top_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "┌")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "┬")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┐\n")?;
  Ok(())
}

fn print_bottom_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "└")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "┴")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┘\n")?;
  Ok(())
}


fn print_cell_contents(content_string: &String, cell_width: usize, f: &mut fmt::Formatter) -> fmt::Result {
  // If the contents exceed the cell width, truncate it and add ellipsis
  if content_string.len() > cell_width {
    let mut truncated_content_string = content_string.clone();
    let content_width = cell_width - 3; 
    truncated_content_string.truncate(content_width);
    truncated_content_string.insert_str(content_width, "...");
    write!(f, "{}", truncated_content_string.clone())?;
  } else {
    write!(f, "{}", content_string.clone())?;
    let cell_padding = cell_width - content_string.len();
    for _ in 0 .. cell_padding {
      write!(f, " ")?;
    }
  }
  Ok(())
}

fn print_inner_span_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "├")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "┬")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┤\n")?;
  Ok(())
}

fn print_inner_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "├")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "┼")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┤\n")?;
  Ok(())
}

fn print_repeated_char(to_print: &str, n: usize, f: &mut fmt::Formatter) -> fmt::Result {
  for _ in 0..n {
    write!(f, "{}", to_print)?;
  }
  Ok(())
}

/*
// ### Table

#[derive(Clone, PartialEq)]
pub struct Table {
  pub id: u64,
  pub rows: u64,
  pub columns: u64,
  pub column_aliases: HashMap<u64,u64>,
  pub column_index_to_alias: Vec<Option<u64>>, 
  pub row_aliases: HashMap<u64, u64>,
  pub data: Vec<Vec<Rc<Value>>>,
}

impl Table {

  pub fn new(tag: u64, rows: u64, columns: u64) -> Table {
    Table {
      id: tag,
      rows: rows,
      columns: columns,
      column_aliases: HashMap::new(),
      column_index_to_alias: Vec::new(),
      row_aliases: HashMap::with_capacity(rows as usize),
      data: vec![vec![Rc::new(Value::Empty); rows as usize]; columns as usize], 
    }
  }

  pub fn clear(&mut self) {
    self.rows = 0;
    self.columns = 0;
    self.row_aliases.clear();
    self.column_aliases.clear();
    self.data.clear();
  }

  pub fn get_row_index(&self, row: &Index) -> Option<usize> {
    match row {
      Index::Index(ix) => Some(*ix),
      Index::Alias(alias) => match self.row_aliases.get(&(*alias as u64)) {
        Some(ix) => Some(ix.clone() as usize),
        None => None,
      },
      Index::All => None,
    }
  }

  pub fn get_column_alias(&self, column: &Index) -> Option<u64> {
    match column {
      Index::Index(ix) => {
        if self.column_index_to_alias.len() > 0 {
          self.column_index_to_alias[*ix as usize - 1]
        } else {
          None
        }
      },
      Index::Alias(alias) => Some(*alias as u64),
      Index::All => None,
    }
  }

  pub fn get_column_index(&self, column: &Index) -> Option<u64> {
    match column {
      Index::Index(ix) => Some(*ix as u64),
      Index::Alias(alias) => match self.column_aliases.get(&(*alias as u64)) {
        Some(ix) => Some(ix.clone()),
        None => None,
      },
      Index::All => None,
    }
  }

  pub fn set_cell(&mut self, row: &Index, column: &Index, value: Rc<Value>) {
    let row_ix = self.get_row_index(row).unwrap() as usize;
    let column_ix = self.get_column_index(column).unwrap() as usize;
    self.grow_to_fit(row_ix as u64, column_ix as u64);
    //let old_value = self.data[column_ix - 1][row_ix - 1].clone();
    self.data[column_ix - 1][row_ix - 1] = value.clone();
    //old_value
  }

  pub fn set_column_alias(&mut self, alias: u64, ix: u64) {
    match self.column_aliases.entry(alias) {
      Entry::Occupied(_) => {
        ()
      },
      Entry::Vacant(v) => {
        v.insert(ix);
        if self.column_index_to_alias.len() <= ix as usize - 1 {
          self.column_index_to_alias.resize(ix as usize, None);
        }
        self.column_index_to_alias[ix as usize - 1] = Some(alias); 
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

  pub fn get_columns_by_ixes(&self, column_ixes: Vec<usize>) -> Vec<Option<&Vec<Value>>> {
    let mut columns: Vec<Option<&Vec<Value>>> = vec![];
    for ix in column_ixes{
      let column = self.get_column_by_ix(ix);
      columns.push(column);
    }
    columns
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
        let new_column = vec![Rc::new(Value::Empty); self.rows as usize];
        self.data.resize(columns as usize, new_column);
      }
      self.columns = columns;
    }
    if rows > self.rows {
      for column in &mut self.data {
        column.resize(rows as usize, Rc::new(Value::Empty));
      }
      self.rows = rows;
    }    
  }

  pub fn shrink_to_fit(&mut self, rows: u64, columns: u64) {
    if columns < self.columns {
      // The new row is larger than the underlying column structure
      if columns > self.data.len() as u64 {
        let new_column = vec![Rc::new(Value::Empty); self.rows as usize];
        self.data.resize(columns as usize, new_column);
      }
      self.columns = columns;
    }
    if rows < self.rows {
      for column in &mut self.data {
        column.resize(rows as usize, Rc::new(Value::Empty));
      }
      self.rows = rows;
    }    
  }
  
  pub fn get_column(&self, column: &Index) -> Option<&Vec<Rc<Value>>> {
    match self.get_column_index(column) {
      Some(column_ix) => {
        Some(&self.data[column_ix as usize - 1])
      }
      None => None,
    }
  }

  pub fn get_row(&self, row: &Index) -> Option<Vec<Rc<Value>>> {
    match self.get_row_index(row) {
      Some(row_ix) => {
        let mut row: Vec<Rc<Value>> = vec![];
        // Get the index for the given attribute
        for column_ix in 0 .. self.columns as usize {
          let cell = self.data[column_ix][row_ix as usize - 1].clone();
          row.push(cell);
        }
        Some(row)
      }
      None => None,
    }
  }

  // Index into a cell without having to access the data member directly
  pub fn index(&self, row: &Index, column: &Index) -> Option<&Value> {
    let row_ix = self.get_row_index(row).unwrap();
    let column_ix = self.get_column_index(column).unwrap();
    if column_ix <= self.columns && row_ix <= self.rows as usize {
      Some(&self.data[column_ix as usize - 1][row_ix as usize - 1])
    } else {
      None
    }
  }

  // Clear a cell, setting it's value to Value::Empty
  pub fn clear_cell(&mut self, row: &Index, column: &Index) {
    self.set_cell(row, column, Rc::new(Value::Empty));
  }

}

// ### Pretty Printing Tables
impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let cell_width: usize = 30;
    
    let columns: usize = if self.columns > self.column_aliases.len() as u64 {
      self.columns as usize
    } else {
      self.column_aliases.len()
    };
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
      let mut column_labels: Vec<Rc<Value>> = Vec::new();
      for i in 1..columns + 1 {
        column_labels.push(Rc::new(Value::from_string(format!("{}", i))));
      }
      for (alias, ix) in self.column_aliases.iter() {
        column_labels[*ix as usize - 1] = Rc::new(Value::from_string(format!("{:?} ({:#x})", ix, alias)));
      }
      print_row(column_labels, cell_width as usize, f);
      print_inner_border(self.columns as usize, cell_width as usize,  f);
      for m in 1 .. max_rows + 1 {
        print_row(self.get_row(&Index::Index(m as usize)).unwrap(), cell_width as usize, f);
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

fn print_row(row: Vec<Rc<Value>>, cell_width: usize, f: &mut fmt::Formatter) {
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
}*/