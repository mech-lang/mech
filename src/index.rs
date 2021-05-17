use table::{Table, TableId, TableIndex};
use value::{Value, ValueMethods, NumberLiteral};
use database::{Store, Database};
use quantities::Quantity;
use ::humanize;
use block::Block;
use hashbrown::HashMap;
//use errors::{ErrorType};
use std::sync::Arc;
use rust_core::fmt;
use std::cell::RefCell;

#[derive(Clone)]
pub struct  ValueIterator {
  pub scope: TableId,
  pub table: Arc<RefCell<Table>>,
  pub row_index: TableIndex,
  pub column_index: TableIndex,
  pub raw_row_iter: IndexIterator,     // I need these two fields for the purpose of resizing the iterator...
  pub raw_column_iter: IndexIterator,  // if there's a way to extract the iterator from the std::itr::Cycle<> in the IndexRepeater then I wouldn't need these anymore.
  pub row_iter: IndexRepeater,   
  pub column_iter: IndexRepeater,
  pub computed_indices: Vec<usize>,
  ix: usize,
  inf_cycle: bool,
}

impl ValueIterator {
  
  pub fn new(table_id: TableId, 
             row_index: TableIndex, 
             column_index: TableIndex, 
             database: &Arc<RefCell<Database>>, 
             block_tables: &mut HashMap<u64, Arc<RefCell<Table>>>,
             block_store: &mut Arc<Store>) -> ValueIterator {
    let mut db = database.borrow_mut();

    // Get the table
    let mut table_rc = match table_id {
      TableId::Global(id) => db.tables.get_mut(&id).unwrap().clone(),
      TableId::Local(id) => match block_tables.get_mut(&id) {
        Some(table) => table.clone(),
        None => {
          // Does this table have an alias?
          let store = unsafe{&mut *Arc::get_mut_unchecked(block_store)};
          let table_id = store.table_alias_to_id.get(&id).unwrap();
          block_tables.get_mut(table_id.unwrap()).unwrap().clone()
        }
      }
    };

    let table = table_rc.borrow();
    let row_iter = unsafe { match row_index {
      TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(ix))),
      TableIndex::All => {
        match table.rows {
          0 => IndexIterator::None,
          r => IndexIterator::Range(1..=r),
        }
      },
      TableIndex::Table(table_id) => {
        let row_table = match table_id {
          TableId::Global(id) => db.tables.get_mut(&id).unwrap().clone(),
          TableId::Local(id) =>  match block_tables.get_mut(&id) {
            Some(table) => table.clone(),
            None => {
              // Does this table have an alias?
              let store = unsafe{&mut *Arc::get_mut_unchecked(block_store)};
              let table_id = store.table_alias_to_id.get(&id).unwrap();
              block_tables.get_mut(table_id.unwrap()).unwrap().clone()
            }
          }
        };
        IndexIterator::Table(TableIterator::new(row_table))
      }
      TableIndex::Alias(alias) => match table_id {
        TableId::Global(_) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
        TableId::Local(_) => IndexIterator::Alias(AliasIterator::new(alias, table_id, block_store.clone())),
      }
      _ => IndexIterator::Range(1..=(*table).rows),
    }};
  
    let column_iter = unsafe { match column_index {
      TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(ix))),
      TableIndex::All => {
        match (*table).columns {
          0 => IndexIterator::None,
          c => IndexIterator::Range(1..=c),
        }
      }
      TableIndex::Table(table_id) => {
        let col_table = match table_id {
          TableId::Global(id) => db.tables.get_mut(&id).unwrap().clone(),
          TableId::Local(id) =>  match block_tables.get_mut(&id) {
            Some(table) => table.clone(),
            None => {
              // Does this table have an alias?
              let store = unsafe{&mut *Arc::get_mut_unchecked(block_store)};
              let table_id = store.table_alias_to_id.get(&id).unwrap();
              block_tables.get_mut(table_id.unwrap()).unwrap().clone()
            }
          }
        };
        IndexIterator::Table(TableIterator::new(col_table))
      }
      TableIndex::Alias(alias) => match table_id {
        TableId::Global(_) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
        TableId::Local(_) => IndexIterator::Alias(AliasIterator::new(alias, table_id, block_store.clone())),
      }
      TableIndex::None => IndexIterator::None,
    }};

    let row_len = row_iter.len();
    let column_len = if column_iter.len() == 0 {1} else {column_iter.len()};
    ValueIterator{
      scope: table_id,
      table: table_rc.clone(),
      row_index: row_index,
      column_index: column_index,
      raw_row_iter: row_iter.clone(),
      raw_column_iter: column_iter.clone(),
      row_iter: IndexRepeater::new(row_iter,column_len,1),
      column_iter: IndexRepeater::new(column_iter,1,row_len as u64),
      computed_indices: vec![],
      ix: 0,
      inf_cycle: false,
    }

  }

  pub fn init_iterators(&mut self) {

    match self.row_index {
      TableIndex::All => {
        match self.table.borrow().rows {
          0 => self.raw_row_iter = IndexIterator::None,
          r => self.raw_row_iter = IndexIterator::Range(1..=r),
        }
      }
      _ => (),
    };
    match self.column_index {
      TableIndex::All => {
        match self.table.borrow().columns {
          0 => self.raw_column_iter = IndexIterator::None,
          r => self.raw_column_iter = IndexIterator::Range(1..=r),
        }
      }
      _ => (),
    };

    if self.elements() != self.computed_indices.len() {
      let row_len = self.raw_row_iter.len();
      let column_len = if self.raw_column_iter.len() == 0 {1} else {self.raw_column_iter.len()};
      self.row_iter = IndexRepeater::new(self.raw_row_iter.clone(),column_len,1);
      self.column_iter = IndexRepeater::new(self.raw_column_iter.clone(),1,row_len as u64);
      self.compute_indices();
    }
  }

  pub fn compute_indices(&mut self) {
    self.computed_indices.resize(self.elements(),0);
    
    let mut row_iter = self.row_iter.clone();
    let mut column_iter = self.column_iter.clone();

    let mut i = 0;
    loop {
      match (row_iter.next(), column_iter.next()) {
        (Some(rix), Some(cix)) => {
          match self.index(&rix,&cix) {
            Some(ix) => {
              self.computed_indices[i] = ix + 1
            },
            None => break,
          }
        },     
        (Some(rix), None) => {
          let ix = rix.unwrap();
          self.computed_indices[i] = ix;
        },   
        _ => break,
      }
      i += 1;
    }
  } 

  pub fn linear_index_iterator(&self) -> LinearIndexIterator {
    LinearIndexIterator::new(self.table.clone(),self.row_iter.clone(),self.column_iter.clone())
  }

  pub fn index_iterator(&self) -> std::iter::Zip<IndexRepeater, IndexRepeater> {
    self.row_iter.clone().zip(self.column_iter.clone())
  }

  pub fn id(&self) -> u64 {
    *self.scope.unwrap()
  }

  pub fn get_column_alias(&self, index: usize) -> Option<TableIndex> {
    self.table.borrow().get_column_alias(index)
  }
  
  pub fn inf_cycle(&mut self) {
    self.inf_cycle = true;
    self.row_iter.inf_cycle();
    self.column_iter.inf_cycle();
  }

  pub fn elements(&self) -> usize {
    self.rows() * self.columns()
  }

  pub fn rows(&self) -> usize {
    if self.table.borrow().rows == 0 {
      0
    } else {
      self.raw_row_iter.len()
    }
  }

  pub fn table_rows(&self) -> usize {
    self.table.borrow().rows
  }
  
  pub fn table_columns(&self) -> usize {
    self.table.borrow().columns
  }  

  pub fn columns(&self) -> usize {
    match self.column_index {
      TableIndex::None => 1,
      _ => self.raw_column_iter.len(),
    }
  }

  pub fn index(&self, row: &TableIndex, column: &TableIndex) -> Option<usize>  {
    self.table.borrow().index(row,column)
  }

  pub fn is_scalar(&self) -> bool {
    self.rows() == 1 && self.columns() == 1
  }

  pub fn get_quantity(&self, row: &TableIndex, column: &TableIndex) -> Option<(Quantity,bool)> {
    self.table.borrow().get_quantity(row,column).clone()
  } 

  pub fn get_u64(&self, row: &TableIndex, column: &TableIndex) -> Option<(u64,bool)> {
    self.table.borrow().get_u64(row,column).clone()
  } 

  pub fn get_f32(&self, row: &TableIndex, column: &TableIndex) -> Option<f32> {
    self.table.borrow().get_f32(row,column).clone()
  } 

  pub fn get(&self, row: &TableIndex, column: &TableIndex) -> Option<(Value,bool)> {
    self.table.borrow().get(row,column).clone()
  }

  pub fn get_unchecked(&self, row: usize, column: usize) -> (Value, bool) {
    self.table.borrow().get_unchecked(row,column).clone()
  }

  pub fn get_unchecked_linear(&self, index: usize) -> (Value, bool) {
    self.table.borrow().get_unchecked_linear(index).clone()
  }

  pub fn get_linear(&self, index: &TableIndex) -> Option<(Value, bool)> {
    self.table.borrow().get_linear(index).clone()
  }

  pub fn set(&self, row: &TableIndex, column: &TableIndex, value: Value) {
    self.table.borrow_mut().set(row, column, value);
  }

  pub fn set_string(&self, row: &TableIndex, column: &TableIndex, value: Value, string: String) {
    let mut table = self.table.borrow_mut();
    table.set(row, column, value);
    table.insert_string(string);
  }

  pub fn set_unchecked(&self, row: usize, column: usize, value: Value) {
    self.table.borrow_mut().set_unchecked(row, column, value);
  }

  pub fn set_unchecked_linear(&self, index: usize, value: Value) {
    self.table.borrow_mut().set_unchecked_linear(index, value);
  }

  pub fn next_address(&mut self) -> Option<(usize, usize)> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        Some((rix.unwrap(),cix.unwrap()))
      },     
      _ => None,
    }
  }

  pub fn reset(&mut self) {
    self.ix = 0;
  }

  pub fn next_index(&mut self) -> Option<usize> {
    if self.computed_indices.len() > 0 {
      if self.ix < self.computed_indices.len() {
        let computed_index = self.computed_indices[self.ix];
        self.ix += 1;
        Some(computed_index)
      } else if self.inf_cycle {
        self.ix = 0;
        let computed_index = self.computed_indices[self.ix];
        self.ix += 1;
        Some(computed_index)
      } else {
        self.ix = 0;
        None
      }
    } else {
      None
    }
  }

  pub fn resize(&mut self, rows: usize, columns: usize)  {

    let mut table = self.table.borrow_mut();

    let columns = if table.columns > columns {
      table.columns
    } else {
      columns
    };

    table.resize(rows, columns);

    match self.row_index {
      TableIndex::All => {
        match table.rows {
          0 => self.raw_row_iter=IndexIterator::None,
          r => self.raw_row_iter=IndexIterator::Range(1..=r),
        }
      },
      _ => (),        
    };

    match self.column_index {
      TableIndex::All => {
        match table.columns {
          0 => self.raw_column_iter = IndexIterator::None,
          c => self.raw_column_iter = IndexIterator::Range(1..=c),
        }
      },  
      _ => (),      
    };

    let row_len = self.raw_row_iter.len();
    let column_len = if self.raw_column_iter.len() == 0 {1} else {self.raw_column_iter.len()};
    self.row_iter = IndexRepeater::new(self.raw_row_iter.clone(),column_len,1);
    self.column_iter = IndexRepeater::new(self.raw_column_iter.clone(),1,row_len as u64);
  }

}

impl Iterator for ValueIterator {
  type Item = (Value, bool);
  fn next(&mut self) -> Option<(Value, bool)> {
    if self.computed_indices.len() > 0 {
      if self.ix < self.computed_indices.len() {
        let computed_index = self.computed_indices[self.ix];
        self.ix += 1;
        Some(self.get_unchecked_linear(computed_index))
      } else if self.inf_cycle {
        self.ix = 0;
        let computed_index = self.computed_indices[self.ix];
        self.ix += 1;
        Some(self.get_unchecked_linear(computed_index))
      } else {
        self.ix = 0;
        None
      }
    } else { 
      match (self.row_iter.next(), self.column_iter.next()) {
        (Some(rix), Some(cix)) => {
          self.get(&rix,&cix)
        },     
        (Some(rix), None) => {
          self.get_linear(&rix)
        },   
        _ => None,
      }
    }
  }
}

impl fmt::Debug for ValueIterator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "table:\n {:?}\n", self.table.borrow())?;
    write!(f, "out size: {:?}x{:?}\n", self.rows(), self.columns())?;
    write!(f, "row index: {:?}\n", self.row_index)?;
    write!(f, "col index: {:?}\n", self.column_index)?;
    write!(f, "row iter: {:?}\n", self.row_iter)?;
    write!(f, "col iter: {:?}\n", self.column_iter)?;
    
    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct LinearIndexIterator {
  pub table: Arc<RefCell<Table>>,
  pub row_iter: IndexRepeater,   
  pub column_iter: IndexRepeater,  
}

impl LinearIndexIterator {
  pub fn new(table: Arc<RefCell<Table>>, row_iter: IndexRepeater, column_iter: IndexRepeater) -> LinearIndexIterator {
    LinearIndexIterator {
      table,
      row_iter,
      column_iter,
    }
  }
}

impl Iterator for LinearIndexIterator {
  type Item = usize;
  fn next(&mut self) -> Option<usize> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        let ix = self.table.borrow().index_unchecked(rix.unwrap(),cix.unwrap()) + 1;
        Some(ix)
      },     
      (Some(rix), None) => {
        Some(rix.unwrap())
      },   
      _ => None,
    }
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

  pub fn inf_cycle(&mut self) {
    self.cycles = 0;
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
    if self.cycles != 0 && self.current_cycle == self.cycles {
      return None
    }
    self.counter += 1;
    self.current
  }
}


#[derive(Debug, Clone)]
pub struct TableIterator {
  table: Arc<RefCell<Table>>,
  current: usize,
}

impl TableIterator {

  pub fn new(table: Arc<RefCell<Table>>) -> TableIterator {
    TableIterator {
      table,
      current: 0,
    }
  }

  pub fn len(&self) -> usize {
    let mut len = 0;
    let table = self.table.borrow();
    let max = table.data.len();
    for ix in 1..=max {
      let (val, _) = table.get_unchecked_linear(ix);
      if val.as_bool() == Some(true) || val.is_number() {
        len += 1;
      }
    }
    len
  }

}

impl Iterator for TableIterator {
  type Item = TableIndex;
  fn next(&mut self) -> Option<TableIndex> {
    loop {
      let table = self.table.borrow();
      if self.current < table.data.len() {
        let value = table.data[self.current];
        self.current += 1;
        match value.as_u64() {
          Some(v) => {
            return Some(TableIndex::Index(v as usize));
          },
          None => match value.as_bool() {
            Some(true) => {
              return Some(TableIndex::Index(self.current));
            },
            Some(false) => {
              continue;
            },
            _x => {
              return Some(TableIndex::None); // TODO This should be an error
            }
          }
        }
      } else {
        return None;
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

#[derive(Clone)]
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

impl fmt::Debug for AliasIterator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "AliasIterator(table: {:?} alias: {:?})", self.table_id, humanize(&self.alias))?;
    Ok(())
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
      IndexIterator::None => 0,
      IndexIterator::Range(itr) => {
        if *itr.end() == 0 {
          0
        } else {
          itr.end() - itr.start() + 1
        }
      },
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