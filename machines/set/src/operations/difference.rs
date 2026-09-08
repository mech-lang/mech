#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::canonical::{SetInput, SetOutput};
use crate::*;

#[derive(Debug)]
pub(crate) struct SetDifferenceFxn {
    lhs: SetInput,
    rhs: SetInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetDifferenceFxn {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalSortUnique
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: SetInput::canonical(lhs)?,
            rhs: SetInput::canonical(rhs)?,
            out: SetOutput::canonical(out)?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }
}

impl MechFunctionImpl for SetDifferenceFxn {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(
            vec![self.lhs.prospective_binary_footprint(&self.rhs, &self.out)?]
                .into_boxed_slice(),
        ))
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let footprint = self.lhs.prospective_binary_footprint(&self.rhs, &self.out)?;
        self.out.with_admitted_set(frame, footprint, |frame| {
            self.lhs.difference_elements(frame, &self.rhs)
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetDifferenceFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetDifferenceFxn"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetDifference {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetDifference {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetDifferenceFxn>(invocation, context)
    }
}
