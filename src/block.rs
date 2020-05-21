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
      state: BlockState::New,
      tables: HashMap::new(),
      store: Rc::new(RefCell::new(Store::new(capacity))),
      transformations: Vec::new(),
      plan: Vec::new(),
      changes: Vec::new(),
    }
  }

  pub fn register_transformation(&mut self, tfm: Transformation) {
    match tfm {
      Transformation::Whenever{table_id, row, column} => {
        self.input.insert(Register{table_id, row, column}.hash());
      }
      Transformation::Function{..} => {
        self.plan.push(tfm.clone());
      }
      _ => (),
    }
    self.transformations.push(tfm);
  }

  pub fn solve(&mut self, database: Rc<RefCell<Database>>) {
    let mut changes = Vec::with_capacity(4000);
    'step_loop: for step in &self.plan {
      match step {
        Transformation::Function{name, lhs, rhs, out} => {
          let (lhs_table_id, lhs_rows, lhs_columns) = lhs;
          let (rhs_table_id, rhs_rows, rhs_columns) = rhs;
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
            match (lhs_table.get(&Index::Index(lrix), &Index::Index(lcix)), 
                   rhs_table.get(&Index::Index(rrix), &Index::Index(rcix))
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
    write!(f, "id: {:?}\n", self.id)?;
    write!(f, "state: {:?}\n", self.state)?;
    write!(f, "ready: \n")?;
    for input in self.ready.iter() {
      write!(f, "       {}\n", humanize(input))?;
    }
    write!(f, "input: \n")?;
    for input in self.input.iter() {
      write!(f, "       {}\n", humanize(input))?;
    }
    write!(f, "transformations: \n")?;
    for tfm in self.transformations.iter() {
      write!(f, "       {:?}\n", tfm)?;
    }
    write!(f, "plan: \n")?;
    for tfm in self.plan.iter() {
      write!(f, "       {:?}\n", tfm)?;
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
  Whenever{table_id: u64, row: Index, column: Index},
  Function{name: u64, lhs: (TableId, Index, Index), rhs: (TableId, Index, Index), out: (TableId, Index, Index)},
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
  "ack", "ama", "ine", "ska", "pha", "gel", "art", 
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
  "fri", "fru", "gee", "gia", "glu", "olf", "gre", 
  "ham", "hap", "har", "haw", "hel", "hig", "hot", 
  "hyd", "ida", "ill", "ind", "ini", "ink", "iwa",
  "and", "ite", "jer", "jig", "joh", "jul", "uly", 
  "kan", "ket", "kil", "kin", "kit", "lac", "lak", 
  "lem", "ard", "lim", "lio", "lit", "lon", "lou",
  "low", "mag", "nes", "mai", "mag", "arc", "mar",
  "mao", "mas", "may", "mex", "mic", "mik", "ril",
  "min", "mir", "mis", "mio", "mob", "moc", "ech",
  "moe", "tan", "oon", "ain", "mup", "sic", "neb",
  "une", "net", "nev", "nin", "een", "nit", "nor",
  "nov", "nut", "oct", "ohi", "okl", "one", "ora",
  "ges", "ore", "osc", "ove", "oxy", "pap", "par", 
  "pey", "pip", "piz", "plu", "pot", "pri", "pur",
  "que", "que", "qui", "red", "riv", "rob", "roi", 
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