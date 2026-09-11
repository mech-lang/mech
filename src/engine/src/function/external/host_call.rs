use mech_core::{
    ExecutionHostFunctionRequest, InitialSolvePolicy, MResult, MechError, MechExecutionServices,
    MechFunctionImpl, ReactiveDependencyScope, Value, ValueCell,
};

#[cfg(feature = "semantic-compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalHostCallFunction {
    pub request: ExecutionHostFunctionRequest,
    pub arguments: Vec<ValueCell>,
    pub output: ValueCell,
    pub initial_solve_policy: InitialSolvePolicy,
}

impl ExternalHostCallFunction {
    pub fn new(
        request: ExecutionHostFunctionRequest,
        arguments: Vec<ValueCell>,
        output: ValueCell,
        initial_solve_policy: InitialSolvePolicy,
    ) -> Self {
        Self {
            request,
            arguments,
            output,
            initial_solve_policy,
        }
    }
}

impl MechFunctionImpl for ExternalHostCallFunction {
    fn payload_output_plan_policy(&self) -> mech_core::PayloadOutputPlanPolicy {
        mech_core::PayloadOutputPlanPolicy::ExternalAdoption
    }

    fn capture_external_output(
        &self,
        services: &mut dyn MechExecutionServices,
        arguments: &[Value],
    ) -> MResult<Option<Value>> {
        Ok(Some(
            services.invoke_host_function(&self.request, arguments)?,
        ))
    }

    fn solve_managed(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        Err(MechError::new(
            mech_core::GenericError {
                msg: "external host calls require a prepared result handoff".to_owned(),
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

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Logical; argument_count])
    }

    fn to_string(&self) -> String {
        format!("ExternalHostCallFunction::{:?}", self.request)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ExternalHostCallFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = super::compile_external_output(&self.output, context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| super::compile_external_cell(argument, context))
            .collect::<MResult<Vec<Register>>>()?;
        let requirement = context
            .intern_requirement(ApplicationRequirement::HostFunction(self.request.clone()))?;
        context.emit_host_call(requirement, output, arguments);
        Ok(output)
    }
}

#[cfg(all(test, feature = "functions", feature = "string"))]
mod tests {
    use super::*;
    use mech_core::{
        AccessMode, AliasPolicy, AllocationRole, ChangeDetectionPolicy, DeliveryMode,
        ExecutionResourceRequest, ExecutionTarget, ExternalInteraction, FunctionInvocation,
        ImplementationMemoryClass, InputPortLayout, InputPortPolicy, MemoryDomain,
        MemoryFailurePoint, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
        ReactiveSolveStatus, ResolvedOperationDescriptor, RuntimeFunctionId, ShapeRule,
        SpecializedFunction,
    };
    #[cfg(feature = "matrixd")]
    use nalgebra::DMatrix;

    struct RecordingServices {
        calls: usize,
        result: Value,
    }

    impl MechExecutionServices for RecordingServices {
        fn invoke_host_function(
            &mut self,
            _request: &ExecutionHostFunctionRequest,
            arguments: &[Value],
        ) -> MResult<Value> {
            self.calls += 1;
            assert_eq!(arguments.len(), 1);
            Ok(self.result.clone())
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
            unreachable!("host-call test does not read resources")
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &Value,
        ) -> MResult<()> {
            unreachable!("host-call test does not write resources")
        }

        fn prepare_live_resource_binding<'a>(
            &'a mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            _target: ValueCell,
        ) -> MResult<mech_core::PreparedLiveResourceBinding<'a>> {
            unreachable!("host-call test does not bind resources")
        }
    }

    fn parts(
        input: ValueCell,
        output: ValueCell,
    ) -> (
        Box<dyn mech_core::MechFunction>,
        FunctionInvocation,
        ResolvedOperationDescriptor,
    ) {
        let request = ExecutionHostFunctionRequest {
            name: "test/captured".to_owned(),
        };
        let implementation = ExternalHostCallFunction::new(
            request,
            vec![input.clone()],
            output.clone(),
            InitialSolvePolicy::Solve,
        );
        let contract = OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
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
            interaction: ExternalInteraction::Pure,
        };
        (
            Box::new(implementation),
            FunctionInvocation::unary(output, input),
            ResolvedOperationDescriptor::from_name("test/external-host", contract).unwrap(),
        )
    }

    fn instance(input: ValueCell, output: ValueCell) -> mech_core::FunctionInstance {
        let (implementation, invocation, operation) = parts(input, output);
        SpecializedFunction::syntax_directed(
            (implementation, invocation),
            operation,
            RuntimeFunctionId::from_name("test/external-host"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::ExternalMarshalling,
        )
        .unwrap()
        .into_instance()
    }

    #[cfg(all(feature = "matrixd", feature = "u8"))]
    #[test]
    fn numeric_marshalling_budget_covers_draft_expansion_and_finalization() {
        let domain = MemoryDomain::new().unwrap();
        let element_count = 128 * 128;
        let input =
            ValueCell::from_exact_in(&domain, DMatrix::<u8>::from_element(128, 128, 7)).unwrap();
        let output = ValueCell::from_exact_in(&domain, "old".to_owned()).unwrap();
        let planned = instance(input.clone(), output.clone());
        let mut plan = planned.memory_plan().clone();
        let required = plan.demand.turn_peak_bytes;
        let native_bytes = element_count as u64;
        let draft_bytes = (element_count as u64)
            .checked_mul(core::mem::size_of::<mech_core::ValueDataDraft>() as u64)
            .unwrap();
        assert!(
            plan.demand.cloned_bytes >= native_bytes + draft_bytes,
            "the plan must include numeric-to-canonical expansion, not only native u8 bytes"
        );
        assert!(required > draft_bytes);
        let insufficient = required - 1;
        plan.target.limits.max_temporary_bytes = Some(insufficient);
        let (implementation, invocation, _) = parts(input.clone(), output.clone());
        let bound = plan.bound_call.clone();
        let error =
            match SpecializedFunction::new((implementation, invocation), bound, plan.clone()) {
                Ok(_) => panic!("an insufficient finite marshalling budget was accepted"),
                Err(error) => error,
            };
        let memory = error
            .kind_as::<mech_core::MemoryRuntimeError>()
            .expect("finite budget rejection is a managed-memory error");
        assert!(matches!(
            memory,
            mech_core::MemoryRuntimeError::BudgetExceeded { .. }
        ));
        assert!(plan.target.limits.max_temporary_bytes == Some(insufficient));

        let sufficient = required.checked_mul(2).unwrap();
        plan.target.limits.max_temporary_bytes = Some(sufficient);
        let (implementation, invocation, _) = parts(input, output.clone());
        let bound = plan.bound_call.clone();
        let admitted = SpecializedFunction::new((implementation, invocation), bound, plan)
            .expect("a sufficient finite R5 marshalling budget must admit the call");
        let mut services = RecordingServices {
            calls: 0,
            result: ValueCell::from_exact("accepted".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        };
        admitted
            .instance()
            .solve_result_with(&mut services)
            .unwrap();
        assert_eq!(services.calls, 1);
        assert_eq!(text(&output), "accepted");
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn parameterized_canonical_marshalling_admits_shape_before_provider_invocation() {
        let domain = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(
            &domain,
            DMatrix::from_row_slice(
                2,
                3,
                &[
                    "a".to_owned(),
                    "b".to_owned(),
                    "c".to_owned(),
                    "d".to_owned(),
                    "e".to_owned(),
                    "f".to_owned(),
                ],
            ),
        )
        .unwrap();
        assert_eq!(input.shape().parameter_values(), &[2, 3]);
        let output = ValueCell::from_exact_in(&domain, "old".to_owned()).unwrap();
        let function = instance(input, output.clone());
        let shape_bytes = 2 * core::mem::size_of::<u64>() as u64;
        let scratch_bytes = function
            .memory_plan()
            .allocations
            .iter()
            .filter(|allocation| allocation.role == AllocationRole::ConstructionWorkspace)
            .map(|allocation| allocation.capacity_bytes)
            .sum::<u64>();
        assert!(
            scratch_bytes >= shape_bytes,
            "parameterized canonical input shape is absent from finite marshalling scratch"
        );

        let mut services = RecordingServices {
            calls: 0,
            result: ValueCell::from_exact("accepted".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        };
        domain
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        assert!(function.solve_result_with(&mut services).is_err());
        assert_eq!(
            services.calls, 0,
            "provider ran before parameterized canonical metadata was admitted"
        );
        assert_eq!(text(&output), "old");

        function.solve_result_with(&mut services).unwrap();
        assert_eq!(services.calls, 1);
        assert_eq!(text(&output), "accepted");
    }

    fn text(cell: &ValueCell) -> String {
        match cell.snapshot().unwrap().data() {
            mech_core::ValueData::String(value) => value.to_string(),
            other => panic!("expected String output, found {other:?}"),
        }
    }

    #[test]
    fn marshalling_is_admitted_before_provider_and_captured_rejection_is_scoped() {
        let domain = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(&domain, "argument".to_owned()).unwrap();
        let output = ValueCell::from_exact_in(&domain, "old".to_owned()).unwrap();
        let function = instance(input, output.clone());
        let mut services = RecordingServices {
            calls: 0,
            result: ValueCell::from_exact("first".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        };

        domain
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        assert!(function.solve_result_with(&mut services).is_err());
        assert_eq!(
            services.calls, 0,
            "provider is unreachable before marshalling admission"
        );
        assert_eq!(text(&output), "old");

        assert_eq!(
            function.solve_reactive_with(&mut services).unwrap(),
            ReactiveSolveStatus::Changed,
        );
        assert_eq!(services.calls, 1);
        assert_eq!(text(&output), "first");

        services.result = ValueCell::from_exact("captured then rejected".repeat(64))
            .unwrap()
            .snapshot()
            .unwrap();
        domain
            .inject_failure_after(MemoryFailurePoint::Admission, 0)
            .unwrap();
        assert!(function.solve_result_with(&mut services).is_err());
        assert_eq!(
            services.calls, 2,
            "the rejected provider result is captured once"
        );
        assert_eq!(text(&output), "first");

        services.result = ValueCell::from_exact("after rejection".to_owned())
            .unwrap()
            .snapshot()
            .unwrap();
        function.solve_result_with(&mut services).unwrap();
        assert_eq!(
            services.calls, 3,
            "no stale captured result survives rejection"
        );
        assert_eq!(text(&output), "after rejection");
    }
}
