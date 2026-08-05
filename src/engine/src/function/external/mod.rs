mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, CompileConst, MResult, Register, ValRef, Value};

#[cfg(feature = "compiler")]
pub(super) fn compile_external_output(
    output: &ValRef,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let value = output.borrow();
    let pointer = external_value_pointer(&value, output.addr());
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
    if initialize {
        let constant = compile_external_constant(&value, context)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}

#[cfg(feature = "compiler")]
pub(super) fn compile_external_value(
    value: &Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let pointer = external_value_pointer(value, std::ptr::from_ref(value).addr());
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
    if initialize {
        let constant = compile_external_constant(value, context)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}

#[cfg(feature = "compiler")]
fn external_value_pointer(value: &Value, fallback: usize) -> usize {
    match value {
        // Mutable references are transparent to the bytecode value model. Use
        // the referenced value's stable pointer so a symbol that wraps another
        // node's output reuses that producer's register instead of compiling a
        // detached constant with the same current value.
        Value::MutableReference(reference) => {
            external_value_pointer(&reference.borrow(), reference.addr())
        }
        Value::Typed(value, _) => external_value_pointer(value, fallback),
        Value::Id(_) | Value::Kind(_) | Value::IndexAll | Value::EmptyKind(_) | Value::Empty => {
            fallback
        }
        _ => value.addr(),
    }
}

#[cfg(feature = "compiler")]
fn compile_external_constant(
    value: &Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<u32> {
    match value {
        Value::MutableReference(reference) => {
            compile_external_constant(&reference.borrow(), context)
        }
        _ => value.compile_const(context),
    }
}

#[cfg(all(test, feature = "f64"))]
mod tests {
    use super::*;
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
