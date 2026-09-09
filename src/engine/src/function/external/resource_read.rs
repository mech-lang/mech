use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExecutionResourceRequest,
    ExternalInteraction, FunctionStatePort, InitialSolvePolicy, InputPortLayout, MResult,
    MechError, MechErrorKind, MechExecutionServices, MechFunctionImpl, ObservationContract,
    ObservationReplayPolicy, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    PreparedLiveResourceBinding, Ref, ResourceDelivery, ShapeRule, Value, ValueCell,
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
}

impl MechFunctionImpl for ExternalResourceReadFunction {
    fn payload_output_plan_policy(&self) -> mech_core::PayloadOutputPlanPolicy {
        mech_core::PayloadOutputPlanPolicy::ExternalAdoption
    }

    fn capture_external_output(
        &self,
        services: &mut dyn MechExecutionServices,
        _arguments: &[Value],
    ) -> MResult<Option<Value>> {
        Ok(Some(services.read_resource(&self.request)?))
    }

    fn solve_managed(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        Err(MechError::new(
            mech_core::GenericError {
                msg: "external resource reads require a prepared result handoff".to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }

    fn stage_prepared_external_output(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
        result: Value,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.stage_output_value(&self.output, result)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn prepare_external_publication<'a>(
        &self,
        services: &'a mut dyn MechExecutionServices,
    ) -> MResult<Option<PreparedLiveResourceBinding<'a>>> {
        if self.request.delivery == ResourceDelivery::Live {
            return services
                .prepare_live_resource_binding(
                    self.interpreter_id,
                    &self.request,
                    self.output.clone(),
                )
                .map(Some);
        }
        Ok(None)
    }

    fn external_publication_committed(&self) {
        *self.initialized.borrow_mut() = 1;
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
            services
                .prepare_live_resource_binding(
                    self.interpreter_id,
                    &self.request,
                    self.output.clone(),
                )?
                .commit();
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

#[cfg(all(test, feature = "functions", feature = "string"))]
mod tests {
    use super::*;
    use mech_core::{
        ExecutionHostFunctionRequest, ExecutionTarget, FunctionInvocation,
        ImplementationMemoryClass, ResolvedOperationDescriptor, RuntimeFunctionId,
        SpecializedFunction,
    };

    struct LiveServices {
        result: Value,
        reject_binding: bool,
        bindings: usize,
    }

    impl MechExecutionServices for LiveServices {
        fn invoke_host_function(
            &mut self,
            _request: &ExecutionHostFunctionRequest,
            _arguments: &[Value],
        ) -> MResult<Value> {
            unreachable!("resource test does not invoke host functions")
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
            Ok(self.result.clone())
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &Value,
        ) -> MResult<()> {
            unreachable!("resource test does not write")
        }

        fn prepare_live_resource_binding<'a>(
            &'a mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            _target: ValueCell,
        ) -> MResult<PreparedLiveResourceBinding<'a>> {
            if self.reject_binding {
                return Err(MechError::new(
                    mech_core::GenericError {
                        msg: "deliberate live-binding rejection".to_owned(),
                    },
                    None,
                ));
            }
            PreparedLiveResourceBinding::try_new(move || {
                self.bindings += 1;
            })
        }
    }

    fn text(cell: &ValueCell) -> String {
        match cell.snapshot().unwrap().data() {
            mech_core::ValueData::String(value) => value.to_string(),
            other => panic!("expected String, found {other:?}"),
        }
    }

    #[test]
    fn failed_live_binding_preserves_output_adapter_state_and_active_instance() {
        let domain = mech_core::MemoryDomain::new().unwrap();
        let output = ValueCell::from_exact_in(&domain, "old".to_owned()).unwrap();
        let request = ExecutionResourceRequest {
            base_uri: "test://provider".to_owned(),
            path: "item".to_owned(),
            context_name: "provider".to_owned(),
            operation: "read".to_owned(),
            intent: mech_core::ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        };
        let implementation = ExternalResourceReadFunction::new(
            7,
            request,
            output.clone(),
            false,
            InitialSolvePolicy::Solve,
            None,
        );
        let state = implementation.initialized.clone();
        let function = SpecializedFunction::syntax_directed(
            (
                Box::new(implementation),
                FunctionInvocation::nullary(output.clone()),
            ),
            ResolvedOperationDescriptor::from_name(
                "test/resource-read",
                RESOURCE_OBSERVATION_CONTRACT.clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("test/resource-read"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::ExternalMarshalling,
        )
        .unwrap();
        let version = output.published_version();
        let revision = function.instance().managed_plan_revision();
        let mut services = LiveServices {
            result: ValueCell::from_exact("new value with a larger admitted payload".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
            reject_binding: true,
            bindings: 0,
        };

        assert!(
            function
                .instance()
                .solve_result_with(&mut services)
                .is_err()
        );
        assert_eq!(text(&output), "old");
        assert_eq!(output.published_version(), version);
        assert_eq!(*state.borrow(), 0);
        assert_eq!(services.bindings, 0);
        assert_eq!(function.instance().managed_plan_revision(), revision);

        services.reject_binding = false;
        function
            .instance()
            .solve_result_with(&mut services)
            .unwrap();
        assert_eq!(text(&output), "new value with a larger admitted payload");
        assert_eq!(output.published_version().get(), version.get() + 1);
        assert_eq!(*state.borrow(), 1);
        assert_eq!(services.bindings, 1);
        assert_ne!(function.instance().managed_plan_revision(), revision);
    }
}
