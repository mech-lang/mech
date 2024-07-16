use crate::*;

use hashbrown::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

// Types
// -------

//pub type TableRef = Rc<RefCell<Table>>;
//pub type BlockRef = Rc<RefCell<Block>>;

//pub type Transaction = Vec<Change>;
//pub type Register = (TableId,RegisterIndex,RegisterIndex);
pub type ParserErrorReport = Vec<ParserErrorContext>;


pub type BlockId = u64;
pub type ArgumentName = u64;
//pub type Argument = (ArgumentName, TableId, Vec<(TableIndex, TableIndex)>);
//pub type Out = (TableId, TableIndex, TableIndex);


//pub type Arg<T> = ColumnV<T>;
//pub type ArgTable = Rc<RefCell<Table>>;
//pub type OutTable = Rc<RefCell<Table>>;

//pub type StringDictionary = Rc<RefCell<HashMap<u64,MechString>>>;

pub type TableIx = usize;
pub type Alias = u64;

// Traits
// -----------

pub trait MechNumArithmetic<T>: Add<Output = T> + 
                                Sub<Output = T> + 
                                Div<Output = T> + 
                                Mul<Output = T> + 
                                Pow<T, Output = T> + 
                                AddAssign +
                                SubAssign +
                                MulAssign +
                                DivAssign +
                                Sized {}

pub trait MechFunctionCompiler {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &Out) -> std::result::Result<(),MechError>;
}

pub trait MechFunction {
  fn solve(&self);
  fn to_string(&self) -> String;
}