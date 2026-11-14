//! Actual generated execution, not merely selection of a region descriptor.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
    emit_program_object,
};

const COUNT: &str = "#Count(n<u64>) => <u64>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Count(n) -> :Step(n, 0u64)\n  :Step(left, total)\n    | left > 0u64 -> :Step(left - 1u64, total + 1u64)\n    | * -> :Done(total)\n  :Done(total) => total.\n#Count(input<u64>)\n";

#[test]
fn closed_tail_has_constant_callback_count_and_finite_cumulative_fuel() {
    for artifact in artifacts(COUNT).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let object = emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(object.metadata()["closed_tail_regions"], 1);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut reports = Vec::new();
        for count in [0, 8, 128] {
            let args = inputs(&artifact, &[("input", Data::U64(count))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            reports.push(report);
        }
        assert_eq!(
            reports[0].visits, reports[2].visits,
            "more Tail transitions must not create more Rust admission callbacks"
        );
        assert!(reports[2].work > reports[1].work);
        let args = inputs(&artifact, &[("input", Data::U64(128))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.execution.max_work = reports[1].work;
        let error = jit
            .evaluate_into(&args, &mut output, denied, &memory)
            .unwrap_err();
        assert_work_denial(&error, denied.execution.max_work);
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}

#[test]
fn closed_tail_arithmetic_failure_reconciles_before_atomic_exit() {
    let source = COUNT.replace("total + 1u64", "total + (10u64 / (left - 1u64))");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata()["closed_tail_regions"],
            1
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let valid = inputs(&artifact, &[("input", Data::U64(0))], &memory);
        let denied = inputs(&artifact, &[("input", Data::U64(1))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let error = jit
            .evaluate_into(&denied, &mut output, limits(), &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                NativeProgramExecutionError::Control(
                    NativeIterationExecutionError::Arithmetic { .. }
                )
            ),
            "{error:?}"
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}

fn assert_work_denial(error: &NativeProgramExecutionError, expected_limit: u64) {
    let (required, limit) = match error {
        NativeProgramExecutionError::Control(NativeIterationExecutionError::Work {
            required,
            limit,
            ..
        })
        | NativeProgramExecutionError::Base(
            mech_gpu::native_artifact::NativeExecutionError::WorkLimit {
                required, limit, ..
            },
        ) => (*required, *limit),
        _ => panic!("expected finite cumulative compute denial: {error:?}"),
    };
    assert_eq!(limit, expected_limit);
    assert!(required > limit);
}

#[test]
fn nonterminating_native_tail_exhausts_fuel_without_refill_or_publication() {
    let source = COUNT.replace("left - 1u64", "left");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata()["closed_tail_regions"],
            1
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let valid = inputs(&artifact, &[("input", Data::U64(0))], &memory);
        let infinite = inputs(&artifact, &[("input", Data::U64(1))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.execution.max_work = report.work + 1_000_000;
        for _ in 0..2 {
            let error = jit
                .evaluate_into(&infinite, &mut output, denied, &memory)
                .unwrap_err();
            assert_work_denial(&error, denied.execution.max_work);
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), before);
        }
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}

#[test]
fn closed_tail_assertion_failure_reconciles_before_atomic_exit() {
    let source = "walk(left<u64>, stop<u64>) => (out<u64>) :=\n  visited! := left != stop\n  out := ((left > 0u64)? | true => walk(left - 1u64, stop) | false => 41u64).\nwalk(steps<u64>, stop<u64>)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(
            emit_program_object(&artifact, &catalog).unwrap().metadata()["closed_tail_regions"],
            1
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let valid = inputs(
            &artifact,
            &[("steps", Data::U64(3)), ("stop", Data::U64(9))],
            &memory,
        );
        let rejected = inputs(
            &artifact,
            &[("steps", Data::U64(3)), ("stop", Data::U64(2))],
            &memory,
        );
        let aliases = rejected.clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for _ in 0..2 {
            let error = jit
                .evaluate_into(&rejected, &mut output, limits(), &memory)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Integrity { .. }
                    )
                ),
                "{error:?}"
            );
            equal(&artifact, &output, &prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(
                rejected
                    .iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &valid));
        drop((valid, rejected, aliases, output, prior));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
