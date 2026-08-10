#[macro_use]
use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;
use simba::scalar::ClosedNeg;

// Negate ---------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct NegateV<O> {
    arg: Ref<O>,
    out: Ref<O>,
    _marker: PhantomData<O>,
}
impl<O> MechFunctionFactory for NegateV<O>
where
    O: Debug
        + Clone
        + Sync
        + Send
        + Neg<Output = O>
        + ClosedNeg
        + RuntimeCheckedNeg
        + PartialEq
        + 'static
        + AsValueKind,
    #[cfg(feature = "compiler")]
    O: CompileConst + ConstElem,
    Ref<O>: ToValue,
    O: FunctionRuntimeType,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Unary(out, arg) => {
                let arg: Ref<O> = arg.try_function_ref(FunctionArgumentRole::Input(0))?;
                let out: Ref<O> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(Self {
                    arg,
                    out,
                    _marker: PhantomData,
                }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
impl<O> MechFunctionImpl for NegateV<O>
where
    O: Debug
        + Clone
        + Sync
        + Send
        + Neg<Output = O>
        + ClosedNeg
        + RuntimeCheckedNeg
        + PartialEq
        + 'static,
    Ref<O>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            let next = (*arg_ptr)
                .runtime_checked_neg()
                .ok_or_else(|| arithmetic_overflow::<O>("negation"))?;
            *out_ptr = next;
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<O> MechFunctionCompiler for NegateV<O>
where
    O: CompileConst + ConstElem + AsValueKind + RuntimeCheckedNeg,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NegateV<{}>", O::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[derive(Debug)]
pub(crate) struct NegateS<O> {
    arg: Ref<O>,
    out: Ref<O>,
    _marker: PhantomData<O>,
}
impl<O> MechFunctionFactory for NegateS<O>
where
    O: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + Neg<Output = O>
        + ClosedNeg
        + RuntimeCheckedNeg
        + PartialEq
        + 'static
        + AsValueKind,
    #[cfg(feature = "compiler")]
    O: CompileConst + ConstElem,
    Ref<O>: ToValue,
    O: FunctionRuntimeType,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Unary(out, arg) => {
                let arg: Ref<O> = arg.try_function_ref(FunctionArgumentRole::Input(0))?;
                let out: Ref<O> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(Self {
                    arg,
                    out,
                    _marker: PhantomData,
                }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
impl<O> MechFunctionImpl for NegateS<O>
where
    O: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + Neg<Output = O>
        + ClosedNeg
        + RuntimeCheckedNeg
        + PartialEq
        + 'static,
    Ref<O>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            let next = (*arg_ptr)
                .runtime_checked_neg()
                .ok_or_else(|| arithmetic_overflow::<O>("negation"))?;
            *out_ptr = next;
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<O> MechFunctionCompiler for NegateS<O>
where
    O: CompileConst + ConstElem + AsValueKind + RuntimeCheckedNeg,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NegateS<{}>", O::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[cfg(all(test, feature = "i8"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_negation_rejects_reactive_overflow_and_retains_output() {
        let arg = Ref::new(7_i8);
        let out = Ref::new(17_i8);
        let function = NegateS {
            arg: arg.clone(),
            out: out.clone(),
            _marker: PhantomData,
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), -7);
        *arg.borrow_mut() = i8::MIN;
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*out.borrow(), -7);
    }
}

#[cfg(feature = "source")]
fn impl_neg_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
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

#[cfg(feature = "source")]
impl_mech_urnop_fxn!(MathNegate, impl_neg_fxn, "math/neg");
