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
pub struct Block {
  pub id: u64,
  pub state: BlockState,
  pub ready: HashSet<u64>,
  pub input: HashSet<u64>,
  pub output: HashSet<u64>,
  pub tables: HashMap<u64, Rc<RefCell<Table>>>,
  pub store: Rc<RefCell<Store>>,
  pub transformations: Vec<Transformation>,
  pub plan: Vec<Transformation>,
  pub changes: Vec<Change>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      ready: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      state: BlockState::New,
      tables: HashMap::new(),
      store: Rc::new(RefCell::new(Store::new(capacity))),
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

  pub fn register_transformation(&mut self, tfm: Transformation) {
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
            self.output.insert(id);
          }
          TableId::Local(id) => {
            self.tables.insert(id, Rc::new(RefCell::new(Table::new(id, rows, columns, self.store.clone()))));
          }
        }
      }
      Transformation::Constant{table_id, ref value} => {
        match table_id {
          TableId::Local(id) => {
            let mut table = self.tables.get(&id).unwrap().borrow_mut();
            table.set(&Index::Index(1), &Index::Index(1), &value);
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
      }
      Transformation::Function{name, ref arguments, out} => {
        let (out_id, _, _) = out;
        match out_id {
          TableId::Global(id) => {self.output.insert(id);},
          _ => (),
        }
        for (table_id, _, _) in arguments {
          match table_id {
            TableId::Global(id) => {self.input.insert(*id);},
            _ => (),
          }
        }
        self.plan.push(tfm.clone());
      }
      _ => (),
    }
    self.transformations.push(tfm);
  }

  pub fn solve(&mut self, database: Rc<RefCell<Database>>) {
    let mut changes = Vec::with_capacity(4000);
    changes.append(&mut self.changes);
    self.changes.clear();
    'step_loop: for step in &self.plan {
      match step {
        Transformation::Function{name, arguments, out} => {
          match name {
            // math/add
            0x13166E07A8EF9CC3 => {
              // TODO test argument count is 2
              let (lhs_table_id, lhs_rows, lhs_columns) = &arguments[0];
              let (rhs_table_id, rhs_rows, rhs_columns) = &arguments[1];
              let (out_table_id, out_rows, out_columns) = out;
              let db = database.borrow_mut();
              let lhs_table = match lhs_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap().borrow(),
                TableId::Local(id) => self.tables.get(id).unwrap().borrow(),
              };
              let rhs_table = match rhs_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap().borrow(),
                TableId::Local(id) => self.tables.get(id).unwrap().borrow(),
              };
              let store = &db.store.borrow();

              // Figure out dimensions
              let equal_dimensions = if lhs_table.rows == rhs_table.rows
              { true } else { false };
              let lhs_scalar = if lhs_table.rows == 1 && lhs_table.columns == 1 
              { true } else { false };
              let rhs_scalar = if rhs_table.rows == 1 && rhs_table.columns == 1
              { true } else { false };

              let iterator_zip = if equal_dimensions {
                IndexIteratorZip::new(
                  IndexIterator::Range(1..=lhs_table.rows),
                  IndexIterator::Constant(std::iter::repeat(*lhs_columns.unwrap())),
                  IndexIterator::Range(1..=rhs_table.rows),
                  IndexIterator::Constant(std::iter::repeat(*rhs_columns.unwrap())),
                )
              } else if rhs_scalar {
                IndexIteratorZip::new(
                  IndexIterator::Range(1..=lhs_table.rows),
                  IndexIterator::Constant(std::iter::repeat(*lhs_columns.unwrap())),
                  IndexIterator::Constant(std::iter::repeat(1)),
                  IndexIterator::Constant(std::iter::repeat(1)),
                )
              } else {
                IndexIteratorZip::new(
                  IndexIterator::Constant(std::iter::repeat(1)),
                  IndexIterator::Constant(std::iter::repeat(1)),
                  IndexIterator::Range(1..=rhs_table.rows),
                  IndexIterator::Constant(std::iter::repeat(*rhs_columns.unwrap())),
                )
              };

              let mut function_result = Value::from_u64(0);
              let mut values = Vec::with_capacity(lhs_table.rows);
              for (lrix, lcix, rrix, rcix) in iterator_zip {
                match (lhs_table.get_address(&Index::Index(lrix), &Index::Index(lcix)), 
                      rhs_table.get_address(&Index::Index(rrix), &Index::Index(rcix))
                      ) 
                {
                  (Some(lhs_ix), Some(rhs_ix)) => {
                    let lhs_value = &store.data[lhs_ix];
                    let rhs_value = &store.data[rhs_ix];
                    match (lhs_value, rhs_value) {
                      (Value::Number(x), Value::Number(y)) => {
                        match x.add(*y) {
                          Ok(result) => {
                            function_result = Value::from_quantity(result);
                          }
                          Err(_) => (), // TODO Handle error here
                        }
                      }
                      _ => (),
                    }
                  }
                  _ => (),
                }
                values.push((Index::Index(lrix), *out_columns, function_result.clone()));
              }
              changes.push(Change::Set{
                table_id: *out_table_id.unwrap(),
                values,
              });

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
                TableId::Global(id) => db.tables.get(id).unwrap().borrow(),
                TableId::Local(id) => self.tables.get(id).unwrap().borrow(),
              };
              let end_table = match end_table_id {
                TableId::Global(id) => db.tables.get(id).unwrap().borrow(),
                TableId::Local(id) => self.tables.get(id).unwrap().borrow(),
              };
              let start_value = start_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
              let end_value = end_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
              let range = end_value.as_u64().unwrap() - start_value.as_u64().unwrap();
              match out_table_id {
                TableId::Local(id) => {
                  let mut out_table = self.tables.get(id).unwrap().borrow_mut();
                  for i in 1..=range as usize {
                    out_table.set(&Index::Index(i), &Index::Index(1), &Value::from_u64(i as u64));
                  }
                }
                TableId::Global(id) => {

                }
              }
            }
            _ => () // TODO Unknown function
          }
        }
        _ => (),
      }
    }
    let txn = Transaction{
      changes,
    };
    database.borrow_mut().process_transaction(&txn);
    database.borrow_mut().transactions.push(txn);
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
    write!(f, "id: {}\n", humanize(&self.id))?;
    write!(f, "state: {:?}\n", self.state)?;
    write!(f, "ready: \n")?;
    for input in self.ready.iter() {
      write!(f, "   {}\n", humanize(input))?;
    }
    write!(f, "input: \n")?;
    for input in self.input.iter() {
      write!(f, "   {}\n", humanize(input))?;
    }
    write!(f, "output: \n")?;
    for output in self.output.iter() {
      write!(f, "   {}\n", humanize(output))?;
    }
    write!(f, "transformations: \n")?;
    for tfm in self.transformations.iter() {
      write!(f, "   {:?}\n", tfm)?;
    }
    write!(f, "plan: \n")?;
    for tfm in self.plan.iter() {
      write!(f, "   {:?}\n", tfm)?;
    }
    write!(f, "tables: \n")?;
    for (_, table) in self.tables.iter() {
      write!(f, "{:?}\n", table.borrow())?;
    }
    
    Ok(())
  }
}

#[derive(Debug, PartialEq, Eq)]
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
  Constant{table_id: TableId, value: Value},
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
    hasher.write_u64(*self.row.unwrap() as u64);
    hasher.write_u64(*self.column.unwrap() as u64);
    hasher.finish()
  }
}

pub enum IndexIterator {
  Range(std::ops::RangeInclusive<usize>),
  Constant(std::iter::Repeat<usize>),
}

impl Iterator for IndexIterator {
  type Item = usize;
  
  fn next(&mut self) -> Option<usize> {
    match self {
      IndexIterator::Range(itr) => itr.next(),
      IndexIterator::Constant(itr) => itr.next(),
    }
  }
}

pub struct IndexIteratorZip {
  lhs_row: IndexIterator,
  lhs_col: IndexIterator,
  rhs_row: IndexIterator, 
  rhs_col: IndexIterator,
}

impl IndexIteratorZip {
  pub fn new(
    lhs_row: IndexIterator,
    lhs_col: IndexIterator,
    rhs_row: IndexIterator, 
    rhs_col: IndexIterator
  ) -> IndexIteratorZip {
    IndexIteratorZip {
      lhs_row,
      lhs_col,
      rhs_row,
      rhs_col,      
    }
  }
}

impl Iterator for IndexIteratorZip {
  type Item = (usize,usize,usize,usize);
  
  fn next(&mut self) -> Option<(usize,usize,usize,usize)> {
    match (self.lhs_row.next(), self.lhs_col.next(), self.rhs_row.next(), self.rhs_col.next()) {
      (Some(a),Some(b),Some(c),Some(d)) => Some((a,b,c,d)),
      (None,_,_,_) |
      (_,None,_,_) |
      (_,_,None,_) |
      (_,_,_,None) => None,
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