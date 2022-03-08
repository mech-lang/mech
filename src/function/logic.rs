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

logic_infix_ss!(AndSS,&&);
logic_infix_ss!(OrSS,||);
logic_infix_ss!(XorSS,^);

logic_infix_vv!(AndVV,&&);
logic_infix_vv!(OrVV,||);
logic_infix_vv!(XorVV,^);

logic_infix_par_vv!(ParAndVV,&&);
logic_infix_par_vv!(ParOrVV,||);
logic_infix_par_vv!(ParXorVV,^);

logic_infix_vs!(AndVS,&&);
logic_infix_vs!(OrVS,||);
logic_infix_vs!(XorVS,^);

logic_infix_par_vs!(ParAndVS,&&);
logic_infix_par_vs!(ParOrVS,||);
logic_infix_par_vs!(ParXorVS,^);

logic_infix_sv!(AndSV,&&);
logic_infix_sv!(OrSV,||);
logic_infix_sv!(XorSV,^);

logic_infix_par_sv!(ParAndSV,&&);
logic_infix_par_sv!(ParOrSV,||);
logic_infix_par_sv!(ParXorSV,^);

logic_compiler!(LogicAnd,AndSS,AndVS,AndSV,AndVV);
logic_compiler!(LoigicOr,OrSS,OrVS,OrSV,OrVV);
logic_compiler!(LogicXor,XorSS,XorVS,XorSV,XorVV);

// Scalar : Scalars
#[macro_export]
macro_rules! logic_infix_ss {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name
    {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }
    impl MechFunction for $func_name 
    {
      fn solve(&self) {
        (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] $op (self.rhs.borrow())[0];
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Vector : Vector
#[macro_export]
macro_rules! logic_infix_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }

    impl MechFunction for $func_name {
      fn solve(&self) {
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! logic_infix_par_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }
    impl MechFunction for $func_name {
      fn solve(&self) {
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).zip(self.rhs.borrow().par_iter()).for_each(|((out, lhs), rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Vector : Scalar
#[macro_export]
macro_rules! logic_infix_vs {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }

    impl MechFunction for $func_name {
      fn solve(&self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs $op rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! logic_infix_par_vs {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }

    impl MechFunction for $func_name {
      fn solve(&self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).for_each(|(out, lhs)| *out = *lhs $op rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Scalar : Vector
#[macro_export]
macro_rules! logic_infix_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }

    impl MechFunction for $func_name {
      fn solve(&self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! logic_infix_par_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name {
      pub lhs: ColumnV<bool>, pub rhs: ColumnV<bool>, pub out: ColumnV<bool>
    }

    impl MechFunction for $func_name {
      fn solve(&self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.rhs.borrow().par_iter()).for_each(|(out, rhs)| *out = lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Not Vector
#[derive(Debug)]
pub struct NotV {
  pub arg: ColumnV<bool>, pub out: ColumnV<bool>
}

impl MechFunction for NotV {
  fn solve(&self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = !(*arg)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Scalar
#[derive(Debug)]
pub struct NotS
{
  pub arg: ColumnV<bool>, pub out: ColumnV<bool>
}
impl MechFunction for NotS 
{
  fn solve(&self) {
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
          x => {return Err(MechError{id: 8213, kind: MechErrorKind::GenericError(format!("{:?}",x))});},
        }
      }
      x => {return Err(MechError{id: 8214, kind: MechErrorKind::GenericError(format!("{:?}",x))});},
    }
    Ok(())
  }
}

#[macro_export]
macro_rules! logic_compiler {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt,$op4:tt) => (

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
              x => {return Err(MechError{id: 8215, kind: MechErrorKind::GenericError(format!("{:?}",x))});},
            }
          }
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
                block.plan.push($op4{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() });
              }
              x => {return Err(MechError{id: 8216, kind: MechErrorKind::GenericError(format!("{:?}",x))});},
            }
          }
          x => {return Err(MechError{id: 8217, kind: MechErrorKind::GenericError(format!("{:?}",x))});},
        }
        Ok(())
      }
    }
  )
}