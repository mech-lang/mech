#[macro_use]
use crate::*;
use simba::scalar::ClosedNeg;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Negate ---------------------------------------------------------------------

#[derive(Debug)]
struct NegateV<O> {
  arg: Ref<O>,
  out: Ref<O>,
  _marker: PhantomData<O>,
}
impl<O> MechFunctionFactory for NegateV<O>
where
  O: Debug + Clone + Sync + Send + Neg<Output = O> + ClosedNeg + PartialEq + 'static +
  CompileConst + ConstElem + AsValueKind,
  Ref<O>: ToValue,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, arg) => {
        let arg: Ref<O> = unsafe { arg.as_unchecked() }.clone();
        let out: Ref<O> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {arg, out, _marker: PhantomData }))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 1, found: args.len() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
impl<O> MechFunctionImpl for NegateV<O>
where
  O: Debug + Clone + Sync + Send + Neg<Output = O> + ClosedNeg + PartialEq + 'static,
  Ref<O>: ToValue,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe { *out_ptr = (*arg_ptr).clone().neg(); }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<O> MechFunctionCompiler for NegateV<O> 
where
  O: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NegateV<{}>", O::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Neg) );
  }
}

register_fxn_descriptor!(NegateV, 
  i8, "i8",
  i16, "i16",
  i32, "i32",
  i64, "i64",
  i128, "i128",
  f32, "f32",
  f64, "f64",
  R64, "r64",
  C64, "c64"
);

#[derive(Debug)]
struct NegateS<O> {
  arg: Ref<O>,
  out: Ref<O>,
  _marker: PhantomData<O>,
}
impl<O> MechFunctionFactory for NegateS<O>
where
  O: Copy + Debug + Clone + Sync + Send + Neg<Output = O> + ClosedNeg + PartialEq + 'static +
  CompileConst + ConstElem + AsValueKind,
  Ref<O>: ToValue,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, arg) => {
        let arg: Ref<O> = unsafe { arg.as_unchecked() }.clone();
        let out: Ref<O> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {arg, out, _marker: PhantomData }))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 1, found: args.len() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
impl<O> MechFunctionImpl for NegateS<O>
where
  O: Copy + Debug + Clone + Sync + Send + Neg<Output = O> + ClosedNeg + PartialEq + 'static,
  Ref<O>: ToValue,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe { *out_ptr = -*arg_ptr; }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<O> MechFunctionCompiler for NegateS<O> 
where
  O: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NegateS<{}>", O::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Neg) );
  }
}

register_fxn_descriptor!(NegateS,
  i8, "i8",
  i16, "i16",
  i32, "i32",
  i64, "i64",
  i128, "i128",
  f32, "f32",
  f64, "f64",
  R64, "r64",
  C64, "c64"
);

fn impl_neg_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms!(
    Negate,
    (lhs_value),
    I8,   i8,   "i8";
    I16,  i16,  "i16";
    I32,  i32,  "i32";
    I64,  i64,  "i64";
    I128, i128, "i128";
    F32,  f32,  "f32";
    F64,  f64,  "f64";
    R64, R64, "rational";
    C64, C64, "complex";
  )
}

impl_mech_urnop_fxn!(MathNegate,impl_neg_fxn,"math/neg");
