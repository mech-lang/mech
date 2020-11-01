use table::{Table, TableId, Index, Value, ValueMethods};
use database::{Database, Store, Change, Transaction};
use hashbrown::{HashMap, HashSet};
use quantities::{Quantity, QuantityMath, ToQuantity, make_quantity};
use operations::{MechFunction, resolve_subscript};
use errors::{ErrorType};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::hash::Hasher;
use ahash::AHasher;
use rust_core::fmt;
use ::humanize;
use ::hash_string;

lazy_static! {
  static ref TABLE_SPLIT: u64 = hash_string("table/split");
  static ref GRAMS: u64 = hash_string("g");
  static ref KILOGRAMS: u64 = hash_string("kg");
}

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
  pub register_map: HashMap<u64, Register>,
  pub ready: HashSet<u64>,
  pub input: HashSet<u64>,
  pub output: HashSet<u64>,
  pub output_dependencies: HashSet<u64>,
  pub output_dependencies_ready: HashSet<u64>,
  pub tables: HashMap<u64, Table>,
  pub store: Arc<Store>,
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<Transformation>,
  pub changes: Vec<Change>,
  pub errors: Vec<ErrorType>,
  pub triggered: usize,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      text: String::new(),
      name: String::new(),
      register_map: HashMap::new(),
      ready: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      output_dependencies: HashSet::new(),
      output_dependencies_ready: HashSet::new(),
      state: BlockState::New,
      tables: HashMap::new(),
      store: Arc::new(Store::new(capacity)),
      transformations: Vec::new(),
      plan: Vec::new(),
      changes: Vec::new(),
      errors: Vec::new(),
      triggered: 0,
    }
  }

  pub fn gen_id(&mut self) {
    let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
    for tfm in &self.transformations {
      hasher.write(format!("{:?}", tfm).as_bytes());
    }
    self.id = hasher.finish() & 0x00FFFFFFFFFFFFFF;  
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
                let register = Register{table_id, row: Index::All, column: Index::Index(i)};
                self.output.insert(register.hash());
                self.register_map.insert(register.hash(), register);
              }
              let register = Register{table_id, row: Index::All, column: Index::All};
              self.output.insert(register.hash());
              self.register_map.insert(register.hash(), register);
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
              let register = Register{table_id: table_id, row: Index::All, column: Index::Alias(column_alias)};
              self.output.insert(register.hash());
              self.register_map.insert(register.hash(), register);
            }
            TableId::Local(id) => {
              let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
              store.column_index_to_alias.insert((*table_id.unwrap(),column_ix),column_alias);
              store.column_alias_to_index.insert((*table_id.unwrap(),column_alias),column_ix);
            }
          }
        }
        Transformation::Constant{table_id, value, unit} => {
          let (domain, scale) = if unit == *GRAMS { (1, 0) }
            else if unit            == *KILOGRAMS { (1, 3) }
//              "m" => (2, 0),
//              "km" => (2, 3),
//              "ms" => (3, 0),
//              "s" => (3, 3),
              else { (0, 0) };
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
          let register = Register{table_id: table_id, row: Index::All, column};
          let register2 = Register{table_id: table_id, row: Index::All, column: Index::All};
          self.output.insert(register.hash());       
          self.output.insert(register2.hash());       
          self.output_dependencies.insert(register.hash());          
          self.register_map.insert(register.hash(), register);
          self.register_map.insert(register2.hash(), register2);
        }
        Transformation::Whenever{table_id, row, column, registers} => {
          match table_id {
            TableId::Global(id) => {
              for register in registers {
                self.input.insert(register);
              }
            }
            _ => (),
          }
        }
        Transformation::Function{name, ref arguments, out} => {
          let (out_id, row, column) = out;
          match out_id {
            TableId::Global(id) => {self.output.insert(Register{table_id: out_id, row, column}.hash());},
            _ => (),
          }
          for (_, table_id, row, column) in arguments {
            match table_id {
              TableId::Global(id) => {
                let rrow = match row {
                  Index::Table{..} => Index::All,
                  x => Index::All,
                };
                let register = Register{table_id: *table_id, row: rrow, column: *column};
                self.input.insert(register.hash());
                self.register_map.insert(register.hash(), register);
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
  pub fn process_changes(&mut self, database: Arc<RefCell<Database>>) {
    if !self.changes.is_empty() {
      let txn = Transaction {
        changes: self.changes.clone(),
      };
      self.changes.clear();
      database.borrow_mut().process_transaction(&txn);
      database.borrow_mut().transactions.push(txn);        
    }
  }

  pub fn solve(&mut self, database: Arc<RefCell<Database>>, functions: &HashMap<u64, Option<MechFunction>>) {
    self.triggered += 1;
    'step_loop: for step in &self.plan {
      match step {
        Transformation::Whenever{table_id, row, column, registers} => {
          match table_id {
            TableId::Global(id) => {
              for register in registers {
                self.ready.remove(&register);
              }             
            }
            TableId::Local(id) => {
              let mut flag = false;
              let table = self.tables.get_mut(&id).unwrap() as *mut Table;
              unsafe {
                for i in 1..=(*table).rows {
                  for j in 1..=(*table).columns {
                    let value = (*table).get_unchecked(i,j);
                    match value.as_bool() {
                      Some(true) => flag = true,
                      _ => (),
                    } 
                  }                  
                }
              }
              if flag == false {
                break 'step_loop;
              } else { 
                for register in registers {
                  self.ready.remove(&register);
                }
              }
            },
          }
        },
        Transformation::Function{name, arguments, out} => {
          let mut vis = vec![];
          for (arg, table, row, column) in arguments {
            let vi = resolve_subscript(*table,*row,*column,&mut self.tables, &database);
            vis.push((arg.clone(),vi));
          }
          let (out_table,out_row,out_col) = out;
          let mut out_vi = resolve_subscript(*out_table,*out_row,*out_col,&mut self.tables, &database);
          match functions.get(name) {
            Some(Some(mech_fn)) => {
              mech_fn(&vis, &mut out_vi);
            }
            _ => {
              if *name == *TABLE_SPLIT {
                let (_, vi) = &vis[0];
                let vi_table = unsafe{&(*vi.table)};
                                
                unsafe{
                  (*out_vi.table).rows = vi.rows();
                  (*out_vi.table).columns = 1;
                  (*out_vi.table).data.resize(vi.rows(), 0);
                }
                for row in vi.row_iter.clone() {
                  let old_table_id = unsafe{(*vi.table).id};
                  let new_table_id = hash_string(&format!("{:?}{:?}",old_table_id,row));
                  let columns = vi.columns().clone();
                  let mut table = Table::new(new_table_id,1,columns,self.store.clone());
                  for column in vi.column_iter.clone() {
                    let value = vi.get(&row,&column).unwrap();
                    table.set(&Index::Index(1),&column, value);
                  }
                  self.tables.insert(new_table_id, table);   
                  unsafe {
                    (*out_vi.table).set(&row,&Index::Index(1),Value::from_id(new_table_id));
                  }
                  let txn = Transaction {
                    changes: vec![Change::NewTable{
                      table_id: new_table_id,
                      rows: 1,
                      columns: vi.columns(),
                    }],
                  };
                  self.changes.clear();
                  let mut db = database.borrow_mut();
                  db.process_transaction(&txn);
                  db.transactions.push(txn); 
                  let new_global_copy_table = db.tables.get_mut(&new_table_id).unwrap() as *mut Table;               
                  unsafe {
                    for i in 1..=vi.columns() {
                      // Add alias to column if it's there
                      match vi_table.store.column_index_to_alias.get(&(vi_table.id,i)) {
                        Some(alias) => {
                          let out_id = (*new_global_copy_table).id;
                          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut (*new_global_copy_table).store)};
                          store.column_index_to_alias.entry((out_id,i)).or_insert(*alias);
                          store.column_alias_to_index.entry((out_id,*alias)).or_insert(i);
                        }
                        _ => (),
                      }
                      (*new_global_copy_table).set_unchecked(1,i, vi_table.get_unchecked(row.unwrap(),i));
                    }
                  }
                }
              } else {
                // TODO Error: Function not found
                //println!("Function not found {:?}", humanize(name));
                return;
              }
            },
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
    write!(f, "│ triggered: {:?}\n", self.triggered)?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ errors: {}\n", self.errors.len())?;
    for (ix, error) in self.errors.iter().enumerate() {
      write!(f, "│    {}. {:?}\n", ix+1, error)?;
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ ready: {}\n", self.ready.len())?;
    for (ix, register) in self.ready.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
    }
    write!(f, "│ input: {} \n", self.input.len())?;
    for (ix, register) in self.input.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
    }
    if self.ready.len() < self.input.len() {
      write!(f, "│ missing: \n")?;
      for (ix, register) in self.input.difference(&self.ready).enumerate() {
        write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
      }
    }
    write!(f, "│ output: {}\n", self.output.len())?;
    for (ix, register) in self.output.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
    }
    write!(f, "│ output dep: {}\n", self.output_dependencies.len())?;
    for (ix, register) in self.output_dependencies.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
    }
    write!(f, "│ output ready: {}\n", self.output_dependencies_ready.len())?;
    for (ix, register) in self.output_dependencies_ready.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self, register))?;
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
    for (ix, tfm) in self.plan.iter().enumerate() {
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

fn format_register(block: &Block, register_id: &u64) -> String {
  match block.register_map.get(register_id) {
    Some(register) => {
      let table_id = register.table_id;
      let row = register.row;
      let column = register.column;
      let mut arg = format!("");
      match table_id {
        TableId::Global(id) => {
          let name = match block.store.strings.get(&id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(&id)),
          };
          arg=format!("{}#{}",arg,name)
        },
        TableId::Local(id) => {
          match block.store.strings.get(&id) {
            Some(name) => arg = format!("{}{}",arg,name),
            None => arg = format!("{}{}",arg,humanize(&id)),
          }
        }
      };
      match row {
        Index::None => arg=format!("{}{{-,",arg),
        Index::All => arg=format!("{}{{:,",arg),
        Index::Index(ix) => arg=format!("{}{{{},",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(&id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(&id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(&id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(&alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match column {
        Index::None => arg=format!("{}-}}",arg),
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(&id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(&id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(&id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(&alias).unwrap();
          arg=format!("{}{}}}",arg,alias_name);
        },
      }
      arg
    }
    None => { humanize(register_id) }  
  }
}

fn format_transformation(block: &Block, tfm: &Transformation) -> String {
  match tfm {
    Transformation::NewTable{table_id, rows, columns} => {
      let mut tfm = format!("+ ");
      match table_id {
        TableId::Global(id) => {
          let name = match block.store.strings.get(id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(id)),
          };
          tfm=format!("{}#{}",tfm,name);
        }
        TableId::Local(id) => {
          match block.store.strings.get(id) {
            Some(name) =>  tfm=format!("{}{}",tfm,name),
            None => tfm=format!("{}{}",tfm,humanize(id)),
          }
        }
      };
      tfm = format!("{} = ({} x {})",tfm,rows,columns);
      tfm
    }
    Transformation::Whenever{table_id, row, column, registers} => {
      let mut arg = format!("~ ");
      match table_id {
        TableId::Global(id) => {
          let name = match block.store.strings.get(id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(id)),
          };
          arg=format!("{}#{}",arg,name)
        },
        TableId::Local(id) => {
          match block.store.strings.get(id) {
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
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match column {
        Index::None => arg=format!("{}-}}",arg),
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
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
            match value.as_reference() {
              Some(reference) => {tfm = format!("{}@{} -> ",tfm, humanize(value));}
              None => {tfm = format!("{}{:?} -> ",tfm, block.store.strings.get(value).unwrap());}
            }
            
          }
        },
      }
      
      match table_id {
        TableId::Global(id) => tfm=format!("{}#{}",tfm,block.store.strings.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.strings.get(id) {
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
        TableId::Global(id) => tfm = format!("{}#{}",tfm,block.store.strings.get(id).unwrap()),
        TableId::Local(id) => {
          match block.store.strings.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}{}",tfm,humanize(id)),
          }
        }
      }
      tfm = format!("{}({:x})",tfm,column_ix);
      tfm = format!("{} -> {}",tfm,block.store.strings.get(column_alias).unwrap());
      tfm
    }
    Transformation::Function{name, arguments, out} => {
      let name_string = match block.store.strings.get(name) {
        Some(name_string) => name_string.clone(),
        None => format!("{}", humanize(name)),
      };
      let mut arg = format!("");
      for (ix,(arg_id, table, row, column)) in arguments.iter().enumerate() {
        match table {
          TableId::Global(id) => {
            let name = match block.store.strings.get(id) {
              Some(name) => name.clone(),
              None => format!("{:}",humanize(id)),
            };
            arg=format!("{}#{}",arg,name)
          },
          TableId::Local(id) => {
            match block.store.strings.get(id) {
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
              TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.strings.get(id) {
                  Some(name) => {
                    arg = format!("{}{{{},",arg,name);
                  },
                  None => arg = format!("{}{{{},",arg,humanize(id)),
                }
              }
            };
          }
          Index::Alias(alias) => {
            let alias_name = block.store.strings.get(alias).unwrap();
            arg=format!("{}{{{},",arg,alias_name);
          },
        }
        match column {
          Index::None => arg=format!("{}-}}",arg),
          Index::All => arg=format!("{}:}}",arg),
          Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
          Index::Table(table) => {
            match table {
              TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.strings.get(id) {
                  Some(name) => arg = format!("{}{}",arg,name),
                  None => arg = format!("{}{}",arg,humanize(id)),
                }
              }
            };
          }
          Index::Alias(alias) => {
            let alias_name = block.store.strings.get(alias).unwrap();
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
        TableId::Global(id) => {
          let name = match block.store.strings.get(id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(id)),
          };
          arg=format!("{}#{}",arg,name);
        } 
        TableId::Local(id) => {
          match block.store.strings.get(id) {
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
            TableId::Global(id) => arg=format!("{}{{#{},",arg,block.store.strings.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(id) {
                Some(name) => arg = format!("{}{{{},",arg,name),
                None => arg = format!("{}{{{},",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match out_column {
        Index::None => arg=format!("{}-}}",arg),
        Index::All => arg=format!("{}:}}",arg),
        Index::Index(ix) => arg=format!("{}{}}}",arg,ix),
        Index::Table(table) => {
          match table {
            TableId::Global(id) => arg=format!("{}#{}",arg,block.store.strings.get(id).unwrap()),
            TableId::Local(id) => {
              match block.store.strings.get(id) {
                Some(name) => arg = format!("{}{}",arg,name),
                None => arg = format!("{}{}",arg,humanize(id)),
              }
            }
          };
        }
        Index::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}.{}}}",arg,alias_name);
        },
      }
      arg      
    },
    x => format!("{:?}", x),
  }
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
  Whenever{table_id: TableId, row: Index, column: Index, registers: Vec<u64>},
  Function{name: u64, arguments: Vec<(u64, TableId, Index, Index)>, out: (TableId, Index, Index)},
  Select{table_id: TableId, row: Index, column: Index},
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Register {
  pub table_id: TableId,
  pub row: Index,
  pub column: Index,
}

impl Register {
  pub fn hash(&self) -> u64 {
    let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);

    let unwrap_index = |index: &Index| -> u64 {
      match index {
        Index::Index(ix) => *ix as u64,
        Index::Alias(alias) => {
          alias.clone()
        },
        Index::Table(table_id) => *table_id.unwrap(),
        Index::None |
        Index::All => 0,
      }
    };


    hasher.write_u64(*self.table_id.unwrap());
    hasher.write_u64(unwrap_index(&self.row));
    hasher.write_u64(unwrap_index(&self.column));
    hasher.finish() & 0x00FFFFFFFFFFFFFF
  }
}

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
    match self.column_index {
      Index::All => unsafe{ (*self.table).columns },
      Index::Index{..} |
      Index::Alias{..} => 1,
      _ => unsafe{ (*self.table).columns },
    }
    
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

impl fmt::Debug for ValueIterator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    unsafe{write!(f, "row index: {:?}\n", (*self.table))?;}
    write!(f, "row index: {:?}\n", self.row_index)?;
    write!(f, "col index: {:?}\n", self.column_index)?;
    write!(f, "row iter: {:?}\n", self.row_iter)?;
    write!(f, "col iter: {:?}\n", self.column_iter)?;
    
    Ok(())
  }
}

#[derive(Debug)]
pub struct IndexRepeater {
  iterator: std::iter::Cycle<IndexIterator>,
  width: usize,
  current: Option<Index>,
  counter: usize,
}

impl IndexRepeater {

  pub fn new(iterator: IndexIterator, width: usize) -> IndexRepeater {
    IndexRepeater {
      iterator: iterator.cycle(),
      width,
      current: None,
      counter: 0,
    }
  }

  pub fn next(&mut self) -> Option<Index> {
    if self.current == None {
      self.current = self.iterator.next();
    }
    if self.counter == self.width {
      self.counter = 0;
      self.current = self.iterator.next();
    }
    self.counter += 1;
    self.current
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
      for ix in 1..=max {
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
      if self.current < (*self.table).data.len() {
        let address = (*self.table).data[self.current];
        self.current += 1;
        let value = (*self.table).store.data[address];
        match value.as_u64() {
          Some(v) => {
            Some(Index::Index(v as usize))
          },
          None => match value.as_bool() {
            Some(true) => {
              Some(Index::Index(self.current))
            },
            Some(false) => {
              Some(Index::None)
            },
            x => {
              Some(Index::None)
            }
          }
        }
      } else {
        None
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct AliasIterator {
  alias: u64,
  table_id: TableId,
  store: Arc<Store>,
  index: Option<Index>,
}

impl AliasIterator {

  pub fn new(alias: u64, table_id: TableId, store: Arc<Store>) -> AliasIterator {
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
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
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
