// # Table

// A table starts with a tag, and has a matrix of memory available for data, 
// where each column represents an attribute, and each row represents an entity.

// ## Prelude

use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use crate::*;
use hashbrown::HashMap;
use indexmap::IndexMap;

// ### Table Id

#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Debug,Copy,Clone,PartialEq,Eq, Serialize, Deserialize, Hash)]
pub enum TableShape {
  Scalar,
  Column(usize),
  Row(usize),
  Matrix(usize,usize),
  Dynamic(usize,usize),
  Pending(TableId),
}

// ### TableIndex

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableIndex {
  Index(usize),
  Alias(u64),
  Aliases(Vec<u64>),
  IxTable(TableId),
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
      TableIndex::IxTable(table_id) => *table_id.unwrap() as usize,
      TableIndex::Aliases(_) |
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
      &TableIndex::Aliases(ref aliases) => write!(f, "IxAliases({:?})", aliases.iter().map(|alias| humanize(alias)).collect::<Vec<String>>()),
      &TableIndex::IxTable(ref table_id) => write!(f, "IxTable({:?})", table_id),
      &TableIndex::ReshapeColumn => write!(f, "IxReshapeColumn"),
      &TableIndex::All => write!(f, "IxAll"),
      &TableIndex::None => write!(f, "IxNone"),
    }
  }
}

// ## Table


/*
The Table struct in Mech represents a data table. A vector of ValueKind to identify the type of data in each column, and maps to store aliases for columns and rows. The data in the table is stored as a vector of Column structs, where each Column struct contains a vector of Values. The implication is that tables can have columns with different types of data and the elements of each column are contiguous in memory, allowing for efficient processing of large amounts of data.


The StringDictionary is used to store strings used in the table to reduce memory usage. This struct is essential in the representation and manipulation of tables in Mech.
*/

pub type StringDictionary = Rc<RefCell<HashMap<u64,MechString>>>;

pub struct Table {
  pub id: u64,     
  pub dynamic: bool,                      
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
      dynamic: false,
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

  pub fn is_empty(&self) -> bool {
    if self.rows == 0 || self.cols == 0 {
      true
    } else {
      false
    }
  }

  pub fn get_col_raw(&self, col_ix: usize) -> std::result::Result<Column,MechError> {
    if col_ix < self.cols {
      Ok(self.data[col_ix].clone())
    } else {
      Err(MechError{msg: "".to_string(), id: 7001, kind: MechErrorKind::None})
    }
  }

  pub fn kind(&self) -> ValueKind {
    if self.col_kinds.len() == 0 {
      return ValueKind::Empty;
    }
    let first_col_kind = self.col_kinds[0].clone();
    if self.col_kinds.iter().all(|kind| *kind == first_col_kind) {
      first_col_kind
    } else {
      ValueKind::Compound(self.col_kinds.clone())
    }
  }

  pub fn name(&self) -> Option<String> {
    if let Some(mstring) = self.dictionary.borrow().get(&self.id) {
      Some(format!("{}", mstring.to_string()))
    } else {
      None
    }
  }

  pub fn get_column(&self, col: &TableIndex) -> Result<Column, MechError> {
    match col {
      TableIndex::Alias(alias) => {
        match self.col_map.get_index(&alias) {
          Ok(ix) => Ok(self.data[ix as usize].clone()),
          Err(x) => {
            Err(x)
          },
        }
      }
      TableIndex::Index(0) => {
        Err(MechError{msg: "".to_string(), id: 7003, kind: MechErrorKind::None})
      }
      TableIndex::Index(ix) => {
        if *ix <= self.cols { 
          Ok(self.data[*ix-1].clone())
        } else {
          Err(MechError{msg: "".to_string(), id: 7004, kind: MechErrorKind::None})
        }
      }
      TableIndex::All => {
        if self.cols == 1 {
          Ok(self.data[0].clone())
        } else {
          Err(MechError{msg: "".to_string(), id: 7005, kind: MechErrorKind::None})
        }
      }
      TableIndex::Aliases(_) |
      TableIndex::ReshapeColumn |
      TableIndex::IxTable(_) |
      TableIndex::None => Err(MechError{msg: "".to_string(), id: 7006, kind: MechErrorKind::None}), 
    }
  }  

  pub fn set_col_alias(&mut self, ix: usize, alias: u64) -> Result<(),MechError> {
    if ix < self.cols {
      self.col_map.insert(ix,alias);
      Ok(())
    } else {
      Err(MechError{msg: "".to_string(), id: 7008, kind: MechErrorKind::None})
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

  pub fn extend(&mut self, other: &Table) -> Result<(),MechError> {
    if self.kind() != other.kind() {
      return Err(MechError{msg: "".to_string(), id: 7059, kind: MechErrorKind::None});
    }
    if self.cols != other.cols {
      return Err(MechError{msg: "".to_string(), id: 7060, kind: MechErrorKind::None});
    }
    self.rows += other.rows;
    for c in 0..self.cols {
      let col = &self.data[c];
      let other_col = &other.data[c];
      col.extend(&other_col);
    } 
    Ok(())
  }

  pub fn set_col_kind(&mut self, col: usize, kind: ValueKind) -> Result<(),MechError> {
    if col < self.cols {
      match (&mut self.data[col], kind) {
        (Column::U8(_), ValueKind::U8) => (),
        (Column::Empty, ValueKind::U8) => {
          let column = ColumnV::<U8>::new(vec![U8::new(0);self.rows]);
          self.data[col] = Column::U8(column);
          self.col_kinds[col] = ValueKind::U8;
        },
        (Column::U16(_), ValueKind::U16) => (),
        (Column::Empty, ValueKind::U16) => {
          let column = ColumnV::<U16>::new(vec![U16::new(0);self.rows]);
          self.data[col] = Column::U16(column);
          self.col_kinds[col] = ValueKind::U16;
        },
        (Column::U32(_), ValueKind::U32) => (),
        (Column::Empty, ValueKind::U32) => {
          let column = ColumnV::<U32>::new(vec![U32::new(0);self.rows]);
          self.data[col] = Column::U32(column);
          self.col_kinds[col] = ValueKind::U32;
        },
        (Column::U64(_), ValueKind::U64) => (),
        (Column::Empty, ValueKind::U64) => {
          let column = ColumnV::<U64>::new(vec![U64::new(0);self.rows]);
          self.data[col] = Column::U64(column);
          self.col_kinds[col] = ValueKind::U64;
        },
        (Column::U128(_), ValueKind::U128) => (),
        (Column::Empty, ValueKind::U128) => {
          let column = ColumnV::<U128>::new(vec![U128::new(0);self.rows]);
          self.data[col] = Column::U128(column);
          self.col_kinds[col] = ValueKind::U128;
        },
        (Column::I8(_), ValueKind::I8) => (),
        (Column::Empty, ValueKind::I8) => {
          let column = ColumnV::<I8>::new(vec![I8::new(0);self.rows]);
          self.data[col] = Column::I8(column);
          self.col_kinds[col] = ValueKind::I8;
        },
        (Column::I16(_), ValueKind::I16) => (),
        (Column::Empty, ValueKind::I16) => {
          let column = ColumnV::<I16>::new(vec![I16::new(0);self.rows]);
          self.data[col] = Column::I16(column);
          self.col_kinds[col] = ValueKind::I16;
        },
        (Column::I32(_), ValueKind::I32) => (),
        (Column::Empty, ValueKind::I32) => {
          let column = ColumnV::<I32>::new(vec![I32::new(0);self.rows]);
          self.data[col] = Column::I32(column);
          self.col_kinds[col] = ValueKind::I32;
        },
        (Column::I64(_), ValueKind::I64) => (),
        (Column::Empty, ValueKind::I64) => {
          let column = ColumnV::<I64>::new(vec![I64::new(0);self.rows]);
          self.data[col] = Column::I64(column);
          self.col_kinds[col] = ValueKind::I64;
        },
        (Column::I128(_), ValueKind::I128) => (),
        (Column::Empty, ValueKind::I128) => {
          let column = ColumnV::<I128>::new(vec![I128::new(0);self.rows]);
          self.data[col] = Column::I128(column);
          self.col_kinds[col] = ValueKind::I128;
        },
        (Column::F32(_), ValueKind::F32) => (),
        (Column::Empty, ValueKind::F32) => {
          let column = ColumnV::<F32>::new(vec![F32::new(0.0);self.rows]);
          self.data[col] = Column::F32(column);
          self.col_kinds[col] = ValueKind::F32;
        },
        (Column::F64(_), ValueKind::F64) => (),
        (Column::Empty, ValueKind::F64) => {
          let column = ColumnV::<F64>::new(vec![F64::new(0.0);self.rows]);
          self.data[col] = Column::F64(column);
          self.col_kinds[col] = ValueKind::F64;
        },
        (Column::f32(_), ValueKind::f32) => (),
        (Column::Empty, ValueKind::f32) => {
          let column = ColumnV::<f32>::new(vec![0.0;self.rows]);
          self.data[col] = Column::f32(column);
          self.col_kinds[col] = ValueKind::f32;
        },
        (Column::Time(_), ValueKind::Time) => (),
        (Column::Empty, ValueKind::Time) => {
          let column = ColumnV::<F32>::new(vec![F32::new(0.0);self.rows]);
          self.data[col] = Column::Time(column);
          self.col_kinds[col] = ValueKind::Time;
        },
        (Column::Length(_), ValueKind::Length) => (),
        (Column::Empty, ValueKind::Length) => {
          let column = ColumnV::<F32>::new(vec![F32::new(0.0);self.rows]);
          self.data[col] = Column::Length(column);
          self.col_kinds[col] = ValueKind::Length;
        },
        (Column::Speed(_), ValueKind::Speed) => (),
        (Column::Empty, ValueKind::Speed) => {
          let column = ColumnV::<F32>::new(vec![F32::new(0.0);self.rows]);
          self.data[col] = Column::Speed(column);
          self.col_kinds[col] = ValueKind::Speed;
        },
        (Column::Bool(_), ValueKind::Bool) => (),
        (Column::Empty, ValueKind::Bool) => {
          let column = ColumnV::<bool>::new(vec![false;self.rows]);
          self.data[col] = Column::Bool(column);
          self.col_kinds[col] = ValueKind::Bool;
        },
        (Column::String(_), ValueKind::String) => (),
        (Column::Empty, ValueKind::String) => {
          let column = ColumnV::<MechString>::new(vec![MechString::new();self.rows]);
          self.data[col] = Column::String(column);
          self.col_kinds[col] = ValueKind::String;
        },
        (Column::Reference(_), ValueKind::Reference) => (),
        (Column::Empty, ValueKind::Reference) => {
          let column = ColumnV::<TableId>::new(vec![TableId::Local(0);self.rows]);
          self.data[col] = Column::Ref(column);
          self.col_kinds[col] = ValueKind::Reference;
        },
        (Column::Any(_), ValueKind::Any) => (),
        (Column::Empty, ValueKind::Any) => {
          let column = ColumnV::<Value>::new(vec![Value::Empty; self.rows]);
          self.data[col] = Column::Any(column);
          self.col_kinds[col] = ValueKind::Any;
        },
        x => {
          return Err(MechError{msg: "".to_string(), id: 7009, kind: MechErrorKind::GenericError(format!("{:?}",x))});
        },
      }
      Ok(())
    } else {
      Err(MechError{msg: "".to_string(), id: 7010, kind: MechErrorKind::None})
    }
  }

  pub fn get_by_index(&self, row: TableIndex, col: TableIndex) -> Result<Value,MechError> {
    match (row, &self.get_column(&col)?) {
      (TableIndex::Index(0),_) => Err(MechError{msg: "".to_string(), id: 7211, kind: MechErrorKind::None}),
      (TableIndex::Index(row),Column::f32(c)) => Ok(Value::f32(c.borrow()[row-1])),
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
      _ => Err(MechError{msg: "".to_string(), id: 7011, kind: MechErrorKind::None}),
    }
  }

  pub fn set_col(&mut self, col_ix: usize, column: Column) -> std::result::Result<(),MechError> {
    if col_ix < self.cols {
      if self.col_kinds[col_ix] == ValueKind::Empty {
        self.col_kinds[col_ix] = column.kind();
        self.data[col_ix] = column;
        Ok(())
      } else {
        Err(MechError{msg: "".to_string(), id: 7012, kind: MechErrorKind::None})
      }
    } else {
      Err(MechError{msg: "".to_string(), id: 7013, kind: MechErrorKind::None})
    }
  }

  pub fn get_columns(&self, col: &TableIndex) -> Result<Vec<Column>, MechError> {
    match col {
      TableIndex::All => {
        Ok(self.data.iter().cloned().collect())
      },
      TableIndex::Aliases(aliases) => {
        let mut ixes = vec![];
        for alias in aliases {
          match self.col_map.alias_to_ix.get(alias) {
            Some(ix) => ixes.push(ix),
            None => {return Err(MechError{msg: "".to_string(), id: 7014, kind: MechErrorKind::None});}
          }
        }
        let mut cols = vec![];
        for ix in ixes {
          cols.push(self.data[*ix].clone());
        }
        Ok(cols)
      },
      x => {Err(MechError{msg: "".to_string(), id: 7044, kind: MechErrorKind::GenericError(format!("{:?}",x))})}
    }
  }

  pub fn set(&self, row: &TableIndex, col: &TableIndex, val: Value ) -> Result<(),MechError> {
    let row_ix = match row {
      TableIndex::Index(0) => {return Err(MechError{msg: "".to_string(), id: 0001, kind: MechErrorKind::None});},
      TableIndex::Index(ix) => ix - 1,
      _ => 0,
    };
    let col_ix = match col {
      TableIndex::Index(0) => {return Err(MechError{msg: "".to_string(), id: 0001, kind: MechErrorKind::None})},
      TableIndex::Index(ix) => ix - 1,
      TableIndex::Alias(alias) => {
        match self.col_map.get_index(alias) {
          Ok(ix) => ix,
          Err(x) => {return Err(MechError{msg: "".to_string(), id: 7015, kind: MechErrorKind::None})}
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
        (Column::Speed(c), Value::Speed(v)) |
        (Column::F32(c), Value::F32(v)) => c.borrow_mut()[row] = v,
        (Column::F32(c), Value::U64(v)) => c.borrow_mut()[row] = v.into(),
        (Column::f32(c), Value::f32(v)) => c.borrow_mut()[row] = v,
        (Column::F64(c), Value::F64(v)) => c.borrow_mut()[row] = v,
        (Column::U8(c), Value::U8(v)) => c.borrow_mut()[row] = v,
        (Column::U8(c), Value::U64(v)) => c.borrow_mut()[row] = v.into(),
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
        (Column::Any(c), v) => c.borrow_mut()[row] = v,
        (Column::Ref(c), Value::Reference(v)) => c.borrow_mut()[row] = v,
        (Column::Empty, Value::Empty) => (),
        (x,y) => {
          return Err(MechError{msg: "".to_string(), id: 7016, kind: MechErrorKind::GenericError(format!("{:?}",y))});
        },
      }
      Ok(())
    } else {
      Err(MechError{msg: "".to_string(), id: 7017, kind: MechErrorKind::None})
    }
  }
  
  pub fn get(&self, row: &TableIndex, col: &TableIndex) -> Result<Value,MechError> {
    let row_ix = match row {
      TableIndex::Index(0) => {return Err(MechError{msg: "".to_string(), id: 7018, kind: MechErrorKind::None})},
      TableIndex::Index(ix) => ix - 1,
      _ => 0,
    };
    let col_ix = match col {
      TableIndex::Index(0) => {return Err(MechError{msg: "".to_string(), id: 7019, kind: MechErrorKind::None})},
      TableIndex::Index(ix) => ix - 1,
      TableIndex::Alias(alias) => {
        match self.col_map.get_index(alias) {
          Ok(ix) => ix,
          Err(_) => {return Err(MechError{msg: "".to_string(), id: 7020, kind: MechErrorKind::None})}
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
        Column::Speed(c) => Ok(Value::Speed(c.borrow()[row])),
        Column::F32(c) => Ok(Value::F32(c.borrow()[row])),
        Column::f32(c) => Ok(Value::f32(c.borrow()[row])),
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
        Column::Any(c) => Ok(c.borrow()[row].clone()),
        Column::Empty => Ok(Value::Empty),
        x => {
          Err(MechError{msg: "".to_string(), id: 7021, kind: MechErrorKind::None})
        },
      }
    } else {
      Err(MechError{msg: "".to_string(), id: 7022, kind: MechErrorKind::None})
    }
  }

  pub fn get_linear_raw(&self, ix: usize) -> Result<Value,MechError> {
    self.get_linear(ix-1)
  }

  pub fn get_linear(&self, ix: usize) -> Result<Value,MechError> {
    if ix < self.rows * self.cols {
      let row = ix / self.cols;
      let col = ix % self.cols;
      self.get_raw(row,col)
    } else {
      Err(MechError{msg: "".to_string(), id: 7023, kind: MechErrorKind::None})
    }
  }

  pub fn len(&self) -> usize {
    self.rows * self.cols
  }

  pub fn get_column_unchecked(&self, col: usize) -> Column {
    self.data[col].clone()
  }

  pub fn has_col_aliases(&self) -> bool {
    self.col_map.alias_to_ix.len() > 0
  }

  pub fn index_to_subscript(&self, ix: usize) -> Result<(usize, usize),MechError> {
    let row = ix / self.cols;
    let col = ix % self.cols;
    if ix < self.rows * self.cols {
      Ok((row,col))
    } else {
      Err(MechError{msg: "".to_string(), id: 7024, kind: MechErrorKind::None})
    }
  }
  
  pub fn set_linear(&self, ix: usize, val: Value) -> Result<(),MechError> {
    if ix < self.rows * self.cols {
      let row = ix / self.cols;
      let col = ix % self.cols;
      self.set_raw(row,col, val)
    } else {
      Err(MechError{msg: "".to_string(), id: 7025, kind: MechErrorKind::None})
    }
  }

  pub fn shape(&self) -> TableShape {
    match (self.rows, self.cols) {
      (0,_) |
      (_,0) => TableShape::Pending(TableId::Global(self.id)),
      (1,1) => TableShape::Scalar,
      (x,1) => TableShape::Column(x),
      (1,x) => TableShape::Row(x),
      (x,y) => TableShape::Matrix(x,y), 
    }
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

  pub fn to_changes(&self) -> Vec<Change> {
    let mut changes = vec![];
    changes.push(Change::NewTable{table_id: self.id, rows: self.rows, columns: self.cols});
    for ((alias,ix)) in self.col_map.iter() {
      changes.push(Change::ColumnAlias{table_id: self.id, column_ix: *ix, column_alias: *alias});
    } 
    for (ix,kind) in self.col_kinds.iter().enumerate() {
      changes.push(Change::ColumnKind{table_id: self.id, column_ix: ix, column_kind: kind.clone()});
    } 
    let mut data_changes = self.data_to_changes();
    changes.append(&mut data_changes);
    changes
  }

  pub fn data_to_changes(&self) -> Vec<Change> {
    let mut changes = vec![];
    let mut values = vec![];
    for i in 0..self.rows {
      for j in 0..self.cols {
        match self.get_raw(i,j) {
          Ok(value) => {values.push((TableIndex::Index(i+1), TableIndex::Index(j+1), value));}
          _ => (),
        }
      }
    }
    changes.push(Change::Set((self.id, values)));
    changes
  }

  collect_columns!(collect_columns_u8,unwrap_u8,U8);
  collect_columns!(collect_columns_u16,unwrap_u16,U16);
  collect_columns!(collect_columns_u32,unwrap_u32,U32);
  collect_columns!(collect_columns_u64,unwrap_u64,U64);
  collect_columns!(collect_columns_u128,unwrap_u128,U128);
  collect_columns!(collect_columns_i8,unwrap_i8,I8);
  collect_columns!(collect_columns_i16,unwrap_i16,I16);
  collect_columns!(collect_columns_i32,unwrap_i32,I32);
  collect_columns!(collect_columns_i64,unwrap_i64,I64);
  collect_columns!(collect_columns_i128,unwrap_i128,I128);
  collect_columns!(collect_columns_f32,unwrap_f32,F32);
  collect_columns!(collect_columns_f64,unwrap_f64,F64);
 
}

#[macro_export]
macro_rules! collect_columns {
  ($function_name:tt,$unwrap:tt,$type:tt) => (
    pub fn $function_name(&self) -> Vec<ColumnV<$type>> {
      let mut cols: Vec<ColumnV<$type>> = vec![];
      for col_ix in 0..self.cols {
        let col = self.data[col_ix].$unwrap().unwrap();
        cols.push(col.clone());
      }
      cols
    }
  )
}

pub type TableIx = usize;
pub type Alias = u64;


#[derive(Debug, Clone)]
pub struct AliasMap {
  pub capacity: usize,
  pub ix_to_alias: Vec<Alias>,  
  pub alias_to_ix: IndexMap<Alias,TableIx>,
}

impl AliasMap {
  pub fn new(capacity: usize) -> Self {
    AliasMap {
      capacity,
      ix_to_alias: vec![0;capacity],
      alias_to_ix: IndexMap::new(),
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
      Err(MechError{msg: "".to_string(), id: 7026, kind: MechErrorKind::None})
    }
  }

  pub fn get_index(&self, alias: &Alias) -> std::result::Result<TableIx,MechError> {
    match self.alias_to_ix.get(alias) {
      Some(ix) => Ok(*ix),
      None => Err(MechError{msg: "".to_string(), id: 7027, kind: MechErrorKind::GenericError(format!("{:?}", humanize(alias)))}),
    }
  }

  pub fn get_alias(&self, ix: &TableIx) -> std::result::Result<Alias,MechError> {
    if ix < &self.capacity {
      Ok(self.ix_to_alias[*ix])
    } else {
      Err(MechError{msg: "".to_string(), id: 7028, kind: MechErrorKind::None})
    }
  }

  pub fn aliases(&self) -> std::slice::Iter<u64> {
    self.ix_to_alias.iter()
  }

  pub fn iter(&self) -> indexmap::map::Iter<u64, usize> {
    self.alias_to_ix.iter()
  }

  pub fn len(&self) -> usize {
    self.alias_to_ix.iter().len()
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
}