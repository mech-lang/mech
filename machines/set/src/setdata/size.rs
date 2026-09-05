use crate::canonical::SetInput;
#[cfg(feature = "source")]
use crate::canonical::specialize_u64;
use crate::*;

#[derive(Debug)]
pub(crate) struct SetSizeFxn {
    input: SetInput,
    out: Ref<u64>,
}

impl MechFunctionFactory for SetSizeFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::U64,
        FunctionValueRepresentation::Set,
    );
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, input) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            input: SetInput::canonical(input)?,
            out: out.try_ref()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_SIZE_CONTRACT)
    }
}

impl MechFunctionImpl for SetSizeFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
    fn solve_result(&self) -> MResult<()> {
        *self.out.borrow_mut() = self.input.canonical_value().set_elements()?.len() as u64;
        Ok(())
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
        let destination = compile_register_brrw!(self.out, ctx);
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
