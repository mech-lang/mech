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
        + FunctionRuntimeType,
    #[cfg(feature = "semantic-compiler")]
    O: CompileConst + ConstElem,
    O: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::unary_full_write_contract(O::REPRESENTATION))
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
    O: CompileConst + ConstElem + FunctionRuntimeType + RuntimeCheckedNeg,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NegateV<{}>", <O as FunctionRuntimeType>::REPRESENTATION);
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
        + FunctionRuntimeType,
    #[cfg(feature = "semantic-compiler")]
    O: CompileConst + ConstElem,
    O: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::unary_full_write_contract(O::REPRESENTATION))
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
    O: CompileConst + ConstElem + FunctionRuntimeType + RuntimeCheckedNeg,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NegateS<{}>", <O as FunctionRuntimeType>::REPRESENTATION);
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

impl_canonical_registered_math_unop_specializer!(MathNegate, "NegateS");

#[cfg(all(test, feature = "i8"))]
mod canonical_port_tests {
    use super::*;

    fn i8_value(cell: &ValueCell) -> i8 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::I8(value) = snapshot.data() else {
            panic!("expected i8 negate output")
        };
        *value
    }

    #[test]
    fn negation_uses_exact_ports_and_rejects_overflow_atomically() {
        let input = ValueCell::from_exact(7_i8).unwrap();
        let output = ValueCell::from_exact(0_i8).unwrap();
        let function =
            NegateS::<i8>::new_invocation(FunctionInvocation::unary(output.clone(), input.clone()))
                .unwrap();
        function.solve_result().unwrap();
        assert_eq!(i8_value(&output), -7);

        input
            .replace(&ValueCell::from_exact(i8::MIN).unwrap().snapshot().unwrap())
            .unwrap();
        assert_eq!(
            function.solve_result().unwrap_err().kind_name(),
            "MathArithmeticOverflow"
        );
        assert_eq!(i8_value(&output), -7);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            output.replace(&ValueCell::from_exact(99_i8)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(i8_value(&output), -7);
    }
}
