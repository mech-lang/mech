use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref LOGIC_AND: u64 = hash_str("logic/and");  
  pub static ref LOGIC_OR: u64 = hash_str("logic/or");
  pub static ref LOGIC_NOT: u64 = hash_str("logic/not");  
  pub static ref LOGIC_XOR: u64 = hash_str("logic/xor");    
}

// And Vector : Vector
#[derive(Debug)]
pub struct AndVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for AndVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs && *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// And Scalar : Scalar
#[derive(Debug)]
pub struct AndSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for AndSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] && (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Or Vector : Vector
#[derive(Debug)]
pub struct OrVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for OrVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs || *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Or Scalar : Scalar
#[derive(Debug)]
pub struct OrSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for OrSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] || (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Xor Vector : Vector
#[derive(Debug)]
pub struct XorVV {
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for XorVV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs ^ *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Xor Scalar : Scalar
#[derive(Debug)]
pub struct XorSS
{
  pub lhs: Arg<bool>, pub rhs: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for XorSS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] ^ (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Vector
#[derive(Debug)]
pub struct NotV {
  pub arg: Arg<bool>, pub out: Out<bool>
}

impl MechFunction for NotV {
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = !(*arg)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Scalar
#[derive(Debug)]
pub struct NotS
{
  pub arg: Arg<bool>, pub out: Out<bool>
}
impl MechFunction for NotS 
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = !(self.arg.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct LogicNot {}

impl MechFunctionCompiler for LogicNot {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_dims = block.get_arg_dims(&arguments)?;
    match &arg_dims[0] {
      TableShape::Column(rows) => {
        let mut argument_columns = block.get_arg_columns(arguments)?;
        let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
        match (&argument_columns[0], &out_column) {
          ((_,Column::Bool(arg),_), Column::Bool(out)) => {
            block.plan.push(NotV{arg: arg.clone(), out: out.clone() });
          }
          _ => {return Err(MechError::GenericError(1964));},
        }
      }
      _ => {return Err(MechError::GenericError(1965));},
    }
    Ok(())
  }
}

logic_compiler!(logic_and,AndSS,AndSS,AndVV);
logic_compiler!(logic_or,OrSS,OrSS,OrVV);
logic_compiler!(logic_xor,XorSS,XorSS,XorVV);

#[macro_export]
macro_rules! logic_compiler {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt) => (

    pub struct $func_name {}

    impl MechFunctionCompiler for $func_name {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let arg_dims = block.get_arg_dims(&arguments)?;
        match (&arg_dims[0],&arg_dims[1]) {
          (TableShape::Scalar, TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                block.plan.push($op1{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() });
              }
              _ => {return Err(MechError::GenericError(1340));},
            }
          }
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() });
              }
              _ => {return Err(MechError::GenericError(1342));},
            }
          }
          _ => {return Err(MechError::GenericError(1341));},
        }
        Ok(())
      }
    }
  )
}