use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExecutionResourceRequest,
    ExternalInteraction, FunctionStatePort, InitialSolvePolicy, InputPortLayout, MResult,
    MechError, MechErrorKind, MechExecutionServices, MechFunctionImpl, ObservationContract,
    ObservationReplayPolicy, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    Ref, ResourceDelivery, ShapeRule, Value, ValueCell,
};
use std::sync::LazyLock;

pub(crate) static RESOURCE_OBSERVATION_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    });

#[cfg(feature = "semantic-compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalResourceReadFunction {
    pub interpreter_id: u64,
    pub request: ExecutionResourceRequest,
    pub output: ValueCell,
    pub initial_solve_policy: InitialSolvePolicy,
    pub semantic_contract: Option<&'static OperationContractDeclaration>,
    initialized: Ref<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalResourceReadUninitializedValue {
    pub request: ExecutionResourceRequest,
}

impl MechErrorKind for ExternalResourceReadUninitializedValue {
    fn name(&self) -> &str {
        "ExternalResourceReadUninitializedValue"
    }

    fn message(&self) -> String {
        format!(
            "resource read {:?} returned an untyped empty value and cannot initialize its stable output",
            self.request,
        )
    }
}

impl ExternalResourceReadFunction {
    pub fn new(
        interpreter_id: u64,
        request: ExecutionResourceRequest,
        output: ValueCell,
        initialized: bool,
        initial_solve_policy: InitialSolvePolicy,
        semantic_contract: Option<&'static OperationContractDeclaration>,
    ) -> Self {
        Self {
            interpreter_id,
            request,
            output,
            initial_solve_policy,
            semantic_contract,
            initialized: Ref::new(usize::from(initialized)),
        }
    }

    fn stage_read_result(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        result: Value,
    ) -> MResult<()> {
        frame.stage_output_value(&self.output, &result)?;
        *self.initialized.borrow_mut() = 1;
        Ok(())
    }

    fn solve_with_services(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let result = services.read_resource(&self.request)?;
        self.stage_read_result(frame, result)?;
        if self.request.delivery == ResourceDelivery::Live {
            services.bind_live_resource(self.interpreter_id, &self.request, self.output.clone())?;
        }
        Ok(())
    }
}

impl MechFunctionImpl for ExternalResourceReadFunction {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        self.solve_with_services(frame, services)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn initialize_preserved_output_with(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        if *self.initialized.borrow() == 0 {
            return Err(MechError::new(
                ExternalResourceReadUninitializedValue {
                    request: self.request.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }
        if self.request.delivery == ResourceDelivery::Live {
            services.bind_live_resource(self.interpreter_id, &self.request, self.output.clone())?;
        }
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        self.semantic_contract
            .or(Some(&RESOURCE_OBSERVATION_CONTRACT))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.initialized)]))
    }

    fn to_string(&self) -> String {
        format!("ExternalResourceReadFunction::{:?}", self.request)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ExternalResourceReadFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = super::compile_runtime_produced_external_output(&self.output, context)?;
        let requirement =
            context.intern_requirement(ApplicationRequirement::Resource(self.request.clone()))?;
        context.emit_resource_read(requirement, output);
        Ok(output)
    }
}
