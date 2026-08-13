use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use mech_core::{LegacyValue, MResult, MechSourceCode, MechTuple, Ref};
use mech_runtime::legacy_interpreter::LegacyInterpreterTestExt as _;
use mech_runtime::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId, CapabilityRequest,
    HostCall, InMemoryDocsProvider, InMemorySourceResolver, MechRuntime, ModuleBuildOptions,
    ObjectId, ObjectRecord, PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction,
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeCapabilityOperation,
    RuntimeCompensatableEffect, RuntimeContext, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimePreparedHostCall, RuntimeResourceProvider, RuntimeResourceWriteIntent,
    RuntimeResourceWriteRequest, RuntimeValueSnapshot, SequentialIdGenerator,
};
use std::hint::black_box;
use std::sync::Arc;

mod support;

struct ExplicitFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
}

#[derive(Debug)]
struct BenchmarkAfterCommitEffect {
    sequence: usize,
}

impl RuntimeAfterCommitEffect for BenchmarkAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "benchmark-after-commit".to_string(),
            },
            "deliver",
        )
        .with_resource(format!("bench://after-commit/{}", self.sequence))
    }

    fn deliver(&mut self) -> MResult<()> {
        black_box(self.sequence);
        Ok(())
    }
}

#[derive(Debug)]
struct BenchmarkCompensatableEffect {
    sequence: usize,
}

impl RuntimeCompensatableEffect for BenchmarkCompensatableEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "benchmark-compensatable".to_string(),
            },
            "apply",
        )
        .with_resource(format!("bench://compensatable/{}", self.sequence))
    }

    fn apply(&mut self) -> MResult<()> {
        black_box(self.sequence);
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        black_box(self.sequence);
        Ok(())
    }
}

fn stage_after_commit(fixture: &mut ExplicitFixture, count: usize) {
    for sequence in 0..count {
        fixture
            .runtime
            .stage_runtime_effect_with_context(
                &mut fixture.context,
                PreparedRuntimeEffect::AfterCommit(Box::new(BenchmarkAfterCommitEffect {
                    sequence,
                })),
            )
            .unwrap();
    }
}

fn stage_compensatable(fixture: &mut ExplicitFixture, count: usize) {
    for sequence in 0..count {
        fixture
            .runtime
            .stage_runtime_effect_with_context(
                &mut fixture.context,
                PreparedRuntimeEffect::Compensatable(Box::new(BenchmarkCompensatableEffect {
                    sequence,
                })),
            )
            .unwrap();
    }
}

fn retained_runtime() -> MechRuntime {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    runtime.run_string("bench-anchor := 1").unwrap();
    runtime
}

fn explicit_fixture() -> ExplicitFixture {
    let mut runtime = retained_runtime();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    ExplicitFixture { runtime, context }
}

fn module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("benchmark", "v0.3", "native", &[], &[])
}

fn retained_root_fixture() -> MechRuntime {
    let resolver = InMemorySourceResolver::new()
        .with_string("root.mec", "+> ./dep.mec\nanswer := dep/value\nanswer\n")
        .with_string("dep.mec", "value := 41\n<+ value\n");
    support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .source_resolver(resolver)
        .build()
        .unwrap()
}

fn retained_root_failure_fixture() -> MechRuntime {
    let resolver = InMemorySourceResolver::new()
        .with_string("root.mec", "+> ./dep.mec\n+> ./missing.mec\nanswer := 1\n")
        .with_string("dep.mec", "value := 41\n<+ value\n");
    support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .source_resolver(resolver)
        .build()
        .unwrap()
}

fn explicit_graph_fixture() -> ExplicitFixture {
    let mut runtime = retained_root_fixture();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    ExplicitFixture { runtime, context }
}

fn execution_session_host_fixture() -> MechRuntime {
    let mut runtime = support::source_runtime_builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "bench/execution-session",
            |_context, _arguments| {
                RuntimeValueSnapshot::try_capture(&LegacyValue::Bool(Ref::new(true)))
            },
            |_services, _context, _arguments| {
                RuntimeValueSnapshot::try_capture(&LegacyValue::Bool(Ref::new(true)))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(9_000),
            &BasicSubject::new(subject),
            &BasicResource::new("host:bench/execution-session"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    runtime
}

fn provider_preparation_fixture() -> InMemoryDocsProvider {
    let mut provider = InMemoryDocsProvider::new();
    provider
        .insert("docs://benchmark", "value", LegacyValue::F64(Ref::new(1.0)))
        .unwrap();
    provider
}

fn extension_wrapper_fixture() -> MechRuntime {
    let resolver = InMemorySourceResolver::new().with_string("bench.mec", "answer := 42");
    support::source_runtime_builder()
        .source_resolver(resolver)
        .build()
        .unwrap()
}

fn subsequent_explicit_fixture() -> ExplicitFixture {
    let mut fixture = explicit_fixture();
    fixture
        .runtime
        .run_string_with_context(&mut fixture.context, "bench-explicit-a := bench-anchor + 1")
        .unwrap();
    fixture
}

fn implicit_failure_source() -> MechSourceCode {
    MechSourceCode::Program(vec![
        MechSourceCode::String("bench-implicit-partial := bench-anchor + 1".to_string()),
        MechSourceCode::String("bench-implicit-error := missing-bench-value + 1".to_string()),
    ])
}

fn explicit_failure_source() -> MechSourceCode {
    MechSourceCode::Program(vec![
        MechSourceCode::String("bench-explicit-partial := bench-explicit-a + 1".to_string()),
        MechSourceCode::String("bench-explicit-error := missing-bench-value + 1".to_string()),
    ])
}

fn staged_effect_rollback_fixture(count: usize) -> (ExplicitFixture, MechSourceCode) {
    let mut runtime = support::source_runtime_builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .host_function(PlannedStagedHostFunction::new(
            "bench/staged",
            |_context, _arguments| Ok(RuntimeValueSnapshot::empty()),
            |_context, _arguments| {
                Ok(RuntimePreparedHostCall {
                    value: RuntimeValueSnapshot::empty(),
                    effect: PreparedRuntimeEffect::AfterCommit(Box::new(
                        BenchmarkAfterCommitEffect { sequence: 0 },
                    )),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    runtime.run_string("bench-anchor := 1").unwrap();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(8_000),
            &BasicSubject::new(runtime.runtime_context().unwrap().subject().to_string()),
            &BasicResource::new("host:bench/staged"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let mut source = Vec::with_capacity(count.saturating_add(1));
    for sequence in 0..count {
        source.push(MechSourceCode::String(format!(
            "bench-staged-{sequence} := bench/staged()",
        )));
    }
    source.push(MechSourceCode::String(
        "bench-staged-error := missing-bench-value + 1".to_string(),
    ));

    (
        ExplicitFixture { runtime, context },
        MechSourceCode::Program(source),
    )
}

fn program_transaction_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_program_transaction");

    group.bench_function("detached_scalar_snapshot", |b| {
        let value = LegacyValue::F64(Ref::new(42.0));
        b.iter(|| {
            black_box(
                RuntimeValueSnapshot::try_capture(black_box(&value)).expect("acyclic fixture"),
            )
        })
    });

    group.bench_function("detached_nested_value_snapshot", |b| {
        let value = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
            LegacyValue::F64(Ref::new(1.0)),
            LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
                LegacyValue::F64(Ref::new(2.0)),
                LegacyValue::Bool(Ref::new(true)),
            ]))),
        ])));
        b.iter(|| {
            black_box(
                RuntimeValueSnapshot::try_capture(black_box(&value)).expect("acyclic fixture"),
            )
        })
    });

    group.bench_function("explicit_execution_session_host_dispatch", |b| {
        b.iter_batched(
            execution_session_host_fixture,
            |mut runtime| {
                black_box(
                    runtime
                        .call_host(HostCall::new("bench/execution-session", Vec::new()))
                        .unwrap(),
                );
                black_box(runtime)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("provider_preparation", |b| {
        b.iter_batched(
            provider_preparation_fixture,
            |provider| {
                black_box(
                    provider
                        .prepare_write(RuntimeResourceWriteRequest {
                            base_uri: "docs://benchmark".to_string(),
                            path: "value".to_string(),
                            context_name: "benchmark".to_string(),
                            operation: RuntimeCapabilityOperation::Write,
                            value: LegacyValue::F64(Ref::new(2.0)),
                            intent: RuntimeResourceWriteIntent::Assign,
                        })
                        .unwrap(),
                );
                black_box(provider)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("panic_free_extension_wrapper_overhead", |b| {
        b.iter_batched(
            extension_wrapper_fixture,
            |runtime| {
                black_box(runtime.resolve_source("bench.mec").unwrap());
                black_box(runtime)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("coordinated_program_finalization", |b| {
        b.iter_batched(
            || {
                let mut fixture = explicit_fixture();
                fixture
                    .runtime
                    .run_string_with_context(
                        &mut fixture.context,
                        "bench-finalization := bench-anchor + 1",
                    )
                    .unwrap();
                fixture
            },
            |mut fixture| {
                black_box(
                    fixture
                        .runtime
                        .commit_runtime_transaction_detailed(&mut fixture.context)
                        .unwrap(),
                );
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("implicit_successful_retained_operation", |b| {
        b.iter_batched(
            || {
                let runtime = retained_runtime();
                let context = runtime.runtime_context().unwrap();
                (runtime, context)
            },
            |(mut runtime, mut context)| {
                let value = runtime
                    .run_string_with_context(
                        &mut context,
                        "bench-implicit-success := bench-anchor + 1",
                    )
                    .unwrap();
                black_box(value);
                black_box((runtime, context))
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_first_operation_and_ownership", |b| {
        b.iter_batched(
            explicit_fixture,
            |mut fixture| {
                let value = fixture
                    .runtime
                    .run_string_with_context(
                        &mut fixture.context,
                        "bench-explicit-first := bench-anchor + 1",
                    )
                    .unwrap();
                black_box(value);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_subsequent_savepoint_operation", |b| {
        b.iter_batched(
            subsequent_explicit_fixture,
            |mut fixture| {
                let value = fixture
                    .runtime
                    .run_string_with_context(
                        &mut fixture.context,
                        "bench-explicit-b := bench-explicit-a + 1",
                    )
                    .unwrap();
                black_box(value);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("failed_implicit_operation_with_rollback", |b| {
        b.iter_batched(
            || {
                let runtime = retained_runtime();
                let context = runtime.runtime_context().unwrap();
                (runtime, context, implicit_failure_source())
            },
            |(mut runtime, mut context, source)| {
                let error = runtime
                    .run_source_with_context(&mut context, &source)
                    .unwrap_err();
                black_box(error);
                black_box((runtime, context, source))
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("failed_explicit_subsequent_operation_with_rollback", |b| {
        b.iter_batched(
            || (subsequent_explicit_fixture(), explicit_failure_source()),
            |(mut fixture, source)| {
                let error = fixture
                    .runtime
                    .run_source_with_context(&mut fixture.context, &source)
                    .unwrap_err();
                black_box(error);
                black_box((fixture, source))
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_outer_abort_after_program_changes", |b| {
        b.iter_batched(
            subsequent_explicit_fixture,
            |mut fixture| {
                fixture
                    .runtime
                    .abort_runtime_transaction(&mut fixture.context, "benchmark outer abort")
                    .unwrap();
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("implicit_transaction_store_commit", |b| {
        b.iter_batched(
            || {
                let runtime = retained_runtime();
                let context = runtime.runtime_context().unwrap();
                (runtime, context)
            },
            |(mut runtime, mut context)| {
                let value = runtime
                    .run_string_with_context(&mut context, "bench-implicit-store-commit := 1")
                    .unwrap();
                black_box(value);
                black_box((runtime, context))
            },
            BatchSize::SmallInput,
        )
    });

    for count in [1, 100] {
        group.bench_function(format!("stage_{count}_after_commit_effects"), |b| {
            b.iter_batched(
                explicit_fixture,
                |mut fixture| {
                    stage_after_commit(&mut fixture, count);
                    black_box(fixture)
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("stage_{count}_compensatable_effects"), |b| {
            b.iter_batched(
                explicit_fixture,
                |mut fixture| {
                    stage_compensatable(&mut fixture, count);
                    black_box(fixture)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.bench_function("explicit_effect_savepoint_rollback_with_100_staged", |b| {
        b.iter_batched(
            || staged_effect_rollback_fixture(100),
            |(mut fixture, source)| {
                let error = fixture
                    .runtime
                    .run_source_with_context(&mut fixture.context, &source)
                    .unwrap_err();
                black_box(error);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("reversible_effect_commit", |b| {
        b.iter_batched(
            || {
                let mut fixture = explicit_fixture();
                stage_compensatable(&mut fixture, 1);
                fixture
            },
            |mut fixture| {
                let outcome = fixture
                    .runtime
                    .commit_runtime_transaction_detailed(&mut fixture.context)
                    .unwrap();
                black_box(outcome);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("store_failure_with_compensation", |b| {
        b.iter_batched(
            || {
                let mut fixture = explicit_fixture();
                stage_compensatable(&mut fixture, 1);
                fixture
                    .runtime
                    .update_object_with_context(
                        &mut fixture.context,
                        ObjectRecord::text(ObjectId(9_000), "missing", "benchmark"),
                    )
                    .unwrap();
                fixture
            },
            |mut fixture| {
                let error = fixture
                    .runtime
                    .commit_runtime_transaction_detailed(&mut fixture.context)
                    .unwrap_err();
                black_box(error);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("capability_scoped_selection", |b| {
        b.iter_batched(
            || {
                let mut fixture = explicit_fixture();
                let capability = Arc::new(BasicCapability::from_keys(
                    CapabilityId(7_000),
                    "bench-subject",
                    "bench://resource",
                    [":read"],
                ));
                fixture
                    .runtime
                    .grant_capability_with_context(&mut fixture.context, capability)
                    .unwrap();
                let request =
                    CapabilityRequest::from_keys("bench-subject", ":read", "bench://resource");
                (fixture, request)
            },
            |(mut fixture, request)| {
                let capability = fixture
                    .runtime
                    .check_capability_with_context(&mut fixture.context, &request)
                    .unwrap();
                black_box(capability);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("deliver_100_after_commit_effects", |b| {
        b.iter_batched(
            || {
                let mut fixture = explicit_fixture();
                stage_after_commit(&mut fixture, 100);
                fixture
            },
            |mut fixture| {
                let outcome = fixture
                    .runtime
                    .commit_runtime_transaction_detailed(&mut fixture.context)
                    .unwrap();
                black_box(outcome);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("small_retained_root_with_one_dependency_commits", |b| {
        b.iter_batched(
            retained_root_fixture,
            |mut runtime| {
                let value = runtime
                    .resolve_and_run_root_module("root.mec", module_options())
                    .unwrap();
                black_box(value);
                black_box(runtime)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("retained_root_graph_failure_rolls_back", |b| {
        b.iter_batched(
            retained_root_failure_fixture,
            |mut runtime| {
                let error = runtime
                    .resolve_and_run_root_module("root.mec", module_options())
                    .unwrap_err();
                black_box(error);
                black_box(runtime)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("explicit_graph_build_followed_by_abort", |b| {
        b.iter_batched(
            explicit_graph_fixture,
            |mut fixture| {
                let version = fixture
                    .runtime
                    .build_module_from_request_with_context(
                        &mut fixture.context,
                        "root.mec",
                        module_options(),
                    )
                    .unwrap();
                fixture
                    .runtime
                    .abort_runtime_transaction(&mut fixture.context, "benchmark graph abort")
                    .unwrap();
                black_box(version);
                black_box(fixture)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, program_transaction_benchmarks);
criterion_main!(benches);
