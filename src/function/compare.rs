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

par_compare_infix_vv!(ParGreaterVV,>);
par_compare_infix_vv!(ParLessVV,<);
par_compare_infix_vv!(ParLessEqualVV,<=);
par_compare_infix_vv!(ParGreaterEqualVV,>=);
par_compare_infix_vv!(ParEqualVV,==);
par_compare_infix_vv!(ParNotEqualVV,!=);

compare_infix_vs!(GreaterVS,>);
compare_infix_vs!(LessVS,<);
compare_infix_vs!(LessEqualVS,<=);
compare_infix_vs!(GreaterEqualVS,>=);
compare_infix_vs!(EqualVS,==);
compare_infix_vs!(NotEqualVS,!=);

par_compare_infix_vs!(ParGreaterVS,>);
par_compare_infix_vs!(ParLessVS,<);
par_compare_infix_vs!(ParLessEqualVS,<=);
par_compare_infix_vs!(ParGreaterEqualVS,>=);
par_compare_infix_vs!(ParEqualVS,==);
par_compare_infix_vs!(ParNotEqualVS,!=);

compare_infix_sv!(GreaterSV,>);
compare_infix_sv!(LessSV,<);
compare_infix_sv!(LessEqualSV,<=);
compare_infix_sv!(GreaterEqualSV,>=);
compare_infix_sv!(EqualSV,==);
compare_infix_sv!(NotEqualSV,!=);

par_compare_infix_sv!(ParGreaterSV,>);
par_compare_infix_sv!(ParLessSV,<);
par_compare_infix_sv!(ParLessEqualSV,<=);
par_compare_infix_sv!(ParGreaterEqualSV,>=);
par_compare_infix_sv!(ParEqualSV,==);
par_compare_infix_sv!(ParNotEqualSV,!=);

compare_eq_compiler!(CompareEqual,Foo,EqualVS,EqualSV,EqualVV);
compare_eq_compiler!(CompareNotEqual,Foo,NotEqualVS,NotEqualSV,NotEqualVV);

compare_compiler!(CompareGreater,Foo,GreaterVS,GreaterSV,GreaterVV);
compare_compiler!(CompareLess,Foo,LessVS,LessSV,LessVV);
compare_compiler!(CompareGreaterEqual,Foo,GreaterEqualVS,GreaterEqualSV,GreaterEqualVV);
compare_compiler!(CompareLessEqual,Foo,LessEqualVS,LessEqualSV,LessEqualVV);

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

#[macro_export]
macro_rules! par_compare_infix_vs {
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
        let rhs = &rhs.borrow()[*rsix];
        self.out.borrow_mut()
                .par_iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].par_iter())
                .for_each(|(out, lhs)| *out = *lhs $op U::into(rhs.clone())); 
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

#[macro_export]
macro_rules! par_compare_infix_sv {
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
        let lhs = &lhs.borrow()[*lsix];
        self.out.borrow_mut()
                .par_iter_mut()
                .zip(rhs.borrow()[*rsix..=*reix].par_iter())
                .for_each(|(out, rhs)| *out = T::into(lhs.clone()) $op *rhs ); 
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
            resize_one(block,out);
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, 1, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),ColumnIndex::All), (_,Column::U8(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::U8(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              ((_,Column::F32(lhs),ColumnIndex::All), (_,Column::F32(rhs),ColumnIndex::All), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,0), out: out.clone()})}
              ((_,Column::F32(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix)), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()})}
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(9027));
              },
            }
          }
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => { block.plan.push($op2{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,0), out: out.clone()}) }
              _ => {return Err(MechError::GenericError(1252));},
            }
          }
          (TableShape::Scalar,TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => { block.plan.push($op3{lhs: (lhs.clone(),0,0), rhs: (rhs.clone(),0,lhs.len()-1), out: out.clone()}) }
              _ => {return Err(MechError::GenericError(9028));},
            }
          }
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError::GenericError(9029));
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let out_column = block.get_out_column(out, *lhs_rows, ValueKind::Bool)?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::Bool(out)) => {block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone()})}
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(9030));},
            }
          }
          x => {return Err(MechError::GenericError(9031));},
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
