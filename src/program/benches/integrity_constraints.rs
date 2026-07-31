use criterion::{Criterion, criterion_group, criterion_main};
use mech_program::{MechProgram, MechProgramConfig};
use std::hint::black_box;

fn program(source: &str) -> MechProgram {
  let mut program = MechProgram::new(MechProgramConfig::default());
  if !source.is_empty() {
    program.run_string(source).unwrap();
  }
  program
}

fn passing_constraints(count: usize) -> String {
  (0..count)
    .map(|index| {
      format!("integrity-{index}! := {index}.0 <= {index}.0")
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn failing_constraints(count: usize) -> String {
  (0..count)
    .map(|index| {
      format!("integrity-{index}! := {}.0 < {index}.0", index + 1)
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn integrity_constraint_benchmarks(c: &mut Criterion) {
  let zero = program("");
  c.bench_function("integrity_constraints/zero", |b| {
    b.iter(|| {
      black_box(zero.integrity_constraint_report().unwrap())
    })
  });

  let one = program(&passing_constraints(1));
  c.bench_function("integrity_constraints/one_passing", |b| {
    b.iter(|| {
      black_box(one.validate_integrity_constraints().unwrap())
    })
  });

  let hundred = program(&passing_constraints(100));
  c.bench_function("integrity_constraints/one_hundred_passing", |b| {
    b.iter(|| {
      black_box(hundred.validate_integrity_constraints().unwrap())
    })
  });

  let one_failed = program(&failing_constraints(1));
  c.bench_function("integrity_constraints/one_failed", |b| {
    b.iter(|| {
      black_box(
        one_failed
          .validate_integrity_constraints()
          .unwrap_err(),
      )
    })
  });

  let ten_failed = program(&failing_constraints(10));
  c.bench_function("integrity_constraints/ten_failed_aggregated", |b| {
    b.iter(|| {
      black_box(
        ten_failed
          .validate_integrity_constraints()
          .unwrap_err(),
      )
    })
  });
}

criterion_group!(benches, integrity_constraint_benchmarks);
criterion_main!(benches);
