use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref MATH_ADD: u64 = hash_str("math/add");
  pub static ref MATH_DIVIDE: u64 = hash_str("math/divide");
  pub static ref MATH_MULTIPLY: u64 = hash_str("math/multiply");
  pub static ref MATH_SUBTRACT: u64 = hash_str("math/subtract");
  pub static ref MATH_EXPONENT: u64 = hash_str("math/exponent");
  pub static ref MATH_NEGATE: u64 = hash_str("math/negate");
}

impl MechNumArithmetic<u8> for u8 {}
impl MechNumArithmetic<u16> for u16 {}
impl MechNumArithmetic<u32> for u32 {}
impl MechNumArithmetic<u64> for u64 {}
impl MechNumArithmetic<u128> for u128 {}
impl MechNumArithmetic<i8> for i8 {}
impl MechNumArithmetic<i16> for i16 {}
impl MechNumArithmetic<i32> for i32 {}
impl MechNumArithmetic<i64> for i64 {}
impl MechNumArithmetic<i128> for i128 {}
impl MechNumArithmetic<f32> for f32 {}
impl MechNumArithmetic<f64> for f64 {}

// Scalar : Scalar
binary_infix_ss!(AddSS,add);
binary_infix_ss!(SubSS,sub);
binary_infix_ss!(MulSS,mul);
binary_infix_ss!(DivSS,div);
//binary_infix_ss!(ExpSS,pow);

// Scalar : Vector
binary_infix_sv!(AddSV,add);
binary_infix_sv!(SubSV,sub);
binary_infix_sv!(MulSV,mul);
binary_infix_sv!(DivSV,div);
//binary_infix_sv!(ExpSV,pow);

// Vector : Scalar
binary_infix_vs!(AddVS,add);
binary_infix_vs!(SubVS,sub);
binary_infix_vs!(MulVS,mul);
binary_infix_vs!(DivVS,div);
//binary_infix_vs!(ExpVS,pow);

// Vector : Vector
binary_infix_vv!(AddVV,add);
binary_infix_vv!(SubVV,sub);
binary_infix_vv!(MulVV,mul);
binary_infix_vv!(DivVV,div);
//binary_infix_vv!(ExpVV,pow);

// Parallel Vector : Scalar
binary_infix_par_vs!(ParAddVS,add);
binary_infix_par_vs!(ParSubVS,sub);
binary_infix_par_vs!(ParMulVS,mul);
binary_infix_par_vs!(ParDivVS,div);
//binary_infix_par_vs!(ExpParVS,pow);

// Parallel Vector : Vector
binary_infix_par_vv!(ParAddVV,add);
binary_infix_par_vv!(ParSubVV,sub);
binary_infix_par_vv!(ParMulVV,mul);
binary_infix_par_vv!(ParDivVV,div);
//binary_infix_par_vv!(ExpParVV,pow);

// Parallel Vector : Vector In Place
binary_infix_par_vvip!(ParAddVVIP,add);

// Parallel Vector : Scalar In Place
binary_infix_par_vsip!(ParAddVSIP,add);

// Parallel Scalar : Vector
binary_infix_par_sv!(ParAddSV,add);
binary_infix_par_sv!(ParSubSV,sub);
binary_infix_par_sv!(ParMulSV,mul);
binary_infix_par_sv!(ParDivSV,div);
//binary_infix_par_sv!(ExpParSV,pow);

math_compiler!(MathAdd,AddSS,AddSV,AddVS,AddVV);
math_compiler!(MathSub,SubSS,SubSV,SubVS,SubVV);
math_compiler!(MathMul,MulSS,MulSV,MulVS,MulVV);
math_compiler!(MathDiv,DivSS,DivSV,DivVS,DivVV);
//math_compiler!(MathExp,ExpSS,ExpSV,ExpVS,ExpVV);

// Negate Vector
#[derive(Debug)]
pub struct NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    (self.out.borrow_mut())[0] = -((self.arg.borrow())[0]);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Negate Vector
#[derive(Debug)]
pub struct NegateV<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  pub arg: Arg<T>, pub out: Out<T>
}

impl<T> MechFunction for NegateV<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  fn solve(&mut self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = -(*arg)); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[macro_export]
macro_rules! binary_infix_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vs {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).for_each(|(out, lhs)| *out = (*lhs).$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vv {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&mut self) {
        self.out.borrow_mut().iter_mut().zip(self.lhs.borrow().iter()).zip(self.rhs.borrow().iter()).for_each(|((out, lhs), rhs)| *out = (*lhs).$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vv {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        self.out.borrow_mut().par_iter_mut().zip(self.lhs.borrow().par_iter()).zip(self.rhs.borrow().par_iter()).for_each(|((out, lhs), rhs)| *out = (*lhs).$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vvip {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        self.out.borrow_mut().par_iter_mut().zip(self.arg.borrow().par_iter()).for_each(|(out, arg)| *out = (*out).$op(*arg)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vs {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        let rhs = self.rhs.borrow()[0];
        self.out.borrow_mut().par_iter_mut().zip(&(*self.lhs.borrow())).for_each(|(out, lhs)| *out = (*lhs).$op(rhs));
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vsip {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&mut self) {
        let arg = self.arg.borrow()[0];
        self.out.borrow_mut().par_iter_mut().for_each(|out| *out = (*out).$op(arg));
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_ss {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub lix: usize, pub rhs: Arg<T>, pub rix: usize, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[self.lix];
        let rhs = self.rhs.borrow()[self.rix];
        self.out.borrow_mut().iter_mut().for_each(|out| *out = lhs.$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: Arg<T>, pub rhs: Arg<T>, pub out: Out<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&mut self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

pub struct MathNegate{}

impl MechFunctionCompiler for MathNegate {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_dims = block.get_arg_dims(&arguments)?;
    match &arg_dims[0] {
      TableShape::Column(rows) => {
        let mut argument_columns = block.get_arg_columns(arguments)?;
        let out_column = block.get_out_column(out, *rows, ValueKind::I8)?;
        match (&argument_columns[0], &out_column) {
          ((_,Column::I8(arg),_), Column::I8(out)) => { block.plan.push(NegateV::<i8>{arg: arg.clone(), out: out.clone() });}
          ((_,Column::F32(arg),_), Column::F32(out)) => { block.plan.push(NegateV::<f32>{arg: arg.clone(), out: out.clone() });}
          _ => {return Err(MechError::GenericError(1961));},
        }
      }
      TableShape::Scalar => {
        let mut argument_columns = block.get_arg_columns(arguments)?;
        let (_,col,_) = &argument_columns[0];
        let out_column = block.get_out_column(out, 1, col.kind())?;
        match (&argument_columns[0], &out_column) {
          ((_,Column::I8(arg),_), Column::I8(out)) => block.plan.push(NegateS::<i8>{arg: arg.clone(), out: out.clone() }),
          ((_,Column::F32(arg),_), Column::F32(out)) => block.plan.push(NegateS::<f32>{arg: arg.clone(), out: out.clone() }),
          x => {
            println!("{:?}", x);
            return Err(MechError::GenericError(1962));
          },
        }
      }
      _ => {return Err(MechError::GenericError(1963));},
    }
    Ok(())
  }
}

#[macro_export]
macro_rules! math_compiler {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt,$op4:tt) => (

    pub struct $func_name {}

    impl MechFunctionCompiler for $func_name {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let arg_shapes = block.get_arg_dims(&arguments)?;
        // Now decide on the correct tfm based on the shape
        match (&arg_shapes[0],&arg_shapes[1]) {
          (TableShape::Scalar, TableShape::Scalar) => {
            let mut argument_scalars = block.get_arg_columns(arguments)?;
            match (&argument_scalars[0], &argument_scalars[1]) {
              ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::U8(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U8)?;
                if let Column::U8(out) = out_column {
                  block.plan.push($op1::<u8>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) 
                }
              },
              ((_,Column::U16(lhs),ColumnIndex::Index(lix)), (_,Column::U16(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U16)?;
                if let Column::U16(out) = out_column {
                  block.plan.push($op1::<u16>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) 
                }
              },
              ((_,Column::U64(lhs),ColumnIndex::Index(lix)), (_,Column::U64(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U64)?;
                if let Column::U64(out) = out_column {
                  block.plan.push($op1::<u64>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) 
                }
              },
              ((_,Column::Time(lhs),ColumnIndex::Index(lix)), (_,Column::Time(rhs),ColumnIndex::Index(rix))) => {
                let mut out_column = block.get_out_column(out, 1, ValueKind::Time)?;
                if let Column::Time(out) = out_column { block.plan.push($op1::<f32>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) }
              }
              ((_,Column::F32(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::F32)?;
                if let Column::F32(out) = out_column { block.plan.push($op1::<f32>{lhs: lhs.clone(), lix: *lix, rhs: rhs.clone(), rix: *rix, out: out.clone()}) }
              },
              x => {
                println!("{:?}", x);
                return Err(MechError::GenericError(1236));
              },
            }
          }
          (TableShape::Scalar, TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let mut out_column = block.get_out_column(out, *rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_), Column::U16(out)) => { block.plan.push($op2::<u16>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_), Column::U32(out)) => { block.plan.push($op2::<u32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_), Column::U64(out)) => { block.plan.push($op2::<u64>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_), Column::U128(out)) => { block.plan.push($op2::<u128>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I8(lhs),_), (_,Column::I8(rhs),_), Column::I8(out)) => { block.plan.push($op2::<i8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_), Column::I16(out)) => { block.plan.push($op2::<i16>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_), Column::I32(out)) => { block.plan.push($op2::<i32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_), Column::I64(out)) => { block.plan.push($op2::<i64>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_), Column::I128(out)) => { block.plan.push($op2::<i128>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op2::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              _ => {return Err(MechError::GenericError(1237));},
            }
          }   
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];

            let out_column = block.get_out_column(out, *rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op3::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              _ => {return Err(MechError::GenericError(1238));},
            }
          }                      
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError::GenericError(6401));
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let out_column = block.get_out_column(out, *lhs_rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op4::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op4::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              x => {
                println!("{:?}",x);
                return Err(MechError::GenericError(1239));
              },
            }
          }
          (TableShape::Row(cols), TableShape::Scalar) => {
            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_column = block.get_arg_column(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,*cols);

            for (col_ix,(_,lhs_column,_)) in lhs_columns.iter().enumerate() {
              match (lhs_column,&rhs_column) {
                (Column::U8(lhs), (_,Column::U8(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op3::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(lhs), (_,Column::F32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op3::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                _ => {return Err(MechError::GenericError(6340));},
              }
            }
          }
          (TableShape::Scalar, TableShape::Row(cols)) => {
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;
            let lhs_column = block.get_arg_column(&arguments[0])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,*cols);

            for (col_ix,(_,rhs_column,_)) in rhs_columns.iter().enumerate() {
              match (rhs_column,&lhs_column) {
                (Column::U8(rhs), (_,Column::U8(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op2::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(rhs), (_,Column::F32(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op2::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                _ => {return Err(MechError::GenericError(6341));},
              }
            }
          }            
          (TableShape::Matrix(lhs_rows,lhs_cols), TableShape::Matrix(rhs_rows,rhs_cols)) => {
            
            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError::GenericError(6342));
            }

            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(*lhs_rows,*lhs_cols);

            for (col_ix,lhs_rhs) in lhs_columns.iter().zip(rhs_columns).enumerate() {
              match (lhs_rhs) {
                (((_,Column::U8(lhs),_), (_,Column::U8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U8);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op4::<u8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() })
                  }
                }
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32);
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4::<f32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() })
                  }
                }
                _ => {return Err(MechError::GenericError(6343));},
              }
            }
          }
          _ => {return Err(MechError::GenericError(6344));},
        }
        Ok(())
      }
    }
  )
}