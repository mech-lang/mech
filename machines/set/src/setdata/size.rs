use crate::canonical::SetInput;
#[cfg(feature = "source")]
use crate::canonical::specialize_u64;
use crate::*;

#[derive(Debug)]
pub(crate) struct SetSizeFxn {
    input: SetInput,
    out: ManagedPort<u64>,
}

impl MechFunctionFactory for SetSizeFxn {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::U64,
        FunctionValueRepresentation::Set,
    );
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, input) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            input: SetInput::canonical(input)?,
            out: out.try_managed()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_SIZE_CONTRACT)
    }
}

impl MechFunctionImpl for SetSizeFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let next = u64::try_from(self.input.elements(frame)?.len()).map_err(|_| {
            function_shape_contract_violation("set/size", "set cardinality exceeds u64")
        })?;
        frame.with_port_init_writer(&self.out, |output| output.write_next(next))?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_SIZE_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetSizeFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_value_cell_register(self.out.cell(), ctx)?;
        let input = self.input.compile_register(ctx)?;
        ctx.emit_unop(hash_str("SetSizeFxn"), destination, input);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetSize {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetSize {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_u64::<SetSizeFxn>(invocation, context)
    }
}
