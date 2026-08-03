#![cfg(feature = "source")]

use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

use mech_core::{
    ApplicationRequirement, BytecodeInstruction, FunctionArgs, FunctionCatalog,
    FunctionCatalogBuilder, MResult, MechError, MechErrorKind, MechFunction, MechFunctionFactory,
    MechFunctionImpl, ParsedProgram, Ref, ResourceDelivery, ResourceIntent, Value, hash_str,
};
#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use mech_native_live_host_fixture::{
    TEST_LIVE_BASE_URI, TEST_LIVE_CONTEXT, TEST_LIVE_INSTANCE, TEST_LIVE_PATH, TEST_LIVE_PROVIDER,
    TestLiveDriverHandle, TestLiveHostFactory, empty_settings,
};
use mech_runtime::{
    HostInstanceConfig, RunResourceGrantConfig, RuntimeBuilder, RuntimeHostInputOutcome,
    RuntimeValueSnapshot,
};

static PROGRAM: &[u8] =
    include_bytes!("../../../tests/architecture/bytecode-v1/phase1/synthetic-live-read.mecb");

static ADD_SHOULD_FAIL: AtomicBool = AtomicBool::new(false);
static ADD_SOLVE_COUNT: AtomicUsize = AtomicUsize::new(0);
static ADD_FAILED_OUTPUT_BITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct AddObservation {
    input: Ref<f64>,
    output: Ref<f64>,
}

thread_local! {
    static ADD_OBSERVATION: RefCell<Option<AddObservation>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ObservedAddState {
    input_identity: usize,
    output_identity: usize,
    input: f64,
    output: f64,
}

#[derive(Debug)]
struct ControlledAdd {
    lhs: Ref<f64>,
    rhs: Ref<f64>,
    output: Ref<f64>,
}

impl MechFunctionImpl for ControlledAdd {
    fn solve(&self) {
        *self.output.borrow_mut() = *self.lhs.borrow() + *self.rhs.borrow();
    }

    fn solve_result(&self) -> MResult<()> {
        ADD_SOLVE_COUNT.fetch_add(1, Ordering::SeqCst);
        self.solve();
        if ADD_SHOULD_FAIL.load(Ordering::SeqCst) {
            ADD_FAILED_OUTPUT_BITS.store(self.output.borrow().to_bits(), Ordering::SeqCst);
            return Err(MechError::new(DeliberateAddFailure, None));
        }
        Ok(())
    }

    fn out(&self) -> Value {
        Value::F64(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "ControlledAddSS<f64>".to_owned()
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ControlledAdd {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn controlled_add_factory(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    let FunctionArgs::Binary(output, lhs, rhs) = arguments else {
        return Err(MechError::new(ControlledAddInvalidArguments, None));
    };
    let (Value::F64(output), Value::F64(lhs), Value::F64(rhs)) = (output, lhs, rhs) else {
        return Err(MechError::new(ControlledAddInvalidArguments, None));
    };
    ADD_OBSERVATION.with(|observation| {
        *observation.borrow_mut() = Some(AddObservation {
            input: lhs.clone(),
            output: output.clone(),
        });
    });
    Ok(Box::new(ControlledAdd { lhs, rhs, output }))
}

#[derive(Clone, Debug)]
struct DeliberateAddFailure;

impl MechErrorKind for DeliberateAddFailure {
    fn name(&self) -> &str {
        "DeliberateAddFailure"
    }

    fn message(&self) -> String {
        "deliberate downstream add failure".to_owned()
    }
}

#[derive(Clone, Debug)]
struct ControlledAddInvalidArguments;

impl MechErrorKind for ControlledAddInvalidArguments {
    fn name(&self) -> &str {
        "ControlledAddInvalidArguments"
    }

    fn message(&self) -> String {
        "controlled AddSS<f64> requires three F64 registers".to_owned()
    }
}

fn function_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory("AddSS<f64>", controlled_add_factory)
        .unwrap();
    builder
        .insert_runtime_factory(
            "VariableDefineF64",
            <mech_engine::intrinsics::define::VariableDefineF64 as MechFunctionFactory>::new,
        )
        .unwrap();
    Arc::new(builder.build().unwrap())
}

fn configured_builder(
    planning: bool,
    catalog: Arc<FunctionCatalog>,
) -> (RuntimeBuilder, TestLiveDriverHandle) {
    let (factory, driver) = TestLiveHostFactory::new().unwrap();
    let mut builder = RuntimeBuilder::new().function_catalog(catalog);
    if planning {
        builder = builder.planning();
    }
    builder = builder
        .host_factory(Box::new(factory))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: TEST_LIVE_INSTANCE.to_owned(),
            provider: TEST_LIVE_PROVIDER.to_owned(),
            settings: empty_settings(),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: format!("{TEST_LIVE_INSTANCE}/{TEST_LIVE_CONTEXT}"),
            operations: vec!["read".to_owned()],
            paths: vec![TEST_LIVE_PATH.to_owned()],
        });
    (builder, driver)
}

fn observed_add_state() -> ObservedAddState {
    ADD_OBSERVATION.with(|observation| {
        let observation = observation.borrow();
        let observation = observation
            .as_ref()
            .expect("bytecode installation must construct AddSS<f64>");
        ObservedAddState {
            input_identity: observation.input.addr(),
            output_identity: observation.output.addr(),
            input: *observation.input.borrow(),
            output: *observation.output.borrow(),
        }
    })
}

fn reset_controlled_add() {
    ADD_SHOULD_FAIL.store(false, Ordering::SeqCst);
    ADD_SOLVE_COUNT.store(0, Ordering::SeqCst);
    ADD_FAILED_OUTPUT_BITS.store(0, Ordering::SeqCst);
    ADD_OBSERVATION.with(|observation| *observation.borrow_mut() = None);
}

fn snapshot_f64(snapshot: RuntimeValueSnapshot) -> f64 {
    match snapshot.into_value() {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected F64 snapshot, got {other:?}"),
    }
}

fn only_turn(outcomes: &[RuntimeHostInputOutcome]) -> &mech_engine::ProgramInputTurnOutcome {
    assert_eq!(outcomes.len(), 1);
    outcomes[0]
        .turn
        .as_ref()
        .expect("bound live input must advance one reactive turn")
}

#[test]
fn synthetic_native_live_input_is_planned_driven_and_rolled_back_atomically() {
    let parsed = ParsedProgram::from_bytes(PROGRAM).unwrap();
    assert_eq!(
        parsed
            .instructions
            .iter()
            .filter_map(BytecodeInstruction::runtime_function)
            .collect::<Vec<_>>(),
        vec![
            hash_str("VariableDefineF64"),
            hash_str("AddSS<f64>"),
            hash_str("VariableDefineF64"),
        ],
        "the authoritative source fixture must retain its real compiler-emitted factories",
    );
    assert_eq!(
        parsed
            .instructions
            .iter()
            .filter(|instruction| matches!(
                instruction,
                BytecodeInstruction::ResourceRead { requirement: 0, .. }
            ))
            .count(),
        1,
    );
    assert!(matches!(
        parsed.instructions.last(),
        Some(BytecodeInstruction::Return { .. })
    ));
    let [ApplicationRequirement::Resource(requirement)] = parsed.requirements.as_slice() else {
        panic!("planning must emit exactly one resource requirement");
    };
    assert_eq!(requirement.base_uri, TEST_LIVE_BASE_URI);
    assert_eq!(requirement.path, TEST_LIVE_PATH);
    assert_eq!(requirement.context_name, TEST_LIVE_CONTEXT);
    assert_eq!(requirement.operation, "read");
    assert_eq!(requirement.intent, ResourceIntent::Read);
    assert_eq!(requirement.delivery, ResourceDelivery::Live);

    reset_controlled_add();
    let (planning_builder, planning_driver) = configured_builder(true, function_catalog());
    let mut planning_runtime = planning_builder.build().unwrap();
    assert!(!planning_driver.is_attached());
    assert!(!planning_driver.is_live());
    assert_eq!(planning_driver.attach_count(), 0);
    assert_eq!(planning_driver.start_count(), 0);
    let mut planning_context = planning_runtime.runtime_context().unwrap();
    let planned = planning_runtime
        .install_bytecode_with_context(&mut planning_context, PROGRAM)
        .unwrap();
    assert_eq!(snapshot_f64(planned), 0.0);
    assert_eq!(planning_runtime.live_input_binding_count(), 0);
    assert_eq!(planning_runtime.input_driver_count(), 0);
    assert_eq!(planning_driver.attach_count(), 0);
    assert_eq!(planning_driver.start_count(), 0);
    drop(planning_runtime);

    reset_controlled_add();
    let (runtime_builder, driver) = configured_builder(false, function_catalog());
    let mut runtime = runtime_builder.build().unwrap();
    assert!(driver.is_attached());
    assert!(!driver.is_live());
    assert_eq!(driver.attach_count(), 1);
    assert_eq!(driver.start_count(), 0);

    let mut context = runtime.runtime_context().unwrap();
    let initial = runtime
        .install_bytecode_with_context(&mut context, PROGRAM)
        .unwrap();
    assert_eq!(snapshot_f64(initial), 0.0);
    assert_eq!(runtime.live_input_binding_count(), 1);
    assert_eq!(runtime.driven_live_input_binding_count().unwrap(), 1);

    // Reinstalling the same retained live root replaces its registration
    // instead of accumulating a duplicate binding.
    let duplicate = runtime
        .install_bytecode_with_context(&mut context, PROGRAM)
        .unwrap();
    assert_eq!(snapshot_f64(duplicate), 0.0);
    assert_eq!(runtime.live_input_binding_count(), 1);

    ADD_SOLVE_COUNT.store(0, Ordering::SeqCst);
    let initial_state = observed_add_state();
    assert_eq!(initial_state.input, 0.0);
    assert_eq!(initial_state.output, 0.0);

    runtime.start_input_drivers().unwrap();
    assert!(driver.is_live());
    assert_eq!(driver.start_count(), 1);
    driver.submit(7.0).unwrap();
    assert_eq!(driver.submit_count(), 1);
    let outcomes = runtime.drain_host_inputs(1).unwrap();
    let successful_turn = only_turn(&outcomes);
    assert_eq!(successful_turn.updated_count, 1);
    assert_eq!(successful_turn.interpreter_turns.len(), 1);
    assert_eq!(
        successful_turn.interpreter_turns[0]
            .turn
            .before_commit
            .executed_nodes
            .len(),
        1,
        "the dependent AddSS<f64> node must run exactly once",
    );
    assert_eq!(ADD_SOLVE_COUNT.load(Ordering::SeqCst), 1);
    let stable_dirty_cells = successful_turn.interpreter_turns[0].dirty_cells.clone();
    let committed = observed_add_state();
    assert_eq!(committed.input_identity, initial_state.input_identity);
    assert_eq!(committed.output_identity, initial_state.output_identity);
    assert_eq!(committed.input, 7.0);
    assert_eq!(committed.output, 14.0);
    assert_eq!(runtime.out_string(), "14");

    ADD_SHOULD_FAIL.store(true, Ordering::SeqCst);
    driver.submit(9.0).unwrap();
    let error = runtime.drain_host_inputs(1).unwrap_err();
    assert_eq!(error.kind_name(), "DeliberateAddFailure");
    assert_eq!(driver.submit_count(), 2);
    assert_eq!(ADD_SOLVE_COUNT.load(Ordering::SeqCst), 2);
    assert_eq!(
        f64::from_bits(ADD_FAILED_OUTPUT_BITS.load(Ordering::SeqCst)),
        18.0,
        "the failing node must dirty its output before the turn is rolled back",
    );
    let rolled_back = observed_add_state();
    assert_eq!(rolled_back.input_identity, initial_state.input_identity);
    assert_eq!(rolled_back.output_identity, initial_state.output_identity);
    assert_eq!(rolled_back.input, 7.0);
    assert_eq!(rolled_back.output, 14.0);
    assert_eq!(runtime.out_string(), "14");
    assert_eq!(runtime.live_input_binding_count(), 1);

    ADD_SHOULD_FAIL.store(false, Ordering::SeqCst);
    driver.submit(8.0).unwrap();
    let recovered = runtime.drain_host_inputs(1).unwrap();
    let recovered_turn = only_turn(&recovered);
    assert_eq!(
        recovered_turn.interpreter_turns[0].dirty_cells, stable_dirty_cells,
        "the live target must keep its stable reactive cell identity",
    );
    let recovered_state = observed_add_state();
    assert_eq!(recovered_state.input_identity, initial_state.input_identity);
    assert_eq!(
        recovered_state.output_identity,
        initial_state.output_identity
    );
    assert_eq!(recovered_state.input, 8.0);
    assert_eq!(recovered_state.output, 16.0);

    runtime.shutdown().unwrap();
    assert!(!driver.is_live());
    assert_eq!(driver.stop_count(), 1);
}
