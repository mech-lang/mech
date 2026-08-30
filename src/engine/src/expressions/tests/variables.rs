use crate::{
    ExecutionHostFunctionRequest, ExecutionResourceRequest, FunctionValueRepresentation,
    GenericError, Interpreter, MResult, MechError, MechExecutionServices, ReactiveDependencyKind,
    Ref, ResourceDelivery, ResourceIntent, Value, ValueCell, hash_str,
};
use nalgebra::DVector;

struct RecordingContextReadServices {
    result: ValueCell,
    fail_read: bool,
    reads: Vec<ExecutionResourceRequest>,
    live_bindings: Vec<(u64, ExecutionResourceRequest, ValueCell)>,
    host_calls: Vec<ExecutionHostFunctionRequest>,
    writes: Vec<(ExecutionResourceRequest, Value)>,
}

impl RecordingContextReadServices {
    fn returning(result: ValueCell) -> Self {
        Self {
            result,
            fail_read: false,
            reads: Vec::new(),
            live_bindings: Vec::new(),
            host_calls: Vec::new(),
            writes: Vec::new(),
        }
    }

    fn failing() -> Self {
        let mut services = Self {
            result: ValueCell::unit(),
            fail_read: false,
            reads: Vec::new(),
            live_bindings: Vec::new(),
            host_calls: Vec::new(),
            writes: Vec::new(),
        };
        services.fail_read = true;
        services
    }
}

impl MechExecutionServices for RecordingContextReadServices {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[Value],
    ) -> MResult<Value> {
        self.host_calls.push(request.clone());
        ValueCell::unit().snapshot()
    }

    fn plan_resource_read_output(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
        self.result.snapshot()
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<Value> {
        self.reads.push(request.clone());
        if self.fail_read {
            return Err(MechError::new(
                GenericError {
                    msg: "deliberate context read failure".to_string(),
                },
                None,
            ));
        }
        self.result.snapshot()
    }

    fn write_resource(&mut self, request: &ExecutionResourceRequest, value: &Value) -> MResult<()> {
        self.writes.push((request.clone(), value.clone()));
        Ok(())
    }

    fn bind_live_resource(
        &mut self,
        interpreter_id: u64,
        request: &ExecutionResourceRequest,
        target: ValueCell,
    ) -> MResult<()> {
        self.live_bindings
            .push((interpreter_id, request.clone(), target));
        Ok(())
    }
}

fn context_read_interpreter() -> Interpreter {
    Interpreter::with_function_catalog(0, 10_000, crate::test_support::catalog::function_catalog())
}

fn interpret_with_context_services(
    source: &str,
    services: &mut RecordingContextReadServices,
) -> (Interpreter, MResult<Option<ValueCell>>) {
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = context_read_interpreter();
    let result = interpreter.interpret_with_services(&tree, services);
    (interpreter, result)
}

fn cell_f64(cell: &ValueCell) -> f64 {
    let snapshot = cell.snapshot().unwrap();
    match snapshot.data() {
        mech_core::ValueData::F64(value) => value.to_f64(),
        other => panic!("expected f64, got {other:?}"),
    }
}

fn symbol_value(interpreter: &Interpreter, name: &str) -> ValueCell {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
}

fn external_read_node_count(interpreter: &Interpreter) -> usize {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    plan.nodes
        .iter()
        .filter(|node| {
            node.function
                .to_string()
                .starts_with("ExternalResourceReadFunction::")
        })
        .count()
}

#[test]
fn variable_kind_cast_is_indexed() {
    let tree = mech_syntax::parser::parse("value<u8> := 1; value<f64>").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(cell_f64(&output), 1.0);
    let output_cell = output.reactive_cell_id();
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let (node_id, node) = (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            (node.outputs.contains(&output_cell) && !node.inputs.is_empty())
                .then_some((node_id, node))
        })
        .expect("converted variable read should be registered in the plan");
    assert!(
        node.inputs
            .iter()
            .all(|dependency| dependency.kind == ReactiveDependencyKind::Reactive)
    );
    assert!(node.outputs.contains(&output_cell));
    assert!(
        !node
            .inputs
            .iter()
            .any(|dependency| dependency.cell == output_cell)
    );
    for dependency in &node.inputs {
        assert!(
            plan.reactive_consumers_for(dependency.cell)
                .contains(&node_id)
        );
        assert!(plan.sampled_consumers_for(dependency.cell).is_empty());
    }
}

#[test]
fn general_context_read_uses_the_external_live_boundary() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (interpreter, output) = interpret_with_context_services(
        "@input := test://provider/root\nvalue := @input/item",
        &mut services,
    );
    let output = output.unwrap().unwrap();
    assert_eq!(cell_f64(&output), 42.0);
    assert_eq!(services.reads.len(), 1);
    assert_eq!(services.live_bindings.len(), 1);
    assert!(services.host_calls.is_empty());
    assert!(services.writes.is_empty());
    assert_eq!(external_read_node_count(&interpreter), 1);
    assert_eq!(
        services.reads[0],
        ExecutionResourceRequest {
            base_uri: "test://provider/root".to_string(),
            path: "item".to_string(),
            context_name: "root".to_string(),
            operation: "read".to_string(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }
    );
}

#[test]
fn context_read_does_not_require_declared_capability() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (_interpreter, output) = interpret_with_context_services(
        "@browser := browser://dom/\nvalue := @browser/body/content/input/_value",
        &mut services,
    );
    assert_eq!(cell_f64(&output.unwrap().unwrap()), 42.0);
    assert_eq!(services.reads.len(), 1);
    assert_eq!(services.live_bindings.len(), 1);
    assert_eq!(services.reads[0].path, "body/content/input/_value");
}

#[test]
fn frozen_ekf_context_read_has_exact_request() {
    let frame = ValueCell::from_exact_matrix_ref(
        Ref::new(DVector::from_vec(vec![1.0, 2.0, 3.0, 4.0])),
        4,
        1,
    )
    .unwrap();
    let mut services = RecordingContextReadServices::returning(frame);
    let (_interpreter, output) = interpret_with_context_services(
        "@trace := gate-d://ekf/frame{:read(sample)}\nframe := @trace/sample",
        &mut services,
    );
    let output = output.unwrap().unwrap();
    assert_eq!(
        output
            .snapshot()
            .unwrap()
            .matrix_view()
            .unwrap()
            .elements()
            .len(),
        4
    );
    assert_eq!(
        services.reads,
        vec![ExecutionResourceRequest {
            base_uri: "gate-d://ekf/frame".to_string(),
            path: "sample".to_string(),
            context_name: "frame".to_string(),
            operation: "read".to_string(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }]
    );
    assert_eq!(services.live_bindings.len(), 1);
}

#[test]
fn repeated_context_read_reuses_one_live_binding() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (interpreter, output) = interpret_with_context_services(
        "@input := test://provider/root\nfirst := @input/item\nsecond := @input/item",
        &mut services,
    );
    output.unwrap();
    assert_eq!(services.reads.len(), 1);
    assert_eq!(services.live_bindings.len(), 1);
    assert_eq!(external_read_node_count(&interpreter), 1);
    assert_eq!(
        symbol_value(&interpreter, "first").reactive_cell_id(),
        symbol_value(&interpreter, "second").reactive_cell_id(),
    );
    let addressed = interpreter
        .symbols()
        .borrow()
        .get(hash_str("@input/item"))
        .expect("successful addressed read must cache its output cell");
    assert!(addressed.same_cell(&services.live_bindings[0].2));
    assert_eq!(
        addressed.reactive_cell_id(),
        symbol_value(&interpreter, "first").reactive_cell_id(),
    );
}

#[test]
fn context_alias_read_uses_the_resolved_binding() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (_interpreter, output) = interpret_with_context_services(
        "@root := test://provider/base\n@alias := @root\nvalue := @alias/item",
        &mut services,
    );
    output.unwrap();
    assert_eq!(services.reads[0].base_uri, "test://provider/base");
    assert_eq!(services.reads[0].path, "item");
    assert_eq!(services.reads[0].context_name, "base");
}

#[test]
fn missing_context_is_not_an_undefined_variable() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (_interpreter, error) =
        interpret_with_context_services("value := @missing/item", &mut services);
    let error = error.unwrap_err();
    assert_eq!(error.kind_name(), "UndefinedContext");
    assert_eq!(
        error
            .kind_as::<crate::UndefinedContextError>()
            .unwrap()
            .context,
        "missing"
    );
    assert!(services.reads.is_empty());
    assert!(services.live_bindings.is_empty());

    let mut ordinary_services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (_interpreter, error) =
        interpret_with_context_services("value := missing", &mut ordinary_services);
    assert_eq!(error.unwrap_err().kind_name(), "UndefinedVariable");
    assert!(ordinary_services.reads.is_empty());
}

#[test]
fn failed_context_read_does_not_cache_or_register() {
    let mut services = RecordingContextReadServices::failing();
    let (interpreter, error) = interpret_with_context_services(
        "@input := test://provider/root\nvalue := @input/item",
        &mut services,
    );
    assert_eq!(error.unwrap_err().kind_name(), "GenericError");
    assert_eq!(services.reads.len(), 1);
    assert!(services.live_bindings.is_empty());
    assert!(
        interpreter
            .symbols()
            .borrow()
            .get(hash_str("@input/item"))
            .is_none()
    );
    assert_eq!(external_read_node_count(&interpreter), 0);
}

#[test]
fn context_read_kind_annotation_uses_the_cached_cell() {
    let mut services =
        RecordingContextReadServices::returning(ValueCell::from_exact(42.0).unwrap());
    let (interpreter, output) = interpret_with_context_services(
        "@input := test://provider/root\ntyped := @input/item<f64>\nraw := @input/item",
        &mut services,
    );
    output.unwrap();
    assert_eq!(services.reads.len(), 1);
    assert_eq!(services.live_bindings.len(), 1);
    assert_eq!(
        symbol_value(&interpreter, "typed").representation(),
        FunctionValueRepresentation::F64
    );
    assert!(
        interpreter
            .symbols()
            .borrow()
            .get(hash_str("@input/item"))
            .is_some()
    );
}
