#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::canonical::{SetInput, SetOutput};
use crate::*;

#[derive(Debug)]
pub(crate) struct SetUnionFxn {
    lhs: SetInput,
    rhs: SetInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetUnionFxn {
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

impl MechFunctionImpl for SetUnionFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }

    fn solve_result(&self) -> MResult<()> {
        self.out.canonical_value().replace_set(
            self.lhs
                .canonical_value()
                .set_union_elements(self.rhs.canonical_value())?,
        )
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetUnionFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetUnionFxn"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetUnion {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetUnion {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetUnionFxn>(invocation, context)
    }
}
