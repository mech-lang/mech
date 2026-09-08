use crate::*;

#[derive(Debug)]
pub struct StrictNotEqValue {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: ManagedPort<bool>,
}

impl MechFunctionImpl for StrictNotEqValue {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let next = !frame.function_value_inputs_equal(&self.lhs, &self.rhs)?;
        frame.with_output_port_view(&self.out, |out| {
            out.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}

impl MechFunctionFactory for StrictNotEqValue {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: lhs.value(),
            rhs: rhs.value(),
            out: out.try_managed_element::<bool>()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for StrictNotEqValue {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_value_cell_register(self.out.cell(), ctx)?;
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("compare/sneq"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct CompareStrictNotEqual;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CompareStrictNotEqual {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if specialization.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: specialization.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = specialization.input(0).expect("validated lhs");
        let rhs = specialization.input(1).expect("validated rhs");
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}
