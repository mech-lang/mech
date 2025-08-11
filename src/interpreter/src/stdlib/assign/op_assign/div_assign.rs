#[macro_use]
use crate::stdlib::*;
use std::fmt::Debug;
use std::ops::DivAssign;
use std::marker::PhantomData;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

// Div Assign -----------------------------------------------------------------

// We will mostly use the assign macros for this

#[macro_export]
macro_rules! impl_div_assign_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        U8, "U8";
        U16, "U16";
        U32, "U32";
        U64, "U64";
        U128, "U128";
        I8, "I8";
        I16, "I16";
        I32, "I32";
        I64, "I64";
        U128, "U128";
        F32, "F32"; 
        F64, "F64" ;
        ComplexNumber, "ComplexNumber";
        RationalNumber, "RationalNumber";
      )
    }
  }
}

macro_rules! impl_div_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

macro_rules! impl_div_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x /= 1 ----------------------------------------------------------------------

#[derive(Debug)]
struct DivAssignSS<T> {
  sink: Ref<T>,
  source: Ref<T>,
}
impl<T> MechFunction for DivAssignSS<T> 
where
  T: Debug + Clone + Sync + Send + 'static +
  Div<Output = T> + DivAssign +
  PartialEq + PartialOrd,
  Ref<T>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      *sink_ptr /= source_ptr.clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
pub struct DivAssignMatMat<T, MatA, MatB> {
  pub sink: Ref<MatA>,
  pub source: Ref<MatB>,
  _marker: PhantomData<T>,
}

impl<T, MatA, MatB> MechFunction for DivAssignMatMat<T, MatA, MatB>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + DivAssign,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  for<'a> &'a MatB: IntoIterator<Item = &'a T>,
  MatA: Debug,
  MatB: Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = self.sink.as_ptr();
      let source_ptr = self.source.as_ptr();
      let sink_ref: &mut MatA = &mut *sink_ptr;
      let source_ref: &MatB = &*source_ptr;
      for (dst, src) in (&mut *sink_ref).into_iter().zip((&*source_ref).into_iter()) {
        *dst /= src.clone();
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}

#[macro_export]
macro_rules! impl_div_assign_value_match_arms {
  ($arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new(DivAssignSS { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $feature, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new(DivAssignMatMat { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}

fn div_assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_div_assign_value_match_arms!(
    (sink, source),
    U8,  "U8";
    U16, "U16";
    U32, "U32";
    U64, "U64";
    U128, "U128";
    I8,  "I8";
    I16, "I16";
    I32, "I32";
    I64, "I64";
    U128, "U128";
    F32, "F32";
    F64, "F64";
    RationalNumber, "RationalNumber";
    ComplexNumber, "ComplexNumber";
  )
}

pub struct DivAssignValue {}
impl NativeFunctionCompiler for DivAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match div_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { div_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { div_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { div_assign_value_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] /= 1 ----------------------------------------------------------------

macro_rules! div_assign_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] /= *($source);
      }
    }
  };}

macro_rules! div_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] /= *($source);
        }
      }
    }
  };}  

macro_rules! div_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] /= ($source)[i];
      }
    }
  };}

macro_rules! div_assign_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] /= ($source)[i];
        }
      }
    }
  };}


impl_div_assign_range_fxn_s!(DivAssign1DRS,div_assign_1d_range,usize);
impl_div_assign_range_fxn_s!(DivAssign1DRB,div_assign_1d_range_b,bool);
impl_div_assign_range_fxn_v!(DivAssign1DRV,div_assign_1d_range_vec,usize);
impl_div_assign_range_fxn_v!(DivAssign1DRVB,div_assign_1d_range_vec_b,bool);

fn div_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_div_assign_match_arms!(DivAssign1DR, range, (sink, ixes.as_slice(), source))
}
 

pub struct DivAssignRange {}
impl NativeFunctionCompiler for DivAssignRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match div_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { div_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { div_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { div_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] /= 1 ------------------------------------------------------------------


macro_rules! div_assign_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
    for val in ($sink).iter_mut() {
      *val /= (*$source);
    }
  };}

macro_rules! div_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    let ncols = ($sink).ncols();
    for (i, val) in ($sink).iter_mut().enumerate() {
      let row = i / ncols;
      if $ix[row] {
        *val /= *($source);
      }
    }
  };}


macro_rules! div_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i, rix) in ($ix).iter().enumerate() {
      let mut sink_row = ($sink).row_mut(rix - 1);
      let src_row = ($source).row(i);
      for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
        *dst /= *src;
      }
    }
  };}

macro_rules! div_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i, rix) in ($ix).iter().enumerate() {
      if *rix {
        let mut sink_row = ($sink).row_mut(i);
        let src_row = ($source).row(i);
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst /= *src;
        }
      }
    }
  };}

impl_div_assign_range_fxn_s!(DivAssign2DRAS,div_assign_2d_vector_all,usize);
impl_div_assign_range_fxn_s!(DivAssign2DRASB,div_assign_2d_vector_all_b,bool);
impl_div_assign_range_fxn_v!(DivAssign2DRAV,div_assign_2d_vector_all_mat,usize);
impl_div_assign_range_fxn_v!(DivAssign2DRAVB,div_assign_2d_vector_all_mat_b,bool);

fn div_assign_vec_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_div_assign_match_arms!(DivAssign2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct DivAssignRangeAll {}
impl NativeFunctionCompiler for DivAssignRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match div_assign_vec_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { div_assign_vec_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { div_assign_vec_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { div_assign_vec_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}