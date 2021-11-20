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

#[derive(Clone)]
pub struct Plan{
  pub plan: Vec<Rc<RefCell<MechFunction>>>
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
pub type Argument = (u64, TableId, TableIndex, TableIndex);
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
  pending_transformations: Vec<Transformation>,
  transformations: Vec<Transformation>,
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

  pub fn gen_id(&mut self) -> u64 {
    let encoded: Vec<u8> = bincode::serialize(&self.transformations).unwrap();
    self.id = hash_bytes(&encoded);
    self.id
  }

  pub fn id(&self) -> u64 {
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

  fn get_arg_column(&self, argument: &Argument) -> Result<(u64,Column,ColumnIndex),MechError> {
    let arg_dims = self.get_arg_dim(argument);
    let (arg_name, table_id, row, col) = argument;
    let table = self.get_table(table_id)?;
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
      (TableIndex::Table(ix_table_id),TableIndex::Alias(alias))  => {
        let ix_table = self.get_table(&ix_table_id)?;
        let ix_table_brrw = ix_table.borrow();

        if ix_table_brrw.cols != 1 {
          return Err(MechError::GenericError(9237));
        }

        let ix = match ix_table_brrw.get_column_unchecked(0) {
          Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
          Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
          _ => {
            return Err(MechError::GenericError(9232));
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
            if ix_table_brrw.cols != 1 || 
                ix_table_brrw.rows != table_brrw.rows * table_brrw.cols 
            {
              return Err(MechError::GenericError(9233));
            }

            let ix = match ix_table_brrw.get_column_unchecked(0) {
              Column::Bool(bool_col) => ColumnIndex::Bool(bool_col),
              Column::Index(ix_col) => ColumnIndex::IndexCol(ix_col),
              _ => {
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

  fn get_arg_columns(&self, arguments: &Vec<Argument>) -> Result<Vec<(u64,Column,ColumnIndex)>,MechError> {
    let mut argument_columns = vec![];
    for argument in arguments {
      let arg_col = self.get_arg_column(argument)?;
      argument_columns.push(arg_col);
    }
    Ok(argument_columns)
  }

  fn get_whole_table_arg_cols(&self, argument: &Argument) -> Result<Vec<Column>,MechError> {
    let (_,table_id,row,col) = argument;
    let lhs_table = self.get_table(&table_id)?;
    let lhs_brrw = lhs_table.borrow();
    Ok(lhs_brrw.get_columns(&col).unwrap())
  }

  fn get_out_column(&self, out: &Out, rows: usize, col_kind: ValueKind) -> Result<Column,MechError> {
    let (out_table_id, _, _) = out;
    let table = self.get_table(out_table_id)?;
    let mut t = table.borrow_mut();
    let cols = t.cols;
    t.resize(rows,cols);
    t.set_col_kind(0, col_kind);
    let column = t.get_column_unchecked(0);
    Ok(column)
  }

  fn get_arg_dims(&self, arguments: &Vec<Argument>) -> Result<Vec<TableShape>,MechError> {
    let mut arg_shapes = Vec::new();
    for argument in arguments {
      arg_shapes.push(self.get_arg_dim(argument)?);
    }
    Ok(arg_shapes)
  }

  fn get_arg_dim(&self, argument: &Argument) -> Result<TableShape,MechError> {
    let (_, table_id, row, column) = argument;
    let table = self.get_table(table_id)?;
    let t = table.borrow();
    let dim = match (row, column) {
      (TableIndex::All, TableIndex::All) => (t.rows, t.cols),
      (TableIndex::All,TableIndex::Index(_)) |
      (TableIndex::All, TableIndex::Alias(_)) => (t.rows, 1),
      (TableIndex::Index(_),TableIndex::None) |
      (TableIndex::Index(_),TableIndex::Index(_)) |
      (TableIndex::Index(_),TableIndex::Alias(_)) => (1,1),
      (TableIndex::Table(ix_table_id),TableIndex::Alias(_)) |
      (TableIndex::Table(ix_table_id),TableIndex::None) => (2,1),
      _ => {return Err(MechError::GenericError(6384));},
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
        Err(MechError::GenericError(7372))
      }
      _ => {
        match self.compile_tfm(tfm.clone()) {
          Ok(()) => Ok(()),
          Err(mech_error_kind) => {
            self.unsatisfied_transformation = Some((mech_error_kind.clone(),tfm));
            self.state = BlockState::Unsatisfied;
            return Err(mech_error_kind);        
          }
        }
      }
    }
  }

  fn compile_tfm(&mut self, tfm: Transformation) -> Result<(), MechError> {
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
        self.tables.insert_alias(*alias, *table_id)?;
      },
      Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
        let mut table = self.tables.get_table_by_id(table_id.unwrap()).unwrap().borrow_mut();
        table.set_column_alias(*column_ix,*column_alias);
      },
      Transformation::Select{table_id, indices, out} => {
        let src_table = self.get_table(table_id)?;
        let out_table = self.get_table(out)?;

        let (row, column) = indices[0].clone();
        let argument = (0,*table_id,row,column);
        //let arg_col = self.get_arg_column(&argument)?;

        match indices[0] {
          // Select an entire table
          (TableIndex::All, TableIndex::All) => {
            match out {
              TableId::Global(gid) => {
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
                self.plan.push(CopyT{arg: src_table.clone(), out: out_table.clone()});
              }
              _ => {return Err(MechError::GenericError(6383));},
            }
          }
          // Select a column by row index
          (TableIndex::All, TableIndex::Index(_)) |
          // Select a column by alias
          (TableIndex::All, TableIndex::Alias(_)) => {
            let (_, arg_col,_) = self.get_arg_column(&(0,*table_id,row,column))?;
            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),arg_col.len(),arg_col.kind())?;
            match (&arg_col, &out_col) {
              (Column::U8(arg), Column::U8(out)) => self.plan.push(CopyVV::<u8>{arg: arg.clone(), out: out.clone()}),
              (Column::Bool(arg), Column::Bool(out)) => self.plan.push(CopyVV::<bool>{arg: arg.clone(), out: out.clone()}),
              _ => {return Err(MechError::GenericError(6382));},
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
              _ => {return Err(MechError::GenericError(6381));},
            }
          }
          // Select a number of specific elements by numerical index or lorgical index
          (TableIndex::Table(ix_table_id), TableIndex::None) => {
            let ix_table = self.get_table(&ix_table_id)?;
            let ix_col = ix_table.borrow().get_column_unchecked(0);
            
            let src_brrw = src_table.borrow();
            let mut arg_col = src_brrw.get_column_unchecked(0);

            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;

            match (&arg_col, &ix_col, &out_col) {
              (Column::U8(arg), Column::Bool(ix), Column::U8(out)) => self.plan.push(CopyVB::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
              (Column::U8(arg), Column::U8(ix), Column::U8(out)) => self.plan.push(CopyVI::<u8>{arg: arg.clone(), ix: ix.clone(), out: out.clone()}),
              _ => {return Err(MechError::GenericError(6380));},
            }
          }
          (TableIndex::Index(row_ix), TableIndex::Alias(column_alias)) => {
            let (_, arg_col,arg_ix) = self.get_arg_column(&(0,*table_id,row,column))?;
            let out_col = self.get_out_column(&(*out,TableIndex::All,TableIndex::All),1,arg_col.kind())?;
            match (&arg_col, &arg_ix, &out_col) {
              (Column::U8(arg), ColumnIndex::Index(ix), Column::U8(out)) => self.plan.push(CopySS::<u8>{arg: arg.clone(), ix: *ix, out: out.clone()}),
              x => {
                return Err(MechError::GenericError(6388));},
            }
          }
          _ => {return Err(MechError::GenericError(6379));},
        }
      }
      Transformation::Set{src_id, src_row, src_col, dest_id, dest_row, dest_col} => {
        let arguments = vec![(0,*src_id,*src_row,*src_col),(0,*dest_id,*dest_row,*dest_col)];
        let arg_shapes = self.get_arg_dims(&arguments)?;
        println!("{:?}", arg_shapes);
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
        table_brrw.set(0,0,value.clone());
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

              for (col_ix,lhs_column) in lhs_columns.iter().enumerate() {
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

              for (col_ix,rhs_column) in rhs_columns.iter().enumerate() {
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
                  ((Column::U8(lhs), Column::U8(rhs)),Column::U8(out)) => {
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
        } else if *name == *MATH_NEGATE {
          let arg_dims = self.get_arg_dims(&arguments)?;
          match &arg_dims[0] {
            TableShape::Column(rows) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *rows, ValueKind::I8)?;
              match (&argument_columns[0], &out_column) {
                ((_,Column::I8(arg),_), Column::I8(out)) => {
                  self.plan.push(NegateV::<i8>{arg: arg.clone(), out: out.clone() });
                }
                _ => {return Err(MechError::GenericError(1961));},
              }
            }
            TableShape::Scalar => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, 1, ValueKind::I8)?;
              match (&argument_columns[0], &out_column) {
                ((_,Column::I8(arg),_), Column::I8(out)) => {
                  self.plan.push(NegateS::<i8>{arg: arg.clone(), out: out.clone() });
                }
                _ => {return Err(MechError::GenericError(1962));},
              }
            }
            _ => {return Err(MechError::GenericError(1963));},
          }
        } else if *name == *LOGIC_NOT {
          let arg_dims = self.get_arg_dims(&arguments)?;
          match &arg_dims[0] {
            TableShape::Column(rows) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *rows, ValueKind::Bool)?;
              match (&argument_columns[0], &out_column) {
                ((_,Column::Bool(arg),_), Column::Bool(out)) => {
                  self.plan.push(NotV{arg: arg.clone(), out: out.clone() });
                }
                _ => {return Err(MechError::GenericError(1964));},
              }
            }
            _ => {return Err(MechError::GenericError(1965));},
          }
        } else if *name == *LOGIC_AND ||
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
        } else if *name == *COMPARE_GREATER__THAN ||
                  *name == *COMPARE_GREATER__THAN__EQUAL ||
                  *name == *COMPARE_LESS__THAN__EQUAL ||
                  *name == *COMPARE_EQUAL ||
                  *name == *COMPARE_NOT__EQUAL ||
                  *name == *COMPARE_LESS__THAN 
        {
          let arg_dims = self.get_arg_dims(&arguments)?;
          match (&arg_dims[0],&arg_dims[1]) {
            (TableShape::Scalar, TableShape::Scalar) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, 1, ValueKind::Bool)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_GREATER__THAN { self.plan.push(GreaterSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN { self.plan.push(LessSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_GREATER__THAN__EQUAL { self.plan.push(GreaterEqualSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN__EQUAL { self.plan.push(LessEqualSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_EQUAL { self.plan.push(EqualSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualSS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else {return Err(MechError::GenericError(1241));}
                }
                ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualSS::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualSS::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }                
                  else {return Err(MechError::GenericError(1242));}
                }
                ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualSS::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualSS::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }                
                  else {return Err(MechError::GenericError(1243));}
                }
                _ => {return Err(MechError::GenericError(1240));},
              }
            }
            (TableShape::Column(rows), TableShape::Scalar) => {
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *rows, ValueKind::Bool)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_GREATER__THAN { self.plan.push(GreaterThanVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN { self.plan.push(LessThanVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_GREATER__THAN__EQUAL { self.plan.push(GreaterThanEqualVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN__EQUAL { self.plan.push(LessThanEqualVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_EQUAL { self.plan.push(EqualVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVS::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                }
                ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualVS::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVS::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }                
                }
                ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualVS::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVS::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }                
                }
                _ => {return Err(MechError::GenericError(1252));},
              }
            }
            (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
              if lhs_rows != rhs_rows {
                return Err(MechError::GenericError(6523));
              }
              let mut argument_columns = self.get_arg_columns(arguments)?;
              let out_column = self.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
              match (&argument_columns[0], &argument_columns[1], &out_column) {
                ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_GREATER__THAN { self.plan.push(GreaterThanVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN { self.plan.push(LessThanVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_GREATER__THAN__EQUAL { self.plan.push(GreaterThanEqualVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_LESS__THAN__EQUAL { self.plan.push(LessThanEqualVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_EQUAL { self.plan.push(EqualVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVV::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                }
                ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualVV::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVV::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                }
                ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
                  if *name == *COMPARE_EQUAL { self.plan.push(EqualVV::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                  else if *name == *COMPARE_NOT__EQUAL { self.plan.push(NotEqualVV::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
                }
                _ => {return Err(MechError::GenericError(1242));},
              }
            }
            x => {return Err(MechError::GenericError(6348));},
          }                    
        } else if *name == *TABLE_APPEND {
          let arg_shape = self.get_arg_dim(&arguments[0])?;
          let (_,_,arow_ix,_) = arguments[0];

          let (_,src_table_id,src_rows,src_cols) = arguments[0];
          let (dest_table_id, _, _) = out;
        
          let src_table = self.get_table(&src_table_id)?;
          let dest_table = self.get_table(dest_table_id)?;

          {
            let mut src_table_brrw = src_table.borrow_mut();
            let mut dest_table_brrw = dest_table.borrow_mut();
            match dest_table_brrw.kind() {
              ValueKind::Empty => {
                dest_table_brrw.resize(src_table_brrw.rows,src_table_brrw.cols);
                dest_table_brrw.set_kind(src_table_brrw.kind());
                dest_table_brrw.rows = 0;
              },
              x => {
              }
            }
          }
          
          let dest_shape = {dest_table.borrow().shape()};
          match (arg_shape,arow_ix,dest_shape) {
            (TableShape::Scalar,TableIndex::Index(ix),TableShape::Column(_)) => {
              self.plan.push(AppendRowSV{arg: src_table.clone(), ix: ix-1, out: dest_table.clone()});
            }
            x => {
              self.plan.push(AppendRowT{arg: src_table.clone(), out: dest_table.clone()});
            },
          }
        } else if *name == *TABLE_RANGE {
          let mut argument_columns = self.get_arg_columns(arguments)?;
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?;
          match (&argument_columns[0], &argument_columns[1]) {
            ((_,Column::U8(start),_), (_,Column::U8(end),_)) => {  
              let fxn = Function::RangeU8((start.clone(),end.clone(),out_table.clone()));
              self.plan.push(fxn);
            }
            _ => {return Err(MechError::GenericError(6349));},
          }
        } else if *name == *STATS_SUM {
          let (arg_name,arg_table_id,_,_) = arguments[0];
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?;
          let mut out_brrw = out_table.borrow_mut();
          out_brrw.set_kind(ValueKind::U8);
          if arg_name == *COLUMN {
            let arg = self.get_arg_columns(arguments)?[0].clone();
            let out_table = self.get_table(out_table_id)?;
            out_brrw.resize(1,1);
            let out_col = out_brrw.get_column_unchecked(0).get_u8().unwrap();
            match arg {
              (_,Column::U8(col),ColumnIndex::Index(_)) => self.plan.push(StatsSumCol::<u8>{col: col.clone(), out: out_col.clone()}),
              (_,Column::U8(col),ColumnIndex::All) => self.plan.push(StatsSumCol::<u8>{col: col.clone(), out: out_col.clone()}),
              (_,Column::U8(col),ColumnIndex::Bool(ix_col)) => self.plan.push(StatsSumColVIx{col: col.clone(), ix: ix_col.clone(), out: out_col.clone()}),
              (_,Column::Reference((ref table, (ColumnIndex::Bool(ix_col), ColumnIndex::None))),_) => self.plan.push(StatsSumColTIx{col: table.clone(), ix: ix_col.clone(), out: out_col.clone()}),
              x => {return Err(MechError::GenericError(6351));},
            }
          } else if arg_name == *ROW { 
            let arg_table = self.get_table(&arg_table_id)?;
            out_brrw.resize(arg_table.borrow().rows,1);
            let out_col = out_brrw.get_column_unchecked(0).get_u8().unwrap();
            self.plan.push(StatsSumRow{table: arg_table.clone(), out: out_col.clone()})
          } else if arg_name == *TABLE {
            let arg_table = self.get_table(&arg_table_id)?;
            out_brrw.resize(1,1);
            let out_col = out_brrw.get_column_unchecked(0).get_u8().unwrap();
            self.plan.push(StatsSumTable{table: arg_table.clone(), out: out_col.clone()})
          } else {
            return Err(MechError::GenericError(6352));
          }
        } else if *name == *SET_ANY {
          let (arg_name, mut arg_column,_) = self.get_arg_columns(arguments)?[0].clone();
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?;
          let mut out_brrw = out_table.borrow_mut();
          out_brrw.set_col_kind(0,ValueKind::Bool);
          out_brrw.resize(1,1);
          let out_col = out_brrw.get_column_unchecked(0).get_bool().unwrap();
          if arg_name == *COLUMN {
            match arg_column {
              Column::Bool(col) => self.plan.push(SetAnyCol{col: col.clone(), out: out_col.clone()}),
              _ => {return Err(MechError::GenericError(6391));},
            }
          }
        } else if *name == *SET_ALL {
          let (arg_name, mut arg_column,_) = self.get_arg_columns(arguments)?[0].clone();
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?;
          let mut out_brrw = out_table.borrow_mut();
          out_brrw.set_col_kind(0,ValueKind::Bool);
          out_brrw.resize(1,1);
          let out_col = out_brrw.get_column_unchecked(0).get_bool().unwrap();
          if arg_name == *COLUMN {
            match arg_column {
              Column::Bool(col) => self.plan.push(SetAllCol{col: col.clone(), out: out_col.clone()}),
              _ => {return Err(MechError::GenericError(6395));},
            }
          }          
        } else if *name == *TABLE_VERTICAL__CONCATENATE {

          // Get all of the tables
          let mut arg_tables = vec![];
          let mut rows = 0;
          let mut cols = 0;
          for (_,table_id,_,_) in arguments {
            let table = self.get_table(table_id)?;
            arg_tables.push(table);
          }

          // Each table should have the same number of columns
          let cols = arg_tables[0].borrow().cols;
          let consistent_cols = arg_tables.iter().all(|arg| {arg.borrow().cols == cols});
          if consistent_cols == false {
            return Err(MechError::GenericError(1243));
          }
          
          // Check to make sure column types are consistent
          let col_kinds: Vec<ValueKind> = arg_tables[0].borrow().col_kinds.clone();
          let consistent_col_kinds = arg_tables.iter().all(|arg| arg.borrow().col_kinds.iter().zip(&col_kinds).all(|(k1,k2)| *k1 == *k2));
          if consistent_cols == false {
            return Err(MechError::GenericError(1244));
          }

          // Add up the rows
          let rows = arg_tables.iter().fold(0, |acc, table| acc + table.borrow().rows);
          
          // Resize out table to match dimensions 
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?;
          let mut out_brrw = out_table.borrow_mut();
          out_brrw.resize(rows,cols);

          // Set out column kind and push a concat function
          for (ix, kind) in (0..cols).zip(col_kinds.clone()) {
            out_brrw.set_col_kind(ix, kind);
            let out_col = out_brrw.get_column_unchecked(ix).clone();
            let mut argument_columns = vec![];       
            for table in &arg_tables {
              let table_brrw = table.borrow();
              let column = table_brrw.get_column(&TableIndex::Index(ix+1))?;
              argument_columns.push(column.clone());
            }

            match out_col {
              Column::U8(ref out_c) => {
                let mut u8_cols:Vec<ColumnV<u8>> = vec![];
                for colv in argument_columns {
                  u8_cols.push(colv.get_u8()?.clone());
                }
                let fxn = ConcatV::<u8>{args: u8_cols, out: out_c.clone()};
                self.plan.push(fxn);
              }
              Column::Bool(ref out_c) => {
                let mut bool_cols:Vec<ColumnV<bool>> = vec![];
                for colv in argument_columns {
                  bool_cols.push(colv.get_bool()?.clone());
                }
                let fxn = ConcatV::<bool>{args: bool_cols, out: out_c.clone()};
                self.plan.push(fxn);
              }
              Column::String(ref out_c) => {
                let mut cols:Vec<ColumnV<MechString>> = vec![];
                for colv in argument_columns {
                  cols.push(colv.get_string()?.clone());
                }
                let fxn = ConcatV::<MechString>{args: cols, out: out_c.clone()};
                self.plan.push(fxn);
              }
              _ => {return Err(MechError::GenericError(6361));},
            }
            
          }
        } else if *name == *TABLE_HORIZONTAL__CONCATENATE {
          // Get all of the tables
          let mut arg_tables = vec![];
          let mut rows = 0;
          let mut cols = 0;
          for (_,table_id,_,_) in arguments {
            let table = self.get_table(table_id)?;
            arg_tables.push(table);
          }
          let arg_shapes = self.get_arg_dims(&arguments)?;

          // Each table should have the same number of rows or be scalar
          let rows = arg_tables.iter().map(|t| t.borrow().rows).max().unwrap();
          let consistent_rows = arg_tables.iter().all(|arg| {
            let t_rows = arg.borrow().rows;
            t_rows == rows || t_rows == 1
          });

          if consistent_rows == false {
            return Err(MechError::GenericError(1245));
          }

          // Add up the columns
          let cols = arg_tables.iter().fold(0, |acc, table| acc + table.borrow().cols);
          
          let (out_table_id, _, _) = out;
          let out_table = self.get_table(out_table_id)?.clone();
          let mut o = out_table.borrow_mut();
          o.resize(rows,cols);
          let mut out_column_ix = 0;

          for (table, shape) in arg_tables.iter().zip(arg_shapes) {
            let t = table.borrow();
            match shape {
              TableShape::Scalar => {
                let mut arg_col = t.get_column_unchecked(0);
                o.set_col_kind(out_column_ix, arg_col.kind());
                let mut out_col = o.get_column_unchecked(out_column_ix);
                match (&arg_col, &out_col) {
                  (Column::U8(arg), Column::U8(out)) => self.plan.push(CopySS::<u8>{arg: arg.clone(), ix: 0, out: out.clone()}),
                  (Column::String(arg), Column::String(out)) => self.plan.push(CopySS::<MechString>{arg: arg.clone(), ix: 0, out: out.clone()}),
                  (Column::Bool(arg), Column::Bool(out)) => self.plan.push(CopySS::<bool>{arg: arg.clone(), ix: 0, out: out.clone()}),
                  _ => {return Err(MechError::GenericError(6366));},
                };
                out_column_ix += 1;
              }
              TableShape::Column(_) => {
                let mut arg_col = t.get_column_unchecked(0);
                o.set_col_kind(out_column_ix, arg_col.kind());
                let mut out_col = o.get_column_unchecked(out_column_ix);
                let fxn = match (&arg_col, &out_col) {
                  (Column::U8(arg), Column::U8(out)) => self.plan.push(CopyVV::<u8>{arg: arg.clone(), out: out.clone()}),
                  (Column::U64(arg), Column::U64(out)) => self.plan.push(CopyVV::<u64>{arg: arg.clone(), out: out.clone()}),
                  _ => {return Err(MechError::MissingFunction(*name));},
                };
                out_column_ix += 1;
              }
              _ => {return Err(MechError::GenericError(6364));},
            }
          }
        } else {
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
use ::hash_str;

lazy_static! {
  static ref TABLE_COPY: u64 = hash_str("table/copy");
  static ref TABLE_SPLIT: u64 = hash_str("table/split");
  static ref GRAMS: u64 = hash_str("g");
  static ref KILOGRAMS: u64 = hash_str("kg");
  static ref HERTZ: u64 = hash_str("Hz");
  static ref SECONDS: u64 = hash_str("s");
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
          let whenever_ix_table_id = hash_str("~");
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
          let whenever_ix_table_id = hash_str("~");
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
                  let split_table_id = hash_str(&format!("table/split/{:?}/{:?}",old_table_id,row));
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