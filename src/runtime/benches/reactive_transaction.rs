use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use mech_core::{GenericError, MResult, MechError, Ref, Value, hash_str};
use mech_engine::Interpreter;
use mech_engine::{MechProgram, MechProgramConfig, ProgramInputId, ProgramInputUpdate};
use mech_runtime::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
    DeterministicHostFunction, HostArgumentValue, MechRuntime, ObjectRecord,
    PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction, PreparedRuntimeEffect,
    ResourcePathCapability, RuntimeAfterCommitEffect, RuntimeContext, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimePreparedHostCall, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeValueSnapshot, SequentialIdGenerator,
};
use std::hint::black_box;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

mod support;

const INPUT_BASE_URI: &str = "bench://clock/ticks";
const INPUT_PATH: &str = "value";

struct StepFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
}

struct HostInputFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
    source: RuntimeHostInputSource,
}

struct TwoInterpreterFixture {
    program: MechProgram,
    updates: Vec<ProgramInputUpdate>,
}

#[derive(Debug)]
struct BenchInputProvider;

impl RuntimeResourceProvider for BenchInputProvider {
    fn scheme(&self) -> &str {
        "bench"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![INPUT_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri == INPUT_BASE_URI && request.path == INPUT_PATH {
            return Ok(Value::F64(Ref::new(1.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "missing benchmark input {} / {}",
                    request.base_uri, request.path,
                ),
            },
            None,
        ))
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri == INPUT_BASE_URI && request.path == INPUT_PATH {
            return Ok(Value::F64(Ref::new(0.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "missing benchmark planning input {} / {}",
                    request.base_uri, request.path,
                ),
            },
            None,
        ))
    }
}

#[derive(Debug)]
struct BenchAfterCommitEffect;

impl RuntimeAfterCommitEffect for BenchAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "reactive-transaction-benchmark".to_string(),
            },
            "deliver",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        black_box(());
        Ok(())
    }
}

fn step_fixture(count: usize, _fail_tail: bool) -> StepFixture {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let source = (0..count)
        .map(|index| format!("step-{index} := {index}.0 + 1.0"))
        .collect::<Vec<_>>()
        .join("\n");
    runtime.run_string(&source).unwrap();
    let context = runtime.runtime_context().unwrap();
    StepFixture { runtime, context }
}

fn grant_input_read(runtime: &mut MechRuntime) {
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        subject,
        INPUT_BASE_URI,
        ["read"],
        INPUT_PATH,
    )
    .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap();
}

fn grant_host_call(runtime: &mut MechRuntime, capability: u64, name: &str) {
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(capability.into()),
            &BasicSubject::new(subject),
            &BasicResource::new(format!("host:{name}")),
            [BasicOperation::new("call")],
        )))
        .unwrap();
}

fn host_input_fixture(source_tail: &str) -> HostInputFixture {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .resource_provider(Box::new(BenchInputProvider))
        .build()
        .unwrap();
    grant_input_read(&mut runtime);
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            &format!("@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n{source_tail}",),
        )
        .unwrap();
    HostInputFixture {
        runtime,
        context,
        source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
    }
}

fn apply_host_input(
    fixture: &mut HostInputFixture,
) -> MResult<mech_runtime::RuntimeHostInputOutcome> {
    fixture.runtime.apply_host_input_with_context(
        &mut fixture.context,
        RuntimeHostInput::single(fixture.source.clone(), RuntimeHostInputValue::F64(2.0)),
    )
}

fn chain_source(length: usize) -> String {
    let mut source = String::new();
    let mut previous = format!("@pulse/{INPUT_PATH}");
    for index in 0..length {
        let name = format!("chain-{index}");
        source.push_str(&format!("{name} := {previous} + 1.0\n"));
        previous = name;
    }
    source
}

fn sparse_source(length: usize) -> String {
    let mut source = String::new();
    for index in 0..length {
        source.push_str(&format!(
            "sparse-{index} := @pulse/{INPUT_PATH} + {index}.0\n",
        ));
    }
    source
}

fn register_source(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!("~register-{index} := 0.0\n"));
    }
    for index in 0..count {
        source.push_str(&format!(
            "register-{index} = @pulse/{INPUT_PATH} + {index}.0\n",
        ));
    }
    source
}

fn copied_host_f64(arguments: &[impl HostArgumentValue]) -> Value {
    let value = match arguments
        .first()
        .map(HostArgumentValue::host_argument_value)
    {
        Some(Value::F64(value)) => *value.borrow(),
        Some(Value::MutableReference(value)) => match &*value.borrow() {
            Value::F64(value) => *value.borrow(),
            other => panic!("expected f64 benchmark host input, got {other:?}",),
        },
        other => panic!("expected f64 benchmark host argument, got {other:?}",),
    };
    Value::F64(Ref::new(value))
}

fn copied_host_snapshot(arguments: &[impl HostArgumentValue]) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&copied_host_f64(arguments)).expect("acyclic fixture")
}

fn failing_host_input_fixture() -> HostInputFixture {
    let fail = Arc::new(AtomicBool::new(false));
    let fail_for_host = fail.clone();
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .resource_provider(Box::new(BenchInputProvider))
        .host_function(DeterministicHostFunction::new(
            "bench/fail-tail",
            |_context, arguments| Ok(copied_host_f64(&arguments)),
            move |_context, arguments| {
                if fail_for_host.load(Ordering::SeqCst) {
                    return Err(MechError::new(
                        GenericError {
                            msg: "benchmark reactive tail failure".to_string(),
                        },
                        None,
                    ));
                }
                Ok(copied_host_f64(&arguments))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_input_read(&mut runtime);
    grant_host_call(&mut runtime, 9_001, "bench/fail-tail");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            &format!(
                "@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n\
         tail := bench/fail-tail(@pulse/{INPUT_PATH})",
            ),
        )
        .unwrap();
    fail.store(true, Ordering::SeqCst);
    HostInputFixture {
        runtime,
        context,
        source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
    }
}

fn capability_host_input_fixture() -> HostInputFixture {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .resource_provider(Box::new(BenchInputProvider))
        .host_function(DeterministicHostFunction::new(
            "bench/capability",
            |_context, arguments| Ok(copied_host_f64(&arguments)),
            |_context, arguments| Ok(copied_host_f64(&arguments)),
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_input_read(&mut runtime);
    grant_host_call(&mut runtime, 9_002, "bench/capability");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            &format!(
                "@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n\
         checked := bench/capability(@pulse/{INPUT_PATH})",
            ),
        )
        .unwrap();
    HostInputFixture {
        runtime,
        context,
        source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
    }
}

fn effect_host_input_fixture() -> HostInputFixture {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .resource_provider(Box::new(BenchInputProvider))
        .host_function(PlannedStagedHostFunction::new(
            "bench/after-commit",
            |_context, arguments| Ok(copied_host_snapshot(arguments)),
            |_context, arguments| {
                Ok(RuntimePreparedHostCall {
                    value: copied_host_snapshot(&arguments),
                    effect: PreparedRuntimeEffect::AfterCommit(Box::new(BenchAfterCommitEffect)),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_input_read(&mut runtime);
    grant_host_call(&mut runtime, 9_003, "bench/after-commit");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            &format!(
                "@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n\
         delivered := bench/after-commit(@pulse/{INPUT_PATH})",
            ),
        )
        .unwrap();
    HostInputFixture {
        runtime,
        context,
        source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
    }
}

fn object_host_input_fixture() -> HostInputFixture {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .resource_provider(Box::new(BenchInputProvider))
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "bench/object",
            |_context, arguments| Ok(copied_host_snapshot(arguments)),
            |services, _context, arguments| {
                let id = services.allocate_object_id()?;
                services.put_object(ObjectRecord::text(id, "benchmark", "reactive turn"))?;
                Ok(copied_host_snapshot(&arguments))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_input_read(&mut runtime);
    grant_host_call(&mut runtime, 9_004, "bench/object");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            &format!(
                "@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n\
         stored := bench/object(@pulse/{INPUT_PATH})",
            ),
        )
        .unwrap();
    HostInputFixture {
        runtime,
        context,
        source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
    }
}

fn two_interpreter_fixture() -> TwoInterpreterFixture {
    let catalog = support::source_catalog();
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), Arc::clone(&catalog));
    let mut updates = Vec::new();
    for interpreter_id in [101, 202] {
        let mut interpreter =
            Interpreter::with_function_catalog(interpreter_id, 10_000, Arc::clone(&catalog));
        interpreter
            .interpret(&mech_syntax::parser::parse("input := 1.0\noutput := input + 1.0").unwrap())
            .unwrap();
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(interpreter_id, Ref::new(Box::new(interpreter)));
        updates.push(ProgramInputUpdate {
            input: ProgramInputId {
                interpreter_id,
                symbol_id: hash_str("input"),
            },
            value: Value::F64(Ref::new(2.0)),
        });
    }
    TwoInterpreterFixture { program, updates }
}

fn reactive_transaction_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_reactive_transaction");
    group.sample_size(10);

    group.bench_function("implicit_selected_step", |b| {
        b.iter_batched(
            || step_fixture(1, false),
            |mut fixture| {
                fixture
                    .runtime
                    .step_with_context(&mut fixture.context, 1)
                    .unwrap();
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("implicit_whole_plan_step", |b| {
        b.iter_batched(
            || step_fixture(10, false),
            |mut fixture| {
                fixture
                    .runtime
                    .step_with_context(&mut fixture.context, 0)
                    .unwrap();
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    for (name, setup) in [(
        "one_scalar_host_input_turn",
        host_input_fixture as fn(&str) -> HostInputFixture,
    )] {
        group.bench_function(name, |b| {
            b.iter_batched(
                || setup("output := @pulse/value + 1.0"),
                |mut fixture| {
                    black_box(apply_host_input(&mut fixture).unwrap());
                    black_box(fixture)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.bench_function("100_node_reactive_chain", |b| {
        b.iter_batched(
            || host_input_fixture(&chain_source(100)),
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("1000_node_sparse_graph", |b| {
        b.iter_batched(
            || host_input_fixture(&sparse_source(1_000)),
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("one_register_turn", |b| {
        b.iter_batched(
            || host_input_fixture(&register_source(1)),
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("100_register_turn", |b| {
        b.iter_batched(
            || host_input_fixture(&register_source(100)),
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("two_affected_interpreters", |b| {
        b.iter_batched(
            two_interpreter_fixture,
            |mut fixture| {
                let outcome = fixture
                    .program
                    .update_inputs_and_advance_turn(&fixture.updates)
                    .unwrap();
                assert!(
                    outcome.interpreter_turns.len() == 2,
                    "benchmark fixture must affect exactly two interpreters",
                );
                black_box(outcome);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("failed_tail_with_complete_rollback", |b| {
        b.iter_batched(
            failing_host_input_fixture,
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap_err());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_first_turn_with_outer_checkpoint", |b| {
        b.iter_batched(
            || {
                let mut fixture = host_input_fixture("output := @pulse/value + 1.0");
                fixture
                    .runtime
                    .begin_transaction(&mut fixture.context)
                    .unwrap();
                fixture
            },
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_later_turn_without_outer_checkpoint", |b| {
        b.iter_batched(
            || {
                let mut fixture = host_input_fixture("output := @pulse/value + 1.0");
                fixture
                    .runtime
                    .begin_transaction(&mut fixture.context)
                    .unwrap();
                apply_host_input(&mut fixture).unwrap();
                fixture
            },
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    for (name, setup) in [
        (
            "turn_using_one_live_capability",
            capability_host_input_fixture as fn() -> HostInputFixture,
        ),
        (
            "turn_with_one_after_commit_effect",
            effect_host_input_fixture as fn() -> HostInputFixture,
        ),
        (
            "turn_with_one_runtime_managed_object_update",
            object_host_input_fixture as fn() -> HostInputFixture,
        ),
    ] {
        group.bench_function(name, |b| {
            b.iter_batched(
                setup,
                |mut fixture| {
                    black_box(apply_host_input(&mut fixture).unwrap());
                    black_box(fixture)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.bench_function("unbound_host_input_packet", |b| {
        b.iter_batched(
            || host_input_fixture("output := @pulse/value + 1.0"),
            |mut fixture| {
                let outcome = fixture
                    .runtime
                    .apply_host_input_with_context(
                        &mut fixture.context,
                        RuntimeHostInput::single(
                            RuntimeHostInputSource::new(INPUT_BASE_URI, "unbound").unwrap(),
                            RuntimeHostInputValue::F64(2.0),
                        ),
                    )
                    .unwrap();
                black_box(outcome);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("valid_turn_with_one_integrity_constraint", |b| {
        b.iter_batched(
            || host_input_fixture("output := @pulse/value + 1.0\noutput-safe! := output <= 100.0"),
            |mut fixture| {
                black_box(apply_host_input(&mut fixture).unwrap());
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, reactive_transaction_benchmarks);
criterion_main!(benches);
