use table::{Table, TableId, Index};
use value::{Value, ValueMethods};
use database::{Store};
//use errors::{ErrorType};
use std::sync::Arc;
use rust_core::fmt;

pub struct  ValueIterator {
  pub scope: TableId,
  pub table: *mut Table,
  pub row_index: Index,
  pub column_index: Index,
  pub row_iter: IndexIterator,
  pub column_iter: IndexIterator,
}

impl ValueIterator {
  
  pub fn rows(&self) -> usize {
    unsafe{ (*self.table).rows }
  }

  pub fn columns(&self) -> usize {
    match self.column_index {
      Index::All => unsafe{ (*self.table).columns },
      Index::Index{..} |
      Index::Alias{..} => 1,
      _ => unsafe{ (*self.table).columns },
    }
    
  }

  pub fn get(&self, row: &Index, column: &Index) -> Option<Value> {
    unsafe{(*self.table).get(row,column)}
  }

  pub fn get_unchecked(&self, row: usize, column: usize) -> (Value, bool) {
    unsafe{(*self.table).get_unchecked(row,column)}
  }

  pub fn get_unchecked_linear(&self, index: usize) -> (Value, bool) {
    unsafe{(*self.table).get_unchecked_linear(index)}
  }

  pub fn set(&self, row: &Index, column: &Index, value: Value) {
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
        Index::All => {
          match (*self.table).rows {
            0 => self.row_iter = IndexIterator::None,
            r => self.row_iter = IndexIterator::Range(1..=r),
          }
        }
        _ => (),
      }

      match self.column_index {
        Index::All => {
          match (*self.table).rows {
            0 => self.column_iter = IndexIterator::None,
            c => self.column_iter = IndexIterator::Range(1..=c),
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
      _ => None,
    }
  }

}

impl fmt::Debug for ValueIterator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    unsafe{write!(f, "row index: {:?}\n", (*self.table))?;}
    write!(f, "row index: {:?}\n", self.row_index)?;
    write!(f, "col index: {:?}\n", self.column_index)?;
    write!(f, "row iter: {:?}\n", self.row_iter)?;
    write!(f, "col iter: {:?}\n", self.column_iter)?;
    
    Ok(())
  }
}

#[derive(Debug)]
pub struct IndexRepeater {
  iterator: std::iter::Cycle<IndexIterator>,
  width: usize,
  current: Option<Index>,
  counter: usize,
}

impl IndexRepeater {

  pub fn new(iterator: IndexIterator, width: usize) -> IndexRepeater {
    IndexRepeater {
      iterator: iterator.cycle(),
      width,
      current: None,
      counter: 0,
    }
  }

  pub fn next(&mut self) -> Option<Index> {
    if self.current == None {
      self.current = self.iterator.next();
    }
    if self.counter == self.width {
      self.counter = 0;
      self.current = self.iterator.next();
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
  type Item = Index;
  fn next(&mut self) -> Option<Index> {
    unsafe{
      if self.current < (*self.table).data.len() {
        let address = (*self.table).data[self.current];
        self.current += 1;
        let value = (*self.table).store.data[address];
        match value.as_u64() {
          Some(v) => {
            Some(Index::Index(v as usize))
          },
          None => match value.as_bool() {
            Some(true) => {
              Some(Index::Index(self.current))
            },
            Some(false) => {
              Some(Index::None)
            },
            _x => {
              Some(Index::None)
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
pub struct AliasIterator {
  alias: u64,
  table_id: TableId,
  store: Arc<Store>,
  index: Option<Index>,
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

}

impl Iterator for AliasIterator {
  type Item = Index;
  
  fn next(&mut self) -> Option<Index> {
    match self.index {
      None => {
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
        match store.column_alias_to_index.get(&(*self.table_id.unwrap(), self.alias)) {
          Some(ix) => {
            self.index = Some(Index::Index(*ix));
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
  Constant(Index),
  Alias(AliasIterator),
  Table(TableIterator),
}

impl Iterator for IndexIterator {
  type Item = Index;
  
  fn next(&mut self) -> Option<Index> {
    match self {
      IndexIterator::None => None,
      IndexIterator::Range(itr) => {
        match itr.next() {
          Some(ix) => Some(Index::Index(ix)),
          None => None,
        }
      }
      IndexIterator::Constant(itr) => Some(*itr),
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
  type Item = Index;

  fn next(&mut self) -> Option<Index> {
    match self {
      CycleIterator::Cycle(itr) => itr.next(),
      CycleIterator::Index(itr) => itr.next(),
    }
  }
}