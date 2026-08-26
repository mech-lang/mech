use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExecutionResourceRequest,
    ExternalInteraction, IdempotencyRequirement, InitialSolvePolicy, InputPortLayout,
    InputPortPolicy, LegacyValue, MResult, MechError, MechErrorKind, MechExecutionServices,
    MechFunctionImpl, NoMechExecutionServices, OperationContractDeclaration,
    ReactiveDependencyScope, ReactiveSolveStatus, ResourceIntent, ValueCell,
};
use std::sync::LazyLock;

static RESOURCE_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::ProviderDefined,
            idempotency: IdempotencyRequirement::Optional,
        }),
    });

#[cfg(feature = "semantic-compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalResourceWriteFunction {
    pub request: ExecutionResourceRequest,
    pub input: LegacyValue,
    pub output: ValueCell,
    pub initial_solve_policy: InitialSolvePolicy,
    pub semantic_contract: Option<&'static OperationContractDeclaration>,
}

impl ExternalResourceWriteFunction {
    fn validate(&self) -> MResult<()> {
        if *self.output.borrow() != LegacyValue::Empty {
            return Err(MechError::new(
                ExternalResourceWriteOutputNotEmpty {
                    found: self.output.borrow().kind(),
                },
                None,
            ));
        }
        if !matches!(
            self.request.intent,
            ResourceIntent::Assign | ResourceIntent::Send
        ) {
            return Err(MechError::new(
                ExternalResourceWriteIntentUnsupported {
                    request: self.request.clone(),
                },
                None,
            ));
        }
        Ok(())
    }

    fn solve_with_services(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.validate()?;
        // Reactive bytecode inputs retain their stable outer register cell, but
        // execution services receive the logical value at the time of the
        // effect. This also keeps source and reconstructed bytecode calls
        // observably equivalent for non-reactive service implementations.
        let input = self.input.try_deep_snapshot()?;
        services.write_resource(&self.request, &input)
    }
}

impl MechFunctionImpl for ExternalResourceWriteFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_with_services(&mut NoMechExecutionServices)
    }

    fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.solve_with_services(services)
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        self.solve_result()?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.solve_with_services(services)?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Logical; argument_count])
    }

    fn initialize_preserved_output_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        // Source specialization deterministically plans the Empty output, but
        // the initial source turn still has to perform the external operation.
        // Activation registration bypasses this hook, so activation-body sends
        // remain deferred until their body executes.
        self.solve_with_services(services)
    }

    fn out(&self) -> LegacyValue {
        self.output.borrow().clone()
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        self.semantic_contract.or(Some(&RESOURCE_EFFECT_CONTRACT))
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(vec![LegacyValue::MutableReference(
            self.output.legacy_ref(),
        )])
    }

    fn to_string(&self) -> String {
        format!("ExternalResourceWriteFunction::{:?}", self.request)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ExternalResourceWriteFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        self.validate()?;
        let output = super::compile_external_output(&self.output, context)?;
        let input = super::compile_external_value(&self.input, context)?;
        let requirement =
            context.intern_requirement(ApplicationRequirement::Resource(self.request.clone()))?;
        match self.request.intent {
            ResourceIntent::Assign => context.emit_resource_write(requirement, output, input),
            ResourceIntent::Send => context.emit_resource_send(requirement, output, input),
            ResourceIntent::Read => unreachable!("validated resource-write intent"),
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalResourceWriteOutputNotEmpty {
    pub found: mech_core::ValueKind,
}

impl MechErrorKind for ExternalResourceWriteOutputNotEmpty {
    fn name(&self) -> &str {
        "ExternalResourceWriteOutputNotEmpty"
    }

    fn message(&self) -> String {
        format!(
            "external resource writes require an Empty output cell, found {:?}",
            self.found,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalResourceWriteIntentUnsupported {
    pub request: ExecutionResourceRequest,
}

impl MechErrorKind for ExternalResourceWriteIntentUnsupported {
    fn name(&self) -> &str {
        "ExternalResourceWriteIntentUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "external resource-write node cannot execute request {:?}",
            self.request,
        )
    }
}

#[cfg(all(test, feature = "f64"))]
mod tests {
    use super::*;
    use mech_core::Ref;

    #[test]
    fn write_requires_an_empty_output_cell_before_calling_services() {
        let function = ExternalResourceWriteFunction {
            request: ExecutionResourceRequest {
                base_uri: "test://provider".into(),
                path: "value".into(),
                context_name: "test".into(),
                operation: "write".into(),
                intent: ResourceIntent::Assign,
                delivery: mech_core::ResourceDelivery::Snapshot,
            },
            input: LegacyValue::F64(Ref::new(1.0)),
            output: ValueCell::new(LegacyValue::F64(Ref::new(2.0))),
            initial_solve_policy: InitialSolvePolicy::Solve,
            semantic_contract: None,
        };

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "ExternalResourceWriteOutputNotEmpty");
        assert_eq!(
            error
                .kind_as::<ExternalResourceWriteOutputNotEmpty>()
                .unwrap()
                .found,
            mech_core::ValueKind::F64,
        );
    }
}
