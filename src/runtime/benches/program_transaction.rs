use criterion::{
  BatchSize, Criterion, criterion_group, criterion_main,
};
use mech_core::{MResult, MechSourceCode, Value};
use mech_runtime::{
  BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
  CapabilityRequest, MechRuntime, ObjectId, ObjectRecord,
  PreparedRuntimeEffect, RuntimeAfterCommitEffect,
  RuntimeCompensatableEffect, RuntimeContext, RuntimeEffectMetadata,
  RuntimeEffectSource, RuntimePreparedHostCall, SequentialIdGenerator,
  StagedClosureHostFunction,
};
use std::hint::black_box;
use std::sync::Arc;

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

fn stage_after_commit(
  fixture: &mut ExplicitFixture,
  count: usize,
) {
  for sequence in 0..count {
    fixture
      .runtime
      .stage_runtime_effect_with_context(
        &mut fixture.context,
        PreparedRuntimeEffect::AfterCommit(Box::new(
          BenchmarkAfterCommitEffect { sequence },
        )),
      )
      .unwrap();
  }
}

fn stage_compensatable(
  fixture: &mut ExplicitFixture,
  count: usize,
) {
  for sequence in 0..count {
    fixture
      .runtime
      .stage_runtime_effect_with_context(
        &mut fixture.context,
        PreparedRuntimeEffect::Compensatable(Box::new(
          BenchmarkCompensatableEffect { sequence },
        )),
      )
      .unwrap();
  }
}

fn retained_runtime() -> MechRuntime {
  let mut runtime = MechRuntime::builder()
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

fn subsequent_explicit_fixture() -> ExplicitFixture {
  let mut fixture = explicit_fixture();
  fixture
    .runtime
    .run_string_with_context(
      &mut fixture.context,
      "bench-explicit-a := bench-anchor + 1",
    )
    .unwrap();
  fixture
}

fn implicit_failure_source() -> MechSourceCode {
  MechSourceCode::Program(vec![
    MechSourceCode::String(
      "bench-implicit-partial := bench-anchor + 1".to_string(),
    ),
    MechSourceCode::String(
      "bench-implicit-error := missing-bench-value + 1".to_string(),
    ),
  ])
}

fn explicit_failure_source() -> MechSourceCode {
  MechSourceCode::Program(vec![
    MechSourceCode::String(
      "bench-explicit-partial := bench-explicit-a + 1".to_string(),
    ),
    MechSourceCode::String(
      "bench-explicit-error := missing-bench-value + 1".to_string(),
    ),
  ])
}

fn staged_effect_rollback_fixture(
  count: usize,
) -> (ExplicitFixture, MechSourceCode) {
  let mut runtime = retained_runtime();
  runtime
    .grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(8_000),
      &BasicSubject::new(runtime.runtime_context().unwrap().subject),
      &BasicResource::new("host:bench/staged"),
      [BasicOperation::new("call")],
    )))
    .unwrap();
  runtime
    .register_mech_host_function(StagedClosureHostFunction::new(
      "bench/staged",
      |_services, _context, _arguments| {
        Ok(RuntimePreparedHostCall {
          value: Value::Empty,
          effect: PreparedRuntimeEffect::AfterCommit(Box::new(
            BenchmarkAfterCommitEffect { sequence: 0 },
          )),
        })
      },
    ))
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
          .abort_runtime_transaction(
            &mut fixture.context,
            "benchmark outer abort",
          )
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
          .run_string_with_context(
            &mut context,
            "bench-implicit-store-commit := 1",
          )
          .unwrap();
        black_box(value);
        black_box((runtime, context))
      },
      BatchSize::SmallInput,
    )
  });

  for count in [1, 100] {
    group.bench_function(
      format!("stage_{count}_after_commit_effects"),
      |b| {
        b.iter_batched(
          explicit_fixture,
          |mut fixture| {
            stage_after_commit(&mut fixture, count);
            black_box(fixture)
          },
          BatchSize::SmallInput,
        )
      },
    );

    group.bench_function(
      format!("stage_{count}_compensatable_effects"),
      |b| {
        b.iter_batched(
          explicit_fixture,
          |mut fixture| {
            stage_compensatable(&mut fixture, count);
            black_box(fixture)
          },
          BatchSize::SmallInput,
        )
      },
    );
  }

  group.bench_function(
    "explicit_effect_savepoint_rollback_with_100_staged",
    |b| {
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
    },
  );

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
            ObjectRecord::text(
              ObjectId(9_000),
              "missing",
              "benchmark",
            ),
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

  group.bench_function("capability_overlay_lookup", |b| {
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
          .grant_capability_with_context(
            &mut fixture.context,
            capability,
          )
          .unwrap();
        let request = CapabilityRequest::from_keys(
          "bench-subject",
          ":read",
          "bench://resource",
        );
        (fixture, request)
      },
      |(mut fixture, request)| {
        let capability = fixture
          .runtime
          .check_capability_with_context(
            &mut fixture.context,
            &request,
          )
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

  group.finish();
}

criterion_group!(benches, program_transaction_benchmarks);
criterion_main!(benches);
