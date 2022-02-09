use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref COMPARE_GREATER__THAN: u64 = hash_str("compare/greater-than");
  pub static ref COMPARE_LESS__THAN: u64 = hash_str("compare/less-than");
  pub static ref COMPARE_GREATER__THAN__EQUAL: u64 = hash_str("compare/greater-than-equal");
  pub static ref COMPARE_LESS__THAN__EQUAL: u64 = hash_str("compare/less-than-equal");
  pub static ref COMPARE_EQUAL: u64 = hash_str("compare/equal");
  pub static ref COMPARE_NOT__EQUAL: u64 = hash_str("compare/not-equal");
}

compare_infix_ss!(GreaterSS,>);
compare_infix_ss!(LessSS,<);
compare_infix_ss!(LessEqualSS,<=);
compare_infix_ss!(GreaterEqualSS,>=);
compare_infix_ss!(EqualSS,==);
compare_infix_ss!(NotEqualSS,!=);

compare_infix_vv!(GreaterVV,>);
compare_infix_vv!(LessVV,<);
compare_infix_vv!(LessEqualVV,<=);
compare_infix_vv!(GreaterEqualVV,>=);
compare_infix_vv!(EqualVV,==);
compare_infix_vv!(NotEqualVV,!=);

compare_infix_par_vv!(ParGreaterVV,>);
compare_infix_par_vv!(ParLessVV,<);
compare_infix_par_vv!(ParLessEqualVV,<=);
compare_infix_par_vv!(ParGreaterEqualVV,>=);
compare_infix_par_vv!(ParEqualVV,==);
compare_infix_par_vv!(ParNotEqualVV,!=);

compare_infix_vs!(GreaterVS,>);
compare_infix_vs!(LessVS,<);
compare_infix_vs!(LessEqualVS,<=);
compare_infix_vs!(GreaterEqualVS,>=);
compare_infix_vs!(EqualVS,==);
compare_infix_vs!(NotEqualVS,!=);

compare_infix_par_vs!(ParGreaterVS,>);
compare_infix_par_vs!(ParLessVS,<);
compare_infix_par_vs!(ParLessEqualVS,<=);
compare_infix_par_vs!(ParGreaterEqualVS,>=);
compare_infix_par_vs!(ParEqualVS,==);
compare_infix_par_vs!(ParNotEqualVS,!=);

compare_infix_sv!(GreaterSV,>);
compare_infix_sv!(LessSV,<);
compare_infix_sv!(LessEqualSV,<=);
compare_infix_sv!(GreaterEqualSV,>=);
compare_infix_sv!(EqualSV,==);
compare_infix_sv!(NotEqualSV,!=);

compare_infix_par_sv!(ParGreaterSV,>);
compare_infix_par_sv!(ParLessSV,<);
compare_infix_par_sv!(ParLessEqualSV,<=);
compare_infix_par_sv!(ParGreaterEqualSV,>=);
compare_infix_par_sv!(ParEqualSV,==);
compare_infix_par_sv!(ParNotEqualSV,!=);

compare_eq_compiler!(compare_equal,EqualSS,EqualVS,EqualSV,EqualVV);
compare_eq_compiler!(compare_not__equal,NotEqualSS,NotEqualVS,NotEqualSV,NotEqualVV);

compare_compiler!(compare_greater__than,GreaterSS,GreaterVS,GreaterSV,GreaterVV);
compare_compiler!(compare_less__than,LessSS,LessVS,LessSV,LessVV);
compare_compiler!(compare_greater__than__equal,GreaterEqualSS,GreaterEqualVS,GreaterEqualSV,GreaterEqualVV);
compare_compiler!(compare_less__than__equal,LessEqualSS,LessEqualVS,LessEqualSV,LessEqualVV);

// Scalar : Scalar
#[macro_export]
macro_rules! compare_infix_ss {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Clone + Debug + std::cmp::PartialOrd
    {
      fn solve(&mut self) {
        (self.out.borrow_mut())[0] = (self.lhs.borrow())[0] $op (self.rhs.borrow())[0];
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Vector : Vector
#[macro_export]
macro_rules! compare_infix_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      fn solve(&mut self) {
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! compare_infix_par_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      fn solve(&mut self) {
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).zip(self.rhs.borrow().par_iter()).for_each(|((out, lhs), rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}


// Vector : Scalar
#[macro_export]
macro_rules! compare_infix_vs {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      fn solve(&mut self) {
        let rhs = &self.rhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! compare_infix_par_vs {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      fn solve(&mut self) {
        let rhs = &self.rhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).for_each(|(out, lhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

// Scalar : Vector
#[macro_export]
macro_rules! compare_infix_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd
    {
      fn solve(&mut self) {
        let lhs = &self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! compare_infix_par_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<bool>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: PartialEq + Eq + Debug + std::cmp::PartialOrd + Send + Sync
    {
      fn solve(&mut self) {
        let lhs = &self.lhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.rhs.borrow().par_iter()).for_each(|(out, rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! compare_compiler {
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
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1240));},
            }
          }
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1252));},
            }
          }
          (TableShape::Scalar,TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1255));},
            }
          }
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError::GenericError(6523));
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op4::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1242));},
            }
          }
          x => {return Err(MechError::GenericError(6348));},
        }
        Ok(())
      }
    }
  )
}

#[macro_export]
macro_rules! compare_eq_compiler {
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
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1240));},
            }
          }
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => { block.plan.push($op2::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => { block.plan.push($op2::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              _ => {return Err(MechError::GenericError(1252));},
            }
          }
          (TableShape::Scalar,TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => { block.plan.push($op3::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => { block.plan.push($op3::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()}) }
              _ => {return Err(MechError::GenericError(1250));},
            }
          }
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError::GenericError(6523));
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op4::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {block.plan.push($op4::<bool>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {block.plan.push($op4::<MechString>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1242));},
            }
          }
          x => {return Err(MechError::GenericError(6348));},
        }
        Ok(())
      }
    }
  )
}
