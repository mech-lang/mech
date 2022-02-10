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
  table::*,
};
use std::cell::RefCell;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use std::fmt;
use serde::Serialize;
use std::mem::transmute;
use std::convert::TryInto;

lazy_static! {
  pub static ref F32: u64 = hash_str("f32");
  pub static ref U8: u64 = hash_str("u8");
  pub static ref U16: u64 = hash_str("u16");
  pub static ref U32: u64 = hash_str("u32");
  pub static ref U64: u64 = hash_str("u64");
  pub static ref M: u64 = hash_str("m");
  pub static ref KM: u64 = hash_str("km");
}

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

#[derive(Clone)]
pub struct Block {
  pub id: BlockId,
  pub state: BlockState,
  pub tables: Database,
  pub plan: Plan,
  pub changes: Transaction,
  pub functions: Option<Rc<RefCell<core::Functions>>>,
  pub global_database: Rc<RefCell<Database>>,
  pub unsatisfied_transformation: Option<(MechError,Transformation)>,
  pub pending_transformations: Vec<Transformation>,
  pub transformations: Vec<Transformation>,
  pub strings: HashMap<u64,MechString>,
  pub output: HashSet<(TableId,TableIndex,TableIndex)>,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      state: BlockState::New,
      tables: Database::new(),
      plan: Plan::new(),
      changes: Vec::new(),
      functions: None,
      global_database: Rc::new(RefCell::new(Database::new())),
      unsatisfied_transformation: None,
      pending_transformations: Vec::new(),
      transformations: Vec::new(),
      strings: HashMap::new(),
      output: HashSet::new(),
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

    // This part handles multi-dimensional indexing e.g. {1,2}{3,4}{5,6}
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

    // get the table
    let (row,col) = &indices.last().unwrap();
    let table = self.get_table(&table_id)?;
    let table_brrw = table.borrow(); 

    // Get the column and row
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
      TableIndex::ReshapeColumn => ColumnIndex::ReshapeColumn,
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
      (TableIndex::ReshapeColumn, TableIndex::All) => (t.rows*t.cols,1),
      (TableIndex::All, TableIndex::All) => (t.rows, t.cols),
      (TableIndex::All, TableIndex::None) => (t.rows*t.cols,1),
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
            self.output.insert((*table_id,TableIndex::All,TableIndex::All));
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
      Transformation::ColumnKind{table_id, column_ix, kind} => {
        let table = self.get_table(table_id)?;
        let mut table_brrw = table.borrow_mut();
        if *kind == *U8 { table_brrw.set_kind(ValueKind::U8); }
        else if *kind == *U16 { table_brrw.set_kind(ValueKind::U16); }
        else if *kind == *U32 { table_brrw.set_kind(ValueKind::U32); }
        else if *kind == *U64 { table_brrw.set_kind(ValueKind::U64); }
      }
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        let mut table = self.tables.get_table_by_id(table_id.unwrap()).unwrap().borrow_mut();
        if *column_ix > table.cols - 1  {
          let rows = table.rows;
          table.resize(rows,*column_ix + 1);
        }
        table.set_column_alias(*column_ix,*column_alias);
        self.output.insert((*table_id,TableIndex::All,TableIndex::Alias(*column_alias)));
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
        let mut bytes = bytes.clone();
        let table_id = hash_str(&format!("{:?}{:?}", kind, bytes));
        let table =  self.get_table(&TableId::Local(table_id))?; 
        let mut t = table.borrow_mut();
        if *kind == *U16 {
          match bytes.len() {
            1..=2 => {
              t.set_kind(ValueKind::U16)?;
              while bytes.len() < 2 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
              let x = u16::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U16(x))?;
            }
            _ => {return Err(MechError::GenericError(6377));},
          }
        } else if *kind == *U8 {
          match bytes.len() {
            1 => {
              t.set_kind(ValueKind::U8)?;
              t.set(0,0,Value::U8(bytes[0]))?;
            }
            _ => {return Err(MechError::GenericError(6383));},
          }
        } else if *kind == *F32 {
          match bytes.len() {
            1..=4 => {
              t.set_kind(ValueKind::F32)?;
              while bytes.len() < 4 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<f32>());
              let x = f32::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::F32(x))?;
            }
            _ => {return Err(MechError::GenericError(6377));},
          }
        } else if *kind == *KM {
          match bytes.len() {
            1..=2 => {
              t.set_kind(ValueKind::U16)?;
              while bytes.len() < 2 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
              let x = u16::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U16(x * 1000))?;
            }
            _ => {return Err(MechError::GenericError(6387));},
          }
        } else if *kind == *M {
          match bytes.len() {
            1..=2 => {
              t.set_kind(ValueKind::U16)?;
              while bytes.len() < 2 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
              let x = u16::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U16(x))?;
            }
            _ => {return Err(MechError::GenericError(6388));},
          }
        } else if *kind == 0 {
          match bytes.len() {
            1 => {
              t.set_col_kind(0, ValueKind::U8)?;
              t.set(0,0,Value::U8(bytes[0] as u8))?;
            }
            2 => {
              t.set_kind(ValueKind::U16)?;
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u16>());
              let x = u16::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U16(x))?;
            }
            3 | 4 => {
              t.set_kind(ValueKind::U32)?;
              if bytes.len() < 4 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u32>());
              let x = u32::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U32(x))?;
            }
            5..=8 => {
              t.set_kind(ValueKind::U64)?;
              while bytes.len() < 8 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u64>());
              let x = u64::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U64(x))?;
            }
            9..=16 => {
              t.set_kind(ValueKind::U128)?;
              while bytes.len() < 16 {
                bytes.insert(0,0);
              }
              let (int_bytes, rest) = bytes.split_at(std::mem::size_of::<u128>());
              let x = u128::from_be_bytes(int_bytes.try_into().unwrap());
              t.set(0,0,Value::U128(x))?;
            }
            _ => {return Err(MechError::GenericError(6376));},
          }
        } else {
          println!("{:?}", kind);
          return Err(MechError::GenericError(6996));
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
        let fxns = self.functions.clone();
        match &fxns {
          Some(functions) => {
            let mut fxns = functions.borrow_mut();
            match fxns.get(*name) {
              Some(fxn) => {
                fxn.compile(self,&arguments,&out)?;
                return Ok(());
              }
              None => {return Err(MechError::MissingFunction(*name));}
            }
          }
          None => {return Err(MechError::GenericError(2352));},
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
    block_drawing.add_header("output");
    block_drawing.add_line(format!("{:#?}", &self.output));
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