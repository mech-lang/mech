#[macro_use]
use crate::stdlib::*;
use std::marker::PhantomData;
use std::fmt::Debug;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

// Set -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_set_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        Bool, "bool";
        U8, "u8";
        U16, "u16";
        U32, "u32";
        U64, "u64";
        U128, "u128";
        I8, "i8";
        I16, "i16";
        I32, "i32";
        I64, "i64";
        U128, "u128";
        F32, "f32"; 
        F64, "f64" ;
        String, "string";
        ComplexNumber, "complex";
        R64, "rational";
      )
    }
  }
}

#[macro_export]
macro_rules! impl_set_all_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, IxVec> {
      pub source: Ref<T>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1, C1, S1, IxVec> MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.sink, self.ixes, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}

#[macro_export]
macro_rules! impl_set_all_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }

    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.sink, self.ixes, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}

// x[1] = 1 ------------------------------------------------------------------

#[macro_export]
macro_rules! set_1d_scalar {
  ($source:expr, $ix:expr, $sink:expr) => {
      ($sink)[$ix - 1] = ($source).clone();
    };}

#[macro_export]
macro_rules! impl_set_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA> {
      pub source: Ref<T>,
      pub ixes: Ref<$ix>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R, C, S> MechFunctionImpl for $struct_name<T, naMatrix<T, R, C, S>>
    where
      Ref<naMatrix<T, R, C, S>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static,
      R: Dim,
      C: Dim,
      S: StorageMut<T, R, C> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let mut sink_ptr = &mut *self.sink.as_mut_ptr();
          let ix_val = (*self.ixes.as_ptr()).clone();
          let source_val = (*self.source.as_ptr()).clone();
          $op!(source_val, ix_val, sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R, C, S> MechFunctionCompiler for $struct_name<T, naMatrix<T, R, C, S>> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      naMatrix<T, R, C, S>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.sink, self.ixes, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}

impl_set_fxn_s!(Set1DS, set_1d_scalar, usize);

#[derive(Debug)]
pub struct Set1DSB<T, MatA> {
  pub source: Ref<T>,
  pub ixes: Ref<bool>,
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}
impl<T, R, C, S> MechFunctionImpl for Set1DSB<T, naMatrix<T, R, C, S>>
where
  Ref<naMatrix<T, R, C, S>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static,
  R: Dim,
  C: Dim,
  S: StorageMut<T, R, C> + Clone + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = &mut *self.sink.as_mut_ptr();
      let ix_val = (*self.ixes.as_ptr()).clone();
      let source_val = (*self.source.as_ptr()).clone();
      if ix_val {
        for iy in 0..sink_ptr.len() {
          sink_ptr[iy] = source_val.clone();
        }
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S> MechFunctionCompiler for Set1DSB<T, naMatrix<T, R, C, S>> 
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("{}<{}>", stringify!(Set1DSB), T::as_value_kind());
    compile_binop!(name, self.sink, self.ixes, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
  }
}

#[macro_export]
macro_rules! impl_set_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident,$value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new(Set1DS { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix1", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DS, scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalar {}
impl NativeFunctionCompiler for MatrixSetScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] = 1 ----------------------------------------------------------------

macro_rules! set_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] = ($source).clone();
      }
    }
  };}

macro_rules! set_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] = ($source).clone();
        }
      }
    }
  };}  

macro_rules! set_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] = ($source)[i].clone();
      }
    }
  };}  

macro_rules! set_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe {
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] = ($source)[i].clone();
        }
      }
    }};}

impl_set_all_fxn_s!(Set1DRS,set_1d_range,usize);
impl_set_all_fxn_s!(Set1DRB,set_1d_range_b,bool);
impl_set_all_fxn_v!(Set1DRV,set_1d_range_vec,usize);
impl_set_all_fxn_v!(Set1DRVB,set_1d_range_vec_b,bool);

fn impl_set_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DR, range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRange {}
impl NativeFunctionCompiler for MatrixSetRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { impl_set_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,Value::MutableReference(source)) => { impl_set_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),source) => { impl_set_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] = 1 ------------------------------------------------------------------
use nalgebra::IsContiguous;
#[derive(Debug)]
pub struct Set1DA<T, Sink> {
  pub source: Ref<T>,
  pub sink: Ref<Sink>,
  pub _marker: PhantomData<T>,
}

impl<T, R, C, S> MechFunctionImpl for Set1DA<T, naMatrix<T, R, C, S>>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  R: Dim,
  C: Dim,
  S: StorageMut<T, R, C> + Debug + IsContiguous,
  Ref<naMatrix<T, R, C, S>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let sink = &mut *self.sink.as_mut_ptr();
      let source_val = (*self.source.as_ptr()).clone();
      let slice = sink.as_mut_slice();
      for elem in slice.iter_mut() {
        *elem = source_val.clone();
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S> MechFunctionCompiler for Set1DA<T, naMatrix<T, R, C, S>> 
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R, C, S>: CompileConst + ConstElem + AsValueKind,
{ 
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("{}<{},{}>", stringify!(Set1DA), T::as_value_kind(), naMatrix::as_value_kind());
    compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
  }
}

#[macro_export]
macro_rules! impl_set_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DA, all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAll {}
impl NativeFunctionCompiler for MatrixSetAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1] = 1 ----------------------------------------------------------------

#[macro_export]
macro_rules! impl_set_scalar_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<usize>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let mut sink_ptr = (&mut *(self.sink.as_mut_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem,
      $matrix_shape<T>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
      }
    }};}

macro_rules! set_2d_scalar_scalar {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      ($sink).column_mut($ix2 - 1)[$ix1 - 1] = ($source).clone();
    };}

impl_set_scalar_scalar_fxn!(Set2DSSMD,DMatrix,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM4,Matrix4,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM3,Matrix3,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM2,Matrix2,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM1,Matrix1,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM2x3,Matrix2x3,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM3x2,Matrix3x2,set_2d_scalar_scalar);

#[macro_export]
macro_rules! impl_set_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(              
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSS, scalar_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarScalar {}
impl NativeFunctionCompiler for MatrixSetScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1] = 1 -----------------------------------------------------------------

macro_rules! set_2d_all_scalar {
  ($source:expr, $ix:expr, $sink:expr) => {
      for i in 0..$sink.nrows() {
        ($sink).column_mut($ix - 1)[i] = ($source).clone();
      }
    };}

macro_rules! set_2d_all_vector {
  ($source:expr, $ix:expr, $sink:expr) => {
      for i in 0..$sink.nrows() {
        ($sink).column_mut($ix - 1)[i] = ($source)[i].clone();
      }
    };}

#[macro_export]
macro_rules! impl_set_scalar_fxn_v {
  ($struct_name:ident, $op:ident) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB> {
      pub source: Ref<MatB>,
      pub ixes: Ref<usize>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }

    impl<T, R1, C1, S1, R2, C2, S2>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr());
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")] 
    impl<T, R1, C1, S1, R2, C2, S2> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}    

impl_set_fxn_s!(Set2DASS, set_2d_all_scalar, usize);
impl_set_scalar_fxn_v!(Set2DASV, set_2d_all_vector);

#[macro_export]
macro_rules! impl_set_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)),   [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)),   [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DAS, all_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAllScalar {}
impl NativeFunctionCompiler for MatrixSetAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,:] = 1 -----------------------------------------------------------------

macro_rules! set_2d_scalar_all {
  ($sink:expr, $ix:expr, $source:expr) => {
      for i in 0..($sink).ncols() {
        ($sink).row_mut($ix - 1)[i] = ($source).clone();
      }
    };}

#[macro_export]
macro_rules! impl_set_scalar_all_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ix: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let ix_ptr = (*(self.ix.as_ptr())).clone();
          let mut sink_ptr = (&mut *(self.sink.as_mut_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(sink_ptr,ix_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      $matrix_shape<T>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.sink, self.source, self.ix, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}

impl_set_scalar_all_fxn!(Set2DSAMD,DMatrix, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM4,Matrix4, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM3,Matrix3, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM2,Matrix2, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM1,Matrix1, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM2x3,Matrix2x3, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM3x2,Matrix3x2, set_2d_scalar_all);

#[macro_export]
macro_rules! impl_set_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSA, scalar_all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarAll {}
impl NativeFunctionCompiler for MatrixSetScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1] = 1 ------------------------------------------------------------------

macro_rules! set_2d_vector_scalar {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      for rix in &$ix1 {
        ($sink).row_mut(rix - 1)[$ix2 - 1] = ($source).clone();
      }
    };}

macro_rules! set_2d_vector_scalar_b {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for rix in 0..($ix1).len() {
        if $ix1[rix] == true {
          ($sink).row_mut(rix)[$ix2 - 1] = ($source).clone();
        }
      }
    }
  };}  

#[macro_export]
macro_rules! impl_set_range_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<DVector<$ix_type>>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let mut sink_ptr = (&mut *(self.sink.as_mut_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem,
      $matrix_shape<T>: CompileConst + ConstElem,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
      }
    }};}

impl_set_range_scalar_fxn!(Set2DRSMD,DMatrix, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM4,Matrix4, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM3,Matrix3, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM2,Matrix2, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM1,Matrix1, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM2x3,Matrix2x3, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM3x2,Matrix3x2, set_2d_vector_scalar, usize);

impl_set_range_scalar_fxn!(Set2DRSMDB,DMatrix, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM4B,Matrix4, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM3B,Matrix3, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM2B,Matrix2, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM1B,Matrix1, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM2x3B,Matrix2x3, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM3x2B,Matrix3x2, set_2d_vector_scalar_b, bool);

#[macro_export]
macro_rules! impl_set_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRS, range_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeScalar {}
impl NativeFunctionCompiler for MatrixSetRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1..3] = 1 ------------------------------------------------------------------

#[derive(Debug)]
pub struct Set2DSR<T, MatA, IxVec> {
  pub source: Ref<T>,
  pub ixes: (Ref<usize>, Ref<IxVec>),
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, R, C, S, IxVec> MechFunctionImpl for Set2DSR<T, na::Matrix<T, R, C, S>, IxVec>
where
  Ref<na::Matrix<T, R, C, S>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static,
  R: nalgebra::Dim,
  C: nalgebra::Dim,
  S: nalgebra::StorageMut<T, R, C> + Clone + Debug,
  IxVec: AsRef<[usize]> + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink = &mut *self.sink.as_mut_ptr();
      let source = &*self.source.as_ptr();
      let ix1 = *self.ixes.0.as_ptr();
      let ix2 = (*self.ixes.1.as_ptr()).as_ref();

      for &rix in ix2 {
        sink.column_mut(rix - 1)[ix1 - 1] = source.clone();
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S, IxVec> MechFunctionCompiler for Set2DSR<T, na::Matrix<T, R, C, S>, IxVec> 
where
  T: CompileConst + ConstElem,
  IxVec: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
}

#[derive(Debug)]
pub struct Set2DSRB<T, MatA, IxVec> {
  pub source: Ref<T>,
  pub ixes: (Ref<usize>, Ref<IxVec>),
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, R, C, S, IxVec> MechFunctionImpl for Set2DSRB<T, na::Matrix<T, R, C, S>, IxVec>
where
  Ref<na::Matrix<T, R, C, S>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static,
  R: nalgebra::Dim,
  C: nalgebra::Dim,
  S: nalgebra::StorageMut<T, R, C> + Clone + Debug,
  IxVec: AsRef<[bool]> + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink = &mut *self.sink.as_mut_ptr();
      let source = &*self.source.as_ptr();
      let ix1 = *self.ixes.0.as_ptr();
      let ix2 = (*self.ixes.1.as_ptr()).as_ref();
      for i in 0..ix2.len() {
        if ix2[i] {
          sink.row_mut(i)[ix1 - 1] = source.clone();
        }
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S, IxVec> MechFunctionCompiler for Set2DSRB<T, na::Matrix<T, R, C, S>, IxVec> 
where
  T: CompileConst + ConstElem,
  IxVec: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
}

#[macro_export]
macro_rules! impl_set_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
        
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSR, scalar_range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarRange {}
impl NativeFunctionCompiler for MatrixSetScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1..3] = 1 ------------------------------------------------------------------

#[derive(Debug)]
pub struct Set2DRRS<T, MatA, IxVec1, IxVec2> {
  pub source: Ref<T>,
  pub ixes: (Ref<IxVec1>, Ref<IxVec2>),
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, R, C, S, IxVec1, IxVec2> MechFunctionImpl for Set2DRRS<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
where
  Ref<na::Matrix<T, R, C, S>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static,
  R: nalgebra::Dim,
  C: nalgebra::Dim,
  S: nalgebra::StorageMut<T, R, C> + Clone + Debug,
  IxVec1: AsRef<[usize]> + Debug,
  IxVec2: AsRef<[usize]> + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink = &mut *self.sink.as_mut_ptr();
      let source = &*self.source.as_ptr();
      let ix1 = (*self.ixes.0.as_ptr()).as_ref();
      let ix2 = (*self.ixes.1.as_ptr()).as_ref();

      for &cix in ix1 {
        for &rix in ix2 {
          sink.column_mut(cix - 1)[rix - 1] = source.clone();
        }
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S, IxVec1, IxVec2> MechFunctionCompiler for Set2DRRS<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2> 
where
  T: CompileConst + ConstElem,
  IxVec1: CompileConst + ConstElem,
  IxVec2: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
}

#[derive(Debug)]
pub struct Set2DRRSB<T, MatA, IxVec1, IxVec2> {
  pub source: Ref<T>,
  pub ixes: (Ref<IxVec1>, Ref<IxVec2>),
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}

impl<T, R, C, S, IxVec1, IxVec2> MechFunctionImpl for Set2DRRSB<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
where
  Ref<na::Matrix<T, R, C, S>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static,
  R: nalgebra::Dim,
  C: nalgebra::Dim,
  S: nalgebra::StorageMut<T, R, C> + Clone + Debug,
  IxVec1: AsRef<[bool]> + Debug,
  IxVec2: AsRef<[bool]> + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink = &mut *self.sink.as_mut_ptr();
      let source = &*self.source.as_ptr();
      let ix1 = (*self.ixes.0.as_ptr()).as_ref();
      let ix2 = (*self.ixes.1.as_ptr()).as_ref();

      for cix in 0..ix1.len() {
        for rix in 0..ix2.len() {
          if ix1[cix] && ix2[rix] {
            sink.row_mut(rix)[cix] = source.clone();
          }
        }
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T, R, C, S, IxVec1, IxVec2> MechFunctionCompiler for Set2DRRSB<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2> 
where
  T: CompileConst + ConstElem,
  IxVec1: CompileConst + ConstElem,
  IxVec2: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_ternop!(self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
}

#[macro_export]
macro_rules! impl_set_range_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste! {
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_set_range_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRR, range_range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeRange {}
impl NativeFunctionCompiler for MatrixSetRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1..3] = 1 ------------------------------------------------------------------

macro_rules! set_2d_all_vector {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in $ix.iter() {
        for rix in 0..($sink).nrows() {
          ($sink).column_mut(cix - 1)[rix] = ($source).clone();
        }
      }
    };}

macro_rules! set_2d_all_vector_b {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..$ix.len() {
        for rix in 0..($sink).nrows() {
          if $ix[cix] == true {
            ($sink).column_mut(cix)[rix] = ($source).clone();
          }
        }
      }
    };}    

impl_set_all_fxn_s!(Set2DAR,set_2d_all_vector,usize);
impl_set_all_fxn_s!(Set2DARB,set_2d_all_vector_b,bool);

#[macro_export]
macro_rules! impl_set_all_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new($fxn_name { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DAR, all_range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAllRange {}
impl NativeFunctionCompiler for MatrixSetAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] = 1 ------------------------------------------------------------------


macro_rules! set_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in $ix.iter() {
          ($sink).column_mut(cix)[rix - 1] = ($source).clone();
        }
      }
    };}

macro_rules! set_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix - 1] = ($source).clone();
        }
      }
    }
  };} 

macro_rules! set_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i, &rix) in $ix.iter().enumerate() {
      let mut sink_row = $sink.row_mut(rix - 1);
      let src_row = $source.row(i);
      for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
        *dst = src.clone();
      }
    }
  };}
  
macro_rules! set_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i, rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut row = ($sink).row_mut(i);
        let src_row = ($source).row(i);
        for (dst, src) in row.iter_mut().zip(src_row.iter()) {
          *dst = src.clone();
        }
      }
    }
  };
}

impl_set_all_fxn_v!(Set2DRAV,set_2d_vector_all_mat,usize);
impl_set_all_fxn_s!(Set2DRAS,set_2d_vector_all,usize);
impl_set_all_fxn_s!(Set2DRASB,set_2d_vector_all_b,bool);
impl_set_all_fxn_v!(Set2DRAVB,set_2d_vector_all_mat_b,bool);

fn impl_set_range_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeAll {}
impl NativeFunctionCompiler for MatrixSetRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { impl_set_range_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { impl_set_range_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { impl_set_range_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}