use crate::*;
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
    #[cfg(feature = "semantic-compiler")]
    O: CompileConst + ConstElem,
    Ref<O>: ToValue,
    O: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<O> = arg.try_ref()?;
        let out: Ref<O> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData,
        }))
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
    O: FunctionStateBacking,
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
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::unary_full_write_contract(O::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
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
    #[cfg(feature = "semantic-compiler")]
    O: CompileConst + ConstElem,
    Ref<O>: ToValue,
    O: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<O> = arg.try_ref()?;
        let out: Ref<O> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData,
        }))
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
    O: FunctionStateBacking,
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
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::unary_full_write_contract(O::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
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
    fn unary_scalar_factory_uses_exact_invocation_ports() {
        let input = Ref::new(7_i8);
        let output = Ref::new(0_i8);
        let function = NegateS::<i8>::new_invocation(
            FunctionArgs::Unary(output.to_value(), input.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(*output.borrow(), -7);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *output.borrow_mut() = 99;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*output.borrow(), -7);
    }

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

impl_canonical_registered_math_unop_specializer!(MathNegate, "NegateS");
