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

compare_infix_vv!(GreaterVV,>);
compare_infix_vv!(LessVV,<);
compare_infix_vv!(LessEqualVV,<=);
compare_infix_vv!(GreaterEqualVV,>=);
compare_infix_vv!(EqualVV,==);
compare_infix_vv!(NotEqualVV,!=);

/*compare_infix_par_vv!(ParGreaterVV,>);
compare_infix_par_vv!(ParLessVV,<);
compare_infix_par_vv!(ParLessEqualVV,<=);
compare_infix_par_vv!(ParGreaterEqualVV,>=);
compare_infix_par_vv!(ParEqualVV,==);
compare_infix_par_vv!(ParNotEqualVV,!=);*/

compare_infix_vs!(GreaterVS,>);
compare_infix_vs!(LessVS,<);
compare_infix_vs!(LessEqualVS,<=);
compare_infix_vs!(GreaterEqualVS,>=);
compare_infix_vs!(EqualVS,==);
compare_infix_vs!(NotEqualVS,!=);

/*par_compare_infix_vs!(ParGreaterVS,>);
par_compare_infix_vs!(ParLessVS,<);
par_compare_infix_vs!(ParLessEqualVS,<=);
par_compare_infix_vs!(ParGreaterEqualVS,>=);
par_compare_infix_vs!(ParEqualVS,==);
par_compare_infix_vs!(ParNotEqualVS,!=);*/

compare_infix_sv!(GreaterSV,>);
compare_infix_sv!(LessSV,<);
compare_infix_sv!(LessEqualSV,<=);
compare_infix_sv!(GreaterEqualSV,>=);
compare_infix_sv!(EqualSV,==);
compare_infix_sv!(NotEqualSV,!=);

/*compare_infix_par_sv!(ParGreaterSV,>);
compare_infix_par_sv!(ParLessSV,<);
compare_infix_par_sv!(ParLessEqualSV,<=);
compare_infix_par_sv!(ParGreaterEqualSV,>=);
compare_infix_par_sv!(ParEqualSV,==);
compare_infix_par_sv!(ParNotEqualSV,!=);
*/

compare_eq_compiler!(compare_equal,EqualSS,EqualVS,EqualSV,EqualVV);
/*compare_eq_compiler!(compare_not__equal,Foo1,NotEqualVS,NotEqualSV,NotEqualVV);

compare_compiler!(compare_greater__than,Foo1,GreaterVS,GreaterSV,GreaterVV);
compare_compiler!(compare_less__than,Foo1,LessVS,LessSV,LessVV);
compare_compiler!(compare_greater__than__equal,Foo1,GreaterEqualVS,GreaterEqualSV,GreaterEqualVV);
compare_compiler!(compare_less__than__equal,Foo1,LessEqualVS,LessEqualSV,LessEqualVV);
*/

// Vector : Vector
#[macro_export]
macro_rules! compare_infix_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T,U> 
    {
      pub lhs: (ColumnV<T>, usize, usize), 
      pub rhs: (ColumnV<U>, usize, usize), 
      pub out: ColumnV<bool>
    }
    impl<T,U> MechFunction for $func_name<T,U> 
    where T: Clone + Debug + PartialEq + PartialOrd + Into<U>,
          U: Clone + Debug + PartialEq + PartialOrd + Into<T>,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        self.out.borrow_mut()
                .iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].iter())
                .zip(rhs.borrow()[*rsix..=*reix].iter())
                .for_each(|((out, lhs), rhs)| *out = *lhs $op U::into(rhs.clone()));
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! par_compare_infix_vv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T,U> 
    {
      pub lhs: (ColumnV<T>, usize, usize), 
      pub rhs: (ColumnV<U>, usize, usize), 
      pub out: ColumnV<bool>
    }
    impl<T,U> MechFunction for $func_name<T,U> 
    where T: Clone + Debug + PartialEq + PartialOrd + Into<U> + Send + Sync,
          U: Clone + Debug + PartialEq + PartialOrd + Into<T> + Send + Sync,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        self.out.borrow_mut()
                .par_iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].par_iter())
                .zip(rhs.borrow()[*rsix..=*reix].par_iter())
                .for_each(|((out, lhs), rhs)| *out = *lhs $op U::into(rhs.clone()));
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
    pub struct $func_name<T,U> 
    {
      pub lhs: (ColumnV<T>, usize, usize), 
      pub rhs: (ColumnV<U>, usize, usize), 
      pub out: ColumnV<bool>
    }
    impl<T,U> MechFunction for $func_name<T,U> 
    where T: Clone + Debug + PartialEq + PartialOrd + Into<U>,
          U: Clone + Debug + PartialEq + PartialOrd + Into<T>,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        let rhs = &rhs.borrow()[*rsix];
        self.out.borrow_mut()
                .iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].iter())
                .for_each(|(out, lhs)| *out = *lhs $op U::into(rhs.clone())); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

/*#[macro_export]
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
      fn solve(&self) {
        let rhs = &self.rhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).for_each(|(out, lhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}*/

// Scalar : Vector
#[macro_export]
macro_rules! compare_infix_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T,U> 
    {
      pub lhs: (ColumnV<T>, usize, usize), 
      pub rhs: (ColumnV<U>, usize, usize), 
      pub out: ColumnV<bool>
    }
    impl<T,U> MechFunction for $func_name<T,U> 
    where T: Clone + Debug + PartialEq + PartialOrd + Into<U>,
          U: Clone + Debug + PartialEq + PartialOrd + Into<T>,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        let lhs = &lhs.borrow()[*lsix];
        self.out.borrow_mut()
                .iter_mut()
                .zip(rhs.borrow()[*rsix..=*reix].iter())
                .for_each(|(out, rhs)| *out = T::into(lhs.clone()) $op *rhs ); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

/*
#[macro_export]
macro_rules! compare_infix_par_sv {
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
      fn solve(&self) {
        let lhs = &self.lhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(self.rhs.borrow().par_iter()).for_each(|(out, rhs)| *out = *lhs $op *rhs); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}*/

#[macro_export]
macro_rules! compare_compiler {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt,$op4:tt) => (
    pub struct $func_name {}

    impl MechFunctionCompiler for $func_name {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let arg_dims = block.get_arg_dims(&arguments)?;
        match (&arg_dims[0],&arg_dims[1]) {
          /*(TableShape::Scalar, TableShape::Scalar) => {
            resize_one(block,out);
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op1::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(1241));
              },
            }
          }
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op2::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1253));},
            }
          }
          (TableShape::Scalar,TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op3::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
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
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op4::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone()})}
              _ => {return Err(MechError::GenericError(1242));},
            }
          }*/
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
            resize_one(block,out);
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),ColumnIndex::All), (_,Column::U8(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::U8(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              ((_,Column::F32(lhs),ColumnIndex::All), (_,Column::F32(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::F32(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              ((_,Column::Bool(lhs),ColumnIndex::All), (_,Column::Bool(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::Bool(lhs),ColumnIndex::Index(lix)), (_,Column::Bool(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              ((_,Column::String(lhs),ColumnIndex::All), (_,Column::String(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::String(lhs),ColumnIndex::Index(lix)), (_,Column::String(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(1842));},
            }
          }
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              _ => {return Err(MechError::GenericError(1252));},
            }
          }
          (TableShape::Scalar,TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
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
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              ((_,Column::Bool(lhs),_), (_,Column::Bool(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              ((_,Column::String(lhs),_), (_,Column::String(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(1242));},
            }
          }
          x => {return Err(MechError::GenericError(6348));},
        }
        Ok(())
      }
    }
  )
}
