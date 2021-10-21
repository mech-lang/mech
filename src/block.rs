// ## Block

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape
// them or perform computations on them. Blocks can be thought of as pure functions where the input and
// output are tables. Blocks have their own internal table store. Local tables can be defined within a
// block, which allows the programmer to break a computation down into steps. The result of the computation
// is then output to one or more global tables, which triggers the execution of other blocks in the network.

// ## Prelude

use crate::{BoxPrinter, Transformation, NumberLiteralKind, Change, Transaction, Value, ValueKind, Column, ColumnU8, Database, humanize, hash_string, Table, TableIndex, TableShape, TableId};
use std::cell::RefCell;
use std::rc::Rc;
use hashbrown::HashMap;
use std::fmt;

pub type Plan = Vec<Rc<RefCell<Transformation>>>;


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockState {
  New,          // Has just been created, but has not been tested for satisfaction
  Ready,        // All inputs are satisfied and the block is ready to execute
  Done,         // All inputs are satisfied and the block has executed
  Unsatisfied,  // One or more inputs are not satisfied
  Error,        // One or more errors exist on the block
  Disabled,     // The block is disabled will not execute if it otherwise would
}

// ## Block

lazy_static! {
  static ref MATH_ADD: u64 = hash_string("math/add");
  static ref MATH_DIVIDE: u64 = hash_string("math/divide");
  static ref MATH_MULTIPLY: u64 = hash_string("math/multiply");
  static ref MATH_SUBTRACT: u64 = hash_string("math/subtract");
  static ref MATH_EXPONENT: u64 = hash_string("math/exponent");
  static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_string("table/horizontal-concatenate");
  static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_string("table/vertical-concatenate");
}

#[derive(Clone)]
pub struct Block {
  pub id: u64,
  pub state: BlockState,
  tables: Database,
  plan: Plan,
  pub changes: Transaction,
  pub global_database: Rc<RefCell<Database>>,
  unsatisfied_transformations: Vec<Transformation>,
  transformations: Vec<Transformation>,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      state: BlockState::New,
      tables: Database::new(),
      plan: Vec::new(),
      changes: Vec::new(),
      global_database: Rc::new(RefCell::new(Database::new())),
      unsatisfied_transformations: Vec::new(),
      transformations: Vec::new(),
    }
  }

  pub fn get_table(&self, table_id: &TableId) -> Option<Rc<RefCell<Table>>> {
    match &table_id {
      TableId::Local(id) => match self.tables.get_table_by_id(id) {
        Some(table) => Some(table.clone()),
        None => None,
      },
      TableId::Global(id) => match self.global_database.borrow().get_table_by_id(id) {
        Some(table) => Some(table.clone()),
        None => None,
      }
    }
  }

  pub fn gen_id(&mut self) -> u64 {
    self.id = hash_string(&format!("{:?}", self.plan));
    self.id
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn ready(&mut self) -> bool {
    let unsatisfied = self.unsatisfied_transformations.clone();
    self.unsatisfied_transformations.clear();
    for tfm in unsatisfied {
      self.add_tfm(tfm);
    }
    if self.unsatisfied_transformations.len() == 0 {
      self.state = BlockState::Ready;
      true
    } else {
      false
    }
  }

  pub fn add_tfm(&mut self, tfm: Transformation) {
    match &tfm {
      Transformation::NewTable{table_id, rows, columns} => {
        match table_id {
          TableId::Local(id) => {
            let table = Table::new(*id, *rows, *columns);
            self.tables.insert_table(table);
          }
          TableId::Global(id) => {
            self.changes.push(Change::NewTable{table_id: *id, rows: *rows, columns: *columns});
          }
        } 
      },
      Transformation::TableAlias{table_id, alias} => {
        self.tables.insert_alias(*alias, *table_id.unwrap());
      },
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        let mut table = self.tables.get_table_by_id(table_id.unwrap()).unwrap().borrow_mut();
        table.set_column_alias(*column_ix,*column_alias);
      },
      Transformation::Select{table_id, indices, out} => {
        let src_table = self.get_table(table_id);
        let out_table = self.get_table(out);
        match (src_table, out_table, indices[0]) {
          (Some(src_table),Some(out_table), (TableIndex::All, TableIndex::All)) => {
            match (table_id, out) {
              (_, TableId::Global(gid)) => {
                let table_id2 = table_id;
                // find all table aliases and copy them as well
                for tfm in self.transformations.iter() {
                  match tfm {
                    Transformation::ColumnAlias{table_id,column_ix,column_alias} => {
                      if table_id2 == table_id {
                        // Remap the local column alias for the global table
                        let remapped_tfm = Change::ColumnAlias{table_id: *gid, column_ix: *column_ix, column_alias: *column_alias};
                        self.changes.push(remapped_tfm);
                      }
                    },
                    _ => (),
                  }
                } 
                let tfm = Transformation::CopyTable((src_table.clone(), out_table.clone()));
                self.plan.push(Rc::new(RefCell::new(tfm)));
              }
              _ => (), // TODO Other possibilities,
            }
          }
          (Some(src),Some(out), (TableIndex::Index(ix), TableIndex::None)) => {
            let src_brrw = src.borrow();
            let (row,col) = src_brrw.index_to_subscript(ix-1).unwrap(); // TODO Make sure the index is in bounds
            let mut out_brrw = out.borrow_mut();
            let mut arg_col = src_brrw.get_column_unchecked(col);
            out_brrw.set_col_kind(0,arg_col.kind());
            let mut out_col = out_brrw.get_column_unchecked(0);
            match (&arg_col, &out_col) {
              (Column::U8(arg), Column::U8(out)) => {
                let tfm = Transformation::CopySSU8((arg.clone(),row,out.clone()));
                self.plan.push(Rc::new(RefCell::new(tfm)));
              }
              _ => (),
            }
          }
          (None, _, _) |
          (_, None, _) => {
            self.unsatisfied_transformations.push(tfm.clone());
            self.state = BlockState::Unsatisfied;
          },
          _ => (), // TODO Raise an error here, we don't handle this case
        }
      }
      Transformation::Set{src_id, src_indices, dest_id, dest_indices} => {
        let src_table = self.get_table(src_id);
        let dest_table = self.get_table(dest_id);
        match (src_table, dest_table, src_indices[0], dest_indices[0]) {
          (Some(src_table),Some(dest_table), (TableIndex::All, TableIndex::All), (TableIndex::All, TableIndex::All)) => {
            match (src_id, dest_id) {
              (_, TableId::Global(gid)) => {
                let src_table_brrw = src_table.borrow();
                let dest_table_brrw = dest_table.borrow();
                // Copy each column because it's (:,:) -> (:,:)
                for col in 0..src_table_brrw.cols {
                  let src_vector = src_table_brrw.get_column_unchecked(col);
                  let dest_vector = dest_table_brrw.get_column_unchecked(col);
                  match (&src_vector, &dest_vector) {
                    (Column::U8(src), Column::U8(dest)) => {
                      let tfm = Transformation::SetVVU8((src.clone(), dest.clone()));
                      self.plan.push(Rc::new(RefCell::new(tfm)));
                    }
                    _ => {
                      self.unsatisfied_transformations.push(tfm);
                      self.state = BlockState::Unsatisfied;
                      return;
                    },
                  }

                }
              }
              _ => (), // TODO Other possibilities,
            }
          }
          (None, _, _, _) |
          (_, None, _, _) => {
            self.unsatisfied_transformations.push(tfm.clone());
            self.state = BlockState::Unsatisfied;
          },
          _ => (), // TODO Raise an error here, we don't handle this case
        }
      }
      Transformation::NumberLiteral{kind, bytes} => {
        let table_id = hash_string(&format!("{:?}{:?}", kind, bytes));
        match self.get_table(&TableId::Local(table_id)) {
          Some(table) => {
            let mut t = table.borrow_mut();
            match kind {
              NumberLiteralKind::Decimal => {
                match bytes.len() {
                  1 => {
                    t.set_col_kind(0, ValueKind::U8);
                    t.set(0,0,Value::U8(bytes[0] as u8));
                  }
                  2 => {
                    t.set_col_kind(0, ValueKind::U16);
                    use std::mem::transmute;
                    use std::convert::TryInto;
                    let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
                    let x = u16::from_ne_bytes(int_bytes.try_into().unwrap());
                    t.set(0,0,Value::U16(x));
                  }
                  _ => (), // TODO Handle other sizes
                }
              }
              _ => (),
            }
          }
          _ => (),
        }
      },
      Transformation::Constant{table_id, value} => {
        let table = self.get_table(table_id).unwrap();
        let mut table_brrw = table.borrow_mut();
        match &value {
          Value::Bool(_) => {table_brrw.set_col_kind(0, ValueKind::Bool);},
          Value::String(_) => {table_brrw.set_col_kind(0, ValueKind::String);},
          _ => (),
        }
        table_brrw.set(0,0,value.clone());
      }
      Transformation::Function{name, ref arguments, out} => {
        if *name == *MATH_ADD || 
           *name == *MATH_DIVIDE || 
           *name == *MATH_MULTIPLY || 
           *name == *MATH_SUBTRACT || 
           *name == *MATH_EXPONENT {
          let mut arg_dims = Vec::new();
          for (_, table_id, row, column) in arguments {
            match self.get_table(table_id) {
              Some(table) => {
                let t = table.borrow();
                let dims = match (row, column) {
                  (TableIndex::All, TableIndex::All) => (t.rows, t.cols),
                  (TableIndex::All, TableIndex::Alias(x)) => (t.rows, 1),
                  _ => (0,0),
                };
                arg_dims.push(dims);
              }
              _ => {
                self.unsatisfied_transformations.push(tfm);
                self.state = BlockState::Unsatisfied;
                return;
              },
            }
          }
          let arg_shapes: Vec<TableShape> = arg_dims.iter().map(|dims| {
            match dims {
              (1,1) => TableShape::Scalar,
              (x,1) => TableShape::Column(*x),
              (1,x) => TableShape::Row(*x),
              (x,y) => TableShape::Matrix(*x,*y),
              _ => TableShape::Pending,
            }
          }).collect();
          // Now decide on the correct tfm based on the shape
          match (&arg_shapes[0],&arg_shapes[1]) {
            (TableShape::Scalar, TableShape::Scalar) => {
              let mut argument_columns = vec![];
              for (_, table_id, _, _) in arguments {
                match self.get_table(table_id) {
                  Some(table) => {
                    let mut column = table.borrow().get_column_unchecked(0);
                    argument_columns.push(column);
                  }
                  _ => {
                    self.unsatisfied_transformations.push(tfm);
                    self.state = BlockState::Unsatisfied;
                    return;
                  },
                }
              }
              let (out_table_id, _, _) = out;
              match self.get_table(out_table_id) {
                Some(table) => {
                  let mut t = table.borrow_mut();
                  t.set_col_kind(0, ValueKind::U8);
                  let column = t.get_column_unchecked(0);
                  argument_columns.push(column);
                }
                _ => {
                  self.unsatisfied_transformations.push(tfm);
                  self.state = BlockState::Unsatisfied;
                  return;
                },
              }
              match (&argument_columns[0], &argument_columns[1], &argument_columns[2]) {
                (Column::U8(lhs), Column::U8(rhs), Column::U8(out)) => {
                  let tfm = if *name == *MATH_ADD { Transformation::AddSSU8(vec![lhs.clone(), rhs.clone(), out.clone()]) }
                  else if *name == *MATH_DIVIDE { Transformation::DivideSSU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  else if *name == *MATH_MULTIPLY { Transformation::MultiplySSU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  else if *name == *MATH_SUBTRACT { Transformation::SubtractSSU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  else if *name == *MATH_EXPONENT { Transformation::ExponentSSU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  else { Transformation::Null };
                  self.plan.push(Rc::new(RefCell::new(tfm)));
                }
                _ => {
                  self.unsatisfied_transformations.push(tfm);
                  self.state = BlockState::Unsatisfied;
                  return;
                },
              }

            }
            (TableShape::Column(lhs_alias), TableShape::Column(rhs_alias)) => {
              let mut argument_columns = vec![];
              for (_, table_id, _, col) in arguments {
                match self.get_table(table_id) {
                  Some(table) => {
                    let mut column = table.borrow().get_column_alias_unchecked(*col);
                    argument_columns.push(column);
                  }
                  _ => {
                    self.unsatisfied_transformations.push(tfm);
                    self.state = BlockState::Unsatisfied;
                    return;
                  },
                }
              }
              let (out_table_id, _, _) = out;
              match self.get_table(out_table_id) {
                Some(table) => {
                  let mut t = table.borrow_mut();
                  let (rows, _) = arg_dims[0];
                  let cols = t.cols;
                  t.resize(rows,cols);
                  t.set_col_kind(0, ValueKind::U8);
                  let column = t.get_column_unchecked(0);
                  argument_columns.push(column);
                }
                _ => {
                  self.unsatisfied_transformations.push(tfm);
                  self.state = BlockState::Unsatisfied;
                  return;
                },
              }
              match (&argument_columns[0], &argument_columns[1], &argument_columns[2]) {
                (Column::U8(lhs), Column::U8(rhs), Column::U8(out)) => {
                  let tfm = if *name == *MATH_ADD { Transformation::AddVVU8(vec![lhs.clone(), rhs.clone(), out.clone()]) }
                  //else if *name == *MATH_DIVIDE { Transformation::DivideVVU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  //else if *name == *MATH_MULTIPLY { Transformation::MultiplyVVU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  //else if *name == *MATH_SUBTRACT { Transformation::SubtractVVU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  //else if *name == *MATH_EXPONENT { Transformation::ExponentVVU8((lhs.clone(), rhs.clone(), out.clone())) } 
                  else { Transformation::Null };
                  self.plan.push(Rc::new(RefCell::new(tfm)));
                }
                _ => {
                  self.unsatisfied_transformations.push(tfm);
                  self.state = BlockState::Unsatisfied;
                  return;
                },
              }

            }
            _ => (),
          }
        } else if *name == *TABLE_VERTICAL__CONCATENATE {
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id).unwrap().clone();
          let mut out_column_ix = 0;
          let mut arg_cols = vec![];
          let mut rows = 0;
          let mut cols = 0;
          for (_, table_id, _, _) in arguments {
            match self.get_table(table_id) {
              Some(table) => {
                let t = table.borrow();
                rows += t.rows;
                cols = if cols == 0 {t.cols} 
                       else if t.cols != cols { 0 /*TODO ERROR*/ }
                       else { cols };
                if arg_cols.len() == 0 {
                  arg_cols = vec![Vec::new(); cols];
                }
                for c in 0..t.cols {
                  let mut arg_col = t.get_column_unchecked(c);
                  arg_cols[c].push(arg_col);
                }
              }
              None => {
                self.unsatisfied_transformations.push(tfm.clone());
                self.state = BlockState::Unsatisfied;
                return;
              },
            }
          }
          let mut out_brrw = out_table.borrow_mut();
          out_brrw.resize(rows,cols);
          for col in 0..cols {
            out_brrw.set_col_kind(col, ValueKind::U8);
            let out_col = out_brrw.get_column_unchecked(col);
            let arg_col = arg_cols[col].iter().filter_map(|arg| match arg {
              Column::U8(col) => Some(col.clone()),
              _ => None,
            }).collect::<Vec<ColumnU8>>();
            match out_col {
              Column::U8(out_col) => {
                let tfm = Transformation::ConcatVU8((arg_col, out_col));
                self.plan.push(Rc::new(RefCell::new(tfm)));
              }
              _ => (),
            }
          }
        } else if *name == *TABLE_HORIZONTAL__CONCATENATE {
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id).unwrap().clone();
          let mut out_column_ix = 0;
          for (_, table_id, _, _) in arguments {
            match self.get_table(table_id) {
              Some(table) => {
                let t = table.borrow();
                let mut o = out_table.borrow_mut();
                for c in 0..t.cols {
                  let mut arg_col = t.get_column_unchecked(c);
                  o.set_col_kind(out_column_ix, arg_col.kind());
                  let mut out_col = o.get_column_unchecked(out_column_ix);
                  o.set_col_kind(out_column_ix, arg_col.kind());
                  match (&arg_col, &out_col) {
                    (Column::U8(arg), Column::U8(out)) => {
                      let tfm = Transformation::CopySSU8((arg.clone(),0,out.clone()));
                      self.plan.push(Rc::new(RefCell::new(tfm)));
                      out_column_ix += 1;
                    }
                    (Column::String(arg), Column::String(out)) => {
                      let tfm = Transformation::CopySSString((arg.clone(),0,out.clone()));
                      self.plan.push(Rc::new(RefCell::new(tfm)));
                      out_column_ix += 1;
                    }
                    _ => (),
                  }
                }
              }
              None => {
                self.unsatisfied_transformations.push(tfm.clone());
                self.state = BlockState::Unsatisfied;
                return;
              },
            }
          }

        } else { 
          self.plan.push(Rc::new(RefCell::new(tfm.clone())));
        }
      } 
      _ => self.plan.push(Rc::new(RefCell::new(tfm.clone()))),
    }
    self.transformations.push(tfm.clone());
  }

  pub fn solve(&mut self) -> bool {
    if self.state == BlockState::Ready {
      for ref mut tfm in &mut self.plan.iter() {
        tfm.borrow_mut().solve();
      }
      true
    } else {
      false
    }
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut block_drawing = BoxPrinter::new();
    block_drawing.add_line(format!("id: {}", humanize(&self.id)));
    block_drawing.add_line(format!("state: {:?}", &self.state));
    block_drawing.add_header("transformations");
    block_drawing.add_line(format!("{:#?}", &self.transformations));
    block_drawing.add_header("tables");
    block_drawing.add_line(format!("{:?}", &self.tables));
    block_drawing.add_header("unsatisfied transformations");
    block_drawing.add_line(format!("{:#?}", &self.unsatisfied_transformations));
    block_drawing.add_header("plan");
    for step in &self.plan {
      block_drawing.add_line(format!("{:#?}", &step.borrow()));
    }
    block_drawing.add_header("changes");
    block_drawing.add_line(format!("{:#?}", &self.changes));
    write!(f,"{:?}",block_drawing)?;
    Ok(())
  }
}

#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Register {
  pub table_id: TableId,
  pub row: TableIndex,
  pub column: TableIndex,
}

/*use table::{Table, TableId, TableIndex};
use value::{Value, ValueMethods, NumberLiteralKind};
use index::{ValueIterator, TableIterator, IndexIterator, AliasIterator, ConstantIterator, IndexRepeater};
use database::{Database, Store, Change, Transaction};
use hashbrown::{HashMap, HashSet};
use quantities::{QuantityMath};
use operations::{MechFunction, Argument};
use errors::{Error, ErrorType};
use std::cell::RefCell;
use std::cell::Cell;
use std::sync::Arc;
use std::rc::Rc;
use rust_core::fmt;
use ::humanize;
use ::hash_string;

lazy_static! {
  static ref TABLE_COPY: u64 = hash_string("table/copy");
  static ref TABLE_SPLIT: u64 = hash_string("table/split");
  static ref GRAMS: u64 = hash_string("g");
  static ref KILOGRAMS: u64 = hash_string("kg");
  static ref HERTZ: u64 = hash_string("Hz");
  static ref SECONDS: u64 = hash_string("s");
}


#[derive(Clone)]
pub struct Block {
  pub id: u64,
  pub state: BlockState,
  pub text: String,
  pub name: String,
  pub ready: HashSet<Register>,
  pub input: HashSet<Register>,
  pub output: HashSet<Register>,
  pub output_dependencies: HashSet<Register>,
  pub output_dependencies_ready: HashSet<Register>,
  pub register_aliases: HashMap<Register, HashSet<Register>>,
  pub tables: HashMap<u64, Arc<RefCell<Table>>>,
  pub store: Arc<Store>,
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<Transformation>,
  pub changes: Vec<Change>,
  pub errors: HashSet<Error>,
  pub triggered: usize,
  pub function_arguments: HashMap<Transformation, Vec<Rc<RefCell<Argument>>>>,
  pub global_database: Arc<RefCell<Database>>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    // We create a dummy database here which will be replaced when the block is registered with
    // a runtime. I tried using an option but I didn't know how to unwrap it without copying
    // it.
    let database = Arc::new(RefCell::new(Database::new(1)));
    Block {
      id: 0,
      text: String::new(),
      name: String::new(),
      ready: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      output_dependencies: HashSet::new(),
      output_dependencies_ready: HashSet::new(),
      register_aliases: HashMap::new(),
      state: BlockState::New,
      tables: HashMap::new(),
      store: Arc::new(Store::new(capacity)),
      transformations: Vec::new(),
      plan: Vec::new(),
      changes: Vec::new(),
      errors: HashSet::new(),
      function_arguments: HashMap::new(),
      triggered: 0,
      global_database: database,
    }
  }

  pub fn gen_id(&mut self) {
    let mut words = "".to_string();
    for tfm in &self.transformations {
      words = format!("{:?}{:?}", words, tfm);
    }
    self.id = seahash::hash(words.as_bytes()) & 0x00FFFFFFFFFFFFFF;
  }

  pub fn register_transformations(&mut self, tfm_tuple: (String, Vec<Transformation>)) {
    self.transformations.push(tfm_tuple.clone());

    let (_, transformations) = tfm_tuple;

    for tfm in transformations {
      match tfm {
        Transformation::TableAlias{table_id, alias} => {
          match table_id {
            TableId::Global(id) => {

            }
            TableId::Local(id)=> {
              let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
              store.table_id_to_alias.insert(table_id, alias);
              store.table_alias_to_id.insert(alias, table_id);
            }
          }
        }
        Transformation::TableReference{table_id, reference} => {
          match table_id {
            TableId::Local(id) => {
              let mut reference_table = Table::new(id, 1, 1, self.store.clone());
              reference_table.set_unchecked(1,1,reference);
              self.tables.insert(id, Arc::new(RefCell::new(reference_table)));
              self.changes.push(
                Change::NewTable{
                  table_id: reference.as_reference().unwrap(),
                  rows: 1,
                  columns: 1,
                }
              );
              let register_all = Register{table_id: TableId::Global(reference.as_reference().unwrap()), row: TableIndex::All, column: TableIndex::All};
              self.output.insert(register_all);
            }
            _ => (),
          }
        }
        Transformation::Select{table_id, row, column, indices, out} => {
          match out {
            TableId::Global(_) => {
              let register = Register{table_id: out, row: TableIndex::All, column: TableIndex::All};
              self.output.insert(register);              
            }
            _ => (),
          }
          match table_id {
            TableId::Global(_) => {
              let register = Register{table_id: table_id, row: TableIndex::All, column: TableIndex::All};
              self.input.insert(register);                
            }
            _ => (),
          }
          for (row, column) in indices {
            match row {
              TableIndex::Table(TableId::Global(id)) => {
                let register = Register{table_id: TableId::Global(id), row: TableIndex::All, column: TableIndex::All};
                self.input.insert(register);
              }
              _ => (),
            }
            match column {
              TableIndex::Table(TableId::Global(id)) => {
                let register = Register{table_id: TableId::Global(id), row: TableIndex::All, column: TableIndex::All};
                self.input.insert(register);
              }
              _ => (),
            }
          }
        }
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
              let register_all = Register{table_id, row: TableIndex::All, column: TableIndex::All};
              self.output.insert(register_all);
            }
            TableId::Local(id) => {
              self.tables.insert(id, Arc::new(RefCell::new(Table::new(id, rows, columns, self.store.clone()))));
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
              let register_all = Register{table_id: table_id, row: TableIndex::All, column: TableIndex::All};
              let register_alias = Register{table_id: table_id, row: TableIndex::All, column: TableIndex::Alias(column_alias)};
              let register_ix = Register{table_id: table_id, row: TableIndex::All, column: TableIndex::Index(column_ix)};
              // Alias mappings
              let aliases = self.register_aliases.entry(register_alias).or_insert(HashSet::new());
              aliases.insert(register_ix);
              aliases.insert(register_all);
              // Index mappings
              let aliases = self.register_aliases.entry(register_ix).or_insert(HashSet::new());
              aliases.insert(register_alias);
              aliases.insert(register_all);
              // All mappings
              let aliases = self.register_aliases.entry(register_all).or_insert(HashSet::new());
              aliases.insert(register_ix);
              aliases.insert(register_alias);
              self.output.insert(register_alias);              
            }
            TableId::Local(_id) => {
              let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
              store.column_index_to_alias.insert((*table_id.unwrap(),column_ix),column_alias);
              store.column_alias_to_index.insert((*table_id.unwrap(),column_alias),column_ix);
            }
          }
        }
        Transformation::Constant{table_id, value, unit} => {
          let (domain, scale) = if unit == *GRAMS { (1, 0) }
            else if unit            == *KILOGRAMS { (1, 3) }
            else if unit            == *HERTZ { (2, 0) }
            else if unit            == *SECONDS { (3, 0) }
//              "m" => (2, 0),
//              "km" => (2, 3),
//              "ms" => (3, 0),
//              "s" => (3, 3),
              else { (0, 0) };
          let q = if value.is_number() {
            value //Value::from_quantity(make_quantity(value.mantissa(), value.range() + scale, domain))
          } else {
            value
          };
          match table_id {
            TableId::Local(id) => {
              let mut table = self.tables.get_mut(&id).unwrap().borrow_mut();
              table.set(&TableIndex::Index(1), &TableIndex::Index(1), q);
            }
            TableId::Global(id) => {
              self.changes.push(
                Change::Set{
                  table_id: id,
                  values: vec![(TableIndex::Index(1), TableIndex::Index(1), q)],
                }
              );
            }
           // _ => (),
          }
        }
        Transformation::Set{table_id, row, column} => {
          let register_all = Register{table_id: table_id, row: TableIndex::All, column: TableIndex::All};
          self.output.insert(register_all);       
          self.output_dependencies.insert(register_all);          
          match row {
            TableIndex::Table(TableId::Global(id)) => {
              let register = Register{table_id: TableId::Global(id), row: TableIndex::All, column: TableIndex::All};
              self.input.insert(register);
            }
            _ => (),
          }
          match column {
            TableIndex::Table(TableId::Global(id)) => {
              let register = Register{table_id: TableId::Global(id), row: TableIndex::All, column: TableIndex::All};
              self.input.insert(register);
            }
            _ => (),
          }
        }
        Transformation::Whenever{table_id, registers, ..} => {
          let whenever_ix_table_id = hash_string("~");
          self.tables.insert(whenever_ix_table_id, Arc::new(RefCell::new(Table::new(whenever_ix_table_id, 0, 1, self.store.clone()))));
          match table_id {
            TableId::Global(_id) => {
              for register in registers {
                self.input.insert(register);
              }
            }
            _ => (),
          }
        }
        Transformation::Function{ref arguments, out, ..} => {
          let (out_id, row, column) = out;
          match out_id {
            TableId::Global(_id) => {
              let row = match row {
                TableIndex::Table(_) => TableIndex::All,
                x => x,
              };
              let column = match column {
                TableIndex::Table(_) => TableIndex::All,
                x => x,
              };
              let register = Register{table_id: out_id, row, column};
              self.output.insert(register);
              let register = Register{table_id: out_id, row: TableIndex::All, column: TableIndex::All};
              self.output.insert(register);
            },
            _ => (),
          }
          for (_, table_id, row, column) in arguments {
            match table_id {
              TableId::Global(_id) => {
                let row2: &TableIndex = match row {
                  TableIndex::Table{..} => &TableIndex::All,
                  TableIndex::None => &TableIndex::All,
                  x => x,
                };
                let column2: &TableIndex = match column {
                  TableIndex::Table{..} => &TableIndex::All,
                  TableIndex::None => &TableIndex::All,
                  x => x,
                };
                let register_ix = Register{table_id: *table_id, row: *row2, column: *column2};
                let register_all = Register{table_id: *table_id, row: TableIndex::All, column: TableIndex::All};
                let aliases = self.register_aliases.entry(register_ix).or_insert(HashSet::new());
                aliases.insert(register_all);
                let aliases = self.register_aliases.entry(register_all).or_insert(HashSet::new());
                aliases.insert(register_ix);
                self.input.insert(register_ix);
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
  pub fn process_changes(&mut self) {
    if !self.changes.is_empty() {
      let txn = Transaction {
        changes: self.changes.clone(),
      };
      self.changes.clear();
      self.global_database.borrow_mut().process_transaction(&txn).ok();
    }
  }

  pub fn solve(&mut self, functions: &HashMap<u64, Option<MechFunction>>) -> Result<(), Error> {
    self.triggered += 1;
    'step_loop: for step in &self.plan {
      match step {
        Transformation::Whenever{table_id, registers, ..} => {
          let register = registers[0];
          // Resolve whenever table subscript so we can iterate through the values
          let mut vi = ValueIterator::new(register.table_id,register.row,register.column,&self.global_database,&mut self.tables, &mut self.store);
          // Get the whenever table from the local store
        {
          let whenever_ix_table_id = hash_string("~");
          let mut whenever_table = self.tables.get_mut(&whenever_ix_table_id).unwrap().borrow_mut();
          // Check to see if the whenever table needs to be resized
          let before_rows = whenever_table.rows;
          if vi.rows() > whenever_table.rows {
            whenever_table.resize(vi.rows() * vi.columns(),1);
            for (ix, (_, changed)) in vi.enumerate() {
              // Mark the new rows as changed even if they are stale
              if ix+1 > before_rows {
                whenever_table.set_unchecked(ix+1, 1, Value::from_bool(true));
              // Use the changed value of old rows
              } else {
                whenever_table.set_unchecked(ix+1, 1, Value::from_bool(changed));
              }
            }
          // If the table hasn't been resized, use the changed value
          } else {
            for (ix, (_, changed)) in vi.enumerate() {
              whenever_table.set_unchecked(ix+1, 1, Value::from_bool(changed));
            }
          }

          // If all of the rows of the whenever table are false, there is nothing for this block to do
          // because none of the values it is watching have changed
          let mut flag = false;
          for ix in 1..=whenever_table.rows {
            let (val, _) = whenever_table.get_unchecked(ix,1);
            match val.as_bool() {
              Some(true) => flag = true,
              _ => (),
            }
          }
          if flag == false {
            break 'step_loop;
          }
        }
          
          match table_id {
            TableId::Global(_id) => {
              for register in registers {
                self.ready.remove(&register);
              }
            }
            TableId::Local(id) => {
              let mut flag = false;
              let table = self.tables.get_mut(&id).unwrap().borrow();
              unsafe {
                for i in 1..=(*table).rows {
                  for j in 1..=(*table).columns {
                    let (val, _) = (*table).get_unchecked(i,j);
                    match val.as_bool() {
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
        Transformation::Select{table_id, row, column, indices, out} => {
          // Get the output Iterator
          let mut out = ValueIterator::new(*out, TableIndex::All, TableIndex::All, &self.global_database.clone(),&mut self.tables, &mut self.store);

          let mut table_id = *table_id;
          for (ix, (row_index, column_index)) in indices.iter().enumerate() {

            let mut vi = ValueIterator::new(table_id, *row_index, *column_index, &self.global_database.clone(),&mut self.tables, &mut self.store);

            // Size the out table if we're on the last index
            if ix == indices.len() - 1 {
              out.resize(vi.rows(), vi.columns());
            }

            let elements = vi.elements();
            let mut out_iterator = out.linear_index_iterator();
            for (value, _) in vi {
              match value.as_reference() {
                Some(reference) => {
                  // We can only follow a reference is the selected table is scalar
                  if elements == 1 && ix != indices.len() - 1 {
                                           //^ We only want to follow a reference if we aren't at the
                                           //  last index in the list.
                    table_id = TableId::Global(reference);
                    continue;
                  // If we are at the last index, we'll store this reference in th out
                  } else if ix == indices.len() - 1 { 
                    match out_iterator.next() {
                      Some(ix) => out.set_unchecked_linear(ix, value),
                      _ => (),
                    }
                  }
                }
                None => {
                  match out_iterator.next() {
                    Some(ix) => out.set_unchecked_linear(ix, value),
                    _ => (),
                  }
                }
              }
            }
          }
        }
        Transformation::Function{name, arguments, out} => {

          let mut args = self.function_arguments.entry(step.clone()).or_insert(vec![]);

          if args.len() == 0 {
            for (arg, table_id, row, column) in arguments {
              let mut vi = ValueIterator::new(*table_id,*row,*column,&self.global_database.clone(),&mut self.tables, &mut self.store);
              vi.compute_indices();
              args.push(Rc::new(RefCell::new(Argument{name: arg.clone(), iterator: vi})));
            }
            let (out_table_id, out_row, out_column) = out;
            let mut out_vi = ValueIterator::new(*out_table_id, *out_row, *out_column, &self.global_database.clone(),&mut self.tables, &mut self.store);
            args.push(Rc::new(RefCell::new(Argument{name: 0, iterator: out_vi})));
          }

          args.iter().for_each(|mut arg| arg.borrow_mut().iterator.init_iterators());
          
          match functions.get(name) {
            Some(Some(mech_fn)) => {
              mech_fn(&mut args);
            }
            _ => {
              if *name == *TABLE_SPLIT {
                let vi = args[0].borrow().iterator.clone();
                let mut out = args.last().unwrap().borrow_mut();

                out.iterator.resize(vi.rows(), 1);

                let mut db = self.global_database.borrow_mut();

                // Create rows for tables
                let old_table_id = vi.id();
                let old_table_columns = vi.columns();
                for row in vi.raw_row_iter.clone() {
                  let split_table_id = hash_string(&format!("table/split/{:?}/{:?}",old_table_id,row));
                  let mut split_table = Table::new(split_table_id,1,old_table_columns,self.store.clone());
                  for column in vi.raw_column_iter.clone() {
                    let (value,_) = vi.get(&row,&column).unwrap();
                    split_table.set(&TableIndex::Index(1),&column, value);
                  }
                  out.iterator.set_unchecked(row.unwrap(),1, Value::from_id(split_table_id));
                  self.tables.insert(split_table_id, Arc::new(RefCell::new(split_table.clone())));
                  db.tables.insert(split_table_id, Arc::new(RefCell::new(split_table)));
                }
              } else {
                let error = Error { 
                  block_id: self.id,
                  step_text: "".to_string(), // TODO Add better text
                  error_type: ErrorType::MissingFunction(*name),
                };
                self.state = BlockState::Error;
                self.errors.insert(error.clone());
                return Err(error);
              }
            },
          }
        }
        _ => (),
      }
    }
    self.state = BlockState::Done;
    Ok(())
  }

  pub fn is_ready(&mut self) -> bool {
    // The block will not execute if it's in an error state or disabled
    if self.state == BlockState::Error || self.state == BlockState::Disabled {
      false
    // The block will not execute if there are any errors listed on it
    } else if self.errors.len() > 0 {
      self.state = BlockState::Error;
      false
    } else {
      // The block is ready if the ready output and input registers equal the total
      if self.ready.len() < self.input.len() || self.output_dependencies_ready.len() < self.output_dependencies.len() {
        false
      } else {
        self.state = BlockState::Ready;
        true
      }
    }
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌─────────────────────────────────────────────┐\n")?;
    write!(f, "│ id: {}                         │\n", humanize(&self.id))?;
    write!(f, "│ state: {:?}                                 │\n", self.state)?;
    write!(f, "│ triggered: {:?}                                │\n", self.triggered)?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ Errors: {}                                   │\n", self.errors.len())?;
    for (ix, error) in self.errors.iter().enumerate() {
      write!(f, "│    {}. {:?}\n", ix+1, error)?;
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ Registers                                   │\n")?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ ready: {}\n", self.ready.len())?;
    for (ix, register) in self.ready.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
    }
    write!(f, "│ input: {} \n", self.input.len())?;
    for (ix, register) in self.input.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
    }
    if self.ready.len() < self.input.len() {
      write!(f, "│ missing: \n")?;
      for (ix, register) in self.input.difference(&self.ready).enumerate() {
        write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
      }
    }
    write!(f, "│ output: {}\n", self.output.len())?;
    for (ix, register) in self.output.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
    }
    write!(f, "│ output dep: {}\n", self.output_dependencies.len())?;
    for (ix, register) in self.output_dependencies.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
    }
    write!(f, "│ output ready: {}\n", self.output_dependencies_ready.len())?;
    for (ix, register) in self.output_dependencies_ready.iter().enumerate() {
      write!(f, "│    {}. {}\n", ix+1, format_register(&self.store.strings, register))?;
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ Transformations                             │\n")?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    for (ix, (text, tfms)) in self.transformations.iter().enumerate() {
      write!(f, "│  {}. {}\n", ix+1, text)?;
      for tfm in tfms {
        let tfm_string = format_transformation(&self,&tfm);
        write!(f, "│       > {}\n", tfm_string)?;
      }
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ Plan                                        │\n")?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    for (ix, tfm) in self.plan.iter().enumerate() {
      let tfm_string = format_transformation(&self,tfm);
      write!(f, "│  {}. {}\n", ix+1, tfm_string)?;
    }
    write!(f, "├─────────────────────────────────────────────┤\n")?;
    write!(f, "│ Tables: {}                                  │\n", self.tables.len())?;
    write!(f, "├─────────────────────────────────────────────┤\n")?;

    for (_, table) in self.tables.iter() {
      write!(f, "{:?}\n", table.borrow())?;
    }

    Ok(())
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Transformation {
  TableAlias{table_id: TableId, alias: u64},
  TableReference{table_id: TableId, reference: Value},
  NewTable{table_id: TableId, rows: usize, columns: usize },
  Constant{table_id: TableId, value: Value, unit: u64},
  ColumnAlias{table_id: TableId, column_ix: usize, column_alias: u64},
  Set{table_id: TableId, row: TableIndex, column: TableIndex},
  RowAlias{table_id: TableId, row_ix: usize, row_alias: u64},
  Whenever{table_id: TableId, row: TableIndex, column: TableIndex, registers: Vec<Register>},
  Function{name: u64, arguments: Vec<(u64, TableId, TableIndex, TableIndex)>, out: (TableId, TableIndex, TableIndex)},
  Select{table_id: TableId, row: TableIndex, column: TableIndex, indices: Vec<(TableIndex, TableIndex)>, out: TableId},
}

#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Register {
  pub table_id: TableId,
  pub row: TableIndex,
  pub column: TableIndex,
}

impl Register {
  pub fn hash(&self) -> u64 {
    let id_bytes = (*self.table_id.unwrap()).to_le_bytes();

    let unwrap_index = |index: &TableIndex| -> u64 {
      match index {
        TableIndex::Index(ix) => *ix as u64,
        TableIndex::Alias(alias) => {
          alias.clone()
        },
        TableIndex::Table(table_id) => *table_id.unwrap(),
        TableIndex::None |
        TableIndex::All => 0,
      }
    };

    let row_bytes = unwrap_index(&self.row).to_le_bytes();
    let column_bytes = unwrap_index(&self.column).to_le_bytes();
    let array = [id_bytes, row_bytes, column_bytes].concat();
    seahash::hash(&array) & 0x00FFFFFFFFFFFFFF
  }
}

pub fn format_register(strings: &HashMap<u64, String>, register: &Register) -> String {

  let table_id = register.table_id;
  let row = register.row;
  let column = register.column;
  let mut arg = format!("");
  match table_id {
    TableId::Global(id) => {
      let name = match strings.get(&id) {
        Some(name) => name.clone(),
        None => format!("{:}",humanize(&id)),
      };
      arg=format!("{}#{}",arg,name)
    },
    TableId::Local(id) => {
      match strings.get(&id) {
        Some(name) => arg = format!("{}{}",arg,name),
        None => arg = format!("{}{}",arg,humanize(&id)),
      }
    }
  };
  match row {
    TableIndex::None => arg=format!("{}{{-,",arg),
    TableIndex::All => arg=format!("{}{{:,",arg),
    TableIndex::Index(ix) => arg=format!("{}{{{},",arg,ix),
    TableIndex::Table(table) => {
      match table {
        TableId::Global(id) => arg=format!("{}#{}",arg,strings.get(&id).unwrap()),
        TableId::Local(id) => {
          match strings.get(&id) {
            Some(name) => arg = format!("{}{}",arg,name),
            None => arg = format!("{}{}",arg,humanize(&id)),
          }
        }
      };
    }
    TableIndex::Alias(alias) => {
      let alias_name = strings.get(&alias).unwrap();
      arg=format!("{}{{{},",arg,alias_name);
    },
  }
  match column {
    TableIndex::None => arg=format!("{}-}}",arg),
    TableIndex::All => arg=format!("{}:}}",arg),
    TableIndex::Index(ix) => arg=format!("{}{}}}",arg,ix),
    TableIndex::Table(table) => {
      match table {
        TableId::Global(id) => arg=format!("{}#{}",arg,strings.get(&id).unwrap()),
        TableId::Local(id) => {
          match strings.get(&id) {
            Some(name) => arg = format!("{}{}",arg,name),
            None => arg = format!("{}{}",arg,humanize(&id)),
          }
        }
      };
    }
    TableIndex::Alias(alias) => {
      match strings.get(&alias) {
        Some(alias_name) => arg=format!("{}{}}}",arg,alias_name),
        None => arg=format!("{}{}}}",arg,&humanize(&alias)),
      };
      
    },
  }
  arg

}

fn format_transformation(block: &Block, tfm: &Transformation) -> String {
  match tfm {
    Transformation::TableAlias{table_id, alias} => {
      let mut tfm = format!("table/alias(");
      match table_id {
        TableId::Global(id) => {
          let name = match block.store.strings.get(id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(id)),
          };
          tfm=format!("{}#{}",tfm,name)
        },
        TableId::Local(id) => {
          match block.store.strings.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}{}",tfm,humanize(id)),
          }
        }
      };
      let alias_string = match block.store.strings.get(alias) {
        Some(name) => name.clone(),
        None => format!("{:}",humanize(alias)),
      };
      let mut tfm = format!("{}) -> {}",tfm, alias_string);
      tfm
    }
    Transformation::NewTable{table_id, rows, columns} => {
      let mut tfm = format!("table/new(");
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
      tfm = format!("{} {} x {})",tfm,rows,columns);
      tfm
    }
    Transformation::Whenever{table_id, row, column, ..} => {
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
        TableIndex::None => arg=format!("{}{{-,",arg),
        TableIndex::All => arg=format!("{}{{:,",arg),
        TableIndex::Index(ix) => arg=format!("{}{{{},",arg,ix),
        TableIndex::Table(table) => {
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
        TableIndex::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match column {
        TableIndex::None => arg=format!("{}-}}",arg),
        TableIndex::All => arg=format!("{}:}}",arg),
        TableIndex::Index(ix) => arg=format!("{}{}}}",arg,ix),
        TableIndex::Table(table) => {
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
        TableIndex::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}{}}}",arg,alias_name);
        },
      }
      arg
    }
    Transformation::Constant{table_id, value, ..} => {
      let mut tfm = format!("const(");
      match value.as_quantity() {
        Some(_quantity) => tfm = format!("{}{:?}", tfm, value),
        None => {
          if value.is_empty() {
            tfm = format!("{} _",tfm);
          } else {
            match value.as_reference() {
              Some(_reference) => {tfm = format!("{}@{}",tfm, humanize(value));}
              None => {
                match value.as_bool() {
                  Some(true) => tfm = format!("{} true",tfm),
                  Some(false) => tfm = format!("{} false",tfm),
                  None => {
                    match value.as_string() {
                      Some(_string_hash) => {
                        tfm = format!("{}{:?}",tfm, block.store.strings.get(value).unwrap());
                      }
                      None => {
                        match block.store.number_literals.get(value) {
                          Some(number_literal) => {
                            match number_literal.kind {
                              NumberLiteralKind::Hexadecimal => {
                                tfm = format!("{}0x",tfm);
                                for byte in &number_literal.bytes {
                                  tfm = format!("{}{:x}",tfm, byte);
                                }
                              }
                              NumberLiteralKind::Binary => {
                                tfm = format!("{}0b",tfm);
                                for byte in &number_literal.bytes {
                                  tfm = format!("{}{:b}",tfm, byte);
                                }
                              }
                              NumberLiteralKind::Octal => {
                                tfm = format!("{}0o",tfm);
                                for byte in &number_literal.bytes {
                                  tfm = format!("{}{:o}",tfm, byte);
                                }
                              }
                              NumberLiteralKind::Decimal => {
                                tfm = format!("{}0d",tfm);
                                for byte in &number_literal.bytes {
                                  tfm = format!("{}{:}",tfm, byte);
                                }
                              }
                            }
                          },
                          None => {
                            format!("{}0x{:0x}",tfm, value & 0x00FFFFFFFFFFFFFF);
                          }
                        };
                      }
                    }
                  }
                }
              }
            }

          }
        },
      }
      tfm = format!("{}) -> ",tfm);
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
        TableId::Global(id) => {
          tfm = match block.store.strings.get(id) {
            Some(string) => format!("{}#{}",tfm,string),
            None => humanize(&id),
          };
        } 
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
    Transformation::Select{table_id, row, column, indices, out} => {
      let mut tfm = format!("table/select(");
      match table_id {
        TableId::Global(id) => {
          let name = match block.store.strings.get(id) {
            Some(name) => name.clone(),
            None => format!("{:}",humanize(id)),
          };
          tfm=format!("{}#{}",tfm,name)
        },
        TableId::Local(id) => {
          match block.store.strings.get(id) {
            Some(name) => tfm = format!("{}{}",tfm,name),
            None => tfm = format!("{}{}",tfm,humanize(id)),
          }
        }
      };
      for (row, column) in indices {
        match row {
          TableIndex::None => tfm=format!("{}{{-,",tfm),
          TableIndex::All => tfm=format!("{}{{:,",tfm),
          TableIndex::Index(ix) => tfm=format!("{}{{{},",tfm,ix),
          TableIndex::Table(table) => {
            match table {
              TableId::Global(id) => tfm=format!("{}{{#{},",tfm,block.store.strings.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.strings.get(id) {
                  Some(name) => {
                    tfm = format!("{}{{{},",tfm,name);
                  },
                  None => tfm = format!("{}{{{},",tfm,humanize(id)),
                }
              }
            };
          }
          TableIndex::Alias(alias) => {
            let alias_name = block.store.strings.get(alias).unwrap();
            tfm=format!("{}{{{},",tfm,alias_name);
          },
        }
        match column {
          TableIndex::None => tfm=format!("{}-}}",tfm),
          TableIndex::All => tfm=format!("{}:}}",tfm),
          TableIndex::Index(ix) => tfm=format!("{}{}}}",tfm,ix),
          TableIndex::Table(table) => {
            match table {
              TableId::Global(id) => tfm=format!("{}#{}",tfm,block.store.strings.get(id).unwrap()),
              TableId::Local(id) => {
                match block.store.strings.get(id) {
                  Some(name) => tfm = format!("{}{}",tfm,name),
                  None => tfm = format!("{}{}",tfm,humanize(id)),
                }
              }
            };
          }
          TableIndex::Alias(alias) => {
            let alias_name = block.store.strings.get(alias).unwrap();
            tfm=format!("{}.{}}}",tfm,alias_name);
          },
        }
      }
      tfm=format!("{}) -> {}", tfm, humanize(&out.unwrap()));
      tfm
    }
    Transformation::Function{name, arguments, out} => {
      let name_string = match block.store.strings.get(name) {
        Some(name_string) => name_string.clone(),
        None => format!("{}", humanize(name)),
      };
      let mut arg = format!("");
      for (ix,(_arg_id, table, row, column)) in arguments.iter().enumerate() {
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
          TableIndex::None => arg=format!("{}{{-,",arg),
          TableIndex::All => arg=format!("{}{{:,",arg),
          TableIndex::Index(ix) => arg=format!("{}{{{},",arg,ix),
          TableIndex::Table(table) => {
            match table {
              TableId::Global(id) => arg=format!("{}{{#{},",arg,block.store.strings.get(id).unwrap()),
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
          TableIndex::Alias(alias) => {
            let alias_name = block.store.strings.get(alias).unwrap();
            arg=format!("{}{{{},",arg,alias_name);
          },
        }
        match column {
          TableIndex::None => arg=format!("{}-}}",arg),
          TableIndex::All => arg=format!("{}:}}",arg),
          TableIndex::Index(ix) => arg=format!("{}{}}}",arg,ix),
          TableIndex::Table(table) => {
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
          TableIndex::Alias(alias) => {
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
        TableIndex::None => arg=format!("{}{{-,",arg),
        TableIndex::All => arg=format!("{}{{:,",arg),
        TableIndex::Index(ix) => arg=format!("{}{{{},",arg,ix),
        TableIndex::Table(table) => {
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
        TableIndex::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}{{{},",arg,alias_name);
        },
      }
      match out_column {
        TableIndex::None => arg=format!("{}-}}",arg),
        TableIndex::All => arg=format!("{}:}}",arg),
        TableIndex::Index(ix) => arg=format!("{}{}}}",arg,ix),
        TableIndex::Table(table) => {
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
        TableIndex::Alias(alias) => {
          let alias_name = block.store.strings.get(alias).unwrap();
          arg=format!("{}.{}}}",arg,alias_name);
        },
      }
      arg
    },
    x => format!("{:?}", x),
  }
}
*/