use crate::*;
use crate::core::*;
use hashbrown::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub enum Change {
  Set((u64, Vec<(TableIndex, TableIndex, Value)>)),
  NewTable{table_id: u64, rows: usize, columns: usize},
  ColumnAlias{table_id: u64, column_ix: usize, column_alias: u64},
  ColumnKind{table_id: u64, column_ix: usize, column_kind: ValueKind},
}

impl fmt::Debug for Change {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Change::Set((table_id,args)) => write!(f,"Set({},{:#?})",humanize(table_id),args)?,
      Change::NewTable{table_id,rows,columns} => write!(f,"NewTable({},{:?},{:?})",humanize(table_id),rows,columns)?,
      Change::ColumnAlias{table_id,column_ix,column_alias} => write!(f,"ColumnAlias({},{:?},{})",humanize(table_id),column_ix,humanize(column_alias))?,
      Change::ColumnKind{table_id,column_ix,column_kind} => write!(f,"ColumnKind({},{:?},{:?})",humanize(table_id),column_ix,column_kind)?,
    }
    Ok(())
  }
}

#[derive(Clone)]
pub struct Database {
  pub transaction_queue: Vec<Vec<Change>>,
  pub dynamic_tables: HashSet<Register>,
  pub tables: HashMap<u64,TableRef>,
  pub table_alias_to_id: HashMap<u64,TableId>,
}

impl Database {
  pub fn new() -> Database {
    Database {
      transaction_queue: Vec::new(),
      dynamic_tables: HashSet::new(),
      tables: HashMap::new(),
      table_alias_to_id: HashMap::new(),
    }
  }

  pub fn clear(&mut self) {
    self.dynamic_tables.clear();
    self.tables.clear();
    self.table_alias_to_id.clear();
  }

  pub fn process_transaction_queue(&mut self) -> Result<HashSet<Register>,MechError> {
    let mut changed_registers = HashSet::new();
    let queue = self.transaction_queue.clone();
    self.transaction_queue.clear();
    for txn in queue {
      let mut r = self.process_transaction(&txn)?;
      changed_registers = changed_registers.union(&mut r).cloned().collect();
    }
    Ok(changed_registers)
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<HashSet<Register>,MechError> {
    let mut changed_registers = HashSet::new();
    for change in txn {
      match change {
        Change::Set((table_id, adds)) => {
          match self.get_table_by_id(table_id) {
            Some(table) => {
              let table_brrw = table.borrow();
              for (row,col,val) in adds {
                match table_brrw.set(row, col, val.clone()) {
                  Ok(()) => {
                    // TODO This is inserting a {:,:} register instead of the one passed in, and that needs to be fixed.
                    changed_registers.insert((TableId::Global(*table_id),RegisterIndex::All,RegisterIndex::All));
                  },
                  Err(x) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1719, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
                }
              }
            }
            None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1720, kind: MechErrorKind::MissingTable(TableId::Global(*table_id))});},
          }
        }
        Change::NewTable{table_id, rows, columns} => {
          let table = Table::new(*table_id,rows.clone(),*columns);
          self.insert_table(table)?;
        }
        Change::ColumnAlias{table_id, column_ix, column_alias} => {
          match self.get_table_by_id(table_id) {
            Some(table) => {
              let mut table_brrw = table.borrow_mut();   
              let rows = table_brrw.rows;
              if *column_ix + 1 > table_brrw.cols {
                table_brrw.resize(rows, column_ix + 1);
              }    
              table_brrw.set_col_alias(*column_ix,*column_alias);     
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1721, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
        Change::ColumnKind{table_id, column_ix, column_kind} => {
          match self.get_table_by_id(table_id) {
            Some(table) => {
              let mut table_brrw = table.borrow_mut();   
              let rows = table_brrw.rows;
              if *column_ix + 1 > table_brrw.cols {
                table_brrw.resize(rows, column_ix + 1);
              }    
              table_brrw.set_col_kind(*column_ix,column_kind.clone());     
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1722, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
      }
    }
    Ok(changed_registers)
  }


  pub fn union(&mut self, other: &Self) -> Result<(),MechError> {
    let mut other_tables = other.tables.clone();
    for (id,other_table) in other_tables.drain() {
      match self.tables.try_insert(id, other_table.clone()) {
        Ok(_) => (),
        Err(x) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1723, kind: MechErrorKind::None});},
      }
    }
    let mut other_table_aliases = other.table_alias_to_id.clone();
    for (id,other_table) in other_table_aliases.drain() {
      match self.table_alias_to_id.try_insert(id, other_table.clone()) {
        Ok(_) => (),
        Err(x) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1724, kind: MechErrorKind::None});},
      }
    }
    Ok(())
  }

  pub fn insert_alias(&mut self, alias: u64, table_id: TableId) -> Result<TableId,MechError> {
    match self.table_alias_to_id.try_insert(alias, table_id) {
      Err(x) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1725, kind: MechErrorKind::DuplicateAlias(*table_id.unwrap())});},
      Ok(x) => Ok(*x), 
    }
  }

  pub fn insert_table(&mut self, table: Table) -> Result<TableRef,MechError> {
    match self.tables.try_insert(table.id, Rc::new(RefCell::new(table))) {
      Ok(x) => Ok(x.clone()),
      Err(x) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1726, kind: MechErrorKind::None});},
    }
  }

  pub fn overwrite_table(&mut self, table: Table) -> Result<TableRef,MechError> {
    match self.tables.insert(table.id, Rc::new(RefCell::new(table))) {
      Some(x) => Ok(x.clone()),
      None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1726, kind: MechErrorKind::None});},
    }
  }

  pub fn insert_table_ref(&mut self, table: TableRef) -> Result<TableRef,MechError> {
    let table_id = {
      let table_brrw = table.borrow();
      table_brrw.id
    };
    match self.tables.try_insert(table_id, table) {
      Ok(x) => Ok(x.clone()),
      Err(x) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1726, kind: MechErrorKind::None});},
    }
  }

  pub fn get_table(&self, table_name: &str) -> Option<&TableRef> {
    let alias = hash_str(table_name);
    match self.table_alias_to_id.get(&alias) {
      Some(table_id) => {
        self.tables.get(table_id.unwrap())
      }
      _ => self.tables.get(&alias),
    }
  }

  pub fn get_table_by_id(&self, table_id: &u64) -> Option<&TableRef> {
    match self.tables.get(table_id) {
      None => {
        match self.table_alias_to_id.get(table_id) {
          None => None,
          Some(table_id) => {
            self.tables.get(table_id.unwrap())
          }
        }
      }
      x => x
    }
  }

  pub fn get_table_by_id_mut(&self, table_id: u64) -> Option<&TableRef> {
    let table_id = match self.tables.contains_key(&table_id) {
      true => table_id,
      false => match self.table_alias_to_id.get(&table_id) {
        Some(table_id) => *table_id.unwrap(),
        None => return None,
      }
    };
    self.tables.get(&table_id)
  }
}

impl fmt::Debug for Database {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut db_drawing = BoxPrinter::new();
    db_drawing.add_header("tables");
    for table in self.tables.values() {
      db_drawing.add_line(format!("{:?}", table.borrow()));
    }
    if self.table_alias_to_id.len() > 0 {
      db_drawing.add_header("table alias → table id");
      for (alias,id) in self.table_alias_to_id.iter() {
        db_drawing.add_line(format!("{} → {:?}", humanize(alias), id));
      }
    }
    write!(f,"{:?}",db_drawing)?;
    Ok(())
  }
}