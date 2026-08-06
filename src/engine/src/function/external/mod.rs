mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "compiler")]
use mech_core::{
    BytecodeCompilerContext, MResult, Register, ValRef, Value, compile_value_register,
};

#[cfg(feature = "compiler")]
pub(super) fn compile_external_output(
    output: &ValRef,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let value = output.borrow();
    compile_external_value_with_fallback(&value, output.addr(), context)
}

#[cfg(feature = "compiler")]
pub(super) fn compile_external_value(
    value: &Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_external_value_with_fallback(value, std::ptr::from_ref(value).addr(), context)
}

#[cfg(feature = "compiler")]
fn compile_external_value_with_fallback(
    value: &Value,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_register(value, fallback, context)
}

#[cfg(all(test, feature = "f64"))]
mod tests {
    use super::*;
    use mech_bytecode::CompileCtx;
    use mech_core::{
        ExecutionHostFunctionRequest, ExecutionResourceRequest, GenericError, InitialSolvePolicy,
        MResult, MechError, MechExecutionServices, MechFunctionImpl, Ref, ResourceDelivery,
        ResourceIntent, ValRef, Value,
    };

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
            _target: ValRef,
        ) -> MResult<()> {
            Err(Self::error("live resource bind"))
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

    #[test]
    fn typed_external_values_do_not_share_bare_registers_in_either_order() {
        for typed_first in [false, true] {
            let scalar = Ref::new(7.0);
            let bare = Value::F64(scalar.clone());
            let typed = Value::Typed(
                Box::new(Value::F64(scalar)),
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
        let bare_argument = Value::F64(scalar.clone());
        let typed_output = Ref::new(Value::Typed(
            Box::new(Value::F64(scalar)),
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
        let output = Ref::new(Value::F64(Ref::new(41.0)));
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
        assert!(matches!(&*output.borrow(), Value::F64(value) if *value.borrow() == 41.0));
    }

    #[test]
    fn resource_read_failure_propagates_without_publishing_a_stale_output() {
        let output = Ref::new(Value::F64(Ref::new(42.0)));
        let function = ExternalResourceReadFunction {
            interpreter_id: 7,
            request: resource_request(ResourceIntent::Read),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
        };
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(error.full_chain_message().contains("resource read failure"));
        assert_eq!(services.resource_reads, 1);
        assert!(matches!(&*output.borrow(), Value::F64(value) if *value.borrow() == 42.0));
    }

    #[test]
    fn resource_write_failure_propagates_without_changing_its_output() {
        let output = Ref::new(Value::Empty);
        let function = ExternalResourceWriteFunction {
            request: resource_request(ResourceIntent::Assign),
            input: Value::F64(Ref::new(43.0)),
            output: output.clone(),
            initial_solve_policy: InitialSolvePolicy::Solve,
        };
        let mut services = FailingServices::default();

        let error = function.solve_result_with(&mut services).unwrap_err();

        assert!(
            error
                .full_chain_message()
                .contains("resource write failure")
        );
        assert_eq!(services.resource_writes, 1);
        assert_eq!(*output.borrow(), Value::Empty);
    }
}
