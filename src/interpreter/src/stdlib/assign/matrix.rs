#[macro_use]
use crate::stdlib::*;
use std::marker::PhantomData;
use std::fmt::Debug;
use nalgebra::{
  IsContiguous,
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

// Assign -----------------------------------------------------------------

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
        C64, "complex";
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
    impl<T, R1, C1, S1: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static + CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + Debug + AsRef<[$ix]> + AsNaKind,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
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
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), IxVec::as_na_kind());
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
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix]>,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
         PartialEq + PartialOrd,
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
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign));
      }
    }  
  };}

// x[1] = 1 ------------------------------------------------------------------

#[macro_export]
macro_rules! assign_1d_scalar {
  ($source:expr, $ix:expr, $sink:expr) => {
      ($sink)[$ix - 1] = ($source).clone();
    };}

#[macro_export]
macro_rules! assign_1d_scalar_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    if $ix {
      for ix in 0..$sink.len() {
        $sink[ix] = $source.clone();
      }
    }
  };}

#[macro_export]
macro_rules! assign_1d_scalar_vb {
  ($source:expr, $ix:expr, $sink:expr) => {
    if *$ix {
      let len = $sink.len().min($source.len());
      for ix in 0..len {
        $sink[ix] = $source[ix].clone();
      }
    }
  };}

#[macro_export]
macro_rules! impl_assign_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA> {
      pub source: Ref<T>,
      pub ixes: Ref<$ix>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R, C, S: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R, C, S>>
    where
      Ref<naMatrix<T, R, C, S>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static +
        CompileConst + ConstElem + AsValueKind,
      R: Dim, C: Dim, S: StorageMut<T, R, C> + Clone + Debug,
      naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<$ix> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R, C, S>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
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
      naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R, C, S>::as_na_kind());
        compile_binop!(name, self.sink, self.ixes, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }};}

impl_assign_fxn_s!(Assign1DS, assign_1d_scalar, usize);
impl_assign_fxn_s!(Assign1DB, assign_1d_scalar_b, bool);
impl_assign_scalar_fxn_v!(Assign1DVB, assign_1d_scalar_vb, bool);

fn impl_assign_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, u8,   "u8")
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms,  Assign1D, arg, String, "string"))

  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, u8,  "u8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_arms_b,  Assign1D, arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind })
}

pub struct MatrixAssignScalar {}
impl NativeFunctionCompiler for MatrixAssignScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("Unsupported argument: {:?}", &x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
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

fn impl_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
                impl_assign_fxn!(impl_set_range_arms, Set1DR, arg, u8,   "u8")
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_set_range_arms,  Set1DR, arg, String, "string"))

  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u8,  "u8"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u16,  "u16"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u32,  "u32"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u64,  "u64"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u128, "u128"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i8,   "i8"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i16,  "i16"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i32,  "i32"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i64,  "i64"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i128, "i128"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, F32,  "f32"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, F64,  "f64"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, R64,  "rational"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, C64,  "complex"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, bool, "bool"))
  .or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
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
    match impl_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,&ixes,source) {
          (Value::MutableReference(sink),_,Value::MutableReference(source)) => { impl_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,_,Value::MutableReference(source)) => { impl_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),_,source) => { impl_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] = 1 ------------------------------------------------------------------

#[derive(Debug)]
pub struct Set1DAS<T, Sink> {
  pub source: Ref<T>,
  pub sink: Ref<Sink>,
  pub _marker: PhantomData<T>,
}
impl<T, R, C, S> MechFunctionFactory for Set1DAS<T, naMatrix<T, R, C, S>>
where
  Ref<naMatrix<T, R, C, S>>: ToValue,
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
    CompileConst + ConstElem + AsValueKind,
  R: Dim,
  C: Dim,
  S: StorageMut<T, R, C> + Debug + IsContiguous + 'static,
  naMatrix<T, R, C, S>: CompileConst + ConstElem + Debug + AsNaKind,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, arg1) => {
        let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
        let sink: Ref<naMatrix<T, R, C, S>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { sink, source, _marker: PhantomData::default() }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!(Set1DA), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T, R, C, S> MechFunctionImpl for Set1DAS<T, naMatrix<T, R, C, S>>
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
impl<T, R, C, S> MechFunctionCompiler for Set1DAS<T, naMatrix<T, R, C, S>> 
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
{ 
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Set1DA<{}{}>", T::as_value_kind(), naMatrix::<T, R, C, S>::as_na_kind());
    compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
  }
}

fn impl_assign_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, u8,   "u8")
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_assign_all_arms, Set1DA, arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}

pub struct MatrixAssignAll {}
impl NativeFunctionCompiler for MatrixAssignAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1] = 1 ----------------------------------------------------------------

#[derive(Debug)]
pub struct Assign2DSSS<T, MatA> {
  pub source: Ref<T>,
  pub ixes: (Ref<usize>, Ref<usize>),
  pub sink: Ref<MatA>,
  pub _marker: PhantomData<T>,
}
impl<T, R1, C1, S1: 'static> MechFunctionFactory for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
  Ref<naMatrix<T, R1, C1, S1>>: ToValue,
  T: Scalar + Clone + Debug + Sync + Send + 'static +
  CompileConst + ConstElem + AsValueKind,
  R1: Dim,
  C1: Dim,
  S1: StorageMut<T, R1, C1> + Clone + Debug,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
        let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
        let ix1: Ref<usize> = unsafe { arg2.as_unchecked() }.clone();
        let ix2: Ref<usize> = unsafe { arg3.as_unchecked() }.clone();
        let sink: Ref<naMatrix<T, R1, C1, S1>> =
        unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {sink,source,ixes: (ix1, ix2),_marker: PhantomData}))
      }
      _ => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Assign2DSSS requires 3 arguments, got {:?}",args),id: line!(),kind: MechErrorKind::IncorrectNumberOfArguments,}),
    }
  }
}
impl<T, R1, C1, S1> MechFunctionImpl for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    Ref<naMatrix<T, R1, C1, S1>>: ToValue,
    T: Scalar + Clone + Debug + Sync + Send + 'static,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = &mut *self.sink.as_mut_ptr();
      let source_val = (*self.source.as_ptr()).clone();
      let r = (*self.ixes.0.as_ptr()).clone();
      let c = (*self.ixes.1.as_ptr()).clone();
      sink_ptr[(r - 1, c - 1)] = source_val;
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
impl<T, R1, C1, S1> MechFunctionCompiler for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
  T: CompileConst + ConstElem + AsValueKind,
  naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Assign2DSSS<{}{}>", T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind());
    compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
}

fn impl_assign_scalar_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, u8,   "u8")
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_assign_scalar_scalar_arms, Assign2DSS, arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}

pub struct MatrixAssignScalarScalar {}
impl NativeFunctionCompiler for MatrixAssignScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_scalar_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_scalar_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1] = 1 -----------------------------------------------------------------

macro_rules! assign_2d_all_scalar {
  ($source:expr, $ix:expr, $sink:expr) => {
      for i in 0..$sink.nrows() {
        ($sink).column_mut($ix - 1)[i] = ($source).clone();
      }
    };}

macro_rules! assign_2d_all_vector {
  ($source:expr, $ix:expr, $sink:expr) => {
      for i in 0..$sink.nrows() {
        ($sink).column_mut($ix - 1)[i] = ($source)[i].clone();
      }
    };}

#[macro_export]
macro_rules! impl_assign_scalar_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB> {
      pub source: Ref<MatB>,
      pub ixes: Ref<$ix>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<$ix> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
         PartialEq + PartialOrd,
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
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::Assign));
      }
    }  
  };}

impl_assign_fxn_s!(Assign2DASS, assign_2d_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DASV, assign_2d_all_vector, usize);

fn impl_assign_all_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_all_scalar_arms!(Assign2DAS, &arg, u8,   "u8")
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, u16,  "u16"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, u32,  "u32"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, u64,  "u64"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, u128, "u128"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, i8,   "i8"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, i16,  "i16"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, i32,  "i32"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, i64,  "i64"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, i128, "i128"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, F32,  "f32"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, F64,  "f64"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, R64,  "rational"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, C64,  "complex"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, bool, "bool"))
  .or_else(|_| impl_assign_all_scalar_arms!(Assign2DAS, &arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}

pub struct MatrixAssignAllScalar {}
impl NativeFunctionCompiler for MatrixAssignAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_all_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_all_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,:] = 1 -----------------------------------------------------------------

macro_rules! assign_2d_scalar_all_scalar {
  ($sink:expr, $ix:expr, $source:expr) => {
      print!("{:?}", $sink);
    };}

macro_rules! assign_2d_scalar_all_vector {
  ($sink:expr, $ix:expr, $source:expr) => {
      todo!();
      //for i in 0..($sink).ncols() {
      //  ($sink).row_mut($ix - 1)[i] = ($source)[i].clone();
      //}
    };}

impl_assign_fxn_s!(Assign2DSAS, assign_2d_scalar_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DSAV, assign_2d_scalar_all_vector, usize);

fn impl_assign_scalar_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_scalar_all_arms!(Assign2DAS, &arg, u8,   "u8")
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, u16,  "u16"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, u32,  "u32"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, u64,  "u64"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, u128, "u128"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, i8,   "i8"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, i16,  "i16"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, i32,  "i32"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, i64,  "i64"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, i128, "i128"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, F32,  "f32"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, F64,  "f64"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, R64,  "rational"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, C64,  "complex"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, bool, "bool"))
  .or_else(|_| impl_assign_scalar_all_arms!(Assign2DAS, &arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}

pub struct MatrixAssignScalarAll {}
impl NativeFunctionCompiler for MatrixAssignScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_scalar_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_scalar_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1] = 1 ------------------------------------------------------------------

macro_rules! assign_2d_range_scalar {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for i in 0..($ix1).len() {
        let rix = $ix1[i] - 1; 
        ($sink).row_mut(rix)[$ix2 - 1] = ($source).clone();
      }
    }
  };}

macro_rules! assign_2d_range_scalar_v {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for i in 0..($ix1).len() {
        let rix = $ix1[i] - 1; 
        ($sink).row_mut(rix)[$ix2 - 1] = ($source)[i].clone();
      }
    }
  };}

macro_rules! assign_2d_range_scalar_b {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    todo!();
    //unsafe { 
    //  for rix in 0..($ix1).len() {
    //    if $ix1[rix] == true {
    //      ($sink).row_mut(rix)[$ix2 - 1] = ($source).clone();
    //    }
    //  }
    //}
  };}  

  
#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_s {
  ($struct_name:ident, $op:tt, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, IxVec> {
      pub source: Ref<T>,
      pub ixes: (Ref<IxVec>, Ref<usize>),
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R, C, S: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
    where
      Ref<naMatrix<T, R, C, S>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static + CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + Debug + AsRef<[$ix]> + AsNaKind,
      R: Dim, C: Dim, S: StorageMut<T, R, C> + Clone + Debug,
      naMatrix<T, R, C, S>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
            let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
            let ix1: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let ix2: Ref<usize> = unsafe { arg3.as_unchecked() }.clone();
            let sink: Ref<na::Matrix<T, R, C, S>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes: (ix1, ix2), _marker: PhantomData }))
          }
          _ => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name),args),id: line!(),kind: MechErrorKind::IncorrectNumberOfArguments,}),
        }
      }
    }
    impl<T, R, C, S, IxVec> MechFunctionImpl for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
    where
      Ref<naMatrix<T, R, C, S>>: ToValue,
      T: Scalar + Clone + Debug + Sync + Send + 'static,
      IxVec: AsRef<[$ix]> + Debug,
      R: Dim, C: Dim, S: StorageMut<T, R, C> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink = &mut *self.sink.as_mut_ptr();
          let source = &*self.source.as_ptr();
          let ix1 = (*self.ixes.0.as_ptr()).as_ref();
          let ix2 = (*self.ixes.1.as_ptr());
          $op!(sink, ix1, ix2, source);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R, C, S, IxVec> MechFunctionCompiler for $struct_name<T, na::Matrix<T, R, C, S>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R, C, S>::as_na_kind(), IxVec::as_na_kind());
        compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
      }
    }
  };}

#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: (Ref<IxVec>, Ref<usize>),
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix]>,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ix1: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let ix2: Ref<usize> = unsafe { arg3.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes: (ix1, ix2), _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
         PartialEq + PartialOrd,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink = &mut *self.sink.as_mut_ptr();
          let source = &*self.source.as_ptr();
          let ix1 = (*self.ixes.0.as_ptr()).as_ref();
          let ix2 = (*self.ixes.1.as_ptr());
          $op!(sink, ix1, ix2, source);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec::as_na_kind());
        compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );  
      }
    }  
  };}

impl_assign_range_scalar_fxn_s!(Assign2DSSMD, assign_2d_range_scalar, usize);

impl_assign_range_scalar_fxn_s!(Assign2DRSS, assign_2d_range_scalar, usize);
impl_assign_range_scalar_fxn_s!(Assign2DRSB, assign_2d_range_scalar_b, bool);
impl_assign_range_scalar_fxn_v!(Assign2DRSV, assign_2d_range_scalar_v, usize);
//impl_assign_range_scalar_fxn_v!(Set2DRSBV, assign_2d_range_scalar_vb, bool);

fn impl_assign_range_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (sink, ixes.as_slice(), source);
               impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, u8,   "u8")
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, u16,  "u16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, u32,  "u32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, u64,  "u64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, u128, "u128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, i8,   "i8"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, i16,  "i16"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, i32,  "i32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, i64,  "i64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, i128, "i128"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, F32,  "f32"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, F64,  "f64"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, R64,  "rational"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, C64,  "complex"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, bool, "bool"))
  .or_else(|_| impl_assign_fxn!(impl_assign_range_scalar_arms, Assign2DRS, arg, String, "string"))

  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u8,  "u8"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u16,  "u16"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u32,  "u32"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u64,  "u64"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, u128, "u128"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i8,   "i8"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i16,  "i16"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i32,  "i32"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i64,  "i64"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, i128, "i128"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, F32,  "f32"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, F64,  "f64"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, R64,  "rational"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, C64,  "complex"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, bool, "bool"))
  //.or_else(|_| impl_set_range_arms_b!(Set1DR, &arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}

pub struct MatrixAssignRangeScalar {}
impl NativeFunctionCompiler for MatrixAssignRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_range_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_assign_range_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          _ => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}, {:?}, {:?}", sink, source, ixes), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
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
  T: CompileConst + ConstElem + AsValueKind,
  IxVec: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Set2DSR<{}>", T::as_value_kind());
    compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
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
  T: CompileConst + ConstElem + AsValueKind,
  IxVec: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Set2DSRB<{}>", T::as_value_kind());
    compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
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

pub struct MatrixAssignScalarRange {}
impl NativeFunctionCompiler for MatrixAssignScalarRange {
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
  T: CompileConst + ConstElem + AsValueKind,
  IxVec1: CompileConst + ConstElem,
  IxVec2: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Set2DRRS<{}>", T::as_value_kind()); 
    compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
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
  T: CompileConst + ConstElem + AsValueKind,
  IxVec1: CompileConst + ConstElem,
  IxVec2: CompileConst + ConstElem,
  na::Matrix<T, R, C, S>: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Set2DRRSB<{}>", T::as_value_kind());
    compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
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

macro_rules! assign_2d_all_range {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in $ix.iter() {
        for rix in 0..($sink).nrows() {
          ($sink).column_mut(cix - 1)[rix] = ($source).clone();
        }
      }
    };}

macro_rules! assign_2d_all_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..$ix.len() {
      for rix in 0..($sink).nrows() {
        if $ix[cix] == true {
          ($sink).column_mut(cix)[rix] = ($source).clone();
        }
      }
    }
  };} 

macro_rules! assign_2d_all_range_v {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let nsrc = $source.nrows();
      for (i, &cix) in $ix.iter().enumerate() {
        let col_index = cix - 1;
        let mut sink_col = $sink.column_mut(col_index);
        let src_col = $source.column(i % nsrc); // wrap around!
        for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
          *dst = src.clone();
        }
      }
    }
  };}

macro_rules! assign_2d_all_range_vb {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let mut src_i = 0;
      for (i, cix) in (&$ix).iter().enumerate() {
        if *cix == true {
          let mut sink_col = ($sink).column_mut(i);
          let src_col = ($source).column(src_i);
          for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
            *dst = src.clone();
          }
          src_i += 1;
        }
      }
    }
  };}

impl_set_all_fxn_v!(Set2DARV, assign_2d_all_range_v, usize);
impl_set_all_fxn_s!(Set2DARS, assign_2d_all_range, usize);
impl_set_all_fxn_s!(Set2DARB, assign_2d_all_range_b, bool);
impl_set_all_fxn_v!(Set2DARVB, assign_2d_all_range_vb, bool);

macro_rules! matrix_assign_all_range_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink, ixes.as_slice(), source);
                     impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, i128, "i128"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, F32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, F64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, C64, "complex"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, bool, "bool"))
        .or_else(|_| impl_assign_fxn!(impl_assign_all_range_arms, $fxn_name, arg, String, "string"))

        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, u8,  "u8"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, u16, "u16"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, u32, "u32"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, u64, "u64"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, u128,"u128"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, i8,  "i8"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, i16, "i16"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, i32, "i32"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, i64, "i64"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, i128,"i128"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, F32, "f32"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, F64, "f64"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, R64, "rational"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, C64, "complex"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, bool, "bool"))
        .or_else(|_| impl_set_all_range_arms_b!($fxn_name, &arg, String, "string"))
        .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
}

matrix_assign_all_range_fxn!(impl_assign_all_range_fxn, Set2DAR);

pub struct MatrixAssignAllRange {}
impl NativeFunctionCompiler for MatrixAssignAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_all_range_fxn(sink.clone(), source.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink, ixes, source) {
          (Value::MutableReference(sink), ixes, Value::MutableReference(source)) => { impl_assign_all_range_fxn(sink.borrow().clone(), source.borrow().clone(), ixes.clone()) },
          (sink, ixes, Value::MutableReference(source)) => { impl_assign_all_range_fxn(sink.clone(), source.borrow().clone(), ixes.clone()) },
          (Value::MutableReference(sink), ixes, source) => { impl_assign_all_range_fxn(sink.borrow().clone(), source.clone(), ixes.clone()) },
          x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] = 1 ------------------------------------------------------------------

macro_rules! assign_2d_range_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in $ix.iter() {
          ($sink).column_mut(cix)[rix - 1] = ($source).clone();
        }
      }
    };}

macro_rules! assign_2d_range_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix - 1] = ($source).clone();
        }
      }
    }
  };} 

  macro_rules! assign_2d_range_all_v {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let nsrc = $source.nrows();
      for (i, &rix) in $ix.iter().enumerate() {
        let row_index = rix - 1;
        let mut sink_row = $sink.row_mut(row_index);
        let src_row = $source.row(i % nsrc); // wrap around!
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst = src.clone();
        }
      }
    }
  };}

macro_rules! assign_2d_range_all_vb {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      for (i, rix) in ($ix).iter().enumerate() {
        if *rix {
          let mut sink_row = ($sink).row_mut(i);
          let src_row = ($source).row(i);
          for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
            *dst = src.clone();
          }
        }
      }
    }
  };
}

impl_set_all_fxn_v!(Set2DRAV,assign_2d_range_all_v,usize);
impl_set_all_fxn_s!(Set2DRAS,assign_2d_range_all,usize);
impl_set_all_fxn_s!(Set2DRAB,assign_2d_range_all_b,bool);
impl_set_all_fxn_v!(Set2DRAVB,assign_2d_range_all_vb,bool);

macro_rules! matrix_assign_range_all_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink, ixes.as_slice(), source);
                     impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, F32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, F64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, C64, "complex"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, bool, "bool"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, String, "string"))

        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, u8,  "u8"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, u16, "u16"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, u32, "u32"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, u64, "u64"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, u128,"u128"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, i8,  "i8"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, i16, "i16"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, i32, "i32"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, i64, "i64"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, i128,"i128"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, F32, "f32"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, F64, "f64"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, R64, "rational"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, C64, "complex"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, bool, "bool"))
        .or_else(|_| impl_set_range_all_arms_b!($fxn_name, &arg, String, "string"))
        .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
}

matrix_assign_range_all_fxn!(impl_assign_range_all_fxn, Set2DRA);

pub struct MatrixSetRangeAll {}
impl NativeFunctionCompiler for MatrixSetRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_assign_range_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { impl_assign_range_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { impl_assign_range_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { impl_assign_range_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}