use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;
use std::cmp::Ordering;

use rayon::prelude::*;
use std::thread;

#[derive(Clone, Serialize, Deserialize)]
pub enum Transformation {
  Identifier{ name: Vec<char>, id: u64 },
  NumberLiteral{kind: NumberLiteralKind, bytes: Vec<u8>},
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  ColumnKind{table_id: TableId, column_ix: usize, column_kind: ValueKind},
  Set{src_id: TableId, src_indices: Vec<(TableIndex, TableIndex)>, dest_id: TableId, dest_indices: Vec<(TableIndex, TableIndex)>},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: TableId, row: TableIndex, column: TableIndex, registers: Vec<Register>},
  Function{name: u64, arguments: Vec<(u64, TableId, TableIndex, TableIndex)>, out: (TableId, TableIndex, TableIndex)},
  Select{table_id: TableId, indices: Vec<(TableIndex, TableIndex)>, out: TableId},
}

impl fmt::Debug for Transformation {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self {
      Transformation::NewTable{table_id, rows, columns} =>  write!(f,"NewTable(table_id: {:?}, rows: {} cols: {})",table_id,rows,columns)?,
      Transformation::Identifier{name,id} => write!(f,"Identifier(name: {:?}, id: {})",name,humanize(id))?,
      Transformation::NumberLiteral{kind,bytes} => write!(f,"NumberLiteral(kind: {:?}, bytes: {:?})",kind,bytes)?,
      Transformation::TableAlias{table_id,alias} => write!(f,"Alias(table_id: {:?}, alias: {})",table_id,humanize(alias))?,
      Transformation::Select{table_id,indices,out} => write!(f,"Select(table_id: {:#?}, indices: {:#?}, out: {:#?})",table_id,indices,out)?,
      Transformation::Set{src_id, src_indices,dest_id,dest_indices} => write!(f,"Set(src_id: {:?}, src_indices: {:?},\n    dest_id: {:?}, dest_indices: {:?})",src_id,src_indices,dest_id,dest_indices)?,
      Transformation::Function{name,arguments,out} => {
        write!(f,"Function(name: {}, args: {:#?}, out: {:#?})",humanize(name),arguments,out)?
      },
      Transformation::Constant{table_id, value} => write!(f,"Constant(table_id: {:?}, value: {:?})",table_id, value)?,
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => write!(f,"ColumnAlias(table_id: {:?}, column_ix: {}, column_alias: {})",table_id,column_ix,humanize(column_alias))?,
      _ => write!(f,"Tfm Print Not Implemented")?
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
      (_,Transformation::NewTable{..}) => Some(Ordering::Greater),
      (Transformation::NewTable{..},_) => Some(Ordering::Less),
      (_,Transformation::TableAlias{..}) => Some(Ordering::Greater),
      (Transformation::TableAlias{..},_) => Some(Ordering::Less),
      (_,Transformation::NumberLiteral{..}) => Some(Ordering::Greater),
      (Transformation::NumberLiteral{..},_) => Some(Ordering::Less),
      (Transformation::Function{name, arguments, out},
       Transformation::Function{name: name2, arguments: arguments2, out: out2}) => {
        let (right_out_id,_,_) = out2;
        let (left_out_id,_,_) = out;
        // left function comes second because it consumes right fxn output
        for (_,left_id,_,_) in arguments {
          if left_id == right_out_id {
            return Some(Ordering::Greater);
          }
        }
        // left function comes first because it is consumed by right function
        for (_,right_id,_,_) in arguments2 {
          if right_id == left_out_id {
            return Some(Ordering::Less);
          }
        }
        // fxns are unrelated
        None
      }
      _ => None,
    }
  }
}

impl Eq for Transformation { }

impl PartialEq for Transformation {
  fn eq(&self, other: &Self) -> bool {
    hash_string(&format!("{:?}",self)) == hash_string(&format!("{:?}",other))
  }
}