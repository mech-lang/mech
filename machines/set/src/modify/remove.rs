use crate::canonical::{ArbitraryInput, SetInput, SetOutput};
#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::*;
use std::sync::LazyLock;

static PURE_SET_REMOVE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
});

#[derive(Debug)]
pub(crate) struct SetRemoveFxn {
    arg1: SetInput,
    arg2: ArbitraryInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetRemoveFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::AnyValue,
    );
    const OUTPUT_SCHEMA_RULE: FunctionOutputSchemaRule =
        FunctionOutputSchemaRule::DynamicSetLikeInput(0);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, set, element) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            arg1: SetInput::canonical(set)?,
            arg2: ArbitraryInput::canonical(element),
            out: SetOutput::canonical(out)?,
        }))
    }
}

impl MechFunctionImpl for SetRemoveFxn {
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
                .set_elements_after_remove(self.arg2.canonical_value())?,
        )
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_REMOVE_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetRemoveFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let set = self.arg1.compile_register(ctx)?;
        let element = self.arg2.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetRemoveFxn"), destination, set, element);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetRemove {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetRemove {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetRemoveFxn>(invocation)
    }
}
