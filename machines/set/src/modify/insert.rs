#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::canonical::{ArbitraryInput, SetInput, SetOutput};
use crate::*;

#[derive(Debug)]
pub(crate) struct SetInsertFxn {
    arg1: SetInput,
    arg2: ArbitraryInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetInsertFxn {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalSortUnique
            }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, set, element) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            arg1: SetInput::canonical(set)?,
            arg2: ArbitraryInput::canonical(element),
            out: SetOutput::canonical(out)?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_UPDATE_CONTRACT)
    }
}

impl MechFunctionImpl for SetInsertFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }
    fn solve_result(&self) -> MResult<()> {
        self.out.canonical_value().replace_set(
            self.arg1
                .canonical_value()
                .set_elements_after_insert(self.arg2.canonical_value())?,
        )
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_UPDATE_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetInsertFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let set = self.arg1.compile_register(ctx)?;
        let element = self.arg2.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetInsertFxn"), destination, set, element);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetInsert {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetInsert {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetInsertFxn>(invocation, context)
    }
}
