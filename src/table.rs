// # Table

// ## Prelude

#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use quantities::{QuantityMath};
use database::{Store};
use value::{Value, ValueMethods};
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
  pub changed: Vec<bool>,
  pub transaction_boundaries: Vec<usize>,
  pub history: Vec<(Index, Index, Value)>,
}

impl Table {

  pub fn new(table_id: u64, rows: usize, columns: usize, store: Arc<Store>) -> Table {
    Table {
      id: table_id,
      store,
      rows,
      columns,
      data: vec![0; rows*columns], // Initialize with address zero, indicating Value::Empty (always the zeroth element of the store)
      changed: vec![false; rows*columns],
      transaction_boundaries: Vec::new(),
      history: Vec::new(),
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
    self.changed.clear();
  }
  
  // Resize the table
  pub fn resize(&mut self, rows: usize, columns: usize) {
    self.rows = rows;
    self.columns = columns;
    self.data.resize(rows * columns, 0);
    self.changed.resize(rows * columns, false);
  }

  pub fn reset_changed(&mut self) {
    for ix in 0..self.changed.len() {
      self.changed[ix] = false;
    }
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
  
  // Get the value in the store at memory address (row, column) and whether it has been changed
  pub fn get_unchecked(&self, row: usize, column: usize) -> (Value, bool) {
    let ix = self.index_unchecked(row, column);
    let address = self.data[ix];
    (self.store.data[address], self.changed[ix])
  }

  // Get the value in the store at memory address (ix) and whether it has been changed
  pub fn get_unchecked_linear(&self, ix: usize) -> (Value, bool) {
    let address = self.data[ix-1];
    (self.store.data[address], self.changed[ix-1])
    
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
      self.changed[ix] = true;
      self.history.push((*row,*column,value));
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
      self.changed[ix] = true;
      self.history.push((Index::Index(row),Index::Index(column),value));
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
    let columns = self.columns;
    let rows = self.rows;
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
      for _j in 0..columns {
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
    write!(f, "{:?}", self.changed)?;
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