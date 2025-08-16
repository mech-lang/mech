#[macro_use]
use crate::*;

// Negate ---------------------------------------------------------------------

#[derive(Debug)]
struct NegateV<O> {
  arg: Ref<O>,
  out: Ref<O>,
  _marker: PhantomData<O>,
}

impl<O> MechFunction for NegateV<O>
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

#[derive(Debug)]
struct NegateS<O> {
  arg: Ref<O>,
  out: Ref<O>,
  _marker: PhantomData<O>,
}

impl<O> MechFunction for NegateS<O>
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

fn impl_neg_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms!(
    Negate,
    (lhs_value),
    I8,   i8,   "i8";
    I16,  i16,  "i16";
    I32,  i32,  "i32";
    I64,  i64,  "i64";
    I128, i128, "i128";
    F32,  F32,  "f32";
    F64,  F64,  "f64";
    RationalNumber, RationalNumber, "rational";
    ComplexNumber, ComplexNumber, "complex";
  )
}

impl_mech_urnop_fxn!(MathNegate,impl_neg_fxn);