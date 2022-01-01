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
      &TableIndex::Alias(ref alias) => write!(f, "IxAlias({})", humanize(alias)),
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
  pub column_ix_to_alias: Vec<u64>,
  pub column_alias_to_ix: HashMap<u64,usize>,
  pub data: Vec<Column>,
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

  pub fn kind(&self) -> ValueKind {

    let first_col_kind = self.col_kinds[0].clone();
    if self.col_kinds.iter().all(|kind| *kind == first_col_kind) {
      first_col_kind
    } else {
      ValueKind::Compound(self.col_kinds.clone())
    }
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

  pub fn get(&self, row: usize, col: usize) -> Result<Value,MechError> {
    if col < self.cols && row < self.rows {
      match &self.data[col] {
        Column::F32(column_f32) => Ok(Value::F32(column_f32.borrow()[row])),
        Column::U8(column_u8) => Ok(Value::U8(column_u8.borrow()[row])),
        Column::U16(column_u16) => Ok(Value::U16(column_u16.borrow()[row])),
        Column::Bool(column_bool) => Ok(Value::Bool(column_bool.borrow()[row])),
        Column::String(column_string) => Ok(Value::String(column_string.borrow()[row].clone())),
        Column::Ref(column_ref) => Ok(Value::Reference(column_ref.borrow()[row].clone())),
        Column::Empty => Ok(Value::Empty),
        _ => Err(MechError::GenericError(1209)),
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
      self.get(row,col)
    } else {
      Err(MechError::GenericError(1213))
    }
  }

  pub fn set_linear(&self, ix: usize, val: Value) -> Result<(),MechError> {
    if ix < self.rows * self.cols {
      let row = ix / self.cols;
      let col = ix % self.cols;
      self.set(row,col, val)
    } else {
      Err(MechError::GenericError(1214))
    }
  }

  pub fn set(&self, row: usize, col: usize, val: Value) -> Result<(),MechError> {
    if col < self.cols && row < self.rows {
      match (&self.data[col], val) {
        (Column::F32(column_f32), Value::F32(value_f32)) => column_f32.borrow_mut()[row] = value_f32,
        (Column::U8(column_u8), Value::U8(value_u8)) => column_u8.borrow_mut()[row] = value_u8,
        (Column::U16(column_u16), Value::U16(value_u16)) => column_u16.borrow_mut()[row] = value_u16,
        (Column::Bool(column_bool), Value::Bool(value_bool)) => column_bool.borrow_mut()[row] = value_bool,
        (Column::String(column_string), Value::String(value_string)) => column_string.borrow_mut()[row] = value_string,
        (Column::Ref(column_ref), Value::Reference(value_ref)) => column_ref.borrow_mut()[row] = value_ref,
        (Column::Empty, Value::Empty) => (),
        (Column::Empty, Value::U8(value_u8)) => {
          //let column: ColumnV<u8> = Rc::new(RefCell::new(Vec::new()));
          //self.data[col] = Column::U8(column);
        },
        x => {
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

  pub fn set_col_kind(&mut self, col: usize, kind: ValueKind) -> Result<(),MechError> {
    if col < self.cols {
      match (&mut self.data[col], kind) {
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
        (Column::Empty, ValueKind::F32) => {
          let column = Rc::new(RefCell::new(vec![0.0;self.rows]));
          self.data[col] = Column::F32(column);
          self.col_kinds[col] = ValueKind::F32;
        },
        (Column::Empty, ValueKind::U64) => {
          let column = Rc::new(RefCell::new(vec![0;self.rows]));
          self.data[col] = Column::U64(column);
          self.col_kinds[col] = ValueKind::U64;
        },
        (Column::Empty, ValueKind::Bool) => {
          let column = Rc::new(RefCell::new(vec![false;self.rows]));
          self.data[col] = Column::Bool(column);
          self.col_kinds[col] = ValueKind::Bool;
        },
        (Column::Empty, ValueKind::String) => {
          let column = Rc::new(RefCell::new(vec![vec![];self.rows]));
          self.data[col] = Column::String(column);
          self.col_kinds[col] = ValueKind::String;
        },
        (Column::Empty, ValueKind::Reference) => {
          let column = Rc::new(RefCell::new(vec![TableId::Local(0);self.rows]));
          self.data[col] = Column::Ref(column);
          self.col_kinds[col] = ValueKind::Reference;
        },
        _ => {return Err(MechError::GenericError(1212));},
      }
      Ok(())
    } else {
      Err(MechError::GenericError(1215))
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

  pub fn get_column(&self, col: &TableIndex) -> Result<Column, MechError> {

    match col {
      TableIndex::Alias(alias) => {
        match self.column_alias_to_ix.get(&alias) {
          Some(ix) => Ok(self.data[*ix as usize].clone()),
          None => Err(MechError::GenericError(2821)),
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
      TableIndex::Table(_) |
      TableIndex::None => Err(MechError::GenericError(2824)), 
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
    table_drawing.add_line(format!("{} ({} x {})", humanize(&self.id),self.rows,self.cols));
    let mut header = "".to_string();
    for (ix, alias) in self.column_ix_to_alias.iter().enumerate() {
      header += &format!(" {}", humanize(alias)); 
    }
    if header != "" {
      table_drawing.add_separator();  
    }
    table_drawing.add_line(header);
    table_drawing.add_table(self);
    write!(f,"{:?}",table_drawing)?;
    Ok(())
  }
}