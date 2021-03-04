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
use std::sync::Arc;
use std::cell::RefCell;
use errors::{Error, ErrorType};
use ::{humanize, hash_string};

// ## Row and Column

pub type Value = u64;

pub enum ValueType {
  Quantity,
  Boolean,
  String,
  Reference,
  NumberLiteral,
  Empty
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NumberLiteralKind {
  Decimal,
  Hexadecimal,
  Octal,
  Binary
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NumberLiteral {
  pub kind: NumberLiteralKind,
  pub bytes: Vec<u8>,
}

pub trait ValueMethods {
  fn empty() -> Value;
  fn from_string(string: String) -> Value;
  fn from_str(string: &str) -> Value;
  fn from_bool(boolean: bool) -> Value;
  fn from_u64(num: u64) -> Value;
  fn from_quantity(num: Quantity) -> Value;
  fn from_i64(num: i64) -> Value;
  fn from_f64(num: f64) -> Value;
  fn from_id(id: u64) -> Value;
  fn from_byte_vector(vector: &Vec<u8>) -> Value;
  fn value_type(&self) -> ValueType;
  fn as_quantity(&self) -> Option<Quantity>;
  fn as_u64(&self) -> Option<u64>;
  fn as_i64(&self) -> Option<i64>;
  fn as_float(&self) -> Option<f64>;
  fn as_string(&self) -> Option<u64>;
  fn as_byte_array(&self) -> Option<u64>;
  fn as_bool(&self) -> Option<bool>;
  fn as_reference(&self) -> Option<u64>;
  fn as_raw(&self) -> u64;
  fn is_empty(&self) -> bool;
  fn is_number(&self) -> bool;
  fn is_reference(&self) -> bool;
  fn equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn not_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn less_than(&self, other: Value) -> Result<Value, ErrorType>;
  fn less_than_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn greater_than(&self, other: Value) -> Result<Value, ErrorType>;
  fn greater_than_equal(&self, other: Value) -> Result<Value, ErrorType>;
  fn add(&self, other: Value) -> Result<Value, ErrorType>;
  fn sub(&self, other: Value) -> Result<Value, ErrorType>;
  fn multiply(&self, other: Value) -> Result<Value, ErrorType>;
  fn divide(&self, other: Value) -> Result<Value, ErrorType>;
  fn and(&self, other: Value) -> Result<Value, ErrorType>;
  fn or(&self, other: Value) -> Result<Value, ErrorType>;
}


impl ValueMethods for Value {

  fn empty() -> Value {
    0x2000000000000000
  }

  fn from_byte_vector(vector: &Vec<u8>) -> Value {
    let mut vector_hash = hash_string(&format!("byte vector: {:?}",vector));
    vector_hash = vector_hash + 0xC000000000000000;
    vector_hash
  }

  fn from_string(string: String) -> Value {
    let mut string_hash = hash_string(&string);
    string_hash = string_hash + 0x8000000000000000;
    string_hash
  }

  fn from_str(string: &str) -> Value {
    let mut string_hash = hash_string(string);
    string_hash = string_hash + 0x8000000000000000;
    string_hash
  }

  fn from_bool(boolean: bool) -> Value {
    match boolean {
      true => 0x4000000000000001,
      false => 0x4000000000000000,
    }
  }

  fn from_id(id: u64) -> Value {
    id + 0x2000000000000000
  }

  fn from_u64(num: u64) -> Value {
    num.to_quantity()
  }

  fn from_quantity(num: Quantity) -> Value {
    num
  }

  fn from_i64(num: i64) -> Value {
    num.to_quantity()
  }

  fn from_f64(num: f64) -> Value {
    num.to_quantity()
  }

  fn is_empty(&self) -> bool {
    if *self == Value::empty() {
      true
    } else {
      false
    }
  }

  fn value_type(&self) -> ValueType {
    match self.as_quantity() {
      Some(_) => ValueType::Quantity,
      None => {
        match self.as_string() {
          Some(_) => ValueType::String,
          None => {
            match self.as_reference() {
              Some(_) => ValueType::Reference,
              None => {
                match self.as_bool() {
                  Some(_) => ValueType::Boolean,
                  None => match self.as_byte_array() {
                    Some(_) => ValueType::NumberLiteral,
                    None => ValueType::Empty,
                  },
                }
              }
            }
          }
        }
      }
    }
  }

  fn as_raw(&self) -> u64 {
    self & 0x00FFFFFFFFFFFFFF
  }

  fn as_quantity(&self) -> Option<Quantity> {
    match self & 0xFF00000000000000 {
      0x2000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 |
      0x4000000000000000 => None,
      _ => Some(*self),
    }
  }

  fn as_reference(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x2000000000000000 => Some(self & 0x00FFFFFFFFFFFFFF),
      _ => None,
    }
  }

  fn as_u64(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x2000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 |
      0x4000000000000000 => None,
      _ => Some(self.to_u64()),
    }
  }

  fn is_number(&self) -> bool {
    match self & 0xFF00000000000000 {
      0x2000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 |
      0x4000000000000000 => false,
      _ => true,
    }
  }

  fn is_reference(&self) -> bool {
    match self & 0xFF00000000000000 {
      0x2000000000000000 => true,
      _ => false,
    }
  }    

  fn as_float(&self) -> Option<f64> {
    match self & 0xFF00000000000000 {
      0x2000000000000000 |
      0x8000000000000000 |
      0xC000000000000000 |
      0x4000000000000000 => None,
      _ => Some(self.to_float()),
    }
  }

  fn as_i64(&self) -> Option<i64> {
    None
  }

  fn as_string(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0x8000000000000000 => Some(*self),
      _ => None,
    }
  }

  fn as_bool(&self) -> Option<bool> {
    match self {
      0x4000000000000001 => Some(true),
      0x4000000000000000 => Some(false),
      _ => None,
    }
  }

  fn as_byte_array(&self) -> Option<u64> {
    match self & 0xFF00000000000000 {
      0xC000000000000000 => Some(*self),
      _ => None,
    }
  }

  fn equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.equal(r).unwrap())),
      _ => {
        match (self.as_string(), other.as_string()) {
          (Some(q), Some(r)) => Ok(Value::from_bool(q == r)),
          _ => Err(ErrorType::IncorrectFunctionArgumentType),
        }
      },
    } 
  }

  fn not_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.not_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn less_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn less_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.less_than_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn greater_than_equal(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q.greater_than_equal(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn add(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.add(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn sub(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.sub(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn multiply(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.multiply(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn divide(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_quantity(), other.as_quantity()) {
      (Some(q), Some(r)) => Ok(Value::from_quantity(q.divide(r).unwrap())),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn or(&self, other: Value) -> Result<Value, ErrorType>{
    match (self.as_bool(), other.as_bool()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q || r)),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
    } 
  }

  fn and(&self, other: Value) -> Result<Value, ErrorType> {
    match (self.as_bool(), other.as_bool()) {
      (Some(q), Some(r)) => Ok(Value::from_bool(q && r)),
      _ => Err(ErrorType::IncorrectFunctionArgumentType),
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
      &TableId::Local(ref id) => write!(f, "Local({:})", humanize(id)),
      &TableId::Global(ref id) => write!(f, "Global({:})", humanize(id)),
    }
  }
}

// ### Row or Column Index

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Index {
  Index(usize),
  Alias(u64),
  Table(TableId),
  All,
  None,
}

impl Index {
  pub fn unwrap(&self) -> usize {
    match self {
      Index::Index(ix) => *ix,
      Index::Alias(alias) => {
        alias.clone() as usize
      },
      Index::Table(table_id) => *table_id.unwrap() as usize,
      Index::None |
      Index::All => 0,
    }
  }
}

impl fmt::Debug for Index {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Index::Index(ref ix) => write!(f, "Ix({:?})", ix),
      &Index::Alias(ref alias) => write!(f, "IxAlias({:#x})", alias),
      &Index::Table(ref table_id) => write!(f, "IxTable({:?})", table_id),
      &Index::All => write!(f, "IxAll"),
      &Index::None => write!(f, "IxNone"),
    }
  }
}

// ## Table

// A 2D table of values.
#[derive(Clone)]
pub struct Table {
  pub id: u64,
  pub store: Arc<Store>,
  pub rows: usize,
  pub columns: usize,
  pub data: Vec<usize>, // Each entry is a memory address into the store
}

impl Table {

  pub fn new(table_id: u64, rows: usize, columns: usize, store: Arc<Store>) -> Table {
    Table {
      id: table_id,
      store,
      rows,
      columns,
      data: vec![0; rows*columns], // Initialize with zeros, indicating Value::Empty (always the zeroth element of the store)
    }
  }

  pub fn clear(&mut self) {
    for i in 1..=self.rows {
      for j in 1..=self.columns {
        let ix = self.index_unchecked(i, j);
        let address = self.data[ix];
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
        store.dereference(address);
      }
    }

    self.rows = 0;
    self.columns = 0;
    self.data.clear();
  }

  // Transform a (row, column) into a linear address into the data. If it's out of range, return None
  pub fn index(&self, row: &Index, column: &Index) -> Option<usize> {
    let rix = match row {
      &Index::Index(ix) => ix,
      _ => 0, // TODO aliases and all
    };
    let cix = match column {
      &Index::Index(0) => return Some(rix - 1),
      &Index::Index(ix) => ix,
      &Index::Alias(alias) => match self.store.column_alias_to_index.get(&(self.id,alias)) {
        Some(cix) => *cix,
        None => return None,
      },
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

  // Given a hash, get associated string
  pub fn get_string(&self, id: &u64) -> Option<&String> {
    self.store.strings.get(id)
  }

  // Get the memory address into the store at a (row, column)
  pub fn get_address(&self, row: &Index, column: &Index) -> Option<usize> {
    match self.index(row, column) {
      Some(ix) => Some(self.data[ix]),
      None => None,
    }
  }

  pub fn has_column_alias(&self, alias: u64) -> bool {
    match self.store.column_alias_to_index.get(&(self.id,alias)) {
      Some(_) => true,
      _ => false,
    }
  }

  // Get the memory address into the store at a (row, column)
  pub fn get_address_unchecked(&self, row: usize, column: usize) -> usize {
    let ix = self.index_unchecked(row, column);
    self.data[ix]
  }  
  
  // Get the value in the store at memory address (row, column)
  pub fn get_unchecked(&self, row: usize, column: usize) -> Value {
    let ix = self.index_unchecked(row, column);
    let address = self.data[ix];
    self.store.data[address]
  }

  // Get the value in the store at memory address (ix)
  pub fn get_unchecked_linear(&self, ix: usize) -> Value {
    let address = self.data[ix-1];
    self.store.data[address]
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
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
    if store.data[old_address] != value {
      store.changed = true;
      store.dereference(old_address);
      let new_address = store.intern(value);
      self.data[ix] = new_address;
    }
  }

  pub fn set_unchecked(&mut self, row: usize, column: usize, value: Value) {
    let ix = self.index_unchecked(row, column);
    let old_address = self.data[ix];
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
    if store.data[old_address] != value {
      store.changed = true;
      store.dereference(old_address);
      let new_address = store.intern(value);
      self.data[ix] = new_address;
    }
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let cell_width = 7;
    let rows = if self.rows > 150 {
      10
    } else {
      self.rows
    };
    
    let table_name = match self.store.strings.get(&self.id) {
      Some(name) => name.to_string(),
      None => format!("{}", humanize(&self.id)),
    };
    let columns = if self.columns == 0 {
      1
    } else {{
      self.columns
    }};
    let rows = if self.rows == 0 {
      1
    } else {{
      self.rows
    }};
    let table_header = format!("#{} ({} x {})", table_name, rows, columns);
    let header_width = table_header.len()+2;

    let aggregate_cell_width = (cell_width+2) * columns + columns-1;
    let cell_width = if header_width > aggregate_cell_width {
      header_width / columns - 2
    } else {
      cell_width
    };
    let table_width = if header_width > aggregate_cell_width {
      header_width
    } else  {
      aggregate_cell_width
    };

    print_top_span_border(columns, cell_width+2, f)?;
    write!(f,"│ ")?;
    print_cell_contents(&table_header, table_width-2, f)?;
    write!(f," │\n")?;
    print_inner_span_border(columns, cell_width+2, f)?;
    write!(f, "│ ", )?;
    for i in 1..=columns {
      let column_header = match self.store.column_index_to_alias.get(&(self.id,i)) {
        Some(alias) => {
          match self.store.strings.get(alias) {
            Some(alias_str) => alias_str.to_string(),
            None => humanize(alias),
          }
        },
        None => format!("{}", i),
      };
      print_cell_contents(&column_header, cell_width, f)?;
      write!(f, " │ ", )?;
    }
    write!(f, "\n")?;
    print_inner_border(columns, cell_width + 2, f)?;

    // Print the first 10 rows
    for i in 0..rows {
      write!(f, "│ ", )?;
      for j in 0..columns {
        match self.get_address(&Index::Index(i+1),&Index::Index(j+1)) {
          Some(x) => {
            let value = &self.store.data[x];
            let text = match value.as_quantity() {
              Some(quantity) => {
                format!("{}", value.format())
              },
              None => {
                match value.as_bool() {
                  Some(b) => format!("{:?}", b),
                  None => {
                    if value.is_empty() {
                      format!("_")
                    } else {
                      match value.as_reference() {
                        Some(value) => format!("@{}", humanize(&value)),
                        None => {
                          match self.store.strings.get(value) {
                            Some(q) => format!("{:?}", q),
                            None => format!("None"),
                          }
                          
                        }
                      }
                    }
                  }
                }
              },
            };
            print_cell_contents(&text, cell_width, f)?;
            write!(f, " │ ")?;
          },
          _ => {
            write!(f, " Empty │ ")?;
          },
        }
        
      }
      write!(f, "\n")?;
    }

    // Print the rest of the rows
    if rows > 150 {
      write!(f, "│ ")?;
      for j in 0..columns {
        print_cell_contents(&"...".to_string(), cell_width, f)?;
        write!(f, " │ ")?;
      }
      write!(f, "\n")?;
      for i in rows-3..rows {
        write!(f, "│ ", )?;
        for j in 0..columns {
          match self.get_address(&Index::Index(i+1),&Index::Index(j+1)) {
            Some(x) => {
              let value = &self.store.data[x];
              let text = match value.as_quantity() {
                Some(quantity) => {
                  format!("{}", value.format())
                },
                None => {
                  match value.as_bool() {
                    Some(b) => format!("{:?}", b),
                    None => {
                      if value.is_empty() {
                        format!("_")
                      } else {
                        match value.as_reference() {
                          Some(value) => format!("@{}", humanize(&value)),
                          None => {
                            format!("{:?}", self.store.strings.get(value).unwrap())
                          }
                        }
                      }
                    }
                  }
                },
              };
              print_cell_contents(&text, cell_width, f)?;
              write!(f, " │ ")?;
            },
            _ => (),
          }
          
        }
        write!(f, "\n")?;
      }
    }
    print_bottom_border(columns, cell_width + 2, f)?;
    
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