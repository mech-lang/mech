use crate::*;
use mech_core::*;
use paste::paste;
use std::ops::Neg;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Not ------------------------------------------------------------------------

// NotS -----------------------------------------------------------------------

#[derive(Debug)]
struct NotS<T> {
  pub arg: Ref<T>,
  pub out: Ref<T>,
  pub _marker: PhantomData<T>,
}
impl<T> MechFunctionFactory for NotS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + 
  CompileConst + ConstElem + AsValueKind +
  Not<Output = T>,
  Ref<T>: ToValue,
{
  fn new(args: FunctionArgs) -> Result<Box<dyn MechFunction>, MechError> {
    match args {
      FunctionArgs::Unary(out, arg) => {
        let arg: Ref<T> = unsafe { arg.as_unchecked() }.clone();
        let out: Ref<T> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {arg, out, _marker: PhantomData::default() }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T> MechFunctionImpl for NotS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + Not<Output = T>,
  Ref<T>: ToValue,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe { *out_ptr = !*arg_ptr; }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for NotS<T> 
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NotS<{}>", T::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Not) );
  }
}
//register_fxn_descriptor!(NotS, bool, "bool");

// NotV -----------------------------------------------------------------------

#[derive(Debug)]
pub struct NotV<T, MatA> {
  pub arg: Ref<MatA>,
  pub out: Ref<MatA>,
  pub _marker: PhantomData<T>,
}
impl<T, MatA> MechFunctionFactory for NotV<T, MatA>
where
  T: Debug + Clone + Sync + Send + 'static + 
  CompileConst + ConstElem + AsValueKind +
  Not<Output = T>,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  MatA: Debug + CompileConst + ConstElem + AsValueKind + 'static,
  Ref<MatA>: ToValue
{
  fn new(args: FunctionArgs) -> Result<Box<dyn MechFunction>, MechError> {
    match args {
      FunctionArgs::Unary(out, arg) => {
        let arg: Ref<MatA> = unsafe { arg.as_unchecked() }.clone();
        let out: Ref<MatA> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {arg, out, _marker: PhantomData::default() }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T, MatA> MechFunctionImpl for NotV<T, MatA>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + 
  CompileConst + ConstElem + AsValueKind +
  Not<Output = T>,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  MatA: Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = self.out.as_mut_ptr();
      let source_ptr = self.arg.as_ptr();
      let sink_ref: &mut MatA = &mut *sink_ptr;
      let source_ref: &MatA = &*source_ptr;
      for (dst, src) in sink_ref.into_iter().zip(source_ref.into_iter()) {
        *dst = !src.clone();
      }
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T, MatA> MechFunctionCompiler for NotV<T, MatA> 
where
  T: CompileConst + ConstElem + AsValueKind,
  MatA: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NotV<{}{}>", T::as_value_kind(), MatA::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Not) );
  }
}

fn impl_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms!(
    Not,
    (arg_value),
    Bool, bool, "bool";
  )
}

impl_mech_urnop_fxn!(LogicNot,impl_not_fxn);