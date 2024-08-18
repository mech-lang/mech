// # Block

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape
// them or perform computations on them. Blocks can be thought of as pure functions where the input and
// output are tables. Blocks have their own internal table store. Local tables can be defined within a
// block, which allows the programmer to break a computation down into steps. The result of the computation
// is then output to one or more global tables, which triggers the execution of other blocks in the network.

// ## Prelude

use crate::*;
#[cfg(feature = "stdlib")]
use crate::function::{
  table::*,
  math::*,
  math_update::*,
};
use std::cell::RefCell;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use std::fmt;
use serde::Serialize;
use std::mem::transmute;
use std::convert::TryInto;

lazy_static! {
  pub static ref cF32L: u64 = hash_str("f32-literal");
  pub static ref cF32: u64 = hash_str("f32");
  pub static ref cF64: u64 = hash_str("f64");
  pub static ref cU8: u64 = hash_str("u8");
  pub static ref cU16: u64 = hash_str("u16");
  pub static ref cU32: u64 = hash_str("u32");
  pub static ref cU64: u64 = hash_str("u64");
  pub static ref cU128: u64 = hash_str("u128");
  pub static ref cI8: u64 = hash_str("i8");
  pub static ref cI16: u64 = hash_str("i16");
  pub static ref cI32: u64 = hash_str("i32");
  pub static ref cI64: u64 = hash_str("i64");
  pub static ref cI128: u64 = hash_str("i128");
  pub static ref cHZ: u64 = hash_str("hz");
  pub static ref cMS: u64 = hash_str("ms");
  pub static ref cS: u64 = hash_str("s");
  pub static ref cM: u64 = hash_str("m");
  pub static ref cM_S: u64 = hash_str("m/s");
  pub static ref cKM: u64 = hash_str("km");
  pub static ref cHEX: u64 = hash_str("hex");
  pub static ref cDEC: u64 = hash_str("dec");
  pub static ref cSTRING: u64 = hash_str("string");
  pub static ref cBOOL: u64 = hash_str("bool");
  pub static ref cANY: u64 = hash_str("_");
}

/*
Defines a Plan struct which represents a sequence of Mech functions to be 
executed in a specific order. It provides methods for manipulating and 
executing the plan, such as adding new functions to the plan and solving the 
plan by iterating over each function in order. The Plan struct is designed to 
help organize and execute functions in a predictable and controlled manner, 
making it easier to reason about program behavior. 
*/

#[derive(Clone)]
pub struct Plan{
  pub plan: Vec<Rc<dyn MechFunction>>
}

impl Plan {
  pub fn new () -> Plan {
    Plan {
      plan: Vec::new(),
    }
  }
  pub fn push<S: MechFunction + 'static>(&mut self, mut fxn: S) {
    fxn.solve();
    self.plan.push(Rc::new(fxn));
  }

  pub fn solve(&self) {
    for fxn in &self.plan {
      fxn.solve();
    }
  }

}

impl fmt::Debug for Plan {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut plan = BoxPrinter::new();
    let mut ix = 1;
    for step in &self.plan {
      plan.add_title("🦿",&format!("Step {}", ix));
      plan.add_line(format!("{}",&step.to_string()));
      ix += 1;
    }
    write!(f,"{:?}",plan)?;
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
  Pending,      // THe block is currently solving and the state is not settled
  Disabled,     // The block is disabled will not execute if it otherwise would
}

// ## Block

/*
Defines the Block, which represents a single unit of computation in Mech. Block instances are created and managed by the Core. When a Mech program is executed, the Core creates a set of Block instances based on the program's source, and schedules those blocks to execute based on their data dependencies. When a block writes data to a table, it can trigger other blocks to execute based on their dependencies. These dependencies are managed by the Core, which tracks which blocks read from and write to which tables, and schedules their execution accordingly. The Core then manages the execution of these Block instances, ensuring that each Block receives input when needed and produces output as expected.

This dataflow architecture means that each block operates independently, reading data from tables and producing output as necessary. This allows for a high degree of concurrency and makes it easier to reason about program behavior. At the same time, the Core ensures that the program as a whole is executed correctly by coordinating the interactions between the various blocks and tables.
*/

#[derive(Clone)]
pub struct Block {
  pub id: BlockId,
  pub ast: nodes::AstNode,
  pub state: BlockState,
  pub tables: Database,
  pub plan: Plan,
  pub functions: Option<Rc<RefCell<core::Functions>>>,
  pub user_functions: Option<Rc<RefCell<HashMap<u64,UserFunction>>>>,
  pub global_database: Rc<RefCell<Database>>,
  pub unsatisfied_transformation: Option<(MechError,Transformation)>,
  pub pending_transformations: Vec<Transformation>,
  pub pending_global_tables: HashMap<u64,TableRef>,
  pub transformations: Vec<Transformation>,
  pub defined_tables: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
  pub required_functions: HashSet<u64>,
  pub strings: StringDictionary,
  pub triggers: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
  pub input: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
  pub output: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
  pub dynamic_tables: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      ast: nodes::AstNode::Null,
      state: BlockState::New,
      tables: Database::new(),
      plan: Plan::new(),
      functions: None,
      user_functions: None,
      required_functions: HashSet::new(),
      global_database: Rc::new(RefCell::new(Database::new())),
      unsatisfied_transformation: None,
      pending_transformations: Vec::new(),
      pending_global_tables: HashMap::new(),
      defined_tables: HashSet::new(),
      transformations: Vec::new(),
      strings: Rc::new(RefCell::new(HashMap::new())),
      triggers: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      dynamic_tables: HashSet::new(),
    }
  }

  pub fn get_table(&self, table_id: &TableId) -> Result<Rc<RefCell<Table>>, MechError> {
    match &table_id {
      TableId::Local(id) => match self.tables.get_table_by_id(id) {
        Some(table) => Ok(table.clone()),
        None => {
          match self.tables.table_alias_to_id.get(table_id.unwrap()) {
            Some(TableId::Global(id)) => match self.global_database.borrow().get_table_by_id(id) {
              Some(table) => Ok(table.clone()),
              None => {
                let table_name: String = self.strings.borrow().get(table_id.unwrap()).unwrap().to_string();
                Err(MechError{tokens: vec![], msg: table_name, id: 2101, kind: MechErrorKind::MissingTable(*table_id)})
              }
            }
            Some(TableId::Local(id)) => match self.tables.get_table_by_id(id) {
              Some(table) => Ok(table.clone()),
              None => {
                let table_name: String = self.strings.borrow().get(table_id.unwrap()).unwrap().to_string();
                Err(MechError{tokens: vec![], msg: table_name, id: 2102, kind: MechErrorKind::MissingTable(*table_id)})
              }
            }
            None => {
              let table_name: String = match self.strings.borrow().get(table_id.unwrap()) {
                Some(table_name) => table_name.to_string(),
                None => humanize(table_id.unwrap()),
              };
              Err(MechError{tokens: vec![], msg: table_name, id: 2103, kind: MechErrorKind::MissingTable(*table_id)})
            }
          }
        },
      },
      TableId::Global(id) => match self.global_database.borrow().get_table_by_id(id) {
        Some(table) => Ok(table.clone()),
        None => {
          match self.pending_global_tables.get(id) {
            Some(table) => Ok(table.clone()),
            None => {
              let table_name: String = self.strings.borrow().get(table_id.unwrap()).unwrap().to_string();
              Err(MechError{tokens: vec![], msg: table_name, id: 2104, kind: MechErrorKind::MissingTable(*table_id)})
            }
          }
        }
      }
    }
  }

  pub fn gen_id(&mut self) -> BlockId {
    self.id = hash_str(&format!("{:?}",self.transformations));
    self.id
  }

  pub fn id(&self) -> BlockId {
    self.id
  }
  
  // Clear everything and recompile it all
  pub fn recompile(&mut self) -> Result<(),MechError> {
    let tfms = self.transformations.clone();
    self.transformations.clear();
    self.plan = Plan::new();
    self.tables.clear();
    for tfm in tfms {
      self.compile_tfm(tfm)?;
    }
    Ok(())
  }

  pub fn ready(&mut self) -> Result<(),MechError> {
    match self.state {
      // If the state is ready, we are good.
      BlockState::Ready => Ok(()),
      // If the block is disable that'd the end of it
      BlockState::Disabled => Err(MechError{tokens: vec![], msg: "".to_string(), id: 2144, kind: MechErrorKind::BlockDisabled}),
      // Other
      BlockState::New | BlockState::Done | BlockState::Unsatisfied |  BlockState::Error |  
      BlockState::Pending => {
        match &self.unsatisfied_transformation {
          Some((_,tfm)) => {
            self.state = BlockState::Pending;
            match self.add_tfm(tfm.clone()) {
              Ok(_) => {
                self.unsatisfied_transformation = None;
                let mut pending_transformations = self.pending_transformations.clone();
                self.pending_transformations.clear();
                for tfm in pending_transformations.drain(..) {
                  self.add_tfm(tfm);
                }
                self.ready()
              }
              Err(x) => {
                Err(x)
              },
            }
          }
          None => {
            self.state = BlockState::Ready;
            for (_,table_ref) in self.pending_global_tables.drain() {
              self.global_database.borrow_mut().insert_table_ref(table_ref);
            }
            Ok(())
          }
        }
      }
    }
  }

  pub fn get_arg_column(&self, argument: &Argument) -> Result<(u64,Column,ColumnIndex),MechError> {

    let (arg_name, table_id, indices) = argument;

    // This part handles multi-dimensional indexing e.g. {1,2}{3,4}{5,6}
    let mut table_id = *table_id;
    for (row,column) in indices.iter().take(indices.len()-1) {
      let argument = (0,table_id,vec![(row.clone(),column.clone())]);
      match self.get_arg_dim(&argument)? {
        TableShape::Scalar => {
          let arg_col = self.get_arg_column(&argument)?;
          match (arg_col,row,column) {
            ((_,Column::Ref(ref_col),ColumnIndex::Index(row)),_,TableIndex::None) => {
              table_id = ref_col.borrow()[row].clone();
            }
            ((_,Column::Ref(ref_col),_),TableIndex::Index(row_ix),_) => {
              table_id = ref_col.borrow()[row_ix-1].clone();
            }
            ((_,Column::Ref(ref_col),_),_,_) => {
              table_id = ref_col.borrow()[0].clone();
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2105, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
        x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2106, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    }

    // get the table
    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let table_brrw = table.borrow(); 

    // Get the column and row
    match (row,col) {
      // Return an error if either index is 0. An index can never be zero
      (_,TableIndex::Index(ix)) |
      (TableIndex::Index(ix),_) if ix == &0 => {
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2107, kind: MechErrorKind::ZeroIndex});
      }
      // x{1,1}
      (TableIndex::Index(row),TableIndex::Index(_)) => {
        let col = table_brrw.get_column(&col)?;
        Ok((*arg_name,col.clone(),ColumnIndex::Index(row-1)))
      }
      // x.y{1}
      (TableIndex::Index(ix),TableIndex::Alias(col_alias)) => {
        let arg_col = table_brrw.get_column(col)?;
        Ok((*arg_name,arg_col.clone(),ColumnIndex::Index(*ix-1)))
      }
      // x{1}
      (TableIndex::Index(ix),TableIndex::None) => {
        let (ix_row,ix_col) = table_brrw.index_to_subscript(ix-1)?;
        let col = table_brrw.get_column(&TableIndex::Index(ix_col + 1))?;
        Ok((*arg_name,col.clone(),ColumnIndex::Index(ix_row)))
      }
      // x{z,1}
      // x.y{z}
      (TableIndex::IxTable(ix_table_id),TableIndex::Index(_)) |
      (TableIndex::IxTable(ix_table_id),TableIndex::Alias(_))  => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        if ix_table_brrw.cols > 1 {
          return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2108, kind: MechErrorKind::GenericError("Index table must be a column vector.".to_string())});
        }
        let ix = match ix_table_brrw.get_column_unchecked(0) {
          Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
          Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
          Column::U8(ix_col) => ColumnIndex::IndexColU8(ix_col),
          Column::F32(ix_col) => {
            ColumnIndex::RealIndex(ix_col)
          },
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2109, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        };
        let arg_col = table_brrw.get_column(col)?;
        Ok((*arg_name,arg_col.clone(),ix))
      }
      // x{z}
      (TableIndex::IxTable(ix_table_id),TableIndex::None) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        match table.borrow().kind() {
          ValueKind::Compound(table_kind) => {
            return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2110, kind: MechErrorKind::GenericError("Can't handle compound".to_string())});
          }
          table_kind => {
            let ix = match ix_table_brrw.get_column_unchecked(0) {
              Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
              Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
              Column::U8(ix_col) => ColumnIndex::Index(ix_col.borrow()[0].unwrap() as usize - 1),
              Column::F32(ix_col) => {
                ColumnIndex::RealIndex(ix_col)
              },
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2111, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            };
            match table_brrw.shape() {
              TableShape::Column(rows) => {
                let col = table_brrw.get_column_unchecked(0);
                Ok((*arg_name,col.clone(),ix))
              }
              _ => Ok((*arg_name,Column::Reference((table.clone(),(ix,ColumnIndex::None))),ColumnIndex::All)),
            }
          }
        }
      }
      // x{:,1}
      (TableIndex::All, TableIndex::Index(col_ix)) => {
        let col = table_brrw.get_column(&TableIndex::Index(*col_ix))?;
        Ok((*arg_name,col.clone(),ColumnIndex::All))
      },
      // x{:,:}
      (TableIndex::All, TableIndex::All) => {
        if table_brrw.cols > 1 {
          let reference = Column::Reference((table.clone(),(ColumnIndex::All,ColumnIndex::All)));
          return Ok((*arg_name,reference,ColumnIndex::All));
        }
        let col = table_brrw.get_column(&col)?;
        if col.len() == 1 {
          Ok((*arg_name,col.clone(),ColumnIndex::Index(0)))
        } else {
          Ok((*arg_name,col.clone(),ColumnIndex::All))
        }
      }
      _ => {
        let col = table_brrw.get_column(&col)?;
        if col.len() == 1 {
          Ok((*arg_name,col.clone(),ColumnIndex::Index(0)))
        } else {
          Ok((*arg_name,col.clone(),ColumnIndex::All))
        }
      }
    }
  }

  pub fn get_arg_columns(&self, arguments: &Vec<Argument>) -> Result<Vec<(u64,Column,ColumnIndex)>,MechError> {
    let mut argument_columns = vec![];
    for argument in arguments {
      let arg_col = self.get_arg_column(argument)?;
      argument_columns.push(arg_col);
    }
    Ok(argument_columns)
  }

  pub fn get_whole_table_arg_cols(&self, argument: &Argument) -> Result<Vec<(u64,Column,ColumnIndex)>,MechError> {
    
    let (arg_name,table_id,indices) = argument;
    let mut table_id = *table_id;
    for (row,column) in indices.iter().take(indices.len()-1) {
      let argument = (0,table_id,vec![(row.clone(),column.clone())]);
      match self.get_arg_dim(&argument)? {
        TableShape::Scalar => {
          let arg_col = self.get_arg_column(&argument)?;
          match (arg_col,row,column) {
            ((_,Column::Ref(ref_col),_),_,TableIndex::None) => {
              table_id = ref_col.borrow()[0].clone();
            }
            ((_,Column::Ref(ref_col),_),TableIndex::Index(row_ix),_) => {
              table_id = ref_col.borrow()[row_ix-1].clone();
            }
            ((_,Column::Ref(ref_col),_),_,_) => {
              table_id = ref_col.borrow()[0].clone();
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2211, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
        x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2112, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    }
    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let table_brrw = table.borrow();
    /*match self.get_arg_dim(argument) {
      Ok(TableShape::Column(rows)) => {
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2113, kind: MechErrorKind::GenericError("Can't handle this unless it's a column vector.".to_string())});
      }
      _ => (),
    };*/
    let row_index = match (row,col) {
      (TableIndex::ReshapeColumn,_) => ColumnIndex::ReshapeColumn,
      (TableIndex::All,_) => ColumnIndex::All,
      (TableIndex::None,_) => ColumnIndex::None,
      (TableIndex::Index(ix),_) => ColumnIndex::Index(ix - 1),
      (TableIndex::Alias(alias),_) => {
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2114,  kind: MechErrorKind::GenericError("Can't index on row alias yet".to_string())});
      },
      (TableIndex::IxTable(ix_table_id),_) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        let ix = match ix_table_brrw.get_column_unchecked(0) {
          Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
          Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
          Column::U8(ix_col) => ColumnIndex::Index(ix_col.borrow()[0].unwrap() as usize - 1),
          Column::F32(ix_col) => ColumnIndex::Index(ix_col.borrow()[0].unwrap() as usize - 1),
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2115, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        };
        ix
      }
      _ => ColumnIndex::All,
    };
    let arg_cols = table_brrw.get_columns(&col)?.iter().map(|arg_col| (*arg_name,arg_col.clone(),row_index.clone())).collect();
    Ok(arg_cols)
  }


  pub fn get_out_column(&self, out: &Out, rows: usize, col_kind: ValueKind) -> Result<Column,MechError> {
    let (out_table_id, _, _) = out;
    let table = self.get_table(out_table_id)?;
    let mut t = table.borrow_mut();
    let cols = if t.cols == 0 { 1 } else { t.cols };
    let rows = if rows == 0 { 1 } else { rows };
    t.resize(rows,cols);
    t.set_col_kind(0, col_kind)?;
    let column = t.get_column_unchecked(0);
    Ok(column)
  }
 
  pub fn get_arg_dims(&self, arguments: &Vec<Argument>) -> Result<Vec<TableShape>,MechError> {
    let mut arg_shapes = Vec::new();
    for argument in arguments {
      arg_shapes.push(self.get_arg_dim(argument)?);
    }
    Ok(arg_shapes)
  }

  pub fn get_arg_dim(&self, argument: &Argument) -> Result<TableShape,MechError> {
    let (_, table_id, indices) = argument;
    let mut table_id = *table_id;
    for (row,column) in indices.iter().take(indices.len()-1) {
      let argument = (0,table_id,vec![(row.clone(),column.clone())]);
      match self.get_arg_dim(&argument)? {
        TableShape::Scalar => {
          let arg_col = self.get_arg_column(&argument)?;
          match (arg_col,row,column) {
            ((_,Column::Ref(ref_col),_),_,TableIndex::None) => {
              table_id = ref_col.borrow()[0].clone();
            }
            ((_,Column::Ref(ref_col),_),TableIndex::Index(row_ix),_) => {
              table_id = ref_col.borrow()[row_ix-1].clone();
            }
            ((_,Column::Ref(ref_col),_),_,_) => {
              table_id = ref_col.borrow()[0].clone();
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2116, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    
          }
        }
        x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2117, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    
      }
    }
    
    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let table_brrw = table.borrow();
    let mut dynamic = table_brrw.dynamic;
    let dim = match (row,col) {
      (TableIndex::ReshapeColumn, TableIndex::All) => (table_brrw.rows*table_brrw.cols,1),
      (TableIndex::All, TableIndex::All) => (table_brrw.rows, table_brrw.cols),
      (TableIndex::All, TableIndex::None) => (table_brrw.rows*table_brrw.cols,1),
      (TableIndex::All,TableIndex::Index(_)) |
      (TableIndex::All, TableIndex::Alias(_)) => (table_brrw.rows, 1),
      (TableIndex::Index(_),TableIndex::None) |
      (TableIndex::Index(_),TableIndex::Index(_)) |
      (TableIndex::Index(_),TableIndex::Alias(_)) => {
        dynamic = false;
        (1,1)
      },
      (TableIndex::Index(_),TableIndex::All) => (1,table_brrw.cols),
      (TableIndex::IxTable(ix_table_id),TableIndex::Alias(_)) |
      (TableIndex::IxTable(ix_table_id),TableIndex::None) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let rows = ix_table.borrow().len();
        (rows,1)
      },
      (TableIndex::IxTable(ix_table_id),TableIndex::All) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let rows = ix_table.borrow().len();
        (rows,table_brrw.cols)
      },
      (TableIndex::All,TableIndex::IxTable(ix_table_id)) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        let rows = ix_table_brrw.len();
        let cols = match ix_table_brrw.kind() {
          ValueKind::Bool => {
            ix_table_brrw.logical_len()
          }
          ValueKind::String => {
            ix_table_brrw.len()
          }
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2128, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    
        };
        (table_brrw.rows, cols)
      }
      (TableIndex::All,TableIndex::Aliases(aliases)) => {
        (table_brrw.rows,aliases.len())
      }
      x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2118, kind: MechErrorKind::GenericError(format!("{:?}", x))});},    
    };
    let arg_shape = match (dim,dynamic) {
      ((_,0),_) |
      ((0,_),_) |
      ((0,0),_) => TableShape::Pending(table_id),
      ((1,1),false) => TableShape::Scalar,
      ((1,x),false) => TableShape::Row(x),
      ((x,1),false) => TableShape::Column(x),
      ((x,y),false) => TableShape::Matrix(x,y),
      ((x,y),true) => TableShape::Dynamic(x,y),
      _ => TableShape::Pending(table_id),
    };
    Ok(arg_shape)
  }

  pub fn add_tfm(&mut self, tfm: Transformation) -> Result<(),MechError> {
    self.init_registers(&tfm);
    match self.state {
      BlockState::Unsatisfied => {
        self.pending_transformations.push(tfm.clone());
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2119, kind: MechErrorKind::GenericError("Unsatisfied block".to_string())});
      }
      _ => {
        match self.compile_tfm(tfm.clone()) {
          Ok(()) => (),
          Err(mech_error) => {
            self.unsatisfied_transformation = Some((mech_error.clone(),tfm));
            self.state = BlockState::Unsatisfied;
            return Err(mech_error);        
          }
        }
      }
    }
    Ok(())
  }

  fn init_registers(&mut self, tfm: &Transformation) {
    match tfm {
      Transformation::TableDefine{table_id, indices, out} => {
        if let TableId::Global(_) = table_id { 
          self.input.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
          self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
        }
      }
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        if let TableId::Global(_) = table_id { 
          self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
          self.input.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
          self.output.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
        }
      }
      Transformation::Function{name, ref arguments, out} => {
        self.required_functions.insert(*name);
        for (_,table_id,indices) in arguments {
          if let TableId::Global(_) = table_id {
            let register = (*table_id,RegisterIndex::All,RegisterIndex::All);
            self.input.insert(register);
            self.triggers.insert(register);
          }
          for (row_ix, col_ix) in indices {
            match row_ix {
              TableIndex::IxTable(TableId::Global(ix_table_id)) => {
                let register = (TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All);
                self.input.insert(register);
                self.triggers.insert(register);
              }
              _ => (),
            }
            match col_ix {
              TableIndex::IxTable(TableId::Global(ix_table_id)) => {
                let register = (TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All);
                self.input.insert(register);
                self.triggers.insert(register);              }
              _ => (),
            }
          }
        }
        if let (TableId::Global(table_id),_,_) = out {
          self.output.insert((TableId::Global(*table_id),RegisterIndex::All,RegisterIndex::All));
        }
      }
      Transformation::Whenever{table_id, indices} => {
        self.triggers.clear();
        self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
      }
      _ => (),
    }
  }

  fn compile_tfm(&mut self, tfm: Transformation) -> Result<(), MechError> {
    self.init_registers(&tfm);
    match &tfm {
      Transformation::Identifier{name, id} => {
        self.strings.borrow_mut().insert(*id, MechString::from_chars(name));
      }
      Transformation::NewTable{table_id, rows, columns} => {
        match table_id {
          TableId::Local(id) => {
            let mut table = Table::new(*id, 0, 0);
            table.dictionary = self.strings.clone();
            self.tables.insert_table(table);
          }
          TableId::Global(id) => {
            let mut table = Table::new(*id, 0, 0);
            table.dictionary = self.strings.clone();
            let table = Rc::new(RefCell::new(table));
            self.pending_global_tables.insert(*table_id.unwrap(),table);
            self.output.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
            self.defined_tables.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
          }
        } 
      },
      Transformation::TableReference{table_id, reference} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        table_brrw.resize(1,1);
        table_brrw.set_kind(ValueKind::Reference);
        table_brrw.set_raw(0,0,reference.clone())?;

        let src_table = self.get_table(&reference.as_table_reference()?)?;
        let src_table_brrw = src_table.borrow();
        let src_id = src_table_brrw.id;
        let rows = src_table_brrw.rows;
        let cols = src_table_brrw.cols;
        let dest_table = Table::new(src_id,rows,cols);
        {
          self.global_database.borrow_mut().insert_table(dest_table);
        }
      }
      Transformation::TableAlias{table_id, alias} => {
        self.tables.insert_alias(*alias, *table_id)?;
      },
      Transformation::ColumnKind{table_id, column_ix, kind} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        if *kind == *cU8 { table_brrw.set_col_kind(*column_ix,ValueKind::U8)?; }
        else if *kind == *cU16 { table_brrw.set_col_kind(*column_ix,ValueKind::U16)?; }
        else if *kind == *cU32 { table_brrw.set_col_kind(*column_ix,ValueKind::U32)?; }
        else if *kind == *cU64 { table_brrw.set_col_kind(*column_ix,ValueKind::U64)?; }
        else if *kind == *cU128 { table_brrw.set_col_kind(*column_ix,ValueKind::U128)?; }
        else if *kind == *cI8 { table_brrw.set_col_kind(*column_ix,ValueKind::I8)?; }
        else if *kind == *cI16 { table_brrw.set_col_kind(*column_ix,ValueKind::I16)?; }
        else if *kind == *cI32 { table_brrw.set_col_kind(*column_ix,ValueKind::I32)?; }
        else if *kind == *cI64 { table_brrw.set_col_kind(*column_ix,ValueKind::I64)?; }
        else if *kind == *cI128 { table_brrw.set_col_kind(*column_ix,ValueKind::I128)?; }
        else if *kind == *cF32 { table_brrw.set_col_kind(*column_ix,ValueKind::F32)?; }
        else if *kind == *cF32L { table_brrw.set_col_kind(*column_ix,ValueKind::F32)?; }
        else if *kind == *cM { table_brrw.set_col_kind(*column_ix,ValueKind::Length)?; }
        else if *kind == *cKM { table_brrw.set_col_kind(*column_ix,ValueKind::Length)?; }
        else if *kind == *cS { table_brrw.set_col_kind(*column_ix,ValueKind::Time)?; }
        else if *kind == *cMS { table_brrw.set_col_kind(*column_ix,ValueKind::Time)?; }
        else if *kind == *cSTRING { table_brrw.set_col_kind(*column_ix,ValueKind::String)?; }
        else if *kind == *cBOOL { table_brrw.set_col_kind(*column_ix,ValueKind::Bool)?; }
        else if *kind == *cM_S { table_brrw.set_col_kind(*column_ix,ValueKind::Speed)?; }
        else if *kind == *cANY { table_brrw.set_col_kind(*column_ix,ValueKind::Any)?; }
        else {
          return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2120, kind: MechErrorKind::UnknownColumnKind(*kind)});
        }
      }
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        if let TableId::Global(_) = table_id { 
          self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
          self.input.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
          self.output.insert((*table_id,RegisterIndex::All,RegisterIndex::Alias(*column_alias)));
        }
        let mut table = self.tables.get_table_by_id(table_id.unwrap()).unwrap().borrow_mut();
        if table.cols == 0 || *column_ix > (table.cols - 1)  {
          let rows = table.rows;
          table.resize(rows,*column_ix + 1);
        }
        table.set_col_alias(*column_ix,*column_alias);
      },
      Transformation::TableDefine{table_id, indices, out} => {
        if let TableId::Global(id) = table_id { 
          self.input.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
          self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
        }
        for (row_ix, col_ix) in indices {
          match row_ix {
            TableIndex::IxTable(TableId::Global(ix_table_id)) => {
              self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
            }
            _ => (),
          }
          match col_ix {
            TableIndex::IxTable(TableId::Global(ix_table_id)) => {
              self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
            }
            _ => (),
          }
        }
        // Compile a Table Define function
        let arguments =  vec![(0,table_id.clone(),indices.clone())];
        let out =  (*out, TableIndex::All, TableIndex::All);
        let fxns = self.functions.clone();
        match &fxns {
          Some(functions) => {
            let mut fxns = functions.borrow_mut();
            #[cfg(feature = "stdlib")]
            match fxns.get(*TABLE_DEFINE) {
              Some(fxn) => {
                fxn.compile(self,&arguments,&out)?;
              }
              None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2223, kind: MechErrorKind::MissingFunction(*TABLE_DEFINE)});},
            }
          }
          None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2224, kind: MechErrorKind::GenericError("No functions are loaded.".to_string())});},
        }
      }
      Transformation::Set{src_id, src_row, src_col, dest_id, dest_row, dest_col} => {
        self.output.insert((*dest_id,RegisterIndex::All,RegisterIndex::All));
        match src_row {
          TableIndex::IxTable(TableId::Global(ix_table_id)) => {
            self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
          }
          _ => (),
        }
        match dest_row {
          TableIndex::IxTable(TableId::Global(ix_table_id)) => {
            self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
          }
          _ => (),
        }
        #[cfg(feature = "stdlib")]
        self.compile_tfm(Transformation::Function{
          name: *TABLE_SET,
          arguments: vec![(0,*src_id,vec![(src_row.clone(), src_col.clone())])],
          out: (*dest_id,dest_row.clone(),dest_col.clone()),
        })?;
      }
      Transformation::UpdateData{name, src_id, src_row, src_col, dest_id, dest_row, dest_col} => {
        self.output.insert((*dest_id,RegisterIndex::All,RegisterIndex::All));
        match dest_row {
          TableIndex::IxTable(TableId::Global(ix_table_id)) => {
            self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
          }
          _ => (),
        }
        self.compile_tfm(Transformation::Function{
          name: *name,
          arguments: vec![(0,*src_id,vec![(src_row.clone(), src_col.clone())])],
          out: (*dest_id,dest_row.clone(),dest_col.clone()),
        })?;
      }
      Transformation::NumberLiteral{kind, bytes} => {
        let mut num = NumberLiteral::new(*kind, bytes.to_vec());
        let mut bytes = bytes.clone();
        let table_id = hash_str(&format!("{:?}{:?}", kind, bytes));
        let table =  self.get_table(&TableId::Local(table_id))?; 
        let mut table_brrw = table.borrow_mut();
        table_brrw.resize(1,1);
        if *kind == *cU8 {
          table_brrw.set_kind(ValueKind::U8)?;
          table_brrw.set_raw(0,0,Value::U8(U8::new(num.as_u8())))?;
        } 
        else if *kind == *cU16 {
          table_brrw.set_kind(ValueKind::U16)?;
          table_brrw.set_raw(0,0,Value::U16(U16::new(num.as_u16())))?;
        } 
        else if *kind == *cU32 {
          table_brrw.set_kind(ValueKind::U32)?;
          table_brrw.set_raw(0,0,Value::U32(U32::new(num.as_u32())))?;
        } 
        else if *kind == *cU64 {
          table_brrw.set_kind(ValueKind::U64)?;
          table_brrw.set_raw(0,0,Value::U64(U64::new(num.as_u64())))?;
        } 
        else if *kind == *cU128 {
          table_brrw.set_kind(ValueKind::U128)?;
          table_brrw.set_raw(0,0,Value::U128(U128::new(num.as_u128())))?;
        } 
        else if *kind == *cI8 {
          table_brrw.set_kind(ValueKind::I8)?;
          table_brrw.set_raw(0,0,Value::I8(I8::new(num.as_i8())))?;
        } 
        else if *kind == *cI16 {
          table_brrw.set_kind(ValueKind::I16)?;
          table_brrw.set_raw(0,0,Value::I16(I16::new(num.as_i16())))?;
        } 
        else if *kind == *cI32 {
          table_brrw.set_kind(ValueKind::I32)?;
          table_brrw.set_raw(0,0,Value::I32(I32::new(num.as_i32())))?;
        } 
        else if *kind == *cI64 {
          table_brrw.set_kind(ValueKind::I64)?;
          table_brrw.set_raw(0,0,Value::I64(I64::new(num.as_i64())))?;
        } 
        else if *kind == *cI128 {
          table_brrw.set_kind(ValueKind::I128)?;
          table_brrw.set_raw(0,0,Value::I128(I128::new(num.as_i128())))?;
        } 
        else if *kind == *cMS {
          table_brrw.set_kind(ValueKind::Time)?;
          table_brrw.set_raw(0,0,Value::Time(F32::new(num.as_f32() / 1000.0)))?;
        } 
        else if *kind == *cS {
          table_brrw.set_kind(ValueKind::Time)?;
          table_brrw.set_raw(0,0,Value::Time(F32::new(num.as_f32())))?;
        } 
        else if *kind == *cF32 {
          table_brrw.set_kind(ValueKind::F32)?;
          table_brrw.set_raw(0,0,Value::F32(F32::new(num.as_f32())))?;
        } 
        else if *kind == *cF64 {
          table_brrw.set_kind(ValueKind::F64)?;
          table_brrw.set_raw(0,0,Value::F64(F64::new(num.as_f64())))?;
        } 
        else if *kind == *cF32L {
          table_brrw.set_kind(ValueKind::F32)?;
          table_brrw.set_raw(0,0,Value::F32(F32::new(num.as_f32())))?;
        } 
        else if *kind == *cKM {
          table_brrw.set_kind(ValueKind::Length)?;
          table_brrw.set_raw(0,0,Value::Length(F32::new(num.as_f32() * 1000.0)))?;
        } 
        else if *kind == *cM {
          table_brrw.set_kind(ValueKind::Length)?;
          table_brrw.set_raw(0,0,Value::Length(F32::new(num.as_f32())))?;
        }
        else if *kind == *cM_S {
          table_brrw.set_kind(ValueKind::Speed)?;
          table_brrw.set_raw(0,0,Value::Speed(F32::new(num.as_f32())))?;
        }
        else if *kind == *cDEC {
          match bytes.len() {
            1 => {
              table_brrw.set_col_kind(0, ValueKind::U8)?;
              table_brrw.set_raw(0,0,Value::U8(U8::new(num.as_u8())))?;
            }
            2 => {
              table_brrw.set_kind(ValueKind::U16)?;
              table_brrw.set_raw(0,0,Value::U16(U16::new(num.as_u16())))?;
            }
            3 | 4 => {
              table_brrw.set_kind(ValueKind::U32)?;
              table_brrw.set_raw(0,0,Value::U32(U32::new(num.as_u32())))?;
            }
            5..=8 => {
              table_brrw.set_kind(ValueKind::U64)?;
              table_brrw.set_raw(0,0,Value::U64(U64::new(num.as_u64())))?;
            }
            9..=16 => {
              table_brrw.set_kind(ValueKind::U128)?;
              table_brrw.set_raw(0,0,Value::U128(U128::new(num.as_u128())))?;
            }
            _ => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2121, kind: MechErrorKind::GenericError("Too many bytes in number".to_string())});},
          }
        } 
        else if *kind == *cHEX {
          let mut x: u128 = 0;
          table_brrw.set_kind(ValueKind::U128)?;
          while bytes.len() < 16 {
            bytes.insert(0,0);
          }
          for half_byte in bytes {
            x = x << 4;
            x = x | half_byte as u128;
          }
          table_brrw.set_raw(0,0,Value::U128(U128::new(x)))?;
        }
        else {
          return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2122, kind: MechErrorKind::UnknownColumnKind(*kind)});
        }
      },
      Transformation::Constant{table_id, value} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        table_brrw.resize(1,1);
        match &value {
          Value::Bool(_) => {table_brrw.set_col_kind(0, ValueKind::Bool)?;},
          Value::String(_) => {table_brrw.set_col_kind(0, ValueKind::String)?;},
          _ => (),
        }
        table_brrw.set_raw(0,0,value.clone())?;
      }
      Transformation::Whenever{table_id, indices} => {
        self.triggers.clear();
        self.triggers.insert((*table_id,RegisterIndex::All,RegisterIndex::All));
      }
      Transformation::Function{name, ref arguments, out} => {   
        for (_,_,indices) in arguments { 
          for (row_ix, col_ix) in indices {
            match row_ix {
              TableIndex::IxTable(TableId::Global(ix_table_id)) => {
                self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
              }
              _ => (),
            }
            match col_ix {
              TableIndex::IxTable(TableId::Global(ix_table_id)) => {
                self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
              }
              _ => (),
            }
          }
        }
        // A list of all the functions that are
        // loaded onto this core.
        let fxns = self.functions.clone();
        match &fxns {
          Some(functions) => {
            let fxns_brrw = functions.borrow();
            match fxns_brrw.get(*name) {
              Some(fxn) => {
                // A function knows how to compile itself
                // based on what arguments are passed.
                // Not all arguments are valid, in which
                // case an error is returned.
                fxn.compile(self,&arguments,&out)?;
              }
              None => {
                // check if it's a user function instead
                let user_fxns = self.user_functions.clone();
                match &user_fxns {
                  Some(user_fxns) => {
                    let mut user_fxns_brrw = user_fxns.borrow();
                    match user_fxns_brrw.get(name) {
                      Some(fxn) => {
                        let compiled_fxn = fxn.compile(self,&arguments,&out)?;
                        self.plan.push(compiled_fxn);
                      },
                      None => return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2123, kind: MechErrorKind::MissingFunction(*name)}),
                    }
                  }
                  None => return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2124, kind: MechErrorKind::MissingFunction(*name)}),
                }
              },
            }
          }
          None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2125, kind: MechErrorKind::GenericError("No functions are loaded.".to_string())});},
        }
      }
      Transformation::FollowedBy{table_id, initial, subsequent} => {
        self.output.remove(&(*table_id,RegisterIndex::All,RegisterIndex::All));

        let indices = vec![(TableIndex::All, TableIndex::All)];
        if let TableId::Global(id) = subsequent { 
          self.input.insert((*subsequent,RegisterIndex::All,RegisterIndex::All));
          self.triggers.insert((*subsequent,RegisterIndex::All,RegisterIndex::All));
        }
        for (row_ix, col_ix) in &indices {
          match row_ix {
            TableIndex::IxTable(TableId::Global(ix_table_id)) => {
              self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
            }
            _ => (),
          }
          match col_ix {
            TableIndex::IxTable(TableId::Global(ix_table_id)) => {
              self.input.insert((TableId::Global(*ix_table_id),RegisterIndex::All,RegisterIndex::All));
            }
            _ => (),
          }
        }
        // Compile a Table Define function
        let arguments =  vec![(0,initial.clone(),indices.clone())];
        let out =  (*table_id, TableIndex::All, TableIndex::All);
        let fxns = self.functions.clone();
        match &fxns {
          Some(functions) => {
            let mut fxns = functions.borrow_mut();
            #[cfg(feature = "stdlib")]
            match fxns.get(*TABLE_DEFINE) {
              Some(fxn) => {
                fxn.compile(self,&arguments,&out)?;
                self.plan.plan.pop();
              }
              None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2423, kind: MechErrorKind::MissingFunction(*TABLE_DEFINE)});},
            }
            let arguments =  vec![(0,subsequent.clone(),indices.clone())];
            #[cfg(feature = "stdlib")]
            match fxns.get(*TABLE_FOLLOWED__BY) {
              Some(fxn) => {
                fxn.compile(self,&arguments,&out)?;
              }
              None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2424, kind: MechErrorKind::MissingFunction(*TABLE_DEFINE)});},
            }
          }
          None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 2425, kind: MechErrorKind::GenericError("No functions are loaded.".to_string())});},
        }
      }
      _ => (),
    }
    self.transformations.push(tfm.clone());
    Ok(())
  }
  
  pub fn solve(&self) -> Result<(),MechError> {
    if self.state == BlockState::Ready {
      // Solve synchronous functions
      for ref mut fxn in &mut self.plan.plan.iter() {
        fxn.solve();
      }
      Ok(())
    } else {
      Err(MechError{tokens: vec![], msg: "".to_string(), id: 2126, kind: MechErrorKind::GenericError("Block not ready".to_string())})
    }
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut block_drawing = BoxPrinter::new();
    block_drawing.add_title("🧊","BLOCK");
    block_drawing.add_line(format!("id: {}", humanize(&self.id)));
    block_drawing.add_line(format!("state: {:?}", &self.state));
    block_drawing.add_title("🌳","Ast");
    block_drawing.add_line(format!("{:?}", &self.ast));

    block_drawing.add_title("⚙️",&format!("triggers ({})",self.triggers.len()));
    if self.triggers.len() > 0 {
      for (table,row,col) in &self.triggers {
        let table_name: String = if let TableId::Global(table_id) = table {
          match self.strings.borrow().get(table_id) {
            Some(s) => s.to_string(),
            None => format!("{:?}",table),
          }
        } else {
          format!("{:?}",table)
        };
        block_drawing.add_line(format!("  - #{}{{{:?}, {:?}}}", table_name,row,col));
      }
    }
    block_drawing.add_title("📭",&format!("input ({})",self.input.len()));
    if self.input.len() > 0 {
      for (table,row,col) in &self.input {
        let table_name: String = if let TableId::Global(table_id) = table {
          match self.strings.borrow().get(table_id) {
            Some(s) => s.to_string(),
            None => format!("{:?}",table),
          }
        } else {
          format!("{:?}",table)
        };
        block_drawing.add_line(format!("  - #{}{{{:?}, {:?}}}", table_name,row,col));
      }
    }
    block_drawing.add_title("📬",&format!("output ({})",self.output.len()));
    if self.output.len() > 0 {
      for (table,row,col) in &self.output {
        let table_name: String = if let TableId::Global(table_id) = table {
          match self.strings.borrow().get(table_id) {
            Some(s) => s.to_string(),
            None => format!("{:?}",table_id),
          } 
        } else {
          format!("{:?}",table)
        };
        block_drawing.add_line(format!("  - #{}{{{:?}, {:?}}}", table_name,row,col));
      }
    }
    block_drawing.add_title("🪄",&format!("transformations ({})",self.transformations.len()));
    block_drawing.add_line(format!("{:#?}", &self.transformations));
    if let Some(ut) = &self.unsatisfied_transformation {
      block_drawing.add_title("😔","unsatisfied transformation");
      block_drawing.add_line(format!("{:#?}", &ut));
    }
    if self.pending_transformations.len() > 0 {
      block_drawing.add_title("⏳","pending transformations");
      block_drawing.add_line(format!("{:#?}", &self.pending_transformations));
    }
    block_drawing.add_title("🗺️","plan");
    block_drawing.add_line(format!("{:?}", &self.plan));
    block_drawing.add_title("📅","tables");
    block_drawing.add_line(format!("{:?}", &self.tables));
    write!(f,"{:?}",block_drawing)?;
    Ok(())
  }
}

// ### RegisterIndex

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegisterIndex {
  Index(usize),
  Alias(u64),
  All,
}

impl RegisterIndex {
  pub fn unwrap(&self) -> usize {
    match self {
      RegisterIndex::Index(ix) => *ix,
      RegisterIndex::Alias(alias) => {
        alias.clone() as usize
      },
      RegisterIndex::All => 0,
    }
  }

}

impl fmt::Debug for RegisterIndex {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &RegisterIndex::Index(ref ix) => write!(f, "Ix({:?})", ix),
      &RegisterIndex::Alias(ref alias) => write!(f, "IxAlias({})", humanize(alias)),
      &RegisterIndex::All => write!(f, "IxAll"),
    }
  }
}

#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Register {
  pub table_id: TableId,
  pub row: RegisterIndex,
  pub column: RegisterIndex,
}