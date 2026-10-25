//! Source-order errors and visit-local operands remain observable.
use super::super::{artifacts, inputs, limits, reference_result};
use super::{Data, JitProgramArtifact, ManagedMemoryBudget, check_cases};
use mech_gpu::native_artifact::{NativeIterationExecutionError, NativeProgramExecutionError};

#[test]
fn a_reached_load_failure_precedes_a_later_false_assertion() {
    let source = "read-first(xs<[u8]>, at<u64>) => (out<u64>) :=\n  loaded<u64> := xs[at]\n  impossible! := false\n  out := loaded.\nread-first(input<[u8]>, at<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("input", Data::Matrix(vec![Data::U8(9)].into())),
                ("at", Data::U64(0)),
            ],
            &memory,
        );
        let resident = reference_result(&artifact, &args).unwrap_err();
        assert!(!resident.contains("LexicalIntegrity"), "{resident}");
        assert!(resident.contains("InvalidInput"), "{resident}");
        let mut output = vec![None; artifact.outputs().len()];
        let before = memory.used_bytes();
        let native = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap_err();
        assert!(format!("{native:?}").contains("InvalidInput"), "{native:?}");
        assert!(output.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), before);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn two_false_checks_report_the_first_reached_check() {
    use mech_engine::{ControlOperationBody, ExecutableNodeBody};
    let source = "checked(n<u64>) => (out<u64>) :=\n  first! := n > 0u64\n  second! := n > 1u64\n  out := n.\nchecked(input<u64>)\n";
    check_cases(
        source,
        &[
            (vec![("input", Data::U64(2))], Some(Data::U64(2))),
            (vec![("input", Data::U64(0))], None),
            (vec![("input", Data::U64(1))], None),
            (vec![("input", Data::U64(3))], Some(Data::U64(3))),
        ],
    );
    for artifact in artifacts(source).0 {
        let expected = artifact
            .nodes()
            .iter()
            .find_map(|node| {
                let ExecutableNodeBody::Match(control) = &node.body else {
                    return None;
                };
                let block = &control.arms[0].body;
                let locals = block
                    .operations
                    .iter()
                    .filter_map(|operation| {
                        matches!(operation.body, ControlOperationBody::Assert)
                            .then_some(operation.node)
                    })
                    .collect::<Vec<_>>();
                (locals.len() == 2).then_some((node.node, block.id.0, locals))
            })
            .unwrap();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for (n, ordinal) in [(0, 0), (1, 1)] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            let resident = reference_result(&artifact, &args).unwrap_err();
            assert!(
                resident.contains(&format!("local: {}", expected.2[ordinal])),
                "{resident}"
            );
            let mut output = vec![None; artifact.outputs().len()];
            let error = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_err();
            let NativeProgramExecutionError::Control(NativeIterationExecutionError::Integrity {
                origin,
            }) = error
            else {
                panic!("{error:?}");
            };
            assert_eq!(
                (origin.node(), origin.block(), origin.local()),
                (expected.0, Some(expected.1), Some(expected.2[ordinal]))
            );
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn evaluating_a_check_can_overflow_before_its_boolean_result_exists() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  room! := n + 1u64 > n\n  out := n.\nchecked(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::U64(u64::MAX))], &memory);
        let resident = reference_result(&artifact, &args).unwrap_err();
        assert!(
            resident.contains("Arithmetic") && !resident.contains("LexicalIntegrity"),
            "{resident}"
        );
        let mut output = vec![None; artifact.outputs().len()];
        let before = memory.used_bytes();
        let native = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap_err();
        assert!(
            matches!(
                native,
                NativeProgramExecutionError::Control(
                    NativeIterationExecutionError::Arithmetic { .. }
                )
            ),
            "{native:?}"
        );
        assert!(output.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), before);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_calls_use_each_calls_current_argument_in_source_order() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(a<u64>, b<u64>) => (out<u64>) :=\n  first := checked(a)\n  second := checked(b)\n  out := first + second.\nouter(a<u64>, b<u64>)\n";
    let named = |a, b| vec![("a", Data::U64(a)), ("b", Data::U64(b))];
    check_cases(
        source,
        &[
            (named(2, 3), Some(Data::U64(5))),
            (named(0, 3), None),
            (named(2, 0), None),
            (named(4, 5), Some(Data::U64(9))),
        ],
    );
}
