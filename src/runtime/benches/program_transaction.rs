use criterion::{
  BatchSize, Criterion, criterion_group, criterion_main,
};
use mech_core::MechSourceCode;
use mech_runtime::{
  MechRuntime, RuntimeContext, SequentialIdGenerator,
};
use std::hint::black_box;

struct ExplicitFixture {
  runtime: MechRuntime,
  context: RuntimeContext,
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

  group.finish();
}

criterion_group!(benches, program_transaction_benchmarks);
criterion_main!(benches);
