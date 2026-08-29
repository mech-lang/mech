use crate::canonical::{SetInput, SetOutput};
#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::*;

#[derive(Debug)]
pub(crate) struct SetIntersectionFxn {
    lhs: SetInput,
    rhs: SetInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetIntersectionFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );
    const OUTPUT_SCHEMA_RULE: FunctionOutputSchemaRule =
        FunctionOutputSchemaRule::DynamicSetLikeInput(0);
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: SetInput::canonical(lhs)?,
            rhs: SetInput::canonical(rhs)?,
            out: SetOutput::canonical(out)?,
        }))
    }
}

impl MechFunctionImpl for SetIntersectionFxn {
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
                .set_intersection_elements(self.rhs.canonical_value())?,
        )
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetIntersectionFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetIntersectionFxn"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetIntersection {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetIntersection {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetIntersectionFxn>(invocation)
    }
}
