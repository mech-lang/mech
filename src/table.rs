// # Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

// ## Prelude

use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use crate::{Column, ValueKind, ColumnU8, ColumnU16, ColumnF32, humanize, Value};

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

// ## Table Shape

#[derive(Debug)]
pub enum TableShape {
  Scalar,
  Column(usize),
  Row(usize),
  Matrix(usize,usize),
  Pending,
}

// ### TableIndex

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TableIndex {
  Index(usize),
  Alias(u64),
  Table(TableId),
  All,
  None,
}

impl TableIndex {
  pub fn unwrap(&self) -> usize {
    match self {
      TableIndex::Index(ix) => *ix,
      TableIndex::Alias(alias) => {
        alias.clone() as usize
      },
      TableIndex::Table(table_id) => *table_id.unwrap() as usize,
      TableIndex::None |
      TableIndex::All => 0,
    }
  }
}

impl fmt::Debug for TableIndex {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &TableIndex::Index(ref ix) => write!(f, "Ix({:?})", ix),
      &TableIndex::Alias(ref alias) => write!(f, "IxAlias({:#x})", alias),
      &TableIndex::Table(ref table_id) => write!(f, "IxTable({:?})", table_id),
      &TableIndex::All => write!(f, "IxAll"),
      &TableIndex::None => write!(f, "IxNone"),
    }
  }
}

// ## Table

#[derive(Clone)]
pub struct Table {
  pub id: u64,
  pub rows: usize,
  pub cols: usize,
  pub col_kinds: Vec<ValueKind>,
  data: Vec<Column>,
}


impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
      rows,
      cols,
      data: Vec::with_capacity(cols),
      col_kinds: Vec::with_capacity(cols),
    };
    for col in 0..cols {
      table.data.push(Column::Empty);
      table.col_kinds.push(ValueKind::Empty);
    }
    table
  }

  pub fn get(&self, row: usize, col: usize) -> Option<Value> {
    if col < self.cols && row < self.rows {
      match &self.data[col] {
        Column::F32(column_f32) => Some(Value::F32(column_f32.borrow()[row])),
        Column::U8(column_u8) => Some(Value::U8(column_u8.borrow()[row])),
        Column::U16(column_u16) => Some(Value::U16(column_u16.borrow()[row])),
        Column::Bool(column_bool) => Some(Value::Bool(column_bool.borrow()[row])),
        Column::Empty => None,
      }
    } else {
      None
    }
  }

  pub fn set(&self, row: usize, col: usize, val: Value) -> Result<(),()> {
    if col < self.cols && row < self.rows {
      match (&self.data[col], val) {
        (Column::F32(column_f32), Value::F32(value_f32)) => column_f32.borrow_mut()[row] = value_f32,
        (Column::U8(column_u8), Value::U8(value_u8)) => column_u8.borrow_mut()[row] = value_u8,
        (Column::U16(column_u16), Value::U16(value_u16)) => column_u16.borrow_mut()[row] = value_u16,
        (Column::Empty, Value::U8(value_u8)) => {
          //let column: ColumnU8 = Rc::new(RefCell::new(Vec::new()));
          //self.data[col] = Column::U8(column);
        },
        _ => (),
      }
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn set_mut(&mut self, row: usize, col: usize, val: Value) -> Result<(),()> {
    if col < self.cols && row < self.rows {
      match (&mut self.data[col], val) {
        (Column::F32(column_f32), Value::F32(value_f32)) => column_f32.borrow_mut()[row] = value_f32,
        (Column::U8(column_u8), Value::U8(value_u8)) => column_u8.borrow_mut()[row] = value_u8,
        (Column::U16(column_u16), Value::U16(value_u16)) => column_u16.borrow_mut()[row] = value_u16,
        (Column::Empty, Value::U8(value_u8)) => {
          let column: ColumnU8 = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U8(column);
        },
        (Column::Empty, Value::U16(value_u16)) => {
          let column: ColumnU16 = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U16(column);
        },
        (Column::Empty, Value::F32(value_f32)) => {
          let column: ColumnF32 = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::F32(column);
        },
        _ => (),
      }
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn set_col_kind(&mut self, col: usize, val: ValueKind) -> Result<(),()> {
    if col < self.cols {
      match (&mut self.data[col], val) {
        (Column::Empty, ValueKind::U8) => {
          let column: ColumnU8 = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U8(column);
          self.col_kinds[col] = ValueKind::U8;
        },
        (Column::Empty, ValueKind::U16) => {
          let column: ColumnU16 = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U16(column);
          self.col_kinds[col] = ValueKind::U16;
        },
        (Column::Empty, ValueKind::F32) => {
          let column: ColumnF32 = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::F32(column);
          self.col_kinds[col] = ValueKind::F32;
        },
        _ => (),
      }
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn get_column(&self, col: usize) -> Option<Column> {
    if col < self.cols { 
      Some(self.data[col].clone())
    } else {
      None
    }
  }

  pub fn get_column_unchecked(&self, col: usize) -> Column {
    self.data[col].clone()
  }

  pub fn resize(&mut self, rows: usize, cols: usize) {
    if self.cols != cols {
      self.cols = cols;
      self.data.resize(cols, Column::Empty);
      self.col_kinds.resize(cols, ValueKind::Empty);
    }
    if self.rows != rows {
      self.rows = rows;
      // TODO NEED TO RESIZE ROWS
    }
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"│ {} ", humanize(&self.id))?;
    for row in 0..self.rows {
      write!(f,"│ ")?;
      for col in 0..self.cols {
        match self.get(row,col) {
          Some(v) => write!(f,"{:0.2?} │ ", v)?, 
          None => write!(f,"_ │")?,
        }
      }
      write!(f,"\n")?;
    }
    Ok(())
  }
}



/*
// # Table

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use quantities::{Quantity, QuantityMath};
use database::{Store};
use value::{Value, ValueMethods, NumberLiteral, NumberLiteralKind};
use std::sync::Arc;
//use errors::{Error, ErrorType};
use ::{humanize};

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

// ### TableIndex

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TableIndex {
  Index(usize),
  Alias(u64),
  Table(TableId),
  All,
  None,
}

impl TableIndex {
  pub fn unwrap(&self) -> usize {
    match self {
      TableIndex::Index(ix) => *ix,
      TableIndex::Alias(alias) => {
        alias.clone() as usize
      },
      TableIndex::Table(table_id) => *table_id.unwrap() as usize,
      TableIndex::None |
      TableIndex::All => 0,
    }
  }
}

impl fmt::Debug for TableIndex {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &TableIndex::Index(ref ix) => write!(f, "Ix({:?})", ix),
      &TableIndex::Alias(ref alias) => write!(f, "IxAlias({:#x})", alias),
      &TableIndex::Table(ref table_id) => write!(f, "IxTable({:?})", table_id),
      &TableIndex::All => write!(f, "IxAll"),
      &TableIndex::None => write!(f, "IxNone"),
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
  pub data: Vec<Value>,
  pub changed: Vec<bool>,
}

impl Table {

  pub fn new(table_id: u64, rows: usize, columns: usize, store: Arc<Store>) -> Table {
    Table {
      id: table_id,
      store,
      rows,
      columns,
      data: vec![Value::empty(); rows * columns], // Initialize with address zero, indicating Value::Empty (always the zeroth element of the store)
      changed: vec![false; rows * columns],
    }
  }

  pub fn clear(&mut self) {
    self.rows = 0;
    self.columns = 0;
    self.data.clear();
    self.changed.clear();
  }
  
  // Resize the table
  pub fn resize(&mut self, rows: usize, columns: usize) {
    self.rows = rows;
    self.columns = columns;
    self.data.resize(rows * columns, Value::empty());
    self.changed.resize(rows * columns, false);
  }

  pub fn reset_changed(&mut self) {
    for ix in 0..self.changed.len() {
      self.changed[ix] = false;
    }
  }

  // Transform a (row, column) into a linear address into the data. If it's out of range, return None
  pub fn index(&self, row: &TableIndex, column: &TableIndex) -> Option<usize> {
    let rix = match row {
      &TableIndex::Index(ix) => ix,
      _ => 0, // TODO aliases and all
    };
    let cix = match column {
      &TableIndex::Index(0) => {
        if rix <= self.rows * self.columns {
          return Some(rix - 1)
        } else {
          return None
        }
      },
      &TableIndex::Index(ix) => ix,
      &TableIndex::Alias(alias) => match self.store.column_alias_to_index.get(&(self.id,alias)) {
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
  pub fn get_string_from_hash(&self, hash: u64) -> Option<&String> {
    self.store.strings.get(&hash)
  }

  // Given a hash, get associated string
  pub fn get_number_literal_from_hash(&self, hash: u64) -> Option<&NumberLiteral> {
    self.store.number_literals.get(&hash)
  }

  // Given a hash, get associated string
  pub fn insert_string(&mut self, string: String) {
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
    store.strings.insert(Value::from_string(&string),string);
  }

  pub fn has_column_alias(&self, alias: u64) -> bool {
    match self.store.column_alias_to_index.get(&(self.id,alias)) {
      Some(_) => true,
      _ => false,
    }
  }

  pub fn get_column_alias(&self, index: usize) -> Option<TableIndex> {
    match self.store.column_index_to_alias.get(&(self.id,index)) {
      Some(alias) => Some(TableIndex::Alias(*alias)),
      _ => None,
    }
  }
  
  // Get the value in the store at memory address (row, column) and whether it has been changed
  pub fn get_unchecked(&self, row: usize, column: usize) -> (Value, bool) {
    let ix = self.index_unchecked(row, column);
    (self.data[ix], self.changed[ix])
  }

  // Get the value in the store at memory address (ix) and whether it has been changed
  pub fn get_unchecked_linear(&self, ix: usize) -> (Value, bool) {
    (self.data[ix-1], self.changed[ix-1])
  }

  // Get the value in the store at memory address (ix) and whether it has been changed
  pub fn get_linear(&self, index: &TableIndex) -> Option<(Value, bool)> {
    let ix = index.unwrap();
    if ix <= self.data.len() && ix > 0 {
      Some((self.data[ix-1], self.changed[ix-1]))
    } else {
      None
    }
  }

  // Get the value in the store at memory address (row, column)
  pub fn get(&self, row: &TableIndex, column: &TableIndex) -> Option<(Value,bool)> {
    match self.index(row, column) {
      Some(ix) => {
        Some((self.data[ix], self.changed[ix]))
      },
      None => None,
    }
  }

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_f64(&self, row: &TableIndex, column: &TableIndex) -> Option<f64> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match value.as_f64() {
          None => None,
          x => x,
        }
      },
      None => None,
    }
  }

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_f32(&self, row: &TableIndex, column: &TableIndex) -> Option<f32> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match value.as_f32() {
          None => None,
          x => x,
        }
      },
      None => None,
    }
  }

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_string(&self, row: &TableIndex, column: &TableIndex) -> Option<(&String,bool)> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match self.get_string_from_hash(value) {
          None => None,
          Some(x) => Some((x,self.changed[ix]))
        }
      },
      None => None,
    }
  }

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_number_literal(&self, row: &TableIndex, column: &TableIndex) -> Option<(&NumberLiteral,bool)> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match self.get_number_literal_from_hash(value) {
          None => None,
          Some(x) => Some((x,self.changed[ix]))
        }
      },
      None => None,
    }
  }

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_quantity(&self, row: &TableIndex, column: &TableIndex) -> Option<(Quantity,bool)> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match value.as_quantity() {
          None => None,
          Some(x) => Some((x,self.changed[ix]))
        }
      },
      None => None,
    }
  } 

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_u64(&self, row: &TableIndex, column: &TableIndex) -> Option<(u64,bool)> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match value.as_u64() {
          None => None,
          Some(x) => Some((x,self.changed[ix]))
        }
      },
      None => None,
    }
  } 

  // Get the value as an f64 in the store at memory address (row, column)
  pub fn get_reference(&self, row: &TableIndex, column: &TableIndex) -> Option<u64> {
    match self.index(row, column) {
      Some(ix) => {
        let value = self.data[ix];
        match value.as_reference() {
          None => None,
          x => x,
        }
      },
      None => None,
    }
  }
  
  // Set the value of at a (row, column). This will decrement the reference count of the value
  // at the old address, and insert the new value into the store while pointing the cell to the
  // new address.
  pub fn set(&mut self, row: &TableIndex, column: &TableIndex, value: Value) {
    match self.index(row, column) {
      Some(ix) => {
        if self.data[ix] != value {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
          store.changed = true;
          self.data[ix] = value;
          self.changed[ix] = true;
        }
      }
      None => (), // TODO Warn user that set was not successful due to out of bounds index
    }
  }

  pub fn set_data(&mut self, data: &Vec<Value>) {
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
    store.changed = true;
    self.data = data.to_vec();
    for ix in 0..self.data.len() {
      self.changed[ix] = true;
    }
  }

  pub fn set_unchecked(&mut self, row: usize, column: usize, value: Value) {
    let ix = self.index_unchecked(row, column);
    if self.data[ix] != value {
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
      store.changed = true;
      self.data[ix] = value;
      self.changed[ix] = true;
    }
  }

  pub fn set_unchecked_linear(&mut self, index: usize, value: Value) {
    let ix = index - 1;
    if self.data[ix] != value {
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
      store.changed = true;
      self.data[ix] = value;
      self.changed[ix] = true;
    }
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let cell_width = 7;   
    let table_name = match self.store.strings.get(&self.id) {
      Some(name) => name.to_string(),
      None => format!("{}", humanize(&self.id)),
    };
    let rows = self.rows;
    let columns = self.columns;
    let table_header = format!("#{} ({} x {})", table_name, rows, columns);
    let columns = if self.columns == 0 {1} else { self.columns };
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
        match self.get(&TableIndex::Index(i+1),&TableIndex::Index(j+1)) {
          Some((value,_)) => {
            let text = match value.as_quantity() {
              Some(_quantity) => {
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
                          match value.as_string() {
                            Some(_) => {
                              match self.store.strings.get(&value) {
                                Some(q) => format!("{:?}", q),
                                None => {format!("Missing String")},
                              }
                            }
                            None => {
                              match self.store.number_literals.get(&value) {
                                Some(number_literal) => {
                                  match number_literal.kind {
                                    NumberLiteralKind::Hexadecimal => {
                                      let mut tfm = format!("0x");
                                      for byte in &number_literal.bytes {
                                        tfm = format!("{}{:02x}",tfm, byte);
                                      }
                                      tfm
                                    }
                                    NumberLiteralKind::Binary => {
                                      let mut tfm = format!("0b");
                                      for byte in &number_literal.bytes {
                                        tfm = format!("{}{:b}",tfm, byte);
                                      }
                                      tfm
                                    }
                                    NumberLiteralKind::Octal => {
                                      let mut tfm = format!("0o");
                                      for byte in &number_literal.bytes {
                                        tfm = format!("{}{:o}",tfm, byte);
                                      }
                                      tfm
                                    }
                                    NumberLiteralKind::Decimal => {
                                      let mut tfm = format!("0d");
                                      for byte in &number_literal.bytes {
                                        tfm = format!("{}{:}",tfm, byte);
                                      }
                                      tfm
                                    }
                                  }
                                }
                                None => {
                                  format!("0x{:0x}", value & 0x00FFFFFFFFFFFFFF)
                                }
                              }
                            }
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
      for _j in 0..columns {
        print_cell_contents(&"...".to_string(), cell_width, f)?;
        write!(f, " │ ")?;
      }
      write!(f, "\n")?;
      for i in rows-3..rows {
        write!(f, "│ ", )?;
        for j in 0..columns {
          match self.get(&TableIndex::Index(i+1),&TableIndex::Index(j+1)) {
            Some((value,_)) => {
              let text = match value.as_quantity() {
                Some(_quantity) => {
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
                            format!("{:?}", self.store.strings.get(&value).unwrap())
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
    //write!(f, "{:?}", self.changed)?;
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
/*
fn print_top_border(n: usize, m: usize, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "┌")?;
  for _ in 0 .. n - 1 {
    print_repeated_char("─", m, f)?;
    write!(f, "┬")?;
  }
  print_repeated_char("─", m, f)?;
  write!(f, "┐\n")?;
  Ok(())
} */

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
*/