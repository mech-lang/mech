use crate::{
    ExecutionHostFunctionRequest, ExecutionResourceRequest, GenericError, Interpreter, MResult,
    MechError, MechExecutionServices, ResourceDelivery, ResourceIntent, Value, ValueCell,
};

#[derive(Default)]
struct RecordingContextServices {
    writes: Vec<(ExecutionResourceRequest, Value)>,
}

impl MechExecutionServices for RecordingContextServices {
    fn invoke_host_function(
        &mut self,
        _request: &ExecutionHostFunctionRequest,
        _arguments: &[Value],
    ) -> MResult<Value> {
        Err(MechError::new(
            GenericError {
                msg: "unexpected host call".to_string(),
            },
            None,
        ))
    }

    fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
        Err(MechError::new(
            GenericError {
                msg: "unexpected resource read".to_string(),
            },
            None,
        ))
    }

    fn write_resource(&mut self, request: &ExecutionResourceRequest, value: &Value) -> MResult<()> {
        self.writes.push((request.clone(), value.clone()));
        Ok(())
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: ValueCell,
    ) -> MResult<()> {
        Err(MechError::new(
            GenericError {
                msg: "unexpected live binding".to_string(),
            },
            None,
        ))
    }
}

fn run(source: &str, services: &mut RecordingContextServices) -> MResult<Interpreter> {
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    interpreter.interpret_with_services(&tree, services)?;
    Ok(interpreter)
}

#[test]
fn unknown_context_alias_is_undefined_context() {
    let mut services = RecordingContextServices::default();
    let error = match run("@alias := @missing", &mut services) {
        Ok(_) => panic!("unknown context alias should fail"),
        Err(error) => error,
    };
    assert_eq!(error.kind_name(), "UndefinedContext");
    assert_eq!(
        error
            .kind_as::<crate::UndefinedContextError>()
            .unwrap()
            .context,
        "missing"
    );
    assert!(services.writes.is_empty());
}

#[test]
fn unknown_context_send_is_undefined_context() {
    let mut services = RecordingContextServices::default();
    let error = match run("@missing/item <- 1.0", &mut services) {
        Ok(_) => panic!("unknown context send should fail"),
        Err(error) => error,
    };
    assert_eq!(error.kind_name(), "UndefinedContext");
    assert_eq!(
        error
            .kind_as::<crate::UndefinedContextError>()
            .unwrap()
            .context,
        "missing"
    );
    assert!(services.writes.is_empty());
}

#[test]
fn context_send_request_bytes_are_unchanged() {
    let mut services = RecordingContextServices::default();
    run(
        "@out := test://provider/root\n@out/item <- 7.0",
        &mut services,
    )
    .unwrap();
    assert_eq!(services.writes.len(), 1);
    assert_eq!(
        services.writes[0].0,
        ExecutionResourceRequest {
            base_uri: "test://provider/root".to_string(),
            path: "item".to_string(),
            context_name: "root".to_string(),
            operation: "write".to_string(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
        }
    );
    assert!(
        matches!(services.writes[0].1.data(), mech_core::ValueData::F64(value) if value.to_f64() == 7.0)
    );
}
