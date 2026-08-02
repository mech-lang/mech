use mech_core::{ExecutionHostFunctionRequest, InitialSolvePolicy, Ref, Value};
use mech_engine::{ExternalHostCallFunction, MechProgram, MechProgramConfig};

#[cfg(feature = "compiler")]
use crate::runtime::test_support::capabilities::grant_host_call;
#[cfg(feature = "compiler")]
use crate::runtime::test_support::providers::test_runtime_builder;
#[cfg(feature = "compiler")]
use crate::{CapabilityId, PlannedPureHostFunction, RuntimeCallContext, RuntimeValueSnapshot};
#[cfg(feature = "compiler")]
use mech_core::{BytecodeInstruction, ParsedProgram};
#[cfg(feature = "compiler")]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn external_host_function_output_round_trips_through_program_checkpoint() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let plan = program.interpreter().plan();
    let value = Ref::new(Value::Empty);
    let value_address = value.addr();
    plan.add_function(Box::new(ExternalHostCallFunction {
        request: ExecutionHostFunctionRequest {
            name: "test/host".to_string(),
        },
        arguments: Vec::new(),
        output: value.clone(),
        initial_solve_policy: InitialSolvePolicy::Solve,
    }));
    let checkpoint = program.checkpoint().unwrap();
    let replacement = Ref::new(Value::Index(Ref::new(99)));
    *value.borrow_mut() = Value::MutableReference(replacement);

    program.restore(checkpoint).unwrap();

    assert_eq!(value.addr(), value_address);
    assert_eq!(*value.borrow(), Value::Empty);
    assert!(program.checkpoint().is_ok());
}

#[cfg(feature = "compiler")]
#[test]
fn source_host_function_compiles_and_reconstructed_bytecode_invokes_once() {
    let plans = Arc::new(AtomicUsize::new(0));
    let invocations = Arc::new(AtomicUsize::new(0));
    let plan_count = Arc::clone(&plans);
    let invocation_count = Arc::clone(&invocations);
    let mut runtime = test_runtime_builder()
        .host_function(PlannedPureHostFunction::new(
            "test/bytecode-host",
            move |_context: &RuntimeCallContext, _arguments: &[RuntimeValueSnapshot]| {
                plan_count.fetch_add(1, Ordering::SeqCst);
                RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(1.0)))
            },
            move |_context: &RuntimeCallContext, _arguments: Vec<RuntimeValueSnapshot>| {
                invocation_count.fetch_add(1, Ordering::SeqCst);
                RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(2.0)))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(820), "test/bytecode-host");

    runtime
        .run_string("bytecode-host-result := test/bytecode-host()")
        .unwrap();
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime
            .program
            .root_symbol_value("bytecode-host-result")
            .unwrap(),
        Value::F64(Ref::new(2.0)),
    );

    let bytecode = runtime.compile_program_bytecode().unwrap();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    assert!(
        parsed
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BytecodeInstruction::HostCall { .. }))
    );

    let mut context = runtime.runtime_context().unwrap();
    let output = runtime
        .install_bytecode_with_context(&mut context, &bytecode)
        .unwrap();
    assert_eq!(output.to_value(), Value::F64(Ref::new(2.0)));
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    assert_eq!(invocations.load(Ordering::SeqCst), 2);
}

#[cfg(feature = "compiler")]
#[test]
fn one_shot_bytecode_host_call_uses_runtime_services() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let invocation_count = Arc::clone(&invocations);
    let mut runtime = test_runtime_builder()
        .host_function(PlannedPureHostFunction::new(
            "test/one-shot-host",
            |_context: &RuntimeCallContext, _arguments: &[RuntimeValueSnapshot]| {
                RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(1.0)))
            },
            move |_context: &RuntimeCallContext, _arguments: Vec<RuntimeValueSnapshot>| {
                invocation_count.fetch_add(1, Ordering::SeqCst);
                RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(2.0)))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(821), "test/one-shot-host");
    runtime
        .run_string("one-shot-result := test/one-shot-host()")
        .unwrap();
    let bytecode = runtime.compile_program_bytecode().unwrap();

    let mut context = runtime.runtime_context().unwrap();
    let output = runtime
        .evaluate_bytecode_once_with_context(&mut context, &bytecode)
        .unwrap();

    assert_eq!(output.to_value(), Value::F64(Ref::new(2.0)));
    assert_eq!(invocations.load(Ordering::SeqCst), 2);
}
