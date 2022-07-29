use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref MATH_ADD__UPDATE: u64 = hash_str("math/add-update");
}

/*
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
binary_infix_par_vs!(ParAddVS,add);
binary_infix_par_vs!(ParSubVS,sub);
binary_infix_par_vs!(ParMulVS,mul);
binary_infix_par_vs!(ParDivVS,div);
binary_infix_par_vs!(ExpParVS,pow);

// Parallel Vector : Vector
binary_infix_par_vv!(ParAddVV,add);
binary_infix_par_vv!(ParSubVV,sub);
binary_infix_par_vv!(ParMulVV,mul);
binary_infix_par_vv!(ParDivVV,div);
binary_infix_par_vv!(ExpParVV,pow);

// Vector : Vector In Place
binary_infix_vvip!(AddVVIP,add);

// Parallel Vector : Vector In Place
binary_infix_par_vvip!(ParAddVVIP,add);

// Parallel Vector : Scalar In Place
binary_infix_par_vsip!(ParAddVSIP,add);

// Vector : Scalar In Place
binary_infix_vsip!(AddVSIP,add);

// Parallel Scalar : Vector
binary_infix_par_sv!(ParAddSV,add);
binary_infix_par_sv!(ParSubSV,sub);
binary_infix_par_sv!(ParMulSV,mul);
binary_infix_par_sv!(ParDivSV,div);
binary_infix_par_sv!(ExpParSV,pow);

math_compiler!(MathAdd,AddSS,AddSV,AddVS,AddVV);
math_compiler!(MathSub,SubSS,SubSV,SubVS,SubVV);
math_compiler!(MathMul,MulSS,MulSV,MulVS,MulVV);
math_compiler!(MathDiv,DivSS,DivSV,DivVS,DivVV);
math_compiler!(MathExp,ExpSS,ExpSV,ExpVS,ExpVV);

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
          ((_,Column::F32(arg),_), Column::F32(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          ((_,Column::F32(arg),_), Column::I8(out)) => { block.plan.push(NegateV{arg: arg.clone(), out: out.clone() });}
          x => {return Err(MechError{id: 6001, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      TableShape::Scalar => {
        let mut argument_columns = block.get_arg_columns(arguments)?;
        let (_,col,_) = &argument_columns[0];
        let out_column = block.get_out_column(out, 1, col.kind())?;
        match (&argument_columns[0], &out_column) {
          ((_,Column::I8(arg),_), Column::I8(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          ((_,Column::F32(arg),_), Column::F32(out)) => block.plan.push(NegateS{arg: arg.clone(), out: out.clone() }),
          x => {return Err(MechError{id: 6002, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 6003, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}


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
}
*/

// Set Scalar : Scalar
#[derive(Debug)]
pub struct MathAddSIxSIx<T,U> {
  pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: usize
}
impl<T,U> MechFunction for MathAddSIxSIx<T,U>
where T: Clone + Debug + Into<U> + std::ops::AddAssign,
      U: Clone + Debug + Into<T> + std::ops::AddAssign
{
  fn solve(&self) {
    println!("{:?}", self.out);
    println!("{:?}", self.arg);
    (self.out.borrow_mut())[self.oix] += T::into((self.arg.borrow())[self.ix].clone());
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

//#[macro_export]
//macro_rules! math_update_compiler {
//  ($func_name:ident, $op1:tt,$op2:tt,$op3:tt,$op4:tt) => (

//    pub struct $func_name {}

//    impl MechFunctionCompiler for $func_name {
    pub struct MathAddUpdate {}

    impl MechFunctionCompiler for MathAddUpdate {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let (_,src_id,src_indices) = &arguments[0];
        let (dest_id,dest_row,dest_col) = out;
        let arg_shapes = block.get_arg_dims(&arguments)?;
        let out_arg = (0,*dest_id,vec![(dest_row.clone(),dest_col.clone())]);
        let dest_shape = block.get_arg_dim(&out_arg)?;
        let src_table = block.get_table(src_id)?;
        let dest_table = block.get_table(dest_id)?;
        let mut arguments = arguments.clone();
        // The destination is pushed into the arguments here in order to use the
        // get_argument_column() machinery later.
        arguments.push(out_arg);
        // Now decide on the correct tfm based on the shape
        match (&arg_shapes[0],&dest_shape) {
          (TableShape::Scalar,TableShape::Scalar) => {
            let arg_cols = block.get_arg_columns(&arguments)?;
            match (&arg_cols[0],&arg_cols[1]) {
              ((_,Column::U8(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push(MathAddSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::F32(out),ColumnIndex::Index(out_ix))) => {block.plan.push(MathAddSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              /*((_,Column::F32(arg),ColumnIndex::Index(ix)),(_,Column::F32(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::F32(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U64(arg),ColumnIndex::Index(ix)),(_,Column::U64(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::U64(src),ColumnIndex::Index(in_ix)),(_,Column::U64(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U128(arg),ColumnIndex::Index(ix)),(_,Column::U128(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::U128(src),ColumnIndex::Index(in_ix)),(_,Column::U128(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::Bool(src),ColumnIndex::Index(in_ix)),(_,Column::Bool(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::Bool(arg),ColumnIndex::Index(ix)),(_,Column::Bool(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::Ref(src),ColumnIndex::Index(in_ix)),(_,Column::Ref(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::String(src),ColumnIndex::Index(in_ix)),(_,Column::String(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}*/
              x => {return Err(MechError{id: 6115, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          /*
          (TableShape::Scalar, TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let mut out_column = block.get_out_column(out, *rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_), Column::U16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_), Column::U32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_), Column::U64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_), Column::U128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              /*((_,Column::I8(lhs),_), (_,Column::I8(rhs),_), Column::I8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_), Column::I16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_), Column::I32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_), Column::I64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_), Column::I128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              */
              x => {return Err(MechError{id: 6005, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::F32)?;
                if let Column::F32(out) = out_column {
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
              ((_,Column::Speed(lhs),_), (_,Column::Time(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Length)?;
                if let Column::Length(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              x => {return Err(MechError{id: 6006, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }                   
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError{id: 6007, kind: MechErrorKind::DimensionMismatch(((*lhs_rows,0),(*rhs_rows,0)))});
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let out_column = block.get_out_column(out, *lhs_rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
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
              x => {return Err(MechError{id: 6008, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op3::<U8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(lhs), (_,Column::F32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op3::<F32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                x => {return Err(MechError{id: 6009, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
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
                x => {return Err(MechError{id: 6010, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }            
          (TableShape::Row(lhs_cols), TableShape::Row(rhs_cols)) => {
            let lhs_rows = 1;
            let rhs_rows = 1;

            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{id: 6011, kind: MechErrorKind::DimensionMismatch(((lhs_rows,*lhs_cols),(rhs_rows,*rhs_cols)))});
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
                (((_,Column::U64(lhs),_), (_,Column::U64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
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
                x => {return Err(MechError{id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Matrix(lhs_rows,lhs_cols), TableShape::Matrix(rhs_rows,rhs_cols)) => {
           
            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{id: 6011, kind: MechErrorKind::DimensionMismatch(((*lhs_rows,*lhs_cols),(*rhs_rows,*rhs_cols)))});
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
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                x => {return Err(MechError{id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Pending(table_id),_) => { return Err(MechError{id: 6013, kind: MechErrorKind::PendingTable(*table_id)}); }
          (_,TableShape::Pending(table_id)) => {return Err(MechError{id: 6014, kind: MechErrorKind::PendingTable(*table_id)}); },
          */
          x => {return Err(MechError{id: 6115, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
        Ok(())
      }
    }
//  )
//}