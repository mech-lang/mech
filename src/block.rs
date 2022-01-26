// # Block

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape
// them or perform computations on them. Blocks can be thought of as pure functions where the input and
// output are tables. Blocks have their own internal table store. Local tables can be defined within a
// block, which allows the programmer to break a computation down into steps. The result of the computation
// is then output to one or more global tables, which triggers the execution of other blocks in the network.

// ## Prelude

use crate::*;
use crate::function::{
  MechFunction,
  Function,
  math::*,
  compare::*,
  stats::*,
  table::*,
  set::*,
  logic::*,
};
use std::cell::RefCell;
use std::rc::Rc;
use hashbrown::HashMap;
use std::fmt;
use serde::Serialize;

#[derive(Clone)]
pub struct Plan{
  pub plan: Vec<Rc<RefCell<dyn MechFunction>>>
}

impl Plan {
  fn new () -> Plan {
    Plan {
      plan: Vec::new(),
    }
  }
  
  pub fn push<S: MechFunction + 'static>(&mut self, mut fxn: S) {
    fxn.solve();
    self.plan.push(Rc::new(RefCell::new(fxn)));
  }
  
}

pub type BlockId = u64;
pub type ArgumentName = u64;
pub type Argument = (ArgumentName, TableId, Vec<(TableIndex, TableIndex)>);
pub type Out = (TableId, TableIndex, TableIndex);

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

lazy_static! {
  static ref COLUMN: u64 = hash_str("column");
  static ref ROW: u64 = hash_str("row");
  static ref TABLE: u64 = hash_str("table");
  static ref STATS_SUM: u64 = hash_str("stats/sum");
  static ref MATH_ADD: u64 = hash_str("math/add");
  static ref MATH_DIVIDE: u64 = hash_str("math/divide");
  static ref MATH_MULTIPLY: u64 = hash_str("math/multiply");
  static ref MATH_SUBTRACT: u64 = hash_str("math/subtract");
  static ref MATH_EXPONENT: u64 = hash_str("math/exponent");
  static ref MATH_NEGATE: u64 = hash_str("math/negate");
  static ref TABLE_RANGE: u64 = hash_str("table/range");
  static ref TABLE_SPLIT: u64 = hash_str("table/split");
  static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_str("table/horizontal-concatenate");
  static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_str("table/vertical-concatenate");
  static ref TABLE_APPEND: u64 = hash_str("table/append");
  static ref LOGIC_AND: u64 = hash_str("logic/and");  
  static ref LOGIC_OR: u64 = hash_str("logic/or");
  static ref LOGIC_NOT: u64 = hash_str("logic/not");  
  static ref LOGIC_XOR: u64 = hash_str("logic/xor");    
  static ref COMPARE_GREATER__THAN: u64 = hash_str("compare/greater-than");
  static ref COMPARE_LESS__THAN: u64 = hash_str("compare/less-than");
  static ref COMPARE_GREATER__THAN__EQUAL: u64 = hash_str("compare/greater-than-equal");
  static ref COMPARE_LESS__THAN__EQUAL: u64 = hash_str("compare/less-than-equal");
  static ref COMPARE_EQUAL: u64 = hash_str("compare/equal");
  static ref COMPARE_NOT__EQUAL: u64 = hash_str("compare/not-equal");
  static ref SET_ANY: u64 = hash_str("set/any");
  static ref SET_ALL: u64 = hash_str("set/all");  
}

#[derive(Clone)]
pub struct Block {
  pub id: BlockId,
  pub state: BlockState,
  tables: Database,
  pub plan: Plan,
  pub changes: Transaction,
  pub global_database: Rc<RefCell<Database>>,
  pub unsatisfied_transformation: Option<(MechError,Transformation)>,
  pub pending_transformations: Vec<Transformation>,
  pub transformations: Vec<Transformation>,
  pub strings: HashMap<u64,MechString>,
  pub output: Vec<TableId>,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      state: BlockState::New,
      tables: Database::new(),
      plan: Plan::new(),
      changes: Vec::new(),
      global_database: Rc::new(RefCell::new(Database::new())),
      unsatisfied_transformation: None,
      pending_transformations: Vec::new(),
      transformations: Vec::new(),
      strings: HashMap::new(),
      output: Vec::new(),
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
              None => Err(MechError::MissingTable(*table_id)),
            }
            Some(TableId::Local(id)) => match self.tables.get_table_by_id(id) {
              Some(table) => Ok(table.clone()),
              None => Err(MechError::MissingTable(*table_id)),
            }
            None => Err(MechError::MissingTable(*table_id))
          }
        },
      },
      TableId::Global(id) => match self.global_database.borrow().get_table_by_id(id) {
        Some(table) => Ok(table.clone()),
        None => Err(MechError::MissingTable(*table_id)),
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

  pub fn ready(&mut self) -> bool {
    match self.state {
      BlockState::Ready => true,
      BlockState::Disabled => false,
      _ => {
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
              Err(_) => false,
            }
          }
          None => {
            self.state = BlockState::Ready;
            true
          }
        }
      }
    }
  }

  pub fn get_arg_column(&self, argument: &Argument) -> Result<(u64,Column,ColumnIndex),MechError> {

    let (arg_name, table_id, indices) = argument;

    let mut table_id = *table_id;
    for (row,column) in indices.iter().take(indices.len()-1) {
      let argument = (0,table_id,vec![(*row,*column)]);
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
            _ => {return Err(MechError::GenericError(6395));}
          }
        }
        _ => {return Err(MechError::GenericError(6394));}
      }
    }

    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let table_brrw = table.borrow(); 

    match (row,col) {
      (_,TableIndex::Index(ix)) |
      (TableIndex::Index(ix),_) if ix == &0 => {
        return Err(MechError::GenericError(3493));
      }
      (TableIndex::Index(row),TableIndex::Index(_)) => {
        let col = table_brrw.get_column(&col)?;
        Ok((*arg_name,col.clone(),ColumnIndex::Index(row-1)))
      }
      (TableIndex::Index(ix),TableIndex::Alias(col_alias)) => {
        let arg_col = table_brrw.get_column(col)?;
        Ok((*arg_name,arg_col.clone(),ColumnIndex::Index(*ix-1)))
      }
      (TableIndex::Index(ix),TableIndex::None) => {
        let (ix_row,ix_col) = table_brrw.index_to_subscript(ix-1)?;
        let col = table_brrw.get_column(&TableIndex::Index(ix_col + 1))?;
        let (row,_) = table_brrw.index_to_subscript(ix - 1)?;

        Ok((*arg_name,col.clone(),ColumnIndex::Index(row)))
      }
      (TableIndex::Table(ix_table_id),TableIndex::Index(_)) |
      (TableIndex::Table(ix_table_id),TableIndex::Alias(_))  => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();

        if ix_table_brrw.cols != 1 {
          return Err(MechError::GenericError(9237));
        }
        let ix = match ix_table_brrw.get_column_unchecked(0) {
          Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
          Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
          Column::U8(ix_col) => ColumnIndex::Index(ix_col.borrow()[0] as usize - 1),
          x => {
            return Err(MechError::GenericError(9239));
          }
        };
        let arg_col = table_brrw.get_column(col)?;
        Ok((*arg_name,arg_col.clone(),ix))
      }
      (TableIndex::Table(ix_table_id),TableIndex::None) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        match table.borrow().kind() {
          ValueKind::Compound(table_kind) => {
            Err(MechError::GenericError(9238))
          }
          table_kind => {
            let ix = match ix_table_brrw.get_column_unchecked(0) {
              Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
              Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
              Column::U8(ix_col) => ColumnIndex::Index(ix_col.borrow()[0] as usize - 1),
              x => {
                return Err(MechError::GenericError(9232));
              }
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
      (TableIndex::All, TableIndex::Index(col_ix)) => {
        let col = table_brrw.get_column(&TableIndex::Index(*col_ix))?;
        Ok((*arg_name,col.clone(),ColumnIndex::All))
      },
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
      let argument = (0,table_id,vec![(*row,*column)]);
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
            _ => {return Err(MechError::GenericError(6395));}
          }
        }
        _ => {return Err(MechError::GenericError(6394));}
      }
    }

    let (row,col) = &indices.last().unwrap();

    let lhs_table = self.get_table(&table_id)?;
    let lhs_brrw = lhs_table.borrow();
    let row_index = match row {
      TableIndex::All => ColumnIndex::All,
      TableIndex::None => ColumnIndex::None,
      TableIndex::Index(ix) => ColumnIndex::Index(ix - 1),
      TableIndex::Alias(alias) => {
        return Err(MechError::GenericError(9257));
      },
      TableIndex::Table(ix_table_id) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();
        let ix = match ix_table_brrw.get_column_unchecked(0) {
          Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
          Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
          Column::U8(ix_col) => ColumnIndex::Index(ix_col.borrow()[0] as usize - 1),
          x => {
            return Err(MechError::GenericError(9253));
          }
        };
        ix
      }
    };
    let arg_cols
      = lhs_brrw.get_columns(&col)?.iter().map(|arg_col| (*arg_name,arg_col.clone(),row_index.clone())).collect();
    Ok(arg_cols)
  }

  pub fn get_out_column(&self, out: &Out, rows: usize, col_kind: ValueKind) -> Result<Column,MechError> {
    let (out_table_id, _, _) = out;
    let table = self.get_table(out_table_id)?;
    let mut t = table.borrow_mut();
    let cols = t.cols;
    t.resize(rows,cols);
    t.set_col_kind(0, col_kind);
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
      let argument = (0,table_id,vec![(*row,*column)]);
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
            _ => {return Err(MechError::GenericError(6695));}
          }
        }
        _ => {return Err(MechError::GenericError(6694));}
      }
    }

    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let t = table.borrow();
    let dim = match (row,col) {
      (TableIndex::All, TableIndex::All) => (t.rows, t.cols),
      (TableIndex::All,TableIndex::Index(_)) |
      (TableIndex::All, TableIndex::Alias(_)) => (t.rows, 1),
      (TableIndex::Index(_),TableIndex::None) |
      (TableIndex::Index(_),TableIndex::Index(_)) |
      (TableIndex::Index(_),TableIndex::Alias(_)) => (1,1),
      (TableIndex::Index(_),TableIndex::All) => (1,t.cols),
      (TableIndex::Table(ix_table_id),TableIndex::Alias(_)) |
      (TableIndex::Table(ix_table_id),TableIndex::None) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let rows = ix_table.borrow().logical_len();
        (rows,1)
      },
      (TableIndex::Table(ix_table_id),TableIndex::All) => {
        let ix_table = self.get_table(&ix_table_id)?;
        let rows = ix_table.borrow().logical_len();
        (rows,t.cols)
      },
      x => {return Err(MechError::GenericError(6384));},
    };
    let arg_shape = match dim {
      (1,1) => TableShape::Scalar,
      (1,x) => TableShape::Row(x),
      (x,1) => TableShape::Column(x),
      (x,y) => TableShape::Matrix(x,y),
      _ => TableShape::Pending,
    };
    Ok(arg_shape)
  }

  pub fn add_tfm(&mut self, tfm: Transformation) -> Result<(),MechError> {
    match self.state {
      BlockState::Unsatisfied => {
        self.pending_transformations.push(tfm.clone());
        return Err(MechError::GenericError(7372));
      }
      _ => {
        match self.compile_tfm(tfm.clone()) {
          Ok(()) => (),
          Err(mech_error_kind) => {
            self.unsatisfied_transformation = Some((mech_error_kind.clone(),tfm));
            self.state = BlockState::Unsatisfied;
            return Err(mech_error_kind);        
          }
        }
      }
    }
    Ok(())
  }

  fn compile_tfm(&mut self, tfm: Transformation) -> Result<(), MechError> {
    match &tfm {
      Transformation::Identifier{name, id} => {
        self.strings.insert(*id, name.to_vec());
      }
      Transformation::NewTable{table_id, rows, columns} => {
        match table_id {
          TableId::Local(id) => {
            let table = Table::new(*id, *rows, *columns);
            self.tables.insert_table(table);
          }
          TableId::Global(id) => {
            let table = Table::new(*id, *rows, *columns);
            self.global_database.borrow_mut().insert_table(table);
            self.output.push(*table_id);
            //self.changes.push(Change::NewTable{table_id: *id, rows: *rows, columns: *columns});
          }
        } 
      },
      Transformation::TableReference{table_id, reference} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        table_brrw.set_kind(ValueKind::Reference);
        table_brrw.set(0,0,reference.clone())?;

        let src_table = self.get_table(&reference.as_table_reference()?)?;
        let src_table_brrw = src_table.borrow();
        let src_id = src_table_brrw.id;
        let rows = src_table_brrw.rows;
        let cols = src_table_brrw.cols;
        let dest_table = Table::new(src_id,rows,cols);
        self.global_database.borrow_mut().insert_table(dest_table);
      }
      Transformation::TableAlias{table_id, alias} => {
        self.tables.insert_alias(*alias, *table_id)?;
      },
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        let mut table = self.tables.get_table_by_id(table_id.unwrap()).unwrap().borrow_mut();
        if *column_ix > table.cols - 1  {
          let rows = table.rows;
          table.resize(rows,*column_ix + 1);
        }
        table.set_column_alias(*column_ix,*column_alias);
      },
      Transformation::TableDefine{table_id, indices, out} => {
        //let arg_col = self.get_arg_column(&argument)?;

        // Iterate through to the last index
        let mut table_id = *table_id;
        for (row,column) in indices.iter().take(indices.len()-1) {
          let argument = (0,table_id,vec![(*row,*column)]);
          match self.get_arg_dim(&argument)? {
            TableShape::Scalar => {
              let arg_col = self.get_arg_column(&argument)?;
              match arg_col {
                (_,Column::Ref(ref_col),_) => {
                  table_id = ref_col.borrow()[0].clone();
                }
                _ => {return Err(MechError::GenericError(6393));}
              }
            }
            _ => {return Err(MechError::GenericError(6392));}
          }
        }

        let src_table = self.get_table(&table_id)?;
        let out_table = self.get_table(out)?;
        let (row, column) = indices.last().unwrap();
        let argument = (0,table_id,vec![(*row,*column)]);

        match (row,column) {
          // Select an entire table
          (TableIndex::All, TableIndex::All) => {
            match out {
              TableId::Global(gid) => {
                self.plan.push(CopyT{arg: src_table.clone(), out: out_table.clone()});
              }
              _ => (),
            }
          }
          // Select a column by row index
          (TableIndex::All, TableIndex::Index(_)) |
          // Select a column by alias
          (TableIndex::All, TableIndex::Alias(_)) => {
            let (_, arg_col,_) = self.get_arg_column(&(0,table_id,vec![(*row,*column)]))?;
            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),arg_col.len(),arg_col.kind())?;
            match (&arg_col, &out_col) {
              (Column::U8(arg), Column::U8(out)) => self.plan.push(CopyVV::<u8>{arg: arg.clone(), out: out.clone()}),
              (Column::Bool(arg), Column::Bool(out)) => self.plan.push(CopyVV::<bool>{arg: arg.clone(), out: out.clone()}),
              _ => {return Err(MechError::GenericError(6398));},
            }
          }
          // Select a specific element by numberical index
          (TableIndex::Index(ix), TableIndex::None) => {
            let src_brrw = src_table.borrow();
            let (row,col) = src_brrw.index_to_subscript(ix-1)?;
            let mut arg_col = src_brrw.get_column_unchecked(col);
            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;
            match (&arg_col, &out_col) {
              (Column::U8(arg), Column::U8(out), ) => self.plan.push(CopySS::<u8>{arg: arg.clone(), ix: row, out: out.clone()}),
              (Column::Ref(arg), Column::Ref(out), ) => self.plan.push(CopySS::<TableId>{arg: arg.clone(), ix: row, out: out.clone()}),
              _ => {return Err(MechError::GenericError(6381));},
            }
          }
          // Select a number of specific elements by numerical index or lorgical index
          (TableIndex::Table(ix_table_id), TableIndex::None) => {
            let src_brrw = src_table.borrow();
            match src_brrw.shape() {
              TableShape::Row(_) => {
                {
                  let mut out_brrw = out_table.borrow_mut();
                  out_brrw.set_kind(src_brrw.kind());
                }
                let ix_table = self.get_table(&ix_table_id)?;
                self.plan.push(CopyTB{arg: src_table.clone(), ix: ix_table.clone(), out: out_table.clone()});
              }
              _ => {
                let (_, arg_col,arg_ix) = self.get_arg_column(&argument)?;
                let mut out_brrw = out_table.borrow_mut();
                out_brrw.set_kind(arg_col.kind());
                let out_col = out_brrw.get_column_unchecked(0);    
                match (&arg_col, &arg_ix, &out_col) {
                  (Column::U8(arg), ColumnIndex::Bool(ix), Column::U8(out)) => self.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
                  (Column::U8(arg), ColumnIndex::IndexCol(ix_col), Column::U8(out)) => self.plan.push(CopyVI::<u8>{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),
                  (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => self.plan.push(CopySS::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
                  x => {return Err(MechError::GenericError(6380));},
                }
              }
            }
          }
          (TableIndex::Table(ix_table_id), TableIndex::All) => {
            let src_brrw = src_table.borrow();
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,src_brrw.cols);
            out_brrw.set_kind(src_brrw.kind());
            for col in 0..src_brrw.cols {
              let (_, arg_col,arg_ix) = self.get_arg_column(&(0,table_id,vec![(*row,TableIndex::Index(col+1))]))?;
              let mut out_col = out_brrw.get_column_unchecked(col); 
              match (&arg_col, &arg_ix, &out_col) {
                (Column::U8(arg), ColumnIndex::Bool(ix), Column::U8(out)) => self.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
                (Column::U8(arg), ColumnIndex::IndexCol(ix_col), Column::U8(out)) => self.plan.push(CopyVI::<u8>{arg: arg.clone(), ix: ix_col.clone(), out: out.clone()}),
                x => {return Err(MechError::GenericError(6382));},
              }
            }
          }
          (TableIndex::Index(row_ix), TableIndex::Alias(column_alias)) => {
            let (_, arg_col,arg_ix) = self.get_arg_column(&(0,table_id,vec![(*row,*column)]))?;
            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => self.plan.push(CopySS::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
              x => {
                return Err(MechError::GenericError(6388));},
            }
          }
          x => {return Err(MechError::GenericError(6379));},
        }
      }
      Transformation::Set{src_id, src_row, src_col, dest_id, dest_row, dest_col} => {
        let arguments = vec![(0,*src_id,vec![(*src_row,*src_col)]),(0,*dest_id,vec![(*dest_row,*dest_col)])];
        let arg_shapes = self.get_arg_dims(&arguments)?;
        match (&arg_shapes[0], &arg_shapes[1]) {
          (TableShape::Scalar, TableShape::Row(_)) |
          (TableShape::Row(_), TableShape::Row(_)) => {
            let src_table = self.get_table(src_id)?;
            let dest_table = self.get_table(dest_id)?;
            let src_table_brrw = src_table.borrow();
            let dest_table_brrw = dest_table.borrow();
            // The source table has named columns, so we need to match them
            // up with the destination columns if they are out of order or
            // incomplete.
            if src_table_brrw.has_col_aliases() {
              for alias in src_table_brrw.column_ix_to_alias.iter() {
                let dest_column = dest_table_brrw.get_column(&TableIndex::Alias(*alias))?;
                let src_column = src_table_brrw.get_column(&TableIndex::Alias(*alias))?;
                match (src_column,dest_column) {
                  (Column::U8(src),Column::U8(out)) => {self.plan.push(SetSIxSIx::<u8>{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
                  _ => {return Err(MechError::GenericError(8839));}
                }
              }
            // No column aliases, use indices instead
            } else {
              if src_table_brrw.cols > dest_table_brrw.cols {
                return Err(MechError::GenericError(8840));
              }
              // Destination has aliases, need to use them instead 
              if dest_table_brrw.has_col_aliases() {
                return Err(MechError::GenericError(8842));
              }
              for col_ix in 1..=src_table_brrw.cols {
                let dest_column = dest_table_brrw.get_column(&TableIndex::Index(col_ix))?;
                let src_column = src_table_brrw.get_column(&TableIndex::Index(col_ix))?;
                match (src_column,dest_column) {
                  (Column::U8(src),Column::U8(out)) => {self.plan.push(SetSIxSIx::<u8>{arg: src.clone(), ix: 0, out: out.clone(), oix: 0});}
                  _ => {return Err(MechError::GenericError(8841));}
                }
              }
            }
          }
          (TableShape::Matrix(_,_),TableShape::Matrix(_,_)) |
          (TableShape::Matrix(_,_),TableShape::Row(_)) |
          (TableShape::Matrix(_,_),TableShape::Scalar) => {
            let src_table = self.get_table(src_id)?;
            let dest_table = self.get_table(dest_id)?;
            let src_table_brrw = src_table.borrow();
            let mut dest_table_brrw = dest_table.borrow_mut();
            dest_table_brrw.resize(src_table_brrw.rows,src_table_brrw.cols);
            dest_table_brrw.set_kind(src_table_brrw.kind());
            for col_ix in 1..=src_table_brrw.cols {
              let dest_column = dest_table_brrw.get_column(&TableIndex::Index(col_ix))?;
              let src_column = src_table_brrw.get_column(&TableIndex::Index(col_ix))?;
              match (src_column,dest_column) {
                (Column::U8(src),Column::U8(out)) => {self.plan.push(SetVV::<u8>{arg: src.clone(), out: out.clone()});}
                (Column::Bool(src),Column::Bool(out)) => {self.plan.push(SetVV::<bool>{arg: src.clone(), out: out.clone()});}
                _ => {return Err(MechError::GenericError(8102));}
              }
            }
          }
          _ |
          (TableShape::Column(_),TableShape::Column(_)) => {
            let arg_cols = self.get_arg_columns(&arguments)?;
            match (&arg_cols[0], &arg_cols[1]) {
              ((_,Column::U8(arg),ColumnIndex::Index(ix)),(_,Column::U8(out),ColumnIndex::Bool(oix))) => 
                self.plan.push(SetSIxVB::<u8>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::U8(out),ColumnIndex::Index(oix))) =>
                self.plan.push(SetSIxSIx::<u8>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix}),
              ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::U8(out),ColumnIndex::Index(oix))) =>
                self.plan.push(SetSIxSIx::<u8>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix}),
              ((_,Column::U8(arg),ColumnIndex::All), (_,Column::U8(out),ColumnIndex::Bool(oix))) =>
                self.plan.push(SetVVB::<u8>{arg: arg.clone(), out: out.clone(), oix: oix.clone()}),
              ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::All)) => {
                let dest_table = self.get_table(dest_id)?;
                let src_table = self.get_table(src_id)?;
                let src_table_brrw = src_table.borrow();
                let mut dest_table_brrw = dest_table.borrow_mut();
                dest_table_brrw.resize(1,1);
                dest_table_brrw.set_kind(ValueKind::U8);
                let out = dest_table_brrw.get_column_unchecked(0).get_u8()?;
                self.plan.push(SetSIxSIx::<u8>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: 0});
              }
              ((_,Column::U8(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
                let dest_table = self.get_table(dest_id)?;
                let src_table = self.get_table(src_id)?;
                let src_table_brrw = src_table.borrow();
                let mut dest_table_brrw = dest_table.borrow_mut();
                dest_table_brrw.set_col_kind(1,ValueKind::U8);
                let out = dest_table_brrw.get_column_unchecked(1).get_u8()?;
                self.plan.push(SetSIxSIx::<u8>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
              }
              ((_,Column::Ref(arg),ColumnIndex::Index(ix)), (_,Column::Empty,ColumnIndex::Index(oix))) => {
                let dest_table = self.get_table(dest_id)?;
                let src_table = self.get_table(src_id)?;
                let src_table_brrw = src_table.borrow();
                let mut dest_table_brrw = dest_table.borrow_mut();
                dest_table_brrw.set_col_kind(1,ValueKind::Reference);
                let out = dest_table_brrw.get_column_unchecked(1).get_reference()?;
                
                self.plan.push(SetSIxSIx::<TableId>{arg: arg.clone(), ix: *ix, out: out.clone(), oix: *oix});
              }
              x => {
                return Err(MechError::GenericError(8835));
              },
            }
          }
          _ => return Err(MechError::GenericError(8837)),
        }
      }
      Transformation::NumberLiteral{kind, bytes} => {
        let table_id = hash_str(&format!("{:?}{:?}", kind, bytes));
        let table =  self.get_table(&TableId::Local(table_id))?; 
        let mut t = table.borrow_mut();
        match kind {
          NumberLiteralKind::Decimal => {
            match bytes.len() {
              1 => {
                t.set_col_kind(0, ValueKind::U8);
                t.set(0,0,Value::U8(bytes[0] as u8))?;
              }
              2 => {
                t.set_col_kind(0, ValueKind::U16);
                use std::mem::transmute;
                use std::convert::TryInto;
                let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
                let x = u16::from_ne_bytes(int_bytes.try_into().unwrap());
                t.set(0,0,Value::U16(x))?;
              }
              _ => {return Err(MechError::GenericError(6376));},
            }
          }
          _ => {return Err(MechError::GenericError(6375));},
        }
      },
      Transformation::Constant{table_id, value} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        match &value {
          Value::Bool(_) => {table_brrw.set_col_kind(0, ValueKind::Bool);},
          Value::String(_) => {table_brrw.set_col_kind(0, ValueKind::String);},
          _ => (),
        }
        table_brrw.set(0,0,value.clone())?;
      }
      Transformation::Function{name, ref arguments, out} => {
        if *name == *MATH_ADD || 
           *name == *MATH_DIVIDE || 
           *name == *MATH_MULTIPLY || 
           *name == *MATH_SUBTRACT || 
           *name == *MATH_EXPONENT {
          let arg_shapes = self.get_arg_dims(&arguments)?;
          // Now decide on the correct tfm based on the shape
          match (&arg_shapes[0],&arg_shapes[1]) {
            (TableShape::Scalar, TableShape::Scalar) => {
              let mut argument_scalars = self.get_arg_columns(arguments)?;
              let mut out_column = self.get_out_column(out, 1, ValueKind::U8)?;
              match (&argument_scalars[0], &argument_scalars[1], &out_column) {
                ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::U8(rhs),ColumnIndex::Index(rix)), Column::U8(out)) => {
                  if *name == *MATH_ADD { self.plan.push(AddSS::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) }
                  else if *name == *MATH_SUBTRACT { self.plan.push(SubSS::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) } 
                  else if *name == *MATH_MULTIPLY { self.plan.push(MulSS::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) } 
                  else if *name == *MATH_DIVIDE { self.plan.push(DivSS::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) } 
                  else if *name == *MATH_EXPONENT { self.plan.push(ExpSS::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1236));},
              }
            }
            (TableShape::Scalar, TableShape::Column(rows)) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let mut out_column = self.get_out_column(out, *rows, ValueKind::U8)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => {
                  if *name == *MATH_ADD { self.plan.push(AddSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                  else if *name == *MATH_SUBTRACT { self.plan.push(SubSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_MULTIPLY { self.plan.push(MulSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_DIVIDE { self.plan.push(DivSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_EXPONENT { self.plan.push(ExpSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1237));},
              }
            }   
            (TableShape::Column(rows), TableShape::Scalar) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let mut out_column = self.get_out_column(out, *rows, ValueKind::U8)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => {
                  if *name == *MATH_ADD { self.plan.push(AddVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                  else if *name == *MATH_SUBTRACT { self.plan.push(SubVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_MULTIPLY { self.plan.push(MulVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_DIVIDE { self.plan.push(DivVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_EXPONENT { self.plan.push(ExpVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1238));},
              }
            }                      
            (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
              if lhs_rows != rhs_rows {
                return Err(MechError::GenericError(6401));
              }
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *lhs_rows, ValueKind::U8)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => {
                  if *name == *MATH_ADD { self.plan.push(AddVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                  else if *name == *MATH_SUBTRACT { self.plan.push(SubVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_MULTIPLY { self.plan.push(MulVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_DIVIDE { self.plan.push(DivVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *MATH_EXPONENT { self.plan.push(ExpVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1239));},
              }
            }
            (TableShape::Row(cols), TableShape::Scalar) => {
              let lhs_columns = self.get_whole_table_arg_cols(&arguments[0])?;
              let rhs_column = self.get_arg_column(&arguments[1])?;

              let (out_table_id, _, _) = out;
              let out_table = self.get_table(out_table_id)?;
              let mut out_brrw = out_table.borrow_mut();
              out_brrw.resize(1,*cols);

              for (col_ix,(_,lhs_column,_)) in lhs_columns.iter().enumerate() {
                out_brrw.set_col_kind(col_ix, ValueKind::U8);
                let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                match (lhs_column,&rhs_column,out_col) {
                  (Column::U8(lhs), (_,Column::U8(rhs),_), Column::U8(out)) => {
                    if *name == *MATH_ADD { self.plan.push(AddVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_SUBTRACT { self.plan.push(SubVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_MULTIPLY { self.plan.push(MulVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_DIVIDE { self.plan.push(DivVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_EXPONENT { self.plan.push(ExpVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  }
                  _ => {return Err(MechError::GenericError(6343));},
                }
              }
            }
            (TableShape::Scalar, TableShape::Row(cols)) => {
              let rhs_columns = self.get_whole_table_arg_cols(&arguments[1])?;
              let lhs_column = self.get_arg_column(&arguments[0])?;

              let (out_table_id, _, _) = out;
              let out_table = self.get_table(out_table_id)?;
              let mut out_brrw = out_table.borrow_mut();
              out_brrw.resize(1,*cols);

              for (col_ix,(_,rhs_column,_)) in rhs_columns.iter().enumerate() {
                out_brrw.set_col_kind(col_ix, ValueKind::U8);
                let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                match (rhs_column,&lhs_column,out_col) {
                  (Column::U8(rhs), (_,Column::U8(lhs),_), Column::U8(out)) => {
                    if *name == *MATH_ADD { self.plan.push(AddSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_SUBTRACT { self.plan.push(SubSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_MULTIPLY { self.plan.push(MulSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_DIVIDE { self.plan.push(DivSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_EXPONENT { self.plan.push(ExpSV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                  }
                  _ => {return Err(MechError::GenericError(6343));},
                }
              }
            }            
            (TableShape::Matrix(lhs_rows,lhs_cols), TableShape::Matrix(rhs_rows,rhs_cols)) => {
              
              if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
                return Err(MechError::GenericError(6343));
              }

              let lhs_columns = self.get_whole_table_arg_cols(&arguments[0])?;
              let rhs_columns = self.get_whole_table_arg_cols(&arguments[1])?;

              let (out_table_id, _, _) = out;
              let out_table = self.get_table(out_table_id)?;
              let mut out_brrw = out_table.borrow_mut();
              out_brrw.resize(*lhs_rows,*lhs_cols);

              for (col_ix,lhs_rhs) in lhs_columns.iter().zip(rhs_columns).enumerate() {
                out_brrw.set_col_kind(col_ix, ValueKind::U8);
                let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                match (lhs_rhs,out_col) {
                  (((_,Column::U8(lhs),_), (_,Column::U8(rhs),_)),Column::U8(out)) => {
                    if *name == *MATH_ADD { self.plan.push(AddVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
                    else if *name == *MATH_SUBTRACT { self.plan.push(SubVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_MULTIPLY { self.plan.push(MulVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_DIVIDE { self.plan.push(DivVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                    else if *name == *MATH_EXPONENT { self.plan.push(ExpVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  }
                  _ => {return Err(MechError::GenericError(6343));},
                }
              }
            }
            _ => {return Err(MechError::GenericError(6345));},
          }
        } 
        else if *name == *MATH_NEGATE { math_negate(self,&arguments,&out)?; } 
        else if *name == *LOGIC_NOT { logic_not(self,&arguments,&out)?; } 
        else if *name == *LOGIC_AND ||
                  *name == *LOGIC_OR ||
                  *name == *LOGIC_XOR 
        {
          let arg_dims = self.get_arg_dims(&arguments)?;
          match (&arg_dims[0],&arg_dims[1]) {
            (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                  if *name == *LOGIC_AND { self.plan.push(AndVV{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }); }
                  else if *name == *LOGIC_OR { self.plan.push(OrVV{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *LOGIC_XOR { self.plan.push(XorVV{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1342));},
              }
            }
            (TableShape::Scalar, TableShape::Scalar) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, 1, ValueKind::Bool)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                  if *name == *LOGIC_AND { self.plan.push(AndSS{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }); }
                  else if *name == *LOGIC_OR { self.plan.push(OrSS{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                  else if *name == *LOGIC_XOR { self.plan.push(XorSS{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) } 
                }
                _ => {return Err(MechError::GenericError(1340));},
              }
            }
            _ => {return Err(MechError::GenericError(1341));},
          }
        }
        else if *name == *COMPARE_EQUAL { compare_equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_NOT__EQUAL { compare_not__equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_LESS__THAN { compare_less__than(self,&arguments,&out)?; } 
        else if *name == *COMPARE_LESS__THAN__EQUAL { compare_less__than__equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_GREATER__THAN { compare_greater__than(self,&arguments,&out)?; } 
        else if *name == *COMPARE_GREATER__THAN__EQUAL { compare_greater__than__equal(self,&arguments,&out)?; } 
        else if *name == *STATS_SUM { stats_sum(self,&arguments,&out)?; } 
        else if *name == *SET_ANY { set_any(self,&arguments,&out)?; } 
        else if *name == *SET_ALL { set_all(self,&arguments,&out)?; } 
        else if *name == *TABLE_SPLIT { table_split(self,&arguments,&out)?;}
        else if *name == *TABLE_VERTICAL__CONCATENATE { table_vertical__concatenate(self,&arguments,&out)?; } 
        else if *name == *TABLE_HORIZONTAL__CONCATENATE { table_horizontal__concatenate(self,&arguments,&out)?; }       
        else if *name == *TABLE_APPEND { table_append(self,&arguments,&out)?; } 
        else if *name == *TABLE_RANGE { table_range(self,&arguments,&out)?; }
        else {
          return Err(MechError::MissingFunction(*name));
        }
      } 
      _ => {},
    }
    self.transformations.push(tfm.clone());
    Ok(())
  }

  pub fn solve(&mut self) -> bool {
    if self.state == BlockState::Ready {
      for ref mut fxn in &mut self.plan.plan.iter() {
        fxn.borrow_mut().solve();
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
    block_drawing.add_header("unsatisfied transformations");
    block_drawing.add_line(format!("{:#?}", &self.unsatisfied_transformation));
    block_drawing.add_header("pending transformations");
    block_drawing.add_line(format!("{:#?}", &self.pending_transformations));
    block_drawing.add_header("tables");
    block_drawing.add_line(format!("{:?}", &self.tables));
    block_drawing.add_header("plan");
    for step in &self.plan.plan {
      block_drawing.add_line(format!("{}", &step.borrow().to_string()));
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