use table::{Table, TableId, Index, Value, ValueMethods};
use database::{Database, Store, Change, Transaction};
use hashbrown::{HashMap, HashSet};
use quantities::{Quantity, QuantityMath, ToQuantity, make_quantity};
use operations::{MechFunction, resolve_subscript};
use errors::{ErrorType};
use std::cell::RefCell;
use std::rc::Rc;
use std::hash::Hasher;
use ahash::AHasher;
use rust_core::fmt;
use ::humanize;

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
  pub output_dependencies: HashSet<u64>,
  pub output_dependencies_ready: HashSet<u64>,
  pub tables: HashMap<u64, Table>,
  pub store: Rc<Store>,
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<(Vec<TransformMap>,Transformation)>,
  pub changes: Vec<Change>,
  pub errors: Vec<ErrorType>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      text: String::new(),
      name: String::new(),
      ready: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      output_dependencies: HashSet::new(),
      output_dependencies_ready: HashSet::new(),
      state: BlockState::New,
      tables: HashMap::new(),
      store: Rc::new(Store::new(capacity)),
      transformations: Vec::new(),
      plan: Vec::new(),
      changes: Vec::new(),
      errors: Vec::new(),
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
              let store = unsafe{&mut *Rc::get_mut_unchecked(&mut self.store)};
              store.column_index_to_alias.insert((*table_id.unwrap(),column_ix),column_alias);
              store.column_alias_to_index.insert((*table_id.unwrap(),column_alias),column_ix);
            }
          }
        }
        Transformation::Constant{table_id, value, unit} => {
          let (domain, scale) = match unit {
            unit_value => match unit_value {
              0x01b779d3bf451717 => (1, 0), // g
              0xc8df0fac549c1104 => (1, 3), // kg
//              "m" => (2, 0),
//              "km" => (2, 3),
//              "ms" => (3, 0),
//              "s" => (3, 3),
              _ => (0, 0),
            },
            _ => (0, 0),
          };
          let q = if value.is_number() {
            Value::from_quantity(make_quantity(value.mantissa(), value.range() + scale, domain))
          } else {
            value
          };
          match table_id {
            TableId::Local(id) => {
              let mut table = self.tables.get_mut(&id).unwrap();
              table.set(&Index::Index(1), &Index::Index(1), q);
            }
            TableId::Global(id) => {
              self.changes.push(
                Change::Set{
                  table_id: id,
                  values: vec![(Index::Index(1), Index::Index(1), q)],
                }
              );
            }
            _ => (),
          }
        }
        Transformation::Set{table_id, row, column} => {
          let hash = Register{table_id: *table_id.unwrap(), row: Index::All, column}.hash();
          self.output.insert(hash);       
          self.output_dependencies.insert(hash);          
        }
        Transformation::Whenever{table_id, row, column} => {
          self.input.insert(Register{table_id, row, column}.hash());
        }
        Transformation::Function{name, ref arguments, out} => {
          let (out_id, row, column) = out;
          match out_id {
            TableId::Global(id) => {self.output.insert(Register{table_id: id, row, column}.hash());},
            _ => (),
          }
          for (_, table_id, row, column) in arguments {
            match table_id {
              TableId::Global(id) => {
                let rrow = match row {
                  Index::Table{..} => Index::All,
                  x => Index::All,
                };
                let register_hash = Register{table_id: *id, row: rrow, column: *column}.hash();
                self.input.insert(register_hash);
              },
              _ => (),
            }
          }
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

  pub fn solve(&mut self, database: Rc<RefCell<Database>>, functions: &HashMap<u64, Option<MechFunction>>) {
    'step_loop: for (masks, step) in &self.plan {
      match step {
        Transformation::Whenever{table_id, row, column} => {
          let register = Register{table_id: *table_id, row: *row, column: *column};
          self.ready.remove(&register.hash());
        },
        Transformation::Function{name, arguments, out} => {
          match functions.get(name) {
            Some(Some(mech_fn)) => {
              let mut vis = vec![];
              for (arg, table, row, column) in arguments {
                let vi = resolve_subscript(*table,*row,*column,&mut self.tables, &database);
                vis.push((arg.clone(),vi));
              }
              let (out_table,out_row,out_col) = out;
              let mut out_vi = resolve_subscript(*out_table,*out_row,*out_col,&mut self.tables, &database);
              mech_fn(&vis, &mut out_vi);
            }
            _ => {
              ()
            },// TODO Error: Function not found
          }
        }
        _ => (),
      }
    }
    self.state = BlockState::Done;
  }

  pub fn is_ready(&mut self) -> bool {
    if self.state == BlockState::Error || self.state == BlockState::Disabled {
      false
    } else if self.errors.len() > 0 {
      self.state = BlockState::Error;
      false
    } else {
      let set_diff: HashSet<u64> = self.input.difference(&self.ready).cloned().collect();
      let out_diff: HashSet<u64> = self.output_dependencies.difference(&self.output_dependencies_ready).cloned().collect();
      // The block is ready if all input registers are ready i.e. the length of the set diff is 0
      if set_diff.len() == 0 && out_diff.len() == 0 {
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
    write!(f, "│ errors: {}\n", self.errors.len())?;
    for (ix, error) in self.errors.iter().enumerate() {
      write!(f, "│    {}. {:?}\n", ix+1, error)?;
    }
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
    write!(f, "│ output dep: {}\n", self.output_dependencies.len())?;
    for (ix, output) in self.output_dependencies.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, humanize(output))?;
    }
    write!(f, "│ output ready: {}\n", self.output_dependencies_ready.len())?;
    for (ix, output) in self.output_dependencies_ready.iter().enumerate() {
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
        TableId::Global(id) => tfm=format!("{}#{}",tfm,block.store.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.identifiers.get(id) {
            Some(name) =>  tfm=format!("{}{}",tfm,name),
            None => tfm=format!("{}{}",tfm,humanize(id)),
          }
        }
      };
      tfm = format!("{} = ({} x {})",tfm,rows,columns);
      tfm
    }
    Transformation::Whenever{table_id, row, column} => {
      let mut arg = format!("~ ");
      arg=format!("{}#{}",arg,block.store.identifiers.get(&table_id).unwrap());
      match row {
        Index::None => arg=format!("{}{{-,",arg),
        Index::All => arg=format!("{}{{:,",arg),
        Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.identifiers.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.identifiers.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match column {
        Index::None => arg=format!("{}-}}",arg),
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.identifiers.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.identifiers.get(alias).unwrap();
          arg=format!("{}{}}}",arg,alias_name);
        },
      }
      arg      
    }
    Transformation::Constant{table_id, value, unit} => {
      let mut tfm = format!("");
      match value.as_quantity() {
        Some(quantity) => tfm = format!("{}{:?} -> ", tfm, value),
        None => {
          if value.is_empty() {
            tfm = format!("{} _ -> ",tfm);
          } else {
            tfm = format!("{}{:?} -> ",tfm, block.store.identifiers.get(value).unwrap());
          }
        },
      }
      
      match table_id {
        TableId::Global(id) => tfm=format!("{}#{}",tfm,block.store.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.identifiers.get(id) {
            Some(name) =>  tfm=format!("{}{}",tfm,name),
            None => tfm=format!("{}{}",tfm,humanize(id)),
          }
        }
      };
      tfm
    }
    Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
      let mut tfm = format!("");
      match table_id {
        TableId::Global(id) => tfm = format!("{}#{}",tfm,block.store.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.identifiers.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}{}",tfm,humanize(id)),
          }
        }
      }
      tfm = format!("{}({:x})",tfm,column_ix);
      tfm = format!("{} -> {}",tfm,block.store.identifiers.get(column_alias).unwrap());
      tfm
    }
    Transformation::Function{name, arguments, out} => {
      let name_string = match block.store.identifiers.get(name) {
        Some(name_string) => name_string.clone(),
        None => format!("{}", humanize(name)),
      };
      let mut arg = format!("");
      for (ix,(arg_id, table, row, column)) in arguments.iter().enumerate() {
        match table {
          TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
          TableId::Local(id) => {
            match block.store.identifiers.get(id) {
              Some(name) => arg = format!("{}{}",arg,name),
              None => arg = format!("{}{}",arg,humanize(id)),
            }
          }
        };
        match row {
          Index::None => arg=format!("{}{{-,",arg),
          Index::All => arg=format!("{}{{:,",arg),
          Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
          Index::Table(table) => {
            match table {
              TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.identifiers.get(id) {
                  Some(name) => {
                    arg = format!("{}{{{},",arg,name);
                  },
                  None => arg = format!("{}{{{},",arg,humanize(id)),
                }
              }
            };
          }
          Index::Alias(alias) => {
            let alias_name = block.store.identifiers.get(alias).unwrap();
            arg=format!("{}{{{},",arg,alias_name);
          },
        }
        match column {
          Index::None => arg=format!("{}-}}",arg),
          Index::All => arg=format!("{}:}}",arg),
          Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
          Index::Table(table) => {
            match table {
              TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.identifiers.get(id) {
                  Some(name) => arg = format!("{}{}",arg,name),
                  None => arg = format!("{}{}",arg,humanize(id)),
                }
              }
            };
          }
          Index::Alias(alias) => {
            let alias_name = block.store.identifiers.get(alias).unwrap();
            arg=format!("{}.{}}}",arg,alias_name);
          },
        }
        if ix < arguments.len()-1 {
          arg=format!("{}, ", arg);
        }
      }
      let mut arg = format!("{}({}) -> ",name_string,arg);
      let (out_table, out_row, out_column) = out;
      match out_table {
        TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.identifiers.get(id) {
            Some(name) => arg = format!("{}{}",arg,name),
            None => arg = format!("{}{}",arg,humanize(id)),
          }
        }
      };
      match out_row {
        Index::None => arg=format!("{}{{-,",arg),
        Index::All => arg=format!("{}{{:,",arg),
        Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}{{#{},",arg,block.store.identifiers.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.identifiers.get(id) {
                Some(name) => arg = format!("{}{{{},",arg,name),
                None => arg = format!("{}{{{},",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.identifiers.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match out_column {
        Index::None => arg=format!("{}-}}",arg),
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.identifiers.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.identifiers.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.identifiers.get(alias).unwrap();
          arg=format!("{}.{}}}",arg,alias_name);
        },
      }
      arg      
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
  Set{table_id: TableId, row: Index, column: Index},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: u64, row: Index, column: Index},
  Function{name: u64, arguments: Vec<(u64, TableId, Index, Index)>, out: (TableId, Index, Index)},
  Select{table_id: TableId, row: Index, column: Index},
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

#[derive(Debug)]
pub struct  ValueIterator {
  pub table: *mut Table,
  pub row_index: Index,
  pub column_index: Index,
  pub row_iter: IndexIterator,
  pub column_iter: IndexIterator,
}

impl ValueIterator {
  
  pub fn rows(&self) -> usize {
    unsafe{ (*self.table).rows }
  }

  pub fn columns(&self) -> usize {
    unsafe{ (*self.table).columns }
  }

  pub fn get(&self, row: &Index, column: &Index) -> Option<Value> {
    unsafe{(*self.table).get(row,column)}
  }

  pub fn set(&self, row: &Index, column: &Index, value: Value) {
    unsafe{(*self.table).set(row, column, value)};
  }

  pub fn next_address(&mut self) -> Option<(usize, usize)> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        Some((rix.unwrap(),cix.unwrap()))
      },     
      _ => None,
    }
  }

}

impl Iterator for ValueIterator {
  type Item = Value;
  fn next(&mut self) -> Option<Value> {
    match (self.row_iter.next(), self.column_iter.next()) {
      (Some(rix), Some(cix)) => {
        let value = unsafe{ (*self.table).get_unchecked(rix.unwrap(),cix.unwrap()) };
        Some(value)
      },     
      _ => None,
    }
  }

}

#[derive(Debug)]
pub struct IndexRepeater {
  iterator: std::iter::Cycle<IndexIterator>,
  width: usize,
  current: Index,
  counter: usize,
}

impl IndexRepeater {

  pub fn new(iterator: IndexIterator, width: usize) -> IndexRepeater {
    IndexRepeater {
      iterator: iterator.cycle(),
      width,
      current: Index::None,
      counter: 0,
    }
  }

  pub fn next(&mut self) -> Option<Index> {
    if self.current == Index::None {
      self.current = self.iterator.next().unwrap();
    }
    if self.counter == self.width {
      self.counter = 0;
      self.current = self.iterator.next().unwrap();
    }
    self.counter += 1;
    Some(self.current)
  }

}

#[derive(Debug, Clone)]
pub struct TableIterator {
  table: *mut Table,
  current: usize,
}

impl TableIterator {

  pub fn new(table: *mut Table) -> TableIterator {
    TableIterator {
      table,
      current: 0,
    }
  }

  pub fn len(&self) -> usize {
    let mut len = 0;
    unsafe{
      let max = (*self.table).data.len();
      for ix in 0..max {
        let val = (*self.table).get_unchecked_linear(ix);
        if val.as_bool() == Some(true) || val.is_number() {
          len += 1;
        }
      }
    }
    len
  }

}

impl Iterator for TableIterator {
  type Item = Index;
  fn next(&mut self) -> Option<Index> {
    unsafe{
      let mut next = None;
      loop {
        if self.current < (*self.table).data.len() {
          let address = (*self.table).data[self.current];
          self.current += 1;
          let value = (*self.table).store.data[address];
          match value.as_u64() {
            Some(v) => {
              next = Some(Index::Index(v as usize));
              break;
            },
            None => match value.as_bool() {
              Some(true) => {
                next = Some(Index::Index(self.current));
                break;
              },
              _ => continue,
            }
          }
        } else {
          next = None;
          break;
        }
      }
      next
    }
  }
}

#[derive(Debug, Clone)]
pub struct AliasIterator {
  alias: u64,
  table_id: TableId,
  store: Rc<Store>,
  index: Option<Index>,
}

impl AliasIterator {

  pub fn new(alias: u64, table_id: TableId, store: Rc<Store>) -> AliasIterator {
    AliasIterator {
      alias,
      table_id,
      store,
      index: None,
    }
  }

}

impl Iterator for AliasIterator {
  type Item = Index;
  
  fn next(&mut self) -> Option<Index> {
    match self.index {
      None => {
        let store = unsafe{&mut *Rc::get_mut_unchecked(&mut self.store)};
        match store.column_alias_to_index.get(&(*self.table_id.unwrap(), self.alias)) {
          Some(ix) => {
            self.index = Some(Index::Index(*ix));
            self.index
          },
          None => None,
        }
      },
      Some(index) => self.index
    }
  }
}

#[derive(Debug, Clone)]
pub enum IndexIterator {
  Range(std::ops::RangeInclusive<usize>),
  Constant(Index),
  Alias(AliasIterator),
  Table(TableIterator),
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
      IndexIterator::Table(itr) => itr.next(),
      IndexIterator::Alias(itr) => itr.next(),
    }
  }
}