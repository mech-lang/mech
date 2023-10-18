
/*

Defines an enum type Transformation that represents a set of different transformations that can be applied to tables in Mech. The transformations include things like selecting rows and columns, setting values, defining new tables, and applying functions:

- Identifier: refers to a named identifier or variable
- NumberLiteral: represents a numerical value
- TableAlias: refers to a table by its alias
- TableReference: refers to a table by its ID and a reference value
- NewTable: creates a new table with a given number of rows and columns
- Constant: sets a constant value for a table
- ColumnKind: sets the data type of a column in a table
- Set: copies the value from one cell in a table to another cell in a different table
- UpdateData: updates the value of a cell in a table
- ColumnAlias: refers to a column in a table by its alias
- RowAlias: refers to a row in a table by its alias
- Whenever: specifies a set of indices to be used for filtering tables
- Function: applies a function to the specified arguments and returns a new table
- TableDefine: creates a new table with specified indices and returns its ID
- Select: selects a subset of a table based on the specified indices
*/


use crate::*;
use crate::Argument;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;
use std::cmp::Ordering;

#[derive(Clone, Serialize, Deserialize)]
pub enum Transformation {
  Identifier{ name: Vec<char>, id: u64 },
  NumberLiteral{kind: u64, bytes: Vec<u8>},
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value},
  ColumnKind{table_id: TableId, column_ix: usize, kind: u64},
  Set{src_id: TableId, src_row: TableIndex, src_col: TableIndex, dest_id: TableId, dest_row: TableIndex, dest_col: TableIndex},
  UpdateData{name: u64, src_id: TableId, src_row: TableIndex, src_col: TableIndex, dest_id: TableId, dest_row: TableIndex, dest_col: TableIndex},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>},
  Function{name: u64, arguments: Vec<Argument>, out: (TableId, TableIndex, TableIndex)},
  TableDefine{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>, out: TableId},
  Select{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>},
  FollowedBy{table_id: TableId, initial: TableId, subsequent: TableId},
}

impl fmt::Debug for Transformation {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {
      Transformation::Identifier{name,id} => 
        write!(f,"👀 Identifier(name: {:?}, id: {})",name,humanize(id))?,
      Transformation::Constant{table_id, value} => 
        write!(f,"🪨 Constant(table_id: {:?}, value: {:?})",table_id, value)?,
      Transformation::Select{table_id,indices} => 
        write!(f,"✅ Select(table_id: {:#?}, indices: {:#?})",table_id,indices)?,
      Transformation::NumberLiteral{kind,bytes} => 
        write!(f,"🐙 NumberLiteral(kind: {:?}, bytes: {:?})",humanize(kind),bytes)?,
      Transformation::Whenever{table_id,indices} => 
        write!(f,"⏲️ Whenever(table_id: {:#?}, indices: {:#?})",table_id,indices)?,
      Transformation::TableAlias{table_id,alias} => 
        write!(f,"🥸 TableAlias(table_id: {:?}, alias: {})",table_id,humanize(alias))?,
      Transformation::NewTable{table_id, rows, columns} =>  
        write!(f,"🌚 NewTable(table_id: {:?}, rows: {} cols: {})",table_id,rows,columns)?,
      Transformation::TableReference{table_id, reference} => 
        write!(f,"📛 TableReference(table_id: {:?}, reference: {:?})",table_id, reference)?,
      Transformation::Function{name,arguments,out} => { 
        write!(f,"🧮 Function(name: {}, args: {:#?}, out: {:#?})",humanize(name),arguments,out)? },
      Transformation::TableDefine{table_id, indices, out} => 
        write!(f,"📅 TableDefine(table_id: {:?}, indices: {:?}, out: {:?})",table_id, indices, out)?,
      Transformation::ColumnKind{table_id, column_ix, kind} => 
        write!(f,"🎩 ColumnKind(table_id: {:?}, column_ix: {}, kind: {})",table_id,column_ix,humanize(kind))?,
      Transformation::RowAlias{table_id, row_ix, row_alias} => 
        write!(f,"🥸 RowAlias(table_id: {:?}, row_ix: {}, row_alias: {})",table_id,row_ix,humanize(row_alias))?,
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => 
        write!(f,"🥸 ColumnAlias(table_id: {:?}, column_ix: {}, column_alias: {})",table_id,column_ix,humanize(column_alias))?,
      Transformation::Set{src_id, src_row, src_col, dest_id, dest_row, dest_col} => 
        write!(f,"♟️ Set(src_id: {:?}, src_indices: ({:?},{:?}),\n    dest_id: {:?}, dest_indices: ({:?},{:?}))",src_id,src_row,src_col,dest_id,dest_row,dest_col)?,
      Transformation::UpdateData{name, src_id, src_row, src_col, dest_id, dest_row, dest_col} => 
        write!(f,"♻️ UpdateData(name: {}, src_id: {:?}, src_indices: ({:?},{:?}),\n    dest_id: {:?}, dest_indices: ({:?},{:?}))",humanize(name),src_id,src_row,src_col,dest_id,dest_row,dest_col)?,
      Transformation::FollowedBy{table_id, initial, subsequent} => 
        write!(f,"➡️ FollowedBy(table_id: {:?}, initial: {:?}, subsequent: {:?})", table_id, initial, subsequent)?,
    }
    Ok(())
  }
}

impl Ord for Transformation {
  fn cmp(&self, other: &Self) -> Ordering {
    Ordering::Equal
  }
}

impl PartialOrd for Transformation {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    match (self,other) {
      (Transformation::Whenever{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::Whenever{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::UpdateData{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::UpdateData{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::Set{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::Set{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::TableDefine{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::TableDefine{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::FollowedBy{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::FollowedBy{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::Function{..},_) => {
        return Some(Ordering::Greater);
      }
      (_,Transformation::Function{..}) => {
        return Some(Ordering::Less);
      }
      (Transformation::TableReference{..},
       Transformation::TableReference{..}) => {
        Some(Ordering::Less)
      }
      (Transformation::NewTable{table_id,..},Transformation::NewTable{table_id: table_id2, ..}) => {
        if table_id.unwrap() > table_id2.unwrap() {
          Some(Ordering::Greater)
        } else { 
          Some(Ordering::Less)
        }
      },
      (Transformation::TableReference{..},Transformation::ColumnAlias{..}) => Some(Ordering::Greater),
      (Transformation::ColumnAlias{..},Transformation::TableReference{..}) => Some(Ordering::Less),
      (_,Transformation::NewTable{..}) => Some(Ordering::Greater),
      (Transformation::NewTable{..},_) => Some(Ordering::Less),
      (_,Transformation::Identifier{..}) => Some(Ordering::Greater),
      (Transformation::Identifier{..},_) => Some(Ordering::Less),
      (_,Transformation::TableAlias{..}) => Some(Ordering::Greater),
      (Transformation::TableAlias{..},_) => Some(Ordering::Less),
      (_,Transformation::NumberLiteral{..}) => Some(Ordering::Greater),
      (Transformation::NumberLiteral{..},_) => Some(Ordering::Less),
      (Transformation::Set{src_id,..},_) => Some(Ordering::Greater),
      (_,Transformation::Set{src_id,..}) => Some(Ordering::Less),
      x => {
        None
      }
    }
  }
}

impl Eq for Transformation { }
impl PartialEq for Transformation {
  fn eq(&self, other: &Self) -> bool {
    hash_str(&format!("{:?}",self)) == hash_str(&format!("{:?}",other))
  }
}