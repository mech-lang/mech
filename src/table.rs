// # Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

// ## Prelude

use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use crate::*;
use hashbrown::HashMap;

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

#[derive(Debug,Copy,Clone,PartialEq,Eq)]
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
  ReshapeColumn,
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
      TableIndex::ReshapeColumn |
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
      &TableIndex::Alias(ref alias) => write!(f, "IxAlias({})", humanize(alias)),
      &TableIndex::Table(ref table_id) => write!(f, "IxTable({:?})", table_id),
      &TableIndex::ReshapeColumn => write!(f, "IxReshapeColumn"),
      &TableIndex::All => write!(f, "IxAll"),
      &TableIndex::None => write!(f, "IxNone"),
    }
  }
}

// ## Table

pub type StringDictionary = Rc<RefCell<HashMap<u64,MechString>>>;



#[derive(Debug)]
pub struct Table {
  pub id: u64,                           
  pub rows: usize,                       
  pub cols: usize,                       
  pub col_kinds: Vec<ValueKind>,                 
  pub col_map: AliasMap,  
  pub row_map: AliasMap,
  pub data: Vec<Column>,
  pub dictionary: StringDictionary,
}

impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
      rows,
      cols,
      col_kinds: Vec::with_capacity(cols),
      col_map: AliasMap::new(cols),
      row_map: AliasMap::new(rows),
      data: Vec::with_capacity(cols),
      dictionary: Rc::new(RefCell::new(HashMap::new())),
    };
    for col in 0..cols {
      table.data.push(Column::Empty);
      table.col_kinds.push(ValueKind::Empty);
    }
    table
  }

  pub fn resize(&mut self, rows: usize, cols: usize) -> std::result::Result<(),MechError> {
    self.rows = rows;
    self.cols = cols;
    self.col_kinds.resize(cols,ValueKind::Empty);
    self.col_map.resize(cols);
    self.row_map.resize(rows);
    for col in &mut self.data {
      col.resize(rows);
    }
    self.data.resize(cols,Column::Empty);
    Ok(())
  }

  pub fn get_col_raw(&self, col_ix: usize) -> std::result::Result<Column,MechError> {
    if col_ix < self.cols {
      Ok(self.data[col_ix].clone())
    } else {
      Err(MechError::GenericError(6353)) 
    }
  }

  pub fn kind(&self) -> ValueKind {

    let first_col_kind = self.col_kinds[0].clone();
    if self.col_kinds.iter().all(|kind| *kind == first_col_kind) {
      first_col_kind
    } else {
      ValueKind::Compound(self.col_kinds.clone())
    }
  }
  
  pub fn get_column(&self, col: &TableIndex) -> Result<Column, MechError> {
    match col {
      TableIndex::Alias(alias) => {
        match self.col_map.get_index(&alias) {
          Ok(ix) => Ok(self.data[ix as usize].clone()),
          Err(_) => Err(MechError::GenericError(2821)),
        }
      }
      TableIndex::Index(0) => {
        Err(MechError::GenericError(2825))
      }
      TableIndex::Index(ix) => {
        if *ix <= self.cols { 
          Ok(self.data[*ix-1].clone())
        } else {
          Err(MechError::GenericError(2822))
        }
      }
      TableIndex::All => {
        if self.cols == 1 {
          Ok(self.data[0].clone())
        } else {
          Err(MechError::GenericError(2823))
        }
      }
      TableIndex::ReshapeColumn |
      TableIndex::Table(_) |
      TableIndex::None => Err(MechError::GenericError(2824)), 
    }
  }  

  /*
  pub fn set_col_kind(&mut self, col: usize, kind: ValueKind) -> Result<(),MechError> {
    if col < self.cols {
      match (&mut self.data[col], kind) {
        (Column::U8(_), ValueKind::U8) => (),
        (Column::Empty, ValueKind::U8) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U8(column);
          self.col_kinds[col] = ValueKind::U8;
        },
        (Column::Empty, ValueKind::U16) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U16(column);
          self.col_kinds[col] = ValueKind::U16;
        },
        (Column::Empty, ValueKind::U32) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U32(column);
          self.col_kinds[col] = ValueKind::U32;
        },
        (Column::Empty, ValueKind::U64) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U64(column);
          self.col_kinds[col] = ValueKind::U64;
        },
        (Column::Empty, ValueKind::U128) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U128(column);
          self.col_kinds[col] = ValueKind::U128;
        },
        (Column::Empty, ValueKind::I8) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::I8(column);
          self.col_kinds[col] = ValueKind::I8;
        },
        (Column::Empty, ValueKind::I16) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::I16(column);
          self.col_kinds[col] = ValueKind::I16;
        },
        (Column::Empty, ValueKind::I32) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::I32(column);
          self.col_kinds[col] = ValueKind::I32;
        },
        (Column::Empty, ValueKind::I64) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::I64(column);
          self.col_kinds[col] = ValueKind::I64;
        },
        (Column::Empty, ValueKind::I128) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::I128(column);
          self.col_kinds[col] = ValueKind::I128;
        },
        (Column::F32(_), ValueKind::F32) => (),
        (Column::Empty, ValueKind::F32) => {
          let column = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::F32(column);
          self.col_kinds[col] = ValueKind::F32;
        },
        (Column::Time(_), ValueKind::Time) => (),
        (Column::Empty, ValueKind::Time) => {
          let column = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::Time(column);
          self.col_kinds[col] = ValueKind::Time;
        },
        (Column::Length(_), ValueKind::Length) => (),
        (Column::Empty, ValueKind::Length) => {
          let column = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::Length(column);
          self.col_kinds[col] = ValueKind::Length;
        },
        (Column::Empty, ValueKind::F64) => {
          let column = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::F64(column);
          self.col_kinds[col] = ValueKind::F64;
        },
        (Column::Empty, ValueKind::Bool) => {
          let column = Rc::new(RefCell::new(vec![false;self.rows]));
          self.data[col] = Column::Bool(column);
          self.col_kinds[col] = ValueKind::Bool;
        },
        (Column::Empty, ValueKind::String) => {
          let column = Rc::new(RefCell::new(vec![MechString::new();self.rows]));
          self.data[col] = Column::String(column);
          self.col_kinds[col] = ValueKind::String;
        },
        (Column::Empty, ValueKind::Reference) => {
          let column = Rc::new(RefCell::new(vec![TableId::Local(0);self.rows]));
          self.data[col] = Column::Ref(column);
          self.col_kinds[col] = ValueKind::Reference;
        },
        x => {
          println!("{:?}", x);
          return Err(MechError::GenericError(1229));
        },
      }
      Ok(())
    } else {
      Err(MechError::GenericError(1215))
    }
  }*/

  pub fn set_col(&mut self, col_ix: usize, column: Column) -> std::result::Result<(),MechError> {
    if col_ix < self.cols {
      if self.col_kinds[col_ix] == ValueKind::Empty {
        self.col_kinds[col_ix] = column.kind();
        self.data[col_ix] = column;
        Ok(())
      } else {
        Err(MechError::GenericError(6354)) 
      }
    } else {
      Err(MechError::GenericError(6355)) 
    }
  }

  pub fn get_columns(&self, col: &TableIndex) -> Result<Vec<Column>, MechError> {
    match col {
      TableIndex::All => {
        Ok(self.data.iter().cloned().collect())
      },
      _ => Err(MechError::GenericError(1216)),
    }
  }

  /*
  pub fn set_raw(&mut self, row_ix: usize, col_ix: usize, value: Value) -> std::result::Result<(),MechError> {
    if col_ix < self.cols && row_ix < self.rows {
      let mut col = &mut self.data[col_ix];
      col.set_unchecked(row_ix,value);
      Ok(())
    } else {
      Err(MechError::GenericError(6356))
    }
  }

  pub fn get_raw(&self, row_ix: usize, col_ix: usize) -> std::result::Result<Value,MechError> {
    if col_ix < self.cols && row_ix < self.rows {
      let col = &self.data[col_ix];
      Ok(col.get_unchecked(row_ix))
    } else {
      Err(MechError::GenericError(6357))
    }
  }*/
}

type TableIx = usize;
type Alias = u64;

#[derive(Debug)]
pub struct AliasMap {
  capacity: usize,
  ix_to_alias: Vec<Alias>,  
  alias_to_ix: HashMap<Alias,TableIx>,
}

impl AliasMap {
  pub fn new(capacity: usize) -> Self {
    AliasMap {
      capacity,
      ix_to_alias: vec![0;capacity],
      alias_to_ix: HashMap::new(),
    }
  }

  pub fn resize(&mut self, new_capacity: usize) {
    self.capacity = new_capacity;
    self.ix_to_alias.resize(new_capacity,0);
  }

  pub fn insert(&mut self, ix: TableIx, alias: Alias) -> std::result::Result<(),MechError> {
    if ix < self.capacity {
      self.ix_to_alias[ix] = alias;
      self.alias_to_ix.insert(alias,ix);
      Ok(())
    } else {
      Err(MechError::GenericError(8210))
    }
  }

  pub fn get_index(&self, alias: &Alias) -> std::result::Result<TableIx,MechError> {
    match self.alias_to_ix.get(alias) {
      Some(ix) => Ok(*ix),
      None => Err(MechError::GenericError(8211)),
    }
  }

  pub fn get_alias(&self, ix: &TableIx) -> std::result::Result<Alias,MechError> {
    if ix < &self.capacity {
      Ok(self.ix_to_alias[*ix])
    } else {
      Err(MechError::GenericError(8212))
    }
  }

}



/*
#[derive(Clone)]
pub struct Table {
  pub id: u64,                           
  pub rows: usize,                       
  pub cols: usize,                       
  pub col_kinds: Vec<ValueKind>,                 
  pub column_ix_to_alias: Vec<u64>,  
  pub column_alias_to_ix: HashMap<u64,usize>,
  pub data: Vec<Column>,
  pub dictionary: StringDictionary,
}


impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
      rows,
      cols,
      column_ix_to_alias: Vec::new(),
      column_alias_to_ix: HashMap::new(),
      data: Vec::with_capacity(cols),
      col_kinds: Vec::with_capacity(cols),
      dictionary: Rc::new(RefCell::new(HashMap::new())),
    };
    for col in 0..cols {
      table.data.push(Column::Empty);
      table.col_kinds.push(ValueKind::Empty);
    }
    table
  }

  pub fn copy(&self) -> Table {
    let mut table = Table::new(0,self.rows,self.cols);
    table.column_ix_to_alias = self.column_ix_to_alias.clone();
    table.column_alias_to_ix = self.column_alias_to_ix.clone();
    table.col_kinds = self.col_kinds.clone();
    for col in 0..self.cols {
      table.data[col] = self.data[col].copy();
    }
    table
  }


  pub fn has_col_aliases(&self) -> bool {
    self.column_ix_to_alias.len() > 0
  }

  pub fn shape(&self) -> TableShape {
    match (self.rows, self.cols) {
      (0,_) |
      (_,0) => TableShape::Pending,
      (1,1) => TableShape::Scalar,
      (x,1) => TableShape::Column(x),
      (1,x) => TableShape::Row(x),
      (x,y) => TableShape::Matrix(x,y), 
    }
  }

  pub fn set_column_alias(&mut self, ix: usize, alias: u64) -> Result<(),MechError> {
    if ix < self.cols {
      self.column_ix_to_alias.resize(self.cols,0);
      self.column_ix_to_alias[ix] = alias;
      self.column_alias_to_ix.insert(alias,ix);
      Ok(())
    } else {
      Err(MechError::GenericError(1210))
    }
  }

  pub fn get_by_index(&self, row: TableIndex, col: TableIndex) -> Result<Value,MechError> {
    match (row, &self.get_column(&col)?) {
      (TableIndex::Index(0),_) => Err(MechError::GenericError(1298)),
      (TableIndex::Index(row),Column::F32(c)) => Ok(Value::F32(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::F64(c)) => Ok(Value::F64(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::U8(c)) => Ok(Value::U8(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::U16(c)) => Ok(Value::U16(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::U32(c)) => Ok(Value::U32(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::U64(c)) => Ok(Value::U64(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::U128(c)) => Ok(Value::U128(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::I8(c)) => Ok(Value::I8(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::I16(c)) => Ok(Value::I16(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::I32(c)) => Ok(Value::I32(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::I64(c)) => Ok(Value::I64(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::I128(c)) => Ok(Value::I128(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::Bool(c)) => Ok(Value::Bool(c.borrow()[row-1])),
      (TableIndex::Index(row),Column::String(c)) => Ok(Value::String(c.borrow()[row-1].clone())),
      (TableIndex::Index(row),Column::Ref(c)) => Ok(Value::Reference(c.borrow()[row-1].clone())),
      (_,Column::Empty) => Ok(Value::Empty),
      _ => Err(MechError::GenericError(1299)),
    }
  }

  pub fn get(&self, row: &TableIndex, col: &TableIndex) -> Result<Value,MechError> {
    let row_ix = match row {
      TableIndex::Index(0) => {return Err(MechError::GenericError(7497))},
      TableIndex::Index(ix) => ix - 1,
      _ => 0,
    };
    let col_ix = match col {
      TableIndex::Index(0) => {return Err(MechError::GenericError(7124))},
      TableIndex::Index(ix) => ix - 1,
      TableIndex::Alias(alias) => {
        match self.column_alias_to_ix.get(alias) {
          Some(ix) => *ix,
          None => {return Err(MechError::GenericError(2384))}
        }
      }
      _ => 0,
    };
    self.get_raw(row_ix,col_ix)
  }

  pub fn get_raw(&self, row: usize, col: usize) -> Result<Value,MechError> {
    if col < self.cols && row < self.rows {
      match &self.data[col] {
        Column::Time(c) => Ok(Value::Time(c.borrow()[row])),
        Column::Length(c) => Ok(Value::Length(c.borrow()[row])),
        Column::F32(c) => Ok(Value::F32(c.borrow()[row])),
        Column::F64(c) => Ok(Value::F64(c.borrow()[row])),
        Column::U8(c) => Ok(Value::U8(c.borrow()[row])),
        Column::U16(c) => Ok(Value::U16(c.borrow()[row])),
        Column::U32(c) => Ok(Value::U32(c.borrow()[row])),
        Column::U64(c) => Ok(Value::U64(c.borrow()[row])),
        Column::U128(c) => Ok(Value::U128(c.borrow()[row])),
        Column::I8(c) => Ok(Value::I8(c.borrow()[row])),
        Column::I16(c) => Ok(Value::I16(c.borrow()[row])),
        Column::I32(c) => Ok(Value::I32(c.borrow()[row])),
        Column::I64(c) => Ok(Value::I64(c.borrow()[row])),
        Column::I128(c) => Ok(Value::I128(c.borrow()[row])),
        Column::Bool(c) => Ok(Value::Bool(c.borrow()[row])),
        Column::String(c) => Ok(Value::String(c.borrow()[row].clone())),
        Column::Ref(c) => Ok(Value::Reference(c.borrow()[row].clone())),
        Column::Empty => Ok(Value::Empty),
        x => {
          println!("{:?}", x);
          Err(MechError::GenericError(1209))
        },
      }
    } else {
      Err(MechError::GenericError(1211))
    }
  }

  pub fn index_to_subscript(&self, ix: usize) -> Result<(usize, usize),MechError> {
    let row = ix / self.cols;
    let col = ix % self.cols;
    if ix < self.rows * self.cols {
      Ok((row,col))
    } else {
      Err(MechError::LinearSubscriptOutOfBounds((ix,self.rows*self.cols)))
    }
  }

  pub fn get_linear(&self, ix: usize) -> Result<Value,MechError> {
    if ix < self.rows * self.cols {
      let row = ix / self.cols;
      let col = ix % self.cols;
      self.get_raw(row,col)
    } else {
      Err(MechError::GenericError(1213))
    }
  }

  pub fn set_linear(&self, ix: usize, val: Value) -> Result<(),MechError> {
    if ix < self.rows * self.cols {
      let row = ix / self.cols;
      let col = ix % self.cols;
      self.set_raw(row,col, val)
    } else {
      Err(MechError::GenericError(1214))
    }
  }

  pub fn set(&self, row: &TableIndex, col: &TableIndex, val: Value ) -> Result<(),MechError> {
    let row_ix = match row {
      TableIndex::Index(0) => {return Err(MechError::GenericError(7495))},
      TableIndex::Index(ix) => ix - 1,
      _ => 0,
    };
    let col_ix = match col {
      TableIndex::Index(0) => {return Err(MechError::GenericError(7123))},
      TableIndex::Index(ix) => ix - 1,
      TableIndex::Alias(alias) => {
        match self.column_alias_to_ix.get(alias) {
          Some(ix) => *ix,
          None => {return Err(MechError::GenericError(2389))}
        }
      }
      _ => 0,
    };
    self.set_raw(row_ix,col_ix,val)
  }

  pub fn set_raw(&self, row: usize, col: usize, val: Value) -> Result<(),MechError> {
    if col < self.cols && row < self.rows {
      match (&self.data[col], val) {
        (Column::Length(c), Value::Length(v)) |
        (Column::Time(c), Value::Time(v)) |
        (Column::F32(c), Value::F32(v)) => c.borrow_mut()[row] = v,
        (Column::F64(c), Value::F64(v)) => c.borrow_mut()[row] = v,
        (Column::U8(c), Value::U8(v)) => c.borrow_mut()[row] = v,
        (Column::U16(c), Value::U16(v)) => c.borrow_mut()[row] = v,
        (Column::U32(c), Value::U32(v)) => c.borrow_mut()[row] = v,
        (Column::U64(c), Value::U64(v)) => c.borrow_mut()[row] = v,
        (Column::U128(c), Value::U128(v)) => c.borrow_mut()[row] = v,
        (Column::I8(c), Value::I8(v)) => c.borrow_mut()[row] = v,
        (Column::I16(c), Value::I16(v)) => c.borrow_mut()[row] = v,
        (Column::I32(c), Value::I32(v)) => c.borrow_mut()[row] = v,
        (Column::I64(c), Value::I64(v)) => c.borrow_mut()[row] = v,
        (Column::I128(c), Value::I128(v)) => c.borrow_mut()[row] = v,
        (Column::Bool(c), Value::Bool(v)) => c.borrow_mut()[row] = v,
        (Column::String(c), Value::String(v)) => c.borrow_mut()[row] = v,
        (Column::Ref(c), Value::Reference(v)) => c.borrow_mut()[row] = v,
        (Column::Empty, Value::Empty) => (),
        x => {
          println!("{:?}", x);
          return Err(MechError::GenericError(1219));
        },
      }
      Ok(())
    } else {
      Err(MechError::GenericError(1212))
    }
  }

  pub fn set_kind(&mut self, kind: ValueKind) -> Result<(),MechError> {
    match kind {
      ValueKind::Compound(kinds) => {
        for col in 0..self.cols {
          self.set_col_kind(col,kinds[col].clone())?;
        }
      }
      kind => {
        for col in 0..self.cols {
          self.set_col_kind(col,kind.clone())?;
        }
      }
    }
    Ok(())
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
      for col in &self.data {
        col.resize(rows);
      }
    }
  }

  pub fn len(&self) -> usize {
    self.rows * self.cols
  }

  pub fn logical_len(&self) -> usize {
    match self.kind() {
      ValueKind::Bool => {
        let mut len = 0;
        for i in 0..self.len() {
          match self.get_linear(i) {
            Ok(Value::Bool(x)) => if x == true { len += 1 },
            _ => (),
          }
        }
        len
      }
      _ => self.len(),
    }
  }
  
}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut table_drawing = BoxPrinter::new();
    table_drawing.add_table(self);
    write!(f,"{:?}",table_drawing)?;
    Ok(())
  }
}*/