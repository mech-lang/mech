use crate::*;
use hashbrown::HashMap;
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

pub type Transaction = Vec<Change>;

#[derive(Clone)]
pub struct Database {
  pub tables: HashMap<u64,Rc<RefCell<Table>>>,
  pub table_alias_to_id: HashMap<u64,TableId>,
}

impl Database {
  pub fn new() -> Database {
    Database {
      tables: HashMap::new(),
      table_alias_to_id: HashMap::new(),
    }
  }

  pub fn union(&mut self, other: &Self) -> Result<(),MechError> {
    let mut other_tables = other.tables.clone();
    for (id,other_table) in other_tables.drain() {
      match self.tables.try_insert(id, other_table.clone()) {
        Ok(_) => (),
        Err(x) => {return Err(MechError{id: 0001, kind: MechErrorKind::None});},
      }
    }
    let mut other_table_aliases = other.table_alias_to_id.clone();
    for (id,other_table) in other_table_aliases.drain() {
      match self.table_alias_to_id.try_insert(id, other_table.clone()) {
        Ok(_) => (),
        Err(x) => {return Err(MechError{id: 0001, kind: MechErrorKind::None});},
      }
    }
    Ok(())
  }

  pub fn insert_alias(&mut self, alias: u64, table_id: TableId) -> Result<TableId,MechError> {
    match self.table_alias_to_id.try_insert(alias, table_id) {
      Err(x) => {return Err(MechError{id: 0001, kind: MechErrorKind::None});},
      Ok(x) => Ok(*x), 
    }
  }

  pub fn insert_table(&mut self, table: Table) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.tables.try_insert(table.id, Rc::new(RefCell::new(table))) {
      Ok(x) => Ok(x.clone()),
      Err(x) => {return Err(MechError{id: 0001, kind: MechErrorKind::None});},
    }
  }

  pub fn get_table(&self, table_name: &str) -> Option<&Rc<RefCell<Table>>> {
    let alias = hash_str(table_name);
    match self.table_alias_to_id.get(&alias) {
      Some(table_id) => {
        self.tables.get(table_id.unwrap())
      }
      _ => self.tables.get(&alias),
    }
  }

  pub fn get_table_by_id(&self, table_id: &u64) -> Option<&Rc<RefCell<Table>>> {
    match self.tables.get(table_id) {
      None => {
        match self.table_alias_to_id.get(&table_id) {
          None => None,
          Some(table_id) => {
            self.tables.get(table_id.unwrap())
          }
        }
      }
      x => x
    }
  }

  pub fn get_table_by_id_mut(&self, table_id: u64) -> Option<&Rc<RefCell<Table>>> {
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