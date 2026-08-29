mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "semantic-compiler")]
use mech_core::{
    BytecodeCompilerContext, MResult, Register, ValueCell,
    compile_runtime_produced_value_cell_register, compile_value_cell_register,
};

#[cfg(all(test, feature = "semantic-compiler"))]
use mech_core::{LegacyValue, compile_value_register};

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_output(
    output: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_cell_register(output, context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_cell(
    value: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_cell_register(value, context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_runtime_produced_external_output(
    output: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_runtime_produced_value_cell_register(output, context)
}

#[cfg(all(test, feature = "semantic-compiler"))]
pub(super) fn compile_external_value(
    value: &LegacyValue,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_register(value, std::ptr::from_ref(value).addr(), context)
}

#[cfg(all(test, feature = "semantic-compiler", feature = "f64"))]
mod tests {
    use super::*;
    use crate::{CompileCtx, CompiledBytecode};
    use mech_core::matrix::Matrix;
    use mech_core::{
        BytecodeInstruction, ExecutionHostFunctionRequest, ExecutionResourceRequest, GenericError,
        InitialSolvePolicy, LegacyValue, MResult, MechError, MechExecutionServices,
        MechFunctionCompiler, MechFunctionImpl, Ref, ResourceDelivery, ResourceIntent, Value,
        ValueCell, with_reactive_journal_participant,
    };
    use nalgebra::DMatrix;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FailingServices {
        host_calls: usize,
        resource_reads: usize,
        resource_writes: usize,
    }

    impl FailingServices {
        fn error(operation: &str) -> MechError {
            MechError::new(
                GenericError {
                    msg: format!("deliberate {operation} failure"),
                },
                None,
            )
        }
    }

    impl MechExecutionServices for FailingServices {
        fn invoke_host_function(
            &mut self,
            _request: &ExecutionHostFunctionRequest,
            _arguments: &[Value],
        ) -> MResult<Value> {
            self.host_calls += 1;
            Err(Self::error("host call"))
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
            self.resource_reads += 1;
            Err(Self::error("resource read"))
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &Value,
        ) -> MResult<()> {
            self.resource_writes += 1;
            Err(Self::error("resource write"))
        }

        fn bind_live_resource(
            &mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            _target: ValueCell,
        ) -> MResult<()> {
            Err(Self::error("live resource bind"))
        }
    }

    struct RecordingReadServices {
        results: VecDeque<Value>,
        planning_representative: Option<Value>,
        planning_calls: usize,
        resource_reads: usize,
        live_bindings: usize,
        bound_targets: Vec<ValueCell>,
    }

    impl RecordingReadServices {
        fn new(results: impl IntoIterator<Item = LegacyValue>) -> Self {
            let results: VecDeque<_> = results.into_iter().map(snapshot).collect();
            Self {
                planning_representative: results.front().cloned(),
                results,
                planning_calls: 0,
                resource_reads: 0,
                live_bindings: 0,
                bound_targets: Vec::new(),
            }
        }
    }

    impl MechExecutionServices for RecordingReadServices {
        fn invoke_host_function(
            &mut self,
            _request: &ExecutionHostFunctionRequest,
            _arguments: &[Value],
        ) -> MResult<Value> {
            Err(FailingServices::error("unexpected host call"))
        }

        fn plan_resource_read_output(
            &mut self,
            _request: &ExecutionResourceRequest,
        ) -> MResult<Value> {
            self.planning_calls += 1;
            let representative = self
                .planning_representative
                .as_ref()
                .ok_or_else(|| FailingServices::error("missing planning representative"))?;
            Ok(representative.clone())
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
            self.resource_reads += 1;
            self.results
                .pop_front()
                .ok_or_else(|| FailingServices::error("missing resource result"))
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &Value,
        ) -> MResult<()> {
            Err(FailingServices::error("unexpected resource write"))
        }

        fn bind_live_resource(
            &mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            target: ValueCell,
        ) -> MResult<()> {
            self.live_bindings += 1;
            self.bound_targets.push(target);
            Ok(())
        }
    }

    struct RecordingHostServices {
        result: Value,
        arguments: Vec<Vec<Value>>,
    }

    impl MechExecutionServices for RecordingHostServices {
        fn invoke_host_function(
            &mut self,
            _request: &ExecutionHostFunctionRequest,
            arguments: &[Value],
        ) -> MResult<Value> {
            self.arguments.push(arguments.to_vec());
            Ok(self.result.clone())
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
            Err(FailingServices::error("unexpected resource read"))
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &Value,
        ) -> MResult<()> {
            Err(FailingServices::error("unexpected resource write"))
        }

        fn bind_live_resource(
            &mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            _target: ValueCell,
        ) -> MResult<()> {
            Err(FailingServices::error("unexpected live binding"))
        }
    }

    fn resource_request(intent: ResourceIntent) -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://provider".into(),
            path: "value".into(),
            context_name: "test".into(),
            operation: match intent {
                ResourceIntent::Read => "read",
                ResourceIntent::Assign => "write",
                ResourceIntent::Send => "send",
            }
            .into(),
            intent,
            delivery: ResourceDelivery::Snapshot,
        }
    }

    fn resource_read_function(
        output: ValueCell,
        delivery: ResourceDelivery,
    ) -> ExternalResourceReadFunction {
        let mut request = resource_request(ResourceIntent::Read);
        request.delivery = delivery;
        ExternalResourceReadFunction::new(7, request, output, true, InitialSolvePolicy::Solve, None)
    }

    fn uninitialized_resource_read_function(
        output: ValueCell,
        delivery: ResourceDelivery,
    ) -> ExternalResourceReadFunction {
        let mut request = resource_request(ResourceIntent::Read);
        request.delivery = delivery;
        ExternalResourceReadFunction::new(
            7,
            request,
            output,
            false,
            InitialSolvePolicy::Solve,
            None,
        )
    }

    fn cell(value: LegacyValue) -> ValueCell {
        mech_core::value_cell_from_legacy_function_value(value)
    }

    fn snapshot(value: LegacyValue) -> Value {
        cell(value).snapshot().unwrap()
    }

    fn legacy(cell: &ValueCell) -> LegacyValue {
        mech_core::legacy_function_value_from_cell(cell).unwrap()
    }

    fn matrix(rows: usize, columns: usize, values: Vec<f64>) -> LegacyValue {
        LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_vec(
            rows, columns, values,
        ))))
    }

    fn compile_resource_read(
        observed: LegacyValue,
        delivery: ResourceDelivery,
    ) -> (CompiledBytecode, Vec<u8>, u32) {
        let output = cell(observed);
        let function = resource_read_function(output, delivery);
        let mut context = CompileCtx::new();
        let destination = function.compile(&mut context).unwrap();
        let compiled = context.finish_program(destination).unwrap();
        let bytes = context.finish(destination).unwrap();
        (compiled, bytes, destination)
    }

    #[test]
    fn typed_external_values_do_not_share_bare_registers_in_either_order() {
        for typed_first in [false, true] {
            let scalar = Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let typed = LegacyValue::Typed(
                Box::new(LegacyValue::F64(scalar)),
                mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
            );
            let typed_clone = typed.clone();
            let mut context = CompileCtx::new();

            let (first, second) = if typed_first {
                (
                    compile_external_value(&typed, &mut context).unwrap(),
                    compile_external_value(&bare, &mut context).unwrap(),
                )
            } else {
                (
                    compile_external_value(&bare, &mut context).unwrap(),
                    compile_external_value(&typed, &mut context).unwrap(),
                )
            };

            assert_ne!(first, second);
            assert_eq!(
                compile_external_value(&typed_clone, &mut context).unwrap(),
                if typed_first { first } else { second },
            );
            let parsed =
                mech_core::ParsedProgram::from_bytes(&context.finish(second).unwrap()).unwrap();
            assert!(parsed.constants.iter().any(|constant| {
                parsed.types[constant.type_id as usize] == mech_core::RuntimeType::F64
            }));
            assert!(parsed.constants.iter().any(|constant| {
                parsed.types[constant.type_id as usize]
                    == mech_core::RuntimeType::Option(Box::new(mech_core::RuntimeType::F64))
            }));
        }
    }

    #[test]
    fn typed_external_outputs_do_not_share_argument_registers() {
        let scalar = Ref::new(7.0);
        let bare_argument = LegacyValue::F64(scalar.clone());
        let typed_output = ValueCell::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(scalar)),
            mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
        ));
        let mut context = CompileCtx::new();

        let argument = compile_external_value(&bare_argument, &mut context).unwrap();
        let output = compile_external_output(&typed_output, &mut context).unwrap();

        assert_ne!(argument, output);
        let parsed =
            mech_core::ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
        assert!(parsed.constants.iter().any(|constant| {
            parsed.types[constant.type_id as usize] == mech_core::RuntimeType::F64
        }));
        assert!(parsed.constants.iter().any(|constant| {
            parsed.types[constant.type_id as usize]
                == mech_core::RuntimeType::Option(Box::new(mech_core::RuntimeType::F64))
        }));
    }

    #[test]
    fn cloned_output_cells_reuse_registers_and_distinct_cells_do_not() {
        let shared = ValueCell::from_exact(0.0_f64).unwrap();
        let clone = shared.clone();
        let distinct = ValueCell::from_exact(0.0_f64).unwrap();
        let mut context = CompileCtx::new();

        let shared_register = compile_external_output(&shared, &mut context).unwrap();
        let clone_register = compile_external_output(&clone, &mut context).unwrap();
        let distinct_register = compile_external_output(&distinct, &mut context).unwrap();

        assert_eq!(shared_register, clone_register);
        assert_ne!(shared_register, distinct_register);
    }

    #[test]
    fn host_call_failure_propagates_without_publishing_a_stale_output() {
        let output = cell(LegacyValue::F64(Ref::new(41.0)));
        let function = ExternalHostCallFunction {
            request: ExecutionHostFunctionRequest {
                name: "test/fail".into(),
            },
            arguments: Vec::new(),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
        };
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(error.full_chain_message().contains("host call failure"));
        assert_eq!(services.host_calls, 1);
        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 41.0));
    }

    #[test]
    fn host_call_snapshots_scalar_and_aggregate_cells_and_replaces_canonical_output() {
        let scalar = cell(LegacyValue::F64(Ref::new(3.0)));
        let aggregate = ValueCell::unit();
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let function = ExternalHostCallFunction {
            request: ExecutionHostFunctionRequest {
                name: "test/canonical".into(),
            },
            arguments: vec![scalar, aggregate],
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
        };
        let mut services = RecordingHostServices {
            result: snapshot(LegacyValue::F64(Ref::new(9.0))),
            arguments: Vec::new(),
        };

        function.solve_result_with(&mut services).unwrap();

        assert!(
            matches!(services.arguments[0][0].data(), mech_core::ValueData::F64(value) if value.to_f64() == 3.0)
        );
        assert!(
            matches!(services.arguments[0][1].data(), mech_core::ValueData::Tuple(values) if values.is_empty())
        );
        assert!(
            matches!(output.snapshot().unwrap().data(), mech_core::ValueData::F64(value) if value.to_f64() == 9.0)
        );
    }

    #[test]
    fn host_call_rejects_a_result_with_the_wrong_schema_without_mutating_output() {
        let output = cell(LegacyValue::F64(Ref::new(7.0)));
        let function = ExternalHostCallFunction {
            request: ExecutionHostFunctionRequest {
                name: "test/schema-mismatch".into(),
            },
            arguments: Vec::new(),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
        };
        let mut services = RecordingHostServices {
            result: snapshot(matrix(1, 1, vec![9.0])),
            arguments: Vec::new(),
        };

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert_eq!(error.kind_name(), "ValueCellSchemaMismatch");
        assert!(
            matches!(output.snapshot().unwrap().data(), mech_core::ValueData::F64(value) if value.to_f64() == 7.0)
        );
    }

    #[test]
    fn resource_read_failure_propagates_without_publishing_a_stale_output() {
        let output = cell(LegacyValue::F64(Ref::new(42.0)));
        let function = ExternalResourceReadFunction::new(
            7,
            resource_request(ResourceIntent::Read),
            output.clone(),
            true,
            InitialSolvePolicy::Solve,
            None,
        );
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(error.full_chain_message().contains("resource read failure"));
        assert_eq!(services.resource_reads, 1);
        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 42.0));
    }

    #[test]
    fn resource_read_initializes_empty_stable_output() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let original_output = output.clone();
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([LegacyValue::F64(Ref::new(42.0))]);

        function.solve_result_with(&mut services).unwrap();

        assert!(output.same_cell(&original_output));
        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 42.0));
        assert_eq!(services.resource_reads, 1);
    }

    #[test]
    fn resource_read_initializes_empty_matrix_output() {
        let output = cell(matrix(2, 2, vec![0.0; 4]));
        let original_output = output.clone();
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([matrix(2, 2, vec![1.0, 2.0, 3.0, 4.0])]);

        function.solve_result_with(&mut services).unwrap();

        assert!(output.same_cell(&original_output));
        assert!(matches!(
            legacy(&output),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (2, 2)
        ));
    }

    #[test]
    fn resource_read_subsequent_same_representation_update_uses_stable_contract() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let original_output = output.clone();
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([
            LegacyValue::F64(Ref::new(1.0)),
            LegacyValue::F64(Ref::new(2.0)),
        ]);

        function.solve_result_with(&mut services).unwrap();
        function.solve_result_with(&mut services).unwrap();

        assert!(output.same_cell(&original_output));
        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 2.0));
    }

    #[test]
    fn resource_read_rejects_representation_change_after_initialization() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services =
            RecordingReadServices::new([LegacyValue::F64(Ref::new(1.0)), matrix(1, 1, vec![2.0])]);

        function.solve_result_with(&mut services).unwrap();
        let error = function.solve_result_with(&mut services).unwrap_err();

        assert_eq!(error.kind_name(), "ValueCellSchemaMismatch");
        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 1.0));
    }

    #[test]
    fn resource_read_preserves_dynamic_matrix_identity_across_shape_changes() {
        let output = cell(matrix(1, 2, vec![0.0, 0.0]));
        let alias = output.clone();
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([
            matrix(1, 2, vec![1.0, 2.0]),
            matrix(2, 1, vec![3.0, 4.0]),
        ]);

        function.solve_result_with(&mut services).unwrap();
        function.solve_result_with(&mut services).unwrap();

        assert!(output.same_cell(&alias));
        assert!(matches!(
            legacy(&output),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (2, 1)
                    && value.borrow().as_slice() == [3.0, 4.0]
        ));
    }

    #[test]
    fn resource_read_rejects_preserving_an_uninitialized_output() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let function = uninitialized_resource_read_function(output, ResourceDelivery::Live);
        let mut services = RecordingReadServices::new([LegacyValue::F64(Ref::new(1.0))]);

        let error = function
            .initialize_preserved_output_with(&mut services)
            .unwrap_err();

        assert_eq!(error.kind_name(), "ExternalResourceReadUninitializedValue");
        assert_eq!(services.live_bindings, 0);
    }

    #[test]
    fn resource_read_live_binding_observes_initialized_cell() {
        let output = cell(matrix(2, 1, vec![0.0, 0.0]));
        let function = uninitialized_resource_read_function(output.clone(), ResourceDelivery::Live);
        let mut services = RecordingReadServices::new([matrix(2, 1, vec![1.0, 2.0])]);

        function.solve_result_with(&mut services).unwrap();

        assert_eq!(services.live_bindings, 1);
        assert!(services.bound_targets[0].same_cell(&output));
        assert!(matches!(
            legacy(&services.bound_targets[0]),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (2, 1)
        ));
    }

    #[test]
    fn repeated_live_bindings_share_one_updated_output_cell() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let function = uninitialized_resource_read_function(output.clone(), ResourceDelivery::Live);
        let mut services = RecordingReadServices::new([
            LegacyValue::F64(Ref::new(1.0)),
            LegacyValue::F64(Ref::new(2.0)),
        ]);

        function.solve_result_with(&mut services).unwrap();
        function.solve_result_with(&mut services).unwrap();

        assert_eq!(services.bound_targets.len(), 2);
        assert!(services.bound_targets[0].same_cell(&output));
        assert!(services.bound_targets[1].same_cell(&output));
        assert!(services.bound_targets[0].same_cell(&services.bound_targets[1]));
        assert!(matches!(
            legacy(&services.bound_targets[0]),
            LegacyValue::F64(value) if *value.borrow() == 2.0
        ));
    }

    #[test]
    fn resource_read_hidden_state_is_exposed_only_as_a_typed_port() {
        let output = cell(LegacyValue::F64(Ref::new(1.0)));
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);

        assert_eq!(function.retained_state_ports().unwrap().len(), 1);
    }

    #[test]
    fn resource_read_rollback_restores_hidden_initialization_state() {
        let output = cell(LegacyValue::F64(Ref::new(0.0)));
        let function =
            uninitialized_resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([LegacyValue::F64(Ref::new(9.0))]);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_value_cell(&output)?;
            participant.capture_function_state(&function)?;
            function.solve_result_with(&mut services)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(matches!(legacy(&output), LegacyValue::F64(value) if *value.borrow() == 0.0));
        let mut services = RecordingReadServices::new([]);
        let error = function
            .initialize_preserved_output_with(&mut services)
            .unwrap_err();
        assert_eq!(error.kind_name(), "ExternalResourceReadUninitializedValue");
    }

    #[test]
    fn resource_read_compile_records_kind_without_const_load() {
        let observed = matrix(4, 1, vec![1.0, 2.0, 3.0, 4.0]);
        let expected_schema = cell(observed.clone()).closed_schema_body().unwrap();
        let (compiled, _bytes, destination) =
            compile_resource_read(observed, ResourceDelivery::Live);

        assert_eq!(compiled.program.register_count, 1);
        assert_eq!(
            compiled.register_schemas[destination as usize].as_ref(),
            Some(&expected_schema)
        );
        assert_eq!(
            compiled
                .program
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    BytecodeInstruction::ResourceRead { dst, .. } if *dst == destination
                ))
                .count(),
            1
        );
        assert!(!compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == destination)
        }));
        assert!(!compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::CompositePack { dst, .. } if *dst == destination)
        }));
        assert!(compiled.program.constants.is_empty());
    }

    #[test]
    fn resource_read_compile_does_not_encode_observed_payload() {
        let (first, first_bytes, _) = compile_resource_read(
            matrix(4, 1, vec![1.0, 2.0, 3.0, 4.0]),
            ResourceDelivery::Live,
        );
        let (second, second_bytes, _) = compile_resource_read(
            matrix(4, 1, vec![101.0, 202.0, 303.0, 404.0]),
            ResourceDelivery::Live,
        );

        assert_eq!(first.program.register_count, second.program.register_count);
        assert_eq!(first.register_schemas, second.register_schemas);
        assert_eq!(first.program.requirements, second.program.requirements);
        assert_eq!(first.program.instructions, second.program.instructions);
        assert_eq!(first.program.constants.len(), 0);
        assert_eq!(second.program.constants.len(), 0);
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn resource_write_failure_propagates_without_changing_its_output() {
        let output = ValueCell::unit();
        let function = ExternalResourceWriteFunction {
            request: resource_request(ResourceIntent::Assign),
            input: cell(LegacyValue::F64(Ref::new(43.0))),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
            semantic_contract: None,
        };
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(
            error
                .full_chain_message()
                .contains("resource write failure")
        );
        assert_eq!(services.resource_writes, 1);
        assert!(
            matches!(output.snapshot().unwrap().data(), mech_core::ValueData::Tuple(values) if values.is_empty())
        );
    }
}
