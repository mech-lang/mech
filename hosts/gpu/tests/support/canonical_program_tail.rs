//! Closed synchronous Tail regressions shared by original and decoded artifacts.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationAction, NativeStructuralObservation,
    lower_native_program_artifact,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeIterationExecutionError,
    NativeProgramExecutionError, NativeValueError, emit_program_object,
};

#[test]
fn raw_string_literal_patterns_are_structural_observations_not_library_calls() {
    let source = "pick(s<string>) => (out<bool>) :=\n  out := (s?\n    | \"é\" => true\n    | \" \" => true\n    | \"\\t\" => true\n    | * => false).\npick(input<string>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let patterns = jit
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .filter(|block| {
                matches!(
                    block.action(),
                    NativeIterationAction::Structural {
                        observation: NativeStructuralObservation::StringEqual { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(patterns, 3);
        assert!(
            jit.descriptor().plan().blocks().iter().all(|block| {
                !matches!(block.action(), NativeIterationAction::LibraryCall { .. })
            })
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for (input, expected) in [
            ("é", true),
            ("e\u{301}", false),
            (" ", true),
            ("\t", true),
            ("", false),
            ("🧑‍🚀", false),
        ] {
            let args = inputs(&artifact, &[("input", Data::String(input.into()))], &memory);
            let ordinary = reference(&artifact, &args);
            let mut output = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Bool(expected)
            );
            assert!(report.comparison_work >= "é".len() as u64);
        }
        // The actual canonical pattern helper must admit its raw-byte
        // comparison before publishing, through the same cumulative ledger.
        let args = inputs(&artifact, &[("input", Data::String("é".into()))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.library.max_comparison_work = Some(0);
        let error = jit
            .evaluate_into(&args, &mut output, denied, &memory)
            .unwrap_err();
        assert!(
            matches!(error, NativeProgramExecutionError::Collection {
            error: mech_core::ResidentKernelError::ResourceLimit {
                dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                required, limit, ..
            }, ..
        } if required > limit && limit == 0),
            "{error:?}"
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

const CAPTURED_TAIL: &str = "#Scan(n<u64>, token<string>) => <(u64,string)>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Scan(n, token) -> :Step(n, 0u64)\n  :Step(left, total)\n    | left > 0u64 -> :Step(left - 1u64, total + 1u64)\n    | * -> :Done(total)\n  :Done(total) => (total, token).\n#Scan(input<u64>, payload<string>)\n";

#[test]
fn tail_work_denial_after_managed_state_transitions_is_atomic_and_recovers() {
    for artifact in artifacts(CAPTURED_TAIL).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            jit.descriptor()
                .plan()
                .iterations()
                .iter()
                .filter(|item| matches!(item, NativeControlLoop::Tail(_)))
                .count(),
            1
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let payload = Data::String("e\u{301}🧑‍🚀".into());
        let short = inputs(
            &artifact,
            &[("input", Data::U64(8)), ("payload", payload.clone())],
            &memory,
        );
        let long = inputs(
            &artifact,
            &[("input", Data::U64(128)), ("payload", payload.clone())],
            &memory,
        );
        let recovered = inputs(
            &artifact,
            &[("input", Data::U64(3)), ("payload", payload.clone())],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&short, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &short));
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(vec![Data::U64(8), payload.clone()].into())
        );
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        // Same schemas, String payload and fixed-width inputs: the complete
        // eight-step run admits all setup and multiple Tail backedges. Its
        // measured finite work budget cannot admit the 128-step invocation.
        // This restricts the test policy; no product/default limit is raised.
        let mut denied = limits();
        denied.execution.max_work = report.work;
        for _ in 0..2 {
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&long, &mut output, denied, &memory)
                .unwrap_err();
            let (required, limit) = match &error {
                NativeProgramExecutionError::Control(NativeIterationExecutionError::Work {
                    required,
                    limit,
                    ..
                })
                | NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit {
                    required,
                    limit,
                    ..
                })
                | NativeProgramExecutionError::Control(NativeIterationExecutionError::Base(
                    NativeExecutionError::WorkLimit {
                        required, limit, ..
                    },
                ))
                | NativeProgramExecutionError::Value(NativeValueError::Base(
                    NativeExecutionError::WorkLimit {
                        required, limit, ..
                    },
                ))
                | NativeProgramExecutionError::Library {
                    error:
                        mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::ResourceLimit {
                                dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                                required,
                                limit,
                                ..
                            },
                        ),
                    ..
                } => (*required, *limit),
                _ => panic!("expected typed native work denial, not another failure: {error:?}"),
            };
            assert!(required > limit);
            assert_eq!(limit, report.work);
            equal(&artifact, &output, &prior);
            assert_eq!(
                memory.used_bytes(),
                before,
                "Tail denial retained invocation-local state"
            );
            jit.evaluate_into(&recovered, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &recovered));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![Data::U64(3), payload.clone()].into())
            );
            let repeated = jit
                .evaluate_into(&short, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &prior);
            assert_eq!(
                repeated, report,
                "no previous Tail progress or arena generation survives"
            );
        }
        drop((short, long, recovered, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn non_u64_state_bindings_and_suspending_machines_do_not_gain_native_fallbacks() {
    for (source, named) in [
        (
            "#TextState(s<string>) => <bool>\n  | :Step(value<string>)\n  | :Done(value<u64>).\n#TextState(s) -> :Step(s)\n  :Step(value) -> :Done(1u64)\n  :Done(value) => true.\n#TextState(input<string>)\n",
            vec![("input", Data::String("state".into()))],
        ),
        (
            "#FloatState(s<f64>) => <bool>\n  | :Step(value<f64>)\n  | :Done(value<u64>).\n#FloatState(s) -> :Step(s)\n  :Step(value) -> :Done(1u64)\n  :Done(value) => true.\n#FloatState(input<f64>)\n",
            vec![(
                "input",
                Data::F64(mech_core::snapshot::F64Bits::from_f64(1.0)),
            )],
        ),
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &named, &memory);
            let ordinary = reference(&artifact, &args);
            assert_eq!(
                ordinary.last().unwrap().canonical_data_draft().unwrap(),
                Data::Bool(true)
            );
            assert!(
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).is_err()
            );
            assert!(
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err()
            );
            assert!(emit_program_object(&artifact, &mech_stdlib::source_catalog()).is_err());
        }
    }
    let asynchronous = "#Deferred() => <u64>\n  | :Start\n  | :Done(value<u64>).\n#Deferred() -> :Start\n  :Start ~> :Done(42u64)\n  :Done(value) => value.\n#Deferred()\n";
    for artifact in artifacts(asynchronous).0 {
        assert!(lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).is_err());
        assert!(JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err());
        assert!(emit_program_object(&artifact, &mech_stdlib::source_catalog()).is_err());
    }
}
