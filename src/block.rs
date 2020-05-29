use table::{Table, TableId, Index, Value};
use database::{Database, Store, Change, Transaction};
use hashbrown::{HashMap, HashSet};
use quantities::{Quantity, QuantityMath, ToQuantity};
use std::cell::RefCell;
use std::rc::Rc;
use std::hash::Hasher;
use ahash::AHasher;
use rust_core::fmt;

// ## Block

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape 
// them or perform computations on them. Blocks can be thought of as pure functions where the input and 
// output are tables. Blocks have their own internal table store. Local tables can be defined within a 
// block, which allows the programmer to break a computation down into steps. The result of the computation 
// is then output to one or more global tables, which triggers the execution of other blocks in the network.
#[derive(Clone)]
pub struct Block {
  pub id: u64,
  pub state: BlockState,
  pub text: String,
  pub name: String,
  pub ready: HashSet<u64>,
  pub input: HashSet<u64>,
  pub output: HashSet<u64>,
  pub tables: HashMap<u64, Table>,
  pub store: Rc<Store>,
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<(Vec<TransformMap>,Transformation)>,
  pub changes: Vec<Change>,
  pub identifiers: HashMap<u64, String>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      text: String::new(),
      name: String::new(),
      identifiers: HashMap::new(),
      ready: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      state: BlockState::New,
      tables: HashMap::new(),
      store: Rc::new(Store::new(capacity)),
      transformations: Vec::new(),
      plan: Vec::new(),
      changes: Vec::new(),
    }
  }

  pub fn gen_id(&mut self) {
    let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
    for tfm in &self.transformations {
      hasher.write(format!("{:?}", tfm).as_bytes());
    }
    self.id = hasher.finish();   
  }

  pub fn register_transformations(&mut self, tfm_tuple: (String, Vec<Transformation>)) {
    self.transformations.push(tfm_tuple.clone());

    let (_, transformations) = tfm_tuple;

    for tfm in transformations {
      match tfm {
        Transformation::NewTable{table_id, rows, columns} => {
          match table_id {
            TableId::Global(id) => {
              self.changes.push(
                Change::NewTable{
                  table_id: id,
                  rows,
                  columns,
                }
              );
              for i in 1..=columns {
                self.output.insert(Register{table_id: id, row: Index::All, column: Index::Index(i)}.hash());
              }
              self.output.insert(Register{table_id: id, row: Index::All, column: Index::All}.hash());
            }
            TableId::Local(id) => {
              self.tables.insert(id, Table::new(id, rows, columns, self.store.clone()));
            }
          }
        }
        Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
          match table_id {
            TableId::Global(id) => {
              self.changes.push(
                Change::SetColumnAlias{
                  table_id: id,
                  column_ix,
                  column_alias,
                }
              );
              self.output.insert(Register{table_id: id, row: Index::All, column: Index::Alias(column_alias)}.hash());
            }
            TableId::Local(id) => {

            }
          }
        }
        Transformation::Constant{table_id, value, unit} => {
          match table_id {
            TableId::Local(id) => {
              let mut table = self.tables.get_mut(&id).unwrap();
              table.set(&Index::Index(1), &Index::Index(1), value);
            }
            _ => (),
          }
        }
        Transformation::Set{table_id, row, column, value} => {
          match table_id {
            TableId::Global(id) => {
              self.changes.push(
                Change::Set{
                  table_id: id,
                  values: vec![(row, column, value)],
                }
              );
              self.output.insert(id);
            }
            _ => (),
          }        
        }
        Transformation::Whenever{table_id, row, column} => {
          self.input.insert(Register{table_id, row, column}.hash());
          self.plan.push((vec![],tfm.clone()));
        }
        Transformation::Function{name, ref arguments, out} => {
          let (out_id, row, column) = out;
          match out_id {
            TableId::Global(id) => {self.output.insert(Register{table_id: id, row, column}.hash());},
            _ => (),
          }
          for (table_id, row, column) in arguments {
            match table_id {
              TableId::Global(id) => {self.input.insert(Register{table_id: *id, row: *row, column: *column}.hash());},
              _ => (),
            }
          }
          self.plan.push((vec![],tfm.clone()) );
        }
        _ => (),
      }
    }
  }

  // Process changes queued on the block
  pub fn process_changes(&mut self, database: Rc<RefCell<Database>>) {
    if !self.changes.is_empty() {
      let txn = Transaction {
        changes: self.changes.clone(),
      };
      self.changes.clear();
      database.borrow_mut().process_transaction(&txn);
      database.borrow_mut().transactions.push(txn);        
    }
  }

  pub fn solve(&mut self, database: Rc<RefCell<Database>>) {
    
    'step_loop: for (masks, step) in &self.plan {
      match step {
        Transformation::Whenever{table_id, row, column} => {
          let register = Register{table_id: *table_id, row: *row, column: *column};
          self.ready.remove(&register.hash());
        },
        Transformation::Function{name, arguments, out} => {
          match name {
            // math/add
            0xD0288E733F38A1B7 => {
              // TODO test argument count is 2
              let (lhs_table_id, lhs_rows, lhs_columns) = &arguments[0];
              let (rhs_table_id, rhs_rows, rhs_columns) = &arguments[1];
              let (out_table_id, out_rows, out_columns) = out;
              let mut db = database.borrow_mut();

              let mut out_table = match out_table_id {
                TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
                TableId::Local(id) => self.tables.get_mut(id).unwrap() as *mut Table,
              };

              let lhs_table = match lhs_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap(),
                TableId::Local(id) => self.tables.get(id).unwrap(),
              };
              let rhs_table = match rhs_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap(),
                TableId::Local(id) => self.tables.get(id).unwrap(),
              };
              let store = &db.store;

              // Figure out dimensions
              let equal_dimensions = if lhs_table.rows == rhs_table.rows
              { true } else { false };
              let lhs_scalar = if lhs_table.rows == 1 && lhs_table.columns == 1 
              { true } else { false };
              let rhs_scalar = if rhs_table.rows == 1 && rhs_table.columns == 1
              { true } else { false };

              let out_rows_count = unsafe{(*out_table).rows};

              let (mut lrix, mut lcix, mut rrix, mut rcix, mut out_rix, mut out_cix) = if rhs_scalar && lhs_scalar {
                (
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),               
                )
              } else if equal_dimensions {
                (
                  IndexIterator::Range(1..=lhs_table.rows),
                  IndexIterator::Constant(*lhs_columns),
                  IndexIterator::Range(1..=rhs_table.rows),
                  IndexIterator::Constant(*rhs_columns),
                  IndexIterator::Range(1..=out_rows_count),
                  IndexIterator::Constant(*out_columns),
                )
              } else if rhs_scalar {
                (
                  IndexIterator::Range(1..=lhs_table.rows),
                  IndexIterator::Constant(*lhs_columns),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Range(1..=out_rows_count),
                  IndexIterator::Constant(*out_columns),
                )
              } else {
                (
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Constant(Index::Index(1)),
                  IndexIterator::Range(1..=rhs_table.rows),
                  IndexIterator::Constant(*rhs_columns),
                  IndexIterator::Range(1..=out_rows_count),
                  IndexIterator::Constant(*out_columns),
                )
              };

              let mut i = 1;

              loop {
                let l1 = lrix.next().unwrap().unwrap();
                let l2 = lcix.next().unwrap().unwrap();
                let r1 = rrix.next().unwrap().unwrap();
                let r2 = rcix.next().unwrap().unwrap();
                let o1 = out_rix.next().unwrap().unwrap();
                let o2 = out_cix.next().unwrap().unwrap();
                match (lhs_table.get_unchecked(l1,l2), 
                       rhs_table.get_unchecked(r1,r2))
                {
                  (lhs_value, rhs_value) => {
                    match (lhs_value, rhs_value) {
                      (Value::Number(x), Value::Number(y)) => {
                        match x.add(y) {
                          Ok(result) => {
                            let function_result = Value::from_quantity(result);
                            unsafe {
                              (*out_table).set_unchecked(o1, o2, function_result);
                            }
                          }
                          Err(_) => (), // TODO Handle error here
                        }
                      }
                      _ => (),
                    }
                  }
                  _ => (),
                }
                if i >= lhs_table.rows {
                  break;
                }
                i += 1;
              }
            }
            // table/range
            0x285A4EFBFCDC2EF4 => {
              // TODO test argument count is 2 or 3
              // 2 -> start, end
              // 3 -> start, increment, end
              let (start_table_id, start_rows, start_columns) = &arguments[0];
              let (end_table_id, end_rows, end_columns) = &arguments[1];
              let (out_table_id, out_rows, out_columns) = out;
              let db = database.borrow_mut();
              let start_table = match start_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap(),
                TableId::Local(id) => self.tables.get(id).unwrap(),
              };
              let end_table = match end_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap(),
                TableId::Local(id) => self.tables.get(id).unwrap(),
              };
              let start_value = start_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
              let end_value = end_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
              let range = end_value.as_u64().unwrap() - start_value.as_u64().unwrap();
              match out_table_id {
                TableId::Local(id) => {
                  let mut out_table = self.tables.get_mut(id).unwrap();
                  for i in 1..=range as usize {
                    out_table.set(&Index::Index(i), &Index::Index(1), Value::from_u64(i as u64));
                  }
                }
                TableId::Global(id) => {

                }
              }
            }
            // table/horizontal-concatenate
            0x1C6A44C6BAFC67F1 => {
              let (out_table_id, out_rows, out_columns) = out;
              let mut db = database.borrow_mut();
              let mut column = 0;
              let mut out_rows = 0;
              // First pass, make sure the dimensions work out
              for (table_id, rows, columns) in arguments {
                let table = match table_id {
                  TableId::Global(id) => db.tables.get(id).unwrap(),
                  TableId::Local(id) => self.tables.get(id).unwrap(),
                };
                if out_rows == 0 {
                  out_rows = table.rows;
                } else if table.rows != 1 && out_rows != table.rows {
                  // TODO Throw an error here
                } else if table.rows > out_rows && out_rows == 1 {
                  out_rows = table.rows
                }
              }
              let mut out_table = match out_table_id {
                TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
                TableId::Local(id) => self.tables.get_mut(id).unwrap() as *mut Table,
              };
              for (table_id, rows, columns) in arguments {
                let table = match table_id {
                  TableId::Global(id) => db.tables.get(id).unwrap(),
                  TableId::Local(id) => self.tables.get(id).unwrap(),
                };
                let rows_iter = if table.rows == 1 {
                  IndexIterator::Constant(Index::Index(1))
                } else {
                  IndexIterator::Range(1..=table.rows)
                };
                for (i,k) in (1..=out_rows).zip(rows_iter) {
                  for j in 1..=table.columns {
                    let value = table.get(&k,&Index::Index(j)).unwrap();
                    unsafe {
                      (*out_table).set(&Index::Index(i), &Index::Index(column+j), value);
                    }
                  }
                }
                column += 1;
              }
            }
            _ => () // TODO Unknown function
          }
        }
        _ => (),
      }
    }
    self.state = BlockState::Done;
  }

  pub fn is_ready(&mut self) -> bool {
    if self.state == BlockState::Error {
      false
    } else {
      let set_diff: HashSet<u64> = self.input.difference(&self.ready).cloned().collect();
      // The block is ready if all input registers are ready i.e. the length of the set diff is 0
      if set_diff.len() == 0 {
        self.state = BlockState::Ready;
        true
      } else {
        false
      }
    }    
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌─────────────────────────────────────────────┐\n")?;
    write!(f, "│ id: {}\n", humanize(&self.id))?;
    write!(f, "│ state: {:?}\n", self.state)?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ ready: {}\n", self.ready.len())?;
    for (ix, input) in self.ready.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, humanize(input))?;
    }
    write!(f, "│ input: {} \n", self.input.len())?;
    for (ix, input) in self.input.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, humanize(input))?;
    }
    if self.ready.len() < self.input.len() {
      write!(f, "│ missing: \n")?;
      for (ix, missing) in self.input.difference(&self.ready).enumerate() {
        write!(f, "│    {}. {}\n", ix+1, humanize(missing))?;
      }
    }
    write!(f, "│ output: {}\n", self.output.len())?;
    for (ix, output) in self.output.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, humanize(output))?;
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ transformations: \n")?;
    for (ix, (text, tfms)) in self.transformations.iter().enumerate() {
      write!(f, "│  {}. {}\n", ix+1, text)?;
      for tfm in tfms {
        let tfm_string = format_transformation(&self,&tfm);
        write!(f, "│       > {}\n", tfm_string)?;
      }
    }
    write!(f, "│ plan: \n")?;
    for (ix, (_,tfm)) in self.plan.iter().enumerate() {
      let tfm_string = format_transformation(&self,tfm);
      write!(f, "│    {}. {}\n", ix+1, tfm_string)?;
    }
    write!(f, "│ tables: {} \n", self.tables.len())?;
    for (_, table) in self.tables.iter() {
      write!(f, "{:?}\n", table)?;
    }
    
    Ok(())
  }
}

fn format_transformation(block: &Block, tfm: &Transformation) -> String {
  match tfm {
    Transformation::NewTable{table_id, rows, columns} => {
      let mut tfm = format!("+ ");
      match table_id {
        TableId::Global(id) => tfm=format!("{}#{}",tfm,block.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.identifiers.get(id) {
            Some(name) =>  tfm=format!("{}{}",tfm,name),
            None => tfm=format!("{}0x{:x}",tfm,id),
          }
        }
      };
      tfm = format!("{} = ({} x {})",tfm,rows,columns);
      tfm
    }
    Transformation::Whenever{table_id, row, column} => {
      let mut arg = format!("~ ");
      arg=format!("{}#{}",arg,block.identifiers.get(&table_id).unwrap());
      match row {
        Index::All => arg=format!("{}{{:,",arg),
        Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
        Index::Alias(alias) => {
          let alias_name = block.identifiers.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match column {
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Alias(alias) => {
          let alias_name = block.identifiers.get(alias).unwrap();
          arg=format!("{}{}}}",arg,alias_name);
        },
      }
      arg      
    }
    Transformation::Constant{table_id, value, unit} => {
      format!("{:?} -> {:?}", value, table_id)
    }
    Transformation::Set{table_id, row, column, value} => {
      let mut tfm = format!("");
      match table_id {
        TableId::Global(id) => tfm = format!("{}#{}",tfm,block.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.identifiers.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}0x{:x}",tfm,id),
          }
        }
      }
      tfm = format!("{}{{{:?}, {:?}}} := {:?}", tfm, row.unwrap(), column.unwrap(), value);
      tfm      
    }
    Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
      let mut tfm = format!("");
      match table_id {
        TableId::Global(id) => tfm = format!("{}#{}",tfm,block.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.identifiers.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}0x{:x}",tfm,id),
          }
        }
      }
      tfm = format!("{}({:x})",tfm,column_ix);
      tfm = format!("{} -> {}",tfm,block.identifiers.get(column_alias).unwrap());
      tfm
    }
    Transformation::Function{name, arguments, out} => {
      let name_string = match block.identifiers.get(name) {
        Some(name_string) => name_string.clone(),
        None => format!("0x{:x}", name),
      };
      let mut arg = format!("");
      for (ix,(table, row, column)) in arguments.iter().enumerate() {
        match table {
          TableId::Global(id) => arg=format!("{}#{}",arg,block.identifiers.get(id).unwrap()),
          TableId::Local(id) => {
            match block.identifiers.get(id) {
              Some(name) => arg = format!("{}{}",arg,name),
              None => arg = format!("{}0x{:x}",arg,id),
            }
          }
        };
        match row {
          Index::All => arg=format!("{}{{:,",arg),
          Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
          Index::Alias(alias) => {
            let alias_name = block.identifiers.get(alias).unwrap();
            arg=format!("{}{{{},",arg,alias_name);
          },
        }
        match column {
          Index::All => arg=format!("{}:}}",arg),
          Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
          Index::Alias(alias) => {
            let alias_name = block.identifiers.get(alias).unwrap();
            arg=format!("{}{}}}",arg,alias_name);
          },
        }
        if ix < arguments.len()-1 {
          arg=format!("{}, ", arg);
        }
      }
      format!("{}({})",name_string,arg)
    },
    x => format!("{:?}", x),
  }
}

#[derive(Clone)]
pub enum TransformMap {
  All,
  Index(usize),
  Range((usize,usize,usize)),
  Mask(Vec<u8>),
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockState {
  New,          // Has just been created, but has not been tested for satisfaction
  Ready,        // All inputs are satisfied and the block is ready to execute
  Done,         // All inputs are satisfied and the block has executed
  Unsatisfied,  // One or more inputs are not satisfied
  Error,        // One or more errors exist on the block
  Disabled,     // The block is disabled will not execute if it otherwise would
}

pub enum Error {
  TableNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transformation {
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value, unit: u64},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  Set{table_id: TableId, row: Index, column: Index, value: Value},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: u64, row: Index, column: Index},
  Function{name: u64, arguments: Vec<(TableId, Index, Index)>, out: (TableId, Index, Index)},
  Scan,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Register {
  pub table_id: u64,
  pub row: Index,
  pub column: Index,
}

impl Register {
  pub fn hash(&self) -> u64 {
    let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
    hasher.write_u64(self.table_id);
    hasher.write_u64(self.row.unwrap() as u64);
    hasher.write_u64(self.column.unwrap() as u64);
    hasher.finish()
  }
}

pub enum IndexIterator {
  Range(std::ops::RangeInclusive<usize>),
  Constant(Index),
}

impl Iterator for IndexIterator {
  type Item = Index;
  
  fn next(&mut self) -> Option<Index> {
    match self {
      IndexIterator::Range(itr) => {
        match itr.next() {
          Some(ix) => Some(Index::Index(ix)),
          None => None,
        }
      }
      IndexIterator::Constant(itr) => Some(*itr),
    }
  }
}

pub fn humanize(hash: &u64) -> String {
  use std::mem::transmute;
  let bytes: [u8; 8] = unsafe { transmute(hash.to_be()) };
  let mut string = "".to_string();
  let mut ix = 0;
  for byte in bytes.iter() {
    string.push_str(&WORDLIST[*byte as usize]);
    if ix < 7 {
      string.push_str("-");
    }
    ix += 1;
  }
  string
}

pub const WORDLIST: &[&str;256] = &[
  "nil", "ama", "ine", "ska", "pha", "gel", "art", 
  "ona", "sas", "ist", "aus", "pen", "ust", "umn",
  "ado", "con", "loo", "man", "eer", "lin", "ium",
  "ack", "som", "lue", "ird", "avo", "dog", "ger",
  "ter", "nia", "bon", "nal", "ina", "pet", "cat",
  "ing", "lie", "ken", "fee", "ola", "old", "rad",
  "met", "cut", "azy", "cup", "ota", "dec", "del",
  "elt", "iet", "don", "ble", "ear", "rth", "eas", 
  "war", "eig", "tee", "ele", "emm", "ene", "qua",
  "fai", "fan", "fif", "fil", "fin", "fis", "fiv", 
  "flo", "for", "foo", "fou", "fot", "fox", "fre",
  "fri", "fru", "gee", "gia", "glu", "fol", "gre", 
  "ham", "hap", "har", "haw", "hel", "hig", "hot", 
  "hyd", "ida", "ill", "ind", "ini", "ink", "iwa",
  "and", "ite", "jer", "jig", "joh", "jul", "uly", 
  "kan", "ket", "kil", "kin", "kit", "lac", "lak", 
  "lem", "ard", "lim", "lio", "lit", "lon", "lou",
  "low", "mag", "nes", "mai", "gam", "arc", "mar",
  "mao", "mas", "may", "mex", "mic", "mik", "ril",
  "min", "mir", "mis", "mio", "mob", "moc", "ech",
  "moe", "tan", "oon", "ain", "mup", "sic", "neb",
  "une", "net", "nev", "nin", "een", "nit", "nor",
  "nov", "nut", "oct", "ohi", "okl", "one", "ora",
  "ges", "ore", "osc", "ove", "oxy", "pap", "par", 
  "pey", "pip", "piz", "plu", "pot", "pri", "pur",
  "que", "uqi", "qui", "red", "riv", "rob", "roi", 
  "rug", "sad", "sal", "sat", "sep", "sev", "eve",
  "sha", "sie", "sin", "sik", "six", "sit", "sky", 
  "soc", "sod", "sol", "sot", "tir", "ker", "spr",
  "sta", "ste", "mam", "mer", "swe", "tab", "tag", 
  "see", "nis", "tex", "thi", "the", "tim", "tri",
  "twe", "ent", "two", "unc", "ess", "uni", "ura", 
  "veg", "ven", "ver", "vic", "vid", "vio", "vir",
  "was", "est", "whi", "hit", "iam", "win", "his",
  "wis", "olf", "wyo", "ray", "ank", "yel", "zeb",
  "ulu", "fix", "gry", "hol", "jup", "lam", "pas",
  "rom", "sne", "ten", "uta"];