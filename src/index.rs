use table::{Table, TableId, TableIndex};
use value::{Value, ValueMethods};
use database::{Store};
//use errors::{ErrorType};
use std::sync::Arc;
use rust_core::fmt;

pub struct  ValueIterator {
  pub scope: TableId,
  pub table: *mut Table,
  pub row_index: TableIndex,
  pub column_index: TableIndex,
  pub row_iter: IndexRepeater,
  pub column_iter: IndexRepeater,
}

impl ValueIterator {
  
  pub fn rows(&self) -> usize {
    unsafe{ (*self.table).rows }
  }

  pub fn columns(&self) -> usize {
    match self.column_index {
      TableIndex::All => unsafe{ (*self.table).columns },
      TableIndex::Index{..} |
      TableIndex::Alias{..} => 1,
      _ => unsafe{ (*self.table).columns },
    }
    
  }

  pub fn get(&self, row: &TableIndex, column: &TableIndex) -> Option<Value> {
    unsafe{(*self.table).get(row,column)}
  }

  pub fn get_unchecked(&self, row: usize, column: usize) -> (Value, bool) {
    unsafe{(*self.table).get_unchecked(row,column)}
  }

  pub fn get_unchecked_linear(&self, index: usize) -> (Value, bool) {
    unsafe{(*self.table).get_unchecked_linear(index)}
  }

  pub fn set(&self, row: &TableIndex, column: &TableIndex, value: Value) {
    unsafe{(*self.table).set(row, column, value)};
  }

  pub fn set_unchecked(&self, row: usize, column: usize, value: Value) {
    unsafe{(*self.table).set_unchecked(row, column, value)};
  }

  pub fn next_address(&mut self) -> Option<(usize, usize)> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        Some((rix.unwrap(),cix.unwrap()))
      },     
      _ => None,
    }
  }

  pub fn resize(&mut self, rows: usize, columns: usize)  {
    unsafe {

      (*self.table).resize(rows, columns);

      match self.row_index {
        TableIndex::All => {
          match (*self.table).rows {
            0 => self.row_iter = IndexRepeater::new(IndexIterator::None,1,1),
            r => self.row_iter = IndexRepeater::new(IndexIterator::Range(1..=r),1,1),
          }
        }
        _ => (),
      }

      match self.column_index {
        TableIndex::All => {
          match (*self.table).rows {
            0 => self.column_iter = IndexRepeater::new(IndexIterator::None,1,1),
            c => self.column_iter = IndexRepeater::new(IndexIterator::Range(1..=c),1,1),
          }
        }
        _ => (),
      }

    }
  }

}

impl Iterator for ValueIterator {
  type Item = (Value, bool);
  fn next(&mut self) -> Option<(Value, bool)> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        let (value, changed) = unsafe{ (*self.table).get_unchecked(rix.unwrap(),cix.unwrap()) };
        Some((value, changed))
      },     
      (Some(rix), None) => {
        let (value, changed) = unsafe{ (*self.table).get_unchecked_linear(rix.unwrap()) };
        Some((value, changed))
      },   
      _ => None,
    }
  }
}

impl fmt::Debug for ValueIterator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "table:\n {:?}\n", unsafe{&(*self.table)})?;
    write!(f, "out size: {:?}x{:?}\n", self.rows(), self.columns())?;
    write!(f, "row index: {:?}\n", self.row_index)?;
    write!(f, "col index: {:?}\n", self.column_index)?;
    write!(f, "row iter: {:?}\n", self.row_iter)?;
    write!(f, "col iter: {:?}\n", self.column_iter)?;
    
    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct IndexRepeater {
  iterator: std::iter::Cycle<IndexIterator>,
  width: usize,
  len: usize,
  current: Option<TableIndex>,
  counter: usize,
  cycle_index: usize,
  cycles: u64,
  current_cycle: u64,
}

impl IndexRepeater {

  pub fn new(iterator: IndexIterator, width: usize, cycles: u64) -> IndexRepeater {
    let len = iterator.len();
    IndexRepeater {
      iterator: iterator.cycle(),
      width,
      len,
      current: None,
      counter: 0,
      cycle_index: 0,
      cycles,
      current_cycle: 0,
    }
  }
}

impl Iterator for IndexRepeater {
  type Item = TableIndex;
  fn next(&mut self) -> Option<TableIndex> {
    if self.current == None {
      self.current = self.iterator.next();
    }
    if self.counter == self.width {
      self.counter = 0;
      self.cycle_index += 1;
      self.current = self.iterator.next();
    }
    if self.cycle_index == self.len {
      self.current_cycle += 1;
      self.cycle_index = 0;      
    }
    if self.current_cycle == self.cycles {
      return None
    }
    self.counter += 1;
    self.current
  }
}


#[derive(Debug, Clone)]
pub struct TableIterator {
  table: *mut Table,
  current: usize,
}

impl TableIterator {

  pub fn new(table: *mut Table) -> TableIterator {
    TableIterator {
      table,
      current: 0,
    }
  }

  pub fn len(&self) -> usize {
    let mut len = 0;
    unsafe{
      let max = (*self.table).data.len();
      for ix in 1..=max {
        let (val, _) = (*self.table).get_unchecked_linear(ix);
        if val.as_bool() == Some(true) || val.is_number() {
          len += 1;
        }
      }
    }
    len
  }

}

impl Iterator for TableIterator {
  type Item = TableIndex;
  fn next(&mut self) -> Option<TableIndex> {
    unsafe{
      if self.current < (*self.table).data.len() {
        let address = (*self.table).data[self.current];
        self.current += 1;
        let value = (*self.table).store.data[address];
        match value.as_u64() {
          Some(v) => {
            Some(TableIndex::Index(v as usize))
          },
          None => match value.as_bool() {
            Some(true) => {
              Some(TableIndex::Index(self.current))
            },
            Some(false) => {
              Some(TableIndex::None)
            },
            _x => {
              Some(TableIndex::None)
            }
          }
        }
      } else {
        None
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct ConstantIterator {
  table_index: TableIndex,
  done: bool,
}

impl ConstantIterator {

  pub fn new(table_index: TableIndex) -> ConstantIterator {
    ConstantIterator {
      table_index: table_index,
      done: false,
    }
  }

  pub fn len(&self) -> usize {
    1
  }

}

impl Iterator for ConstantIterator {
  type Item = TableIndex;
  
  fn next(&mut self) -> Option<TableIndex> {
    match self.done {
      true => None,
      false => {
        self.done = true;
        Some(self.table_index)
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct AliasIterator {
  alias: u64,
  table_id: TableId,
  store: Arc<Store>,
  index: Option<TableIndex>,
}

impl AliasIterator {

  pub fn new(alias: u64, table_id: TableId, store: Arc<Store>) -> AliasIterator {
    AliasIterator {
      alias,
      table_id,
      store,
      index: None,
    }
  }

  pub fn len(&self) -> usize {
    1
  }

}

impl Iterator for AliasIterator {
  type Item = TableIndex;
  
  fn next(&mut self) -> Option<TableIndex> {
    match self.index {
      None => {
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
        match store.column_alias_to_index.get(&(*self.table_id.unwrap(), self.alias)) {
          Some(ix) => {
            self.index = Some(TableIndex::Index(*ix));
            self.index
          },
          None => None,
        }
      },
      Some(_index) => self.index
    }
  }
}

#[derive(Debug, Clone)]
pub enum IndexIterator {
  None,
  Range(std::ops::RangeInclusive<usize>),
  Constant(ConstantIterator),
  Alias(AliasIterator),
  Table(TableIterator),
}

impl IndexIterator {
  pub fn len(&self) -> usize {
    match self {
      IndexIterator::None => 1,
      IndexIterator::Range(itr) => {itr.end() - itr.start() + 1},
      IndexIterator::Constant(itr) => itr.len(),
      IndexIterator::Table(itr) => itr.len(),
      IndexIterator::Alias(itr) => itr.len(),
    }
  }
}

impl Iterator for IndexIterator {
  type Item = TableIndex;
  
  fn next(&mut self) -> Option<TableIndex> {
    match self {
      IndexIterator::None => None,
      IndexIterator::Range(itr) => {
        match itr.next() {
          Some(ix) => Some(TableIndex::Index(ix)),
          None => None,
        }
      }
      IndexIterator::Constant(itr) => itr.next(),
      IndexIterator::Table(itr) => itr.next(),
      IndexIterator::Alias(itr) => itr.next(),
    }
  }
}

pub enum CycleIterator {
  Cycle(std::iter::Cycle<IndexIterator>),
  Index(IndexIterator),
}

impl Iterator for CycleIterator {
  type Item = TableIndex;

  fn next(&mut self) -> Option<TableIndex> {
    match self {
      CycleIterator::Cycle(itr) => itr.next(),
      CycleIterator::Index(itr) => itr.next(),
    }
  }
}