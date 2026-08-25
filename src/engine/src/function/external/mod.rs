mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "semantic-compiler")]
use mech_core::{
    BytecodeCompilerContext, LegacyValue, MResult, Register, ValRef, compile_value_register,
};

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_output(
    output: &ValRef,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let value = output.borrow();
    compile_external_value_with_fallback(&value, output.addr(), context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_runtime_produced_external_output(
    output: &ValRef,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let value = output.borrow();
    mech_core::compile_runtime_produced_register(&value, output.addr(), context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_value(
    value: &LegacyValue,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_external_value_with_fallback(value, std::ptr::from_ref(value).addr(), context)
}

#[cfg(feature = "semantic-compiler")]
fn compile_external_value_with_fallback(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_register(value, fallback, context)
}

#[cfg(all(test, feature = "semantic-compiler", feature = "f64"))]
mod tests {
    use super::*;
    use crate::{CompileCtx, CompiledBytecode};
    use mech_core::matrix::Matrix;
    use mech_core::{
        BytecodeInstruction, ExecutionHostFunctionRequest, ExecutionResourceRequest, GenericError,
        InitialSolvePolicy, LegacyValue, MResult, MechError, MechExecutionServices,
        MechFunctionCompiler, MechFunctionImpl, Ref, ResourceDelivery, ResourceIntent, ValRef,
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
            _arguments: &[LegacyValue],
        ) -> MResult<LegacyValue> {
            self.host_calls += 1;
            Err(Self::error("host call"))
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
            self.resource_reads += 1;
            Err(Self::error("resource read"))
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &LegacyValue,
        ) -> MResult<()> {
            self.resource_writes += 1;
            Err(Self::error("resource write"))
        }

        fn bind_live_resource(
            &mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            _target: ValRef,
        ) -> MResult<()> {
            Err(Self::error("live resource bind"))
        }
    }

    struct RecordingReadServices {
        results: VecDeque<LegacyValue>,
        planning_representative: Option<LegacyValue>,
        planning_calls: usize,
        resource_reads: usize,
        live_bindings: usize,
        bound_targets: Vec<ValRef>,
    }

    impl RecordingReadServices {
        fn new(results: impl IntoIterator<Item = LegacyValue>) -> Self {
            let results: VecDeque<_> = results.into_iter().collect();
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
            _arguments: &[LegacyValue],
        ) -> MResult<LegacyValue> {
            Err(FailingServices::error("unexpected host call"))
        }

        fn plan_resource_read_output(
            &mut self,
            _request: &ExecutionResourceRequest,
        ) -> MResult<LegacyValue> {
            self.planning_calls += 1;
            self.planning_representative
                .as_ref()
                .ok_or_else(|| FailingServices::error("missing planning representative"))?
                .try_deep_snapshot()
        }

        fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
            self.resource_reads += 1;
            self.results
                .pop_front()
                .ok_or_else(|| FailingServices::error("missing resource result"))
        }

        fn write_resource(
            &mut self,
            _request: &ExecutionResourceRequest,
            _value: &LegacyValue,
        ) -> MResult<()> {
            Err(FailingServices::error("unexpected resource write"))
        }

        fn bind_live_resource(
            &mut self,
            _interpreter_id: u64,
            _request: &ExecutionResourceRequest,
            target: ValRef,
        ) -> MResult<()> {
            self.live_bindings += 1;
            self.bound_targets.push(target);
            Ok(())
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
        output: ValRef,
        delivery: ResourceDelivery,
    ) -> ExternalResourceReadFunction {
        let mut request = resource_request(ResourceIntent::Read);
        request.delivery = delivery;
        ExternalResourceReadFunction {
            interpreter_id: 7,
            request,
            output,
            initial_solve_policy: InitialSolvePolicy::Solve,
            semantic_contract: None,
        }
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
        let output = Ref::new(observed);
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
        let typed_output = Ref::new(LegacyValue::Typed(
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
    fn host_call_failure_propagates_without_publishing_a_stale_output() {
        let output = Ref::new(LegacyValue::F64(Ref::new(41.0)));
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
        assert!(matches!(&*output.borrow(), LegacyValue::F64(value) if *value.borrow() == 41.0));
    }

    #[test]
    fn resource_read_failure_propagates_without_publishing_a_stale_output() {
        let output = Ref::new(LegacyValue::F64(Ref::new(42.0)));
        let function = ExternalResourceReadFunction {
            interpreter_id: 7,
            request: resource_request(ResourceIntent::Read),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
            semantic_contract: None,
        };
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(error.full_chain_message().contains("resource read failure"));
        assert_eq!(services.resource_reads, 1);
        assert!(matches!(&*output.borrow(), LegacyValue::F64(value) if *value.borrow() == 42.0));
    }

    #[test]
    fn resource_read_initializes_empty_stable_output() {
        let output = Ref::new(LegacyValue::Empty);
        let output_address = output.addr();
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([LegacyValue::F64(Ref::new(42.0))]);

        function.solve_result_with(&mut services).unwrap();

        assert_eq!(output.addr(), output_address);
        assert!(matches!(&*output.borrow(), LegacyValue::F64(value) if *value.borrow() == 42.0));
        assert_eq!(services.resource_reads, 1);
    }

    #[test]
    fn resource_read_initializes_empty_matrix_output() {
        let output = Ref::new(LegacyValue::Empty);
        let output_address = output.addr();
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([matrix(2, 2, vec![1.0, 2.0, 3.0, 4.0])]);

        function.solve_result_with(&mut services).unwrap();

        assert_eq!(output.addr(), output_address);
        assert!(matches!(
            &*output.borrow(),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (2, 2)
        ));
    }

    #[test]
    fn resource_read_subsequent_same_representation_update_uses_stable_contract() {
        let output = Ref::new(LegacyValue::Empty);
        let output_address = output.addr();
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([
            LegacyValue::F64(Ref::new(1.0)),
            LegacyValue::F64(Ref::new(2.0)),
        ]);

        function.solve_result_with(&mut services).unwrap();
        function.solve_result_with(&mut services).unwrap();

        assert_eq!(output.addr(), output_address);
        assert!(matches!(&*output.borrow(), LegacyValue::F64(value) if *value.borrow() == 2.0));
    }

    #[test]
    fn resource_read_rejects_representation_change_after_initialization() {
        let output = Ref::new(LegacyValue::Empty);
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services =
            RecordingReadServices::new([LegacyValue::F64(Ref::new(1.0)), matrix(1, 1, vec![2.0])]);

        function.solve_result_with(&mut services).unwrap();
        let error = function.solve_result_with(&mut services).unwrap_err();

        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(matches!(&*output.borrow(), LegacyValue::F64(value) if *value.borrow() == 1.0));
    }

    #[test]
    fn resource_read_rejects_shape_change_after_initialization() {
        let output = Ref::new(LegacyValue::Empty);
        let function = resource_read_function(output.clone(), ResourceDelivery::Snapshot);
        let mut services = RecordingReadServices::new([
            matrix(1, 2, vec![1.0, 2.0]),
            matrix(2, 1, vec![3.0, 4.0]),
        ]);

        function.solve_result_with(&mut services).unwrap();
        let error = function.solve_result_with(&mut services).unwrap_err();

        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(matches!(
            &*output.borrow(),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (1, 2)
                    && value.borrow().as_slice() == [1.0, 2.0]
        ));
    }

    #[test]
    fn resource_read_rejects_empty_initial_provider_result() {
        let output = Ref::new(LegacyValue::Empty);
        let function = resource_read_function(output.clone(), ResourceDelivery::Live);
        let mut services = RecordingReadServices::new([LegacyValue::Empty]);

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert_eq!(error.kind_name(), "ExternalResourceReadUninitializedValue");
        assert_eq!(*output.borrow(), LegacyValue::Empty);
        assert_eq!(services.live_bindings, 0);
    }

    #[test]
    fn resource_read_live_binding_observes_initialized_cell() {
        let output = Ref::new(LegacyValue::Empty);
        let output_address = output.addr();
        let function = resource_read_function(output.clone(), ResourceDelivery::Live);
        let mut services = RecordingReadServices::new([matrix(2, 1, vec![1.0, 2.0])]);

        function.solve_result_with(&mut services).unwrap();

        assert_eq!(services.live_bindings, 1);
        assert_eq!(services.bound_targets[0].addr(), output_address);
        assert!(matches!(
            &*services.bound_targets[0].borrow(),
            LegacyValue::MatrixF64(Matrix::DMatrix(value))
                if value.borrow().shape() == (2, 1)
        ));
    }

    #[test]
    fn resource_read_compile_records_kind_without_const_load() {
        let observed = matrix(4, 1, vec![1.0, 2.0, 3.0, 4.0]);
        let expected_kind = observed.kind();
        let (compiled, _bytes, destination) =
            compile_resource_read(observed, ResourceDelivery::Live);

        assert_eq!(compiled.program.register_count, 1);
        assert_eq!(
            compiled.register_kinds[destination as usize].as_ref(),
            Some(&expected_kind)
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
        assert_eq!(first.register_kinds, second.register_kinds);
        assert_eq!(first.program.requirements, second.program.requirements);
        assert_eq!(first.program.instructions, second.program.instructions);
        assert_eq!(first.program.constants.len(), 0);
        assert_eq!(second.program.constants.len(), 0);
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn resource_write_failure_propagates_without_changing_its_output() {
        let output = Ref::new(LegacyValue::Empty);
        let function = ExternalResourceWriteFunction {
            request: resource_request(ResourceIntent::Assign),
            input: LegacyValue::F64(Ref::new(43.0)),
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
        assert_eq!(*output.borrow(), LegacyValue::Empty);
    }
}
