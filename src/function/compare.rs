use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

// Greater Than Vector : Vector
#[derive(Debug)]
pub struct GreaterVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs > *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Less Than Vector : Vector
#[derive(Debug)]
pub struct LessVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs < *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Less Than Equal Vector : Vector
#[derive(Debug)]
pub struct LessEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs <= *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Vector
#[derive(Debug)]
pub struct GreaterEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterEqualVV<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs >= *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Equal Vector : Vector
#[derive(Debug)]
pub struct EqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs == *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Equal Vector : Vector
#[derive(Debug)]
pub struct NotEqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVV<T> 
where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Clone
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs != *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Greater Than Vector : Scalar
#[derive(Debug)]
pub struct GreaterVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs > rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Less Than Vector : Scalar
#[derive(Debug)]
pub struct LessVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs < rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Less Than Equal Vector : Scalar
#[derive(Debug)]
pub struct LessEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs <= rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// GreaterThanEqual Vector : Scalar
#[derive(Debug)]
pub struct GreaterEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterEqualVS<T> 
where T: PartialEq + Eq + Copy + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs >= rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Equal Vector : Scalar
#[derive(Debug)]
pub struct EqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = &self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs == *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Not Equal Vector : Scalar
#[derive(Debug)]
pub struct NotEqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualVS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    let rhs = &self.rhs.borrow()[0];
    self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs != *rhs); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// == Scalar : Scalar
#[derive(Debug)]
pub struct EqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for EqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] == (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// != Equal Scalar : Scalar
#[derive(Debug)]
pub struct NotEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for NotEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] != (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// > Scalar : Scalar
#[derive(Debug)]
pub struct GreaterSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] > (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// < Scalar : Scalar
#[derive(Debug)]
pub struct LessSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] < (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// >= Scalar : Scalar
#[derive(Debug)]
pub struct GreaterEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for GreaterEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] >= (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// < Scalar : Scalar
#[derive(Debug)]
pub struct LessEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
}
impl<T> MechFunction for LessEqualSS<T> 
where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] <= (self.rhs.borrow())[0];
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

compare_infix!(compare_greater__than,GreaterSS,GreaterVS,GreaterVV);
compare_infix!(compare_less__than,LessSS,LessVS,LessVV);
compare_infix!(compare_greater__than__equal,GreaterEqualSS,GreaterEqualVS,GreaterEqualVV);
compare_infix!(compare_less__than__equal,LessEqualSS,LessEqualVS,LessEqualVV);

#[macro_export]
macro_rules! compare_infix {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt) => (
    pub fn $func_name(block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
      let arg_dims = block.get_arg_dims(&arguments)?;
      match (&arg_dims[0],&arg_dims[1]) {
        (TableShape::Scalar, TableShape::Scalar) => {
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op1::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            _ => {return Err(MechError::GenericError(1240));},
          }
        }
        (TableShape::Column(rows), TableShape::Scalar) => {
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            _ => {return Err(MechError::GenericError(1252));},
          }
        }
        (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
          if lhs_rows != rhs_rows {
            return Err(MechError::GenericError(6523));
          }
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            _ => {return Err(MechError::GenericError(1242));},
          }
        }
        x => {return Err(MechError::GenericError(6348));},
      }
      Ok(())
    }
  )
}

compare_infix_eq!(compare_equal,EqualSS,EqualVS,EqualVV);
compare_infix_eq!(compare_not__equal,NotEqualSS,NotEqualVS,NotEqualVV);

#[macro_export]
macro_rules! compare_infix_eq {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt) => (
    pub fn $func_name(block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
      let arg_dims = block.get_arg_dims(&arguments)?;
      match (&arg_dims[0],&arg_dims[1]) {
        (TableShape::Scalar, TableShape::Scalar) => {
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op1::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
              block.plan.push($op1::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
              block.plan.push($op1::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            _ => {return Err(MechError::GenericError(1240));},
          }
        }
        (TableShape::Column(rows), TableShape::Scalar) => {
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) 
            }
            ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
              block.plan.push($op2::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) 
            }
            ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
              block.plan.push($op2::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) 
            }
            _ => {return Err(MechError::GenericError(1252));},
          }
        }
        (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
          if lhs_rows != rhs_rows {
            return Err(MechError::GenericError(6523));
          }
          let mut argument_columns = block.get_arg_columns(arguments)?;
          let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
          match (&argument_columns[0], &argument_columns[1], &out_column) {
            ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {
              block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {
              block.plan.push($op3::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {
              block.plan.push($op3::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})
            }
            _ => {return Err(MechError::GenericError(1242));},
          }
        }
        x => {return Err(MechError::GenericError(6348));},
      }
      Ok(())
    }
  )
}
