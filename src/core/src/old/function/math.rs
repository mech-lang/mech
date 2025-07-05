use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

#[cfg(feature = "parallel")]
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

impl MechNumArithmetic<U8> for U8 {}
impl MechNumArithmetic<U16> for U16 {}
impl MechNumArithmetic<U32> for U32 {}
impl MechNumArithmetic<U64> for U64 {}
impl MechNumArithmetic<U128> for U128 {}
impl MechNumArithmetic<I8> for I8 {}
impl MechNumArithmetic<I16> for I16 {}
impl MechNumArithmetic<I32> for I32 {}
impl MechNumArithmetic<I64> for I64 {}
impl MechNumArithmetic<I128> for I128 {}
impl MechNumArithmetic<F32> for F32 {}
impl MechNumArithmetic<F64> for F64 {}
impl MechNumArithmetic<f32> for f32 {}
impl MechNumArithmetic<f64> for f64 {}


// Scalar : Scalar
binary_infix_ss!(AddSS,add);
binary_infix_ss!(SubSS,sub);
binary_infix_ss!(MulSS,mul);
binary_infix_ss!(DivSS,div);
binary_infix_ss!(ExpSS,pow);

// Scalar : Vector
binary_infix_sv!(AddSV,add);
binary_infix_sv!(SubSV,sub);
binary_infix_sv!(MulSV,mul);
binary_infix_sv!(DivSV,div);
binary_infix_sv!(ExpSV,pow);

// Vector : Scalar
binary_infix_vs!(AddVS,add);
binary_infix_vs!(SubVS,sub);
binary_infix_vs!(MulVS,mul);
binary_infix_vs!(DivVS,div);
binary_infix_vs!(ExpVS,pow);

// Vector : Vector
binary_infix_vv!(AddVV,add);
binary_infix_vv!(SubVV,sub);
binary_infix_vv!(MulVV,mul);
binary_infix_vv!(DivVV,div);
binary_infix_vv!(ExpVV,pow);

// Parallel Vector : Scalar
#[cfg(feature = "parallel")]
binary_infix_par_vs!(ParAddVS,add);
#[cfg(feature = "parallel")]
binary_infix_par_vs!(ParSubVS,sub);
#[cfg(feature = "parallel")]
binary_infix_par_vs!(ParMulVS,mul);
#[cfg(feature = "parallel")]
binary_infix_par_vs!(ParDivVS,div);
#[cfg(feature = "parallel")]
binary_infix_par_vs!(ExpParVS,pow);

// Parallel Vector : Vector
#[cfg(feature = "parallel")]
binary_infix_par_vv!(ParAddVV,add);
#[cfg(feature = "parallel")]
binary_infix_par_vv!(ParSubVV,sub);
#[cfg(feature = "parallel")]
binary_infix_par_vv!(ParMulVV,mul);
#[cfg(feature = "parallel")]
binary_infix_par_vv!(ParDivVV,div);
#[cfg(feature = "parallel")]
binary_infix_par_vv!(ExpParVV,pow);

// Vector : Vector In Place
binary_infix_vvip!(AddVVIP,add);

// Parallel Vector : Vector In Place
#[cfg(feature = "parallel")]
binary_infix_par_vvip!(ParAddVVIP,add);

// Parallel Vector : Scalar In Place
#[cfg(feature = "parallel")]
binary_infix_par_vsip!(ParAddVSIP,add);

// Vector : Scalar In Place
binary_infix_vsip!(AddVSIP,add);

// Parallel Scalar : Vector
#[cfg(feature = "parallel")]
binary_infix_par_sv!(ParAddSV,add);
#[cfg(feature = "parallel")]
binary_infix_par_sv!(ParSubSV,sub);
#[cfg(feature = "parallel")]
binary_infix_par_sv!(ParMulSV,mul);
#[cfg(feature = "parallel")]
binary_infix_par_sv!(ParDivSV,div);
#[cfg(feature = "parallel")]
binary_infix_par_sv!(ExpParSV,pow);

// Dynamic : Dynamic
binary_infix_dd!(AddDD,add);
binary_infix_dd!(SubDD,sub);
binary_infix_dd!(MulDD,mul);
binary_infix_dd!(DivDD,div);
binary_infix_dd!(ExpDD,pow);

math_compiler!(MathAdd,AddSS,AddSV,AddVS,AddVV,AddDD);
math_compiler!(MathSub,SubSS,SubSV,SubVS,SubVV,SubDD);
math_compiler!(MathMul,MulSS,MulSV,MulVS,MulVV,MulDD);
math_compiler!(MathDiv,DivSS,DivSV,DivVS,DivVV,DivDD);
math_compiler!(MathExp,ExpSS,ExpSV,ExpVS,ExpVV,ExpDD);

// Negate Vector
#[derive(Debug)]
pub struct NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  pub arg: ColumnV<T>, pub out: ColumnV<T>
}

impl<T> MechFunction for NegateS<T> 
where T: std::ops::Neg<Output = T> + Copy + Debug
{
  fn solve(&self) {
    (self.out.borrow_mut())[0] = -((self.arg.borrow())[0]);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Negate Vector
#[derive(Debug)]
pub struct NegateV<T,U> 
{
  pub arg: ColumnV<T>, pub out: ColumnV<U>
}

impl<T,U> MechFunction for NegateV<T,U>  
where T: std::ops::Neg<Output = T> + Into<U> + Copy + Debug,
      U: std::ops::Neg<Output = U> + Into<T> + Copy + Debug,
{
  fn solve(&self) {
    self.out.borrow_mut().iter_mut().zip(self.arg.borrow().iter()).for_each(|(out, arg)| *out = -(T::into(*arg))); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


#[macro_export]
macro_rules! binary_infix_sv {
  ($func_name:ident, $op:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: ColumnV<T>, pub rhs: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&self) {
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
      pub lhs: ColumnV<T>, pub rhs: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&self) {
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
    pub struct $func_name<T,U,V> {
      pub lhs: (ColumnV<T>, usize, usize),
      pub rhs: (ColumnV<U>, usize, usize),
      pub out: ColumnV<V>
    }
    impl<T,U,V> MechFunction for $func_name<T,U,V> 
    where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send,
          U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send,
          V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        self.out.borrow_mut()
                .iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].iter().map(|x| T::into(*x)))
                .zip(rhs.borrow()[*rsix..=*reix].iter().map(|x| U::into(*x)))
                .for_each(|((out, lhs),rhs)| *out = lhs.$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vv {
  ($func_name:ident, $op:tt) => (
    #[cfg(feature = "parallel")]
    #[derive(Debug)]
    pub struct $func_name<T,U,V> {
      pub lhs: (ColumnV<T>, usize, usize),
      pub rhs: (ColumnV<U>, usize, usize),
      pub out: ColumnV<V>
    }
    impl<T,U,V> MechFunction for $func_name<T,U,V> 
    where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send,
          U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send,
          V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send,
    {
      fn solve(&self) {
        let (lhs,lsix,leix) = &self.lhs;
        let (rhs,rsix,reix) = &self.rhs;
        self.out.borrow_mut()
                .par_iter_mut()
                .zip(lhs.borrow()[*lsix..=*leix].par_iter().map(|x| T::into(*x)))
                .zip(rhs.borrow()[*rsix..=*reix].par_iter().map(|x| U::into(*x)))
                .for_each(|((out, lhs),rhs)| *out = lhs.$op(rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vvip {
  ($func_name:ident, $op:tt) => (
    #[cfg(feature = "parallel")]
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&self) {
        self.out.borrow_mut()
                .par_iter_mut()
                .zip(self.arg.borrow().par_iter())
                .for_each(|(out, arg)| *out = (*out).$op(*arg)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vvip {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&self) {
        self.out.borrow_mut()
                .iter_mut()
                .zip(self.arg.borrow().iter())
                .for_each(|(out, arg)| *out = (*out).$op(*arg)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_par_vs {
  ($func_name:ident, $op:tt) => (
    #[cfg(feature = "parallel")]
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: ColumnV<T>, pub rhs: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&self) {
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
    #[cfg(feature = "parallel")]
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&self) {
        let arg = self.arg.borrow()[0];
        self.out.borrow_mut().par_iter_mut().for_each(|out| *out = (*out).$op(arg));
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_vsip {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T> {
      pub arg: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug + Send + Sync
    {
      fn solve(&self) {
        let arg = self.arg.borrow()[0];
        self.out.borrow_mut().iter_mut().for_each(|out| *out = (*out).$op(arg));
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
      pub lhs: ColumnV<T>, pub lix: usize, pub rhs: ColumnV<T>, pub rix: usize, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&self) {
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
    #[cfg(feature = "parallel")]
    #[derive(Debug)]
    pub struct $func_name<T> {
      pub lhs: ColumnV<T>, pub rhs: ColumnV<T>, pub out: ColumnV<T>
    }
    impl<T> MechFunction for $func_name<T> 
    where T: MechNumArithmetic<T> + Copy + Debug
    {
      fn solve(&self) {
        let lhs = self.lhs.borrow()[0];
        self.out.borrow_mut().iter_mut().zip(self.rhs.borrow().iter()).for_each(|(out, rhs)| *out = lhs.$op(*rhs)); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! binary_infix_dd {
  ($func_name:ident, $op:tt) => (

    #[derive(Debug)]
    pub struct $func_name<T,U,V> {
      pub lhs: ColumnV<T>, 
      pub rhs: ColumnV<U>,
      pub out_col: ColumnV<V>, 
      pub out: OutTable,
    }
    impl<T,U,V> MechFunction for $func_name<T,U,V> 
    where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V>  + Sync + Send,
          U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send,
          V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send,
    {
      fn solve(&self) {
        let lhs = &self.lhs.borrow();
        let rhs = &self.rhs.borrow();
        let mut out_table_brrw = self.out.borrow_mut();
        out_table_brrw.resize(lhs.len(),1);
        self.out_col.borrow_mut()
                    .iter_mut()
                    .zip(lhs.iter().map(|x| T::into(*x)))
                    .zip(rhs.iter().map(|x| U::into(*x)))
                    .for_each(|((out, lhs),rhs)| *out = lhs.$op(rhs));   
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
          ((_,Column::I8(arg),_), Column::I8(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::I16(arg),_), Column::I16(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::I32(arg),_), Column::I32(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::I64(arg),_), Column::I64(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::I128(arg),_), Column::I128(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::F32(arg),_), Column::F32(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::F64(arg),_), Column::F64(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::F32(arg),_), Column::I8(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6001, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      TableShape::Scalar => {
        let mut argument_columns = block.get_arg_columns(arguments)?;
        let (_,col,_) = &argument_columns[0];
        let out_column = block.get_out_column(out, 1, col.kind())?;
        match (&argument_columns[0], &out_column) {
          ((_,Column::I8(arg),_), Column::I8(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::I16(arg),_), Column::I16(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::I32(arg),_), Column::I32(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::I64(arg),_), Column::I64(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::I128(arg),_), Column::I128(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::F64(arg),_), Column::F64(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::F32(arg),_), Column::F32(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6002, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6003, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}

/*
#[derive(Debug)]
pub struct ConcatVV<T,U> {
  pub lhs: (ColumnV<T>, usize, usize),
  pub rhs: (ColumnV<U>, usize, usize),
  pub out: ColumnV<MechString>
}
impl<T,U> MechFunction for ConcatVV<T,U> 
where T: Copy + Debug + Clone + Sync + Send,
      U: Copy + Debug + Clone + Sync + Send,
{
  fn solve(&self) {
    let (lhs,lsix,leix) = &self.lhs;
    let (rhs,rsix,reix) = &self.rhs;
    self.out.borrow_mut()
            .iter_mut()
            .zip(lhs.borrow()[*lsix..=*leix].iter())
            .zip(rhs.borrow()[*rsix..=*reix].iter())
            .for_each(|((out, lhs),rhs)| 
              *out = MechString::from_string(format!("{:?}{:?}", lhs, *rhs))); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}*/

#[macro_export]
macro_rules! math_compiler {
  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt,$op4:tt,$op5:tt) => (

    pub struct $func_name {}

    impl MechFunctionCompiler for $func_name {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let arg_shapes = block.get_arg_dims(&arguments)?;
        // Now decide on the correct tfm based on the shape
        match (&arg_shapes[0],&arg_shapes[1]) {
          (TableShape::Scalar, TableShape::Scalar) => {
            let mut argument_scalars = block.get_arg_columns(arguments)?;
            resize_one(block,out);
            match (&argument_scalars[0], &argument_scalars[1]) {
              ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::U8(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U8)?;
                if let Column::U8(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U8(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U8)?;
                if let Column::U8(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U16(lhs),ColumnIndex::Index(lix)), (_,Column::U16(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U16)?;
                if let Column::U16(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U32(lhs),ColumnIndex::Index(lix)), (_,Column::U32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U32)?;
                if let Column::U32(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U64(lhs),ColumnIndex::Index(lix)), (_,Column::U64(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U64)?;
                if let Column::U64(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U128(lhs),ColumnIndex::Index(lix)), (_,Column::U128(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::U128)?;
                if let Column::U128(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::I8(lhs),ColumnIndex::Index(lix)), (_,Column::I8(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::I8)?;
                if let Column::I8(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::I16(lhs),ColumnIndex::Index(lix)), (_,Column::I16(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::I16)?;
                if let Column::I16(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::I32(lhs),ColumnIndex::Index(lix)), (_,Column::I32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::I32)?;
                if let Column::I32(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::I64(lhs),ColumnIndex::Index(lix)), (_,Column::I64(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::I64)?;
                if let Column::I64(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::I128(lhs),ColumnIndex::Index(lix)), (_,Column::I128(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::I128)?;
                if let Column::I128(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::U64(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::F32)?;
                if let Column::F32(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::Speed(lhs),ColumnIndex::Index(lix)), (_,Column::Time(rhs),ColumnIndex::Index(rix))) => {
                let mut out_column = block.get_out_column(out, 1, ValueKind::Length)?;
                if let Column::Length(out) = out_column { block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) }
              }
              ((_,Column::Length(lhs),ColumnIndex::Index(lix)), (_,Column::Length(rhs),ColumnIndex::Index(rix))) => {
                let mut out_column = block.get_out_column(out, 1, ValueKind::Length)?;
                if let Column::Length(out) = out_column { block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) }
              }
              ((_,Column::Time(lhs),ColumnIndex::Index(lix)), (_,Column::Time(rhs),ColumnIndex::Index(rix))) => {
                let mut out_column = block.get_out_column(out, 1, ValueKind::Time)?;
                if let Column::Time(out) = out_column { block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) }
              }
              ((_,Column::F32(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::F32)?;
                if let Column::F32(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::F64(lhs),ColumnIndex::Index(lix)), (_,Column::F64(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::F64)?;
                if let Column::F64(out) = out_column {
                  block.plan.push($op4{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              ((_,Column::String(lhs),ColumnIndex::Index(lix)), (_,Column::F32(rhs),ColumnIndex::Index(rix))) => { 
                let mut out_column = block.get_out_column(out, 1, ValueKind::String)?;
                if let Column::String(out) = out_column {
                  //block.plan.push(ConcatVV{lhs: (lhs.clone(),*lix,*lix), rhs: (rhs.clone(),*rix,*rix), out: out.clone()}) 
                }
              },
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6004, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          (TableShape::Scalar, TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let mut out_column = block.get_out_column(out, *rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::F64(lhs),_), (_,Column::F64(rhs),_), Column::F64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_), Column::U16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_), Column::U32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_), Column::U64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_), Column::U128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I8(lhs),_), (_,Column::I8(rhs),_), Column::I8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_), Column::I16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_), Column::I32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_), Column::I64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_), Column::I128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6005, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }   
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            match (&argument_columns[0], &argument_columns[1]) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U8)?;
                if let Column::U8(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U16)?;
                if let Column::U16(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U32)?;
                if let Column::U32(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U64)?;
                if let Column::U64(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U128)?;
                if let Column::U128(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::I8(lhs),_), (_,Column::I8(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::I8)?;
                if let Column::I8(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::I16)?;
                if let Column::I16(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::I32)?;
                if let Column::I32(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::I64)?;
                if let Column::I64(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::I128)?;
                if let Column::I128(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::F32)?;
                if let Column::F32(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::F64(lhs),_), (_,Column::F64(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::F64)?;
                if let Column::F64(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Length(lhs),_), (_,Column::Length(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Length)?;
                if let Column::Length(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Speed(lhs),_), (_,Column::Speed(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Speed)?;
                if let Column::Speed(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Time(lhs),_), (_,Column::Time(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Time)?;
                if let Column::Time(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6006, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }                   
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6007, kind: MechErrorKind::DimensionMismatch(vec![(*lhs_rows,0),(*rhs_rows,0)])});
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let out_column = block.get_out_column(out, *lhs_rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::I8(lhs),_),(_,Column::I8(rhs),_),Column::I8(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::I16(lhs),_),(_,Column::I16(rhs),_),Column::I16(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::I32(lhs),_),(_,Column::I32(rhs),_),Column::I32(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::I64(lhs),_),(_,Column::I64(rhs),_),Column::I64(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::I128(lhs),_),(_,Column::I128(rhs),_),Column::I128(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U8(lhs),_),(_,Column::U8(rhs),_),Column::U8(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U16(lhs),_),(_,Column::U16(rhs),_),Column::U16(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U32(lhs),_),(_,Column::U32(rhs),_),Column::U32(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U64(lhs),_),(_,Column::U64(rhs),_),Column::U64(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U128(lhs),_),(_,Column::U128(rhs),_),Column::U128(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::F32(lhs),_),(_,Column::F32(rhs),_),Column::F32(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::F64(lhs),_),(_,Column::F64(rhs),_),Column::F64(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Length(lhs),_),(_,Column::Length(rhs),_),Column::Length(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Speed(lhs),_),(_,Column::Speed(rhs),_),Column::Speed(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Time(lhs),_),(_,Column::Time(rhs),_),Column::Time(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6008, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          (TableShape::Matrix(_,cols), TableShape::Scalar) |
          (TableShape::Row(cols), TableShape::Scalar) => {
            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_column = block.get_arg_column(&arguments[1])?;

            let rows: usize = match &arg_shapes[0] {
              TableShape::Matrix(rows,_) => *rows,
              _ => 1,
            };

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(rows,*cols);

            for (col_ix,(_,lhs_column,_)) in lhs_columns.iter().enumerate() {
              match (lhs_column,&rhs_column) {
                (Column::U8(lhs), (_,Column::U8(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op3::<U8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U16(lhs), (_,Column::U16(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U16(out) = out_col {
                    block.plan.push($op3::<U16>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U32(lhs), (_,Column::U32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U32(out) = out_col {
                    block.plan.push($op3::<U32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U64(lhs), (_,Column::U64(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
                    block.plan.push($op3::<U64>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U128(lhs), (_,Column::U128(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U128(out) = out_col {
                    block.plan.push($op3::<U128>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I8(lhs), (_,Column::I8(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I8(out) = out_col {
                    block.plan.push($op3::<I8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I16(lhs), (_,Column::I16(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I16(out) = out_col {
                    block.plan.push($op3::<I16>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I32(lhs), (_,Column::I32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I32(out) = out_col {
                    block.plan.push($op3::<I32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I64(lhs), (_,Column::I64(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I64(out) = out_col {
                    block.plan.push($op3::<I64>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I128(lhs), (_,Column::I128(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I128(out) = out_col {
                    block.plan.push($op3::<I128>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(lhs), (_,Column::F32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op3::<F32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F64(lhs), (_,Column::F64(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F64(out) = out_col {
                    block.plan.push($op3::<F64>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6009, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Scalar, TableShape::Matrix(_, cols)) |
          (TableShape::Scalar, TableShape::Row(cols)) => {
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;
            let lhs_column = block.get_arg_column(&arguments[0])?;

            let rows: usize = match &arg_shapes[1] {
              TableShape::Matrix(rows,_) => *rows,
              _ => 1,
            };

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(rows,*cols);

            for (col_ix,(_,rhs_column,_)) in rhs_columns.iter().enumerate() {
              match (rhs_column,&lhs_column) {
                (Column::U8(rhs), (_,Column::U8(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U16(rhs), (_,Column::U16(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U16(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U32(rhs), (_,Column::U32(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U32(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U64(rhs), (_,Column::U64(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::U128(rhs), (_,Column::U128(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U128(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I8(rhs), (_,Column::I8(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I8(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I16(rhs), (_,Column::I16(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I16(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I32(rhs), (_,Column::I32(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I32(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I64(rhs), (_,Column::I64(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I64(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::I128(rhs), (_,Column::I128(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::I128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I128(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(rhs), (_,Column::F32(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F64(rhs), (_,Column::F64(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F64(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6010, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }            
          (TableShape::Row(lhs_cols), TableShape::Row(rhs_cols)) => {
            let lhs_rows = 1;
            let rhs_rows = 1;

            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6011, kind: MechErrorKind::DimensionMismatch(vec![(lhs_rows,*lhs_cols),(rhs_rows,*rhs_cols)])});
            }

            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(lhs_rows,*lhs_cols);

            for (col_ix,lhs_rhs) in lhs_columns.iter().zip(rhs_columns).enumerate() {
              match (lhs_rhs) {
                 (((_,Column::U8(lhs),_), (_,Column::U8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U16(lhs),_), (_,Column::U16(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U16(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                } 
                (((_,Column::U32(lhs),_), (_,Column::U32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U64(lhs),_), (_,Column::U64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U128(lhs),_), (_,Column::U128(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U128(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I8(lhs),_), (_,Column::I8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I16(lhs),_), (_,Column::I16(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I16(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                } 
                (((_,Column::I32(lhs),_), (_,Column::I32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I64(lhs),_), (_,Column::I64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I128(lhs),_), (_,Column::I128(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I128(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F64(lhs),_), (_,Column::F64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Matrix(lhs_rows,lhs_cols), TableShape::Matrix(rhs_rows,rhs_cols)) => {
           
            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6011, kind: MechErrorKind::DimensionMismatch(vec![(*lhs_rows,*lhs_cols),(*rhs_rows,*rhs_cols)])});
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
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U16(lhs),_), (_,Column::U16(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U16(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U32(lhs),_), (_,Column::U32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U64(lhs),_), (_,Column::U64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U128(lhs),_), (_,Column::U128(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U128(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I8(lhs),_), (_,Column::I8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I16(lhs),_), (_,Column::I16(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I16)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I16(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I32(lhs),_), (_,Column::I32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I64(lhs),_), (_,Column::I64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::I128(lhs),_), (_,Column::I128(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::I128)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::I128(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F64(lhs),_), (_,Column::F64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Dynamic(lhs_rows,1),TableShape::Dynamic(rhs_rows,1)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6013, kind: MechErrorKind::DimensionMismatch(vec![(*lhs_rows,0),(*rhs_rows,0)])});
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let (out_table_id,_,_) = out;
            let out_table = block.get_table(out_table_id)?;
            let out_column = block.get_out_column(out, *lhs_rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1],out_column) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_),Column::U8(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_),Column::U16(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_),Column::U32(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_),Column::U64(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_),Column::U128(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::I8(lhs),_), (_,Column::I8(rhs),_),Column::I8(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_),Column::I16(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_),Column::I32(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_),Column::I64(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_),Column::I128(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_),Column::F32(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::F64(lhs),_), (_,Column::F64(rhs),_),Column::F64(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              ((_,Column::U8(lhs),_), (_,Column::F32(rhs),_),Column::U8(out)) => {block.plan.push($op5{lhs: lhs.clone(), rhs: rhs.clone(), out_col: out, out: out_table.clone()})}
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6014, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          (TableShape::Pending(table_id),_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6015, kind: MechErrorKind::PendingTable(*table_id)}); }
          (_,TableShape::Pending(table_id)) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6016, kind: MechErrorKind::PendingTable(*table_id)}); },
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6017, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
        Ok(())
      }
    }
  )
}
