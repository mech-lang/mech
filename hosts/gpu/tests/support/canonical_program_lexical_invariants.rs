//! Lexical checks keep source order, branch/visit scope, and atomic publication.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramDescriptor,
    NativeProgramExecutionError, NativeProgramExecutionLimits,
};

#[path = "canonical_program_lexical_invariant_visits.rs"]
mod visits;

#[path = "canonical_program_lexical_invariant_order.rs"]
mod order;

pub(super) fn check_cases(source: &str, cases: &[(Vec<(&str, Data)>, Option<Data>)]) {
    check_cases_with_policy(source, cases, None, limits());
}

fn check_cases_with_policy(
    source: &str,
    cases: &[(Vec<(&str, Data)>, Option<Data>)],
    resident_denial: Option<&str>,
    policy: NativeProgramExecutionLimits,
) {
    for artifact in artifacts(source).0 {
        assert!(artifact.constraints().is_empty());
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: unchanged checked build metadata from this retained JIT owner;
        // the restored descriptor is inspected without binding foreign code.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (named, expected) in cases {
            let args = inputs(&artifact, named, &memory);
            if let Some(expected) = expected {
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                assert_eq!(
                    &output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected
                );
            } else {
                let error = reference_result(&artifact, &args).unwrap_err();
                if let Some(dimension) = resident_denial {
                    assert!(
                        error.contains("ResourceLimit")
                            && error.contains(dimension)
                            && error.contains("limit: 16777216"),
                        "{error}"
                    );
                    eprintln!("DEFLATE_ENTRY_RESIDENT_DEFAULT_DENIAL {error}");
                } else {
                    assert!(error.contains("LexicalIntegrity"), "{error}");
                }
                let prior = output.clone();
                let before = memory.used_bytes();
                if resident_denial.is_some() {
                    // The ordinary native test allowance must still expose
                    // argument-frame copying rather than hide or waive it.
                    let denial = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    let detail = format!("{denial:?}");
                    assert!(
                        detail.contains("ResourceLimit") && detail.contains("ComputeWork"),
                        "{detail}"
                    );
                    assert_eq!(memory.used_bytes(), before);
                    assert!(output.iter().all(Option::is_none));
                    eprintln!("DEFLATE_ENTRY_NATIVE_GENERIC_DENIAL {detail}");
                }
                let error = jit
                    .evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap_err();
                let NativeProgramExecutionError::Control(
                    NativeIterationExecutionError::Integrity { origin },
                ) = error
                else {
                    panic!("expected lexical integrity failure: {error:?}")
                };
                assert!(origin.block().is_some() && origin.local().is_some());
                assert!(jit.descriptor().plan().blocks().iter().any(|block| {
                    block.origin() == origin && block.terminator() ==
                        mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                            mech_engine::native_artifact::NativeMatchTerminator::IntegrityFailure)
                }));
                for (left, right) in output.iter().zip(&prior) {
                    match (left, right) {
                        (Some(left), Some(right)) => assert!(
                            left.snapshot_eq(artifact.schemas(), right, artifact.schemas())
                                .unwrap()
                        ),
                        (None, None) => {}
                        _ => panic!("failed assertion changed publication"),
                    }
                }
                assert_eq!(memory.used_bytes(), before);
            }
            for (declaration, value) in artifact.inputs().iter().zip(&args) {
                let name = mech_engine::decode_source_input_name(&declaration.name).unwrap();
                let expected = &named.iter().find(|(key, _)| *key == name).unwrap().1;
                assert_eq!(
                    &value.canonical_data_draft().unwrap(),
                    expected,
                    "input {name}"
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn function_invariant_precedes_the_later_division_and_recovers() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := 12u64 / n.\nchecked(input<u64>)\n";
    check_cases(
        source,
        &[
            (vec![("input", Data::U64(3))], Some(Data::U64(4))),
            (vec![("input", Data::U64(0))], None),
            (vec![("input", Data::U64(2))], Some(Data::U64(6))),
        ],
    );
}

#[test]
fn function_invariant_precedes_later_closed_arithmetic() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := 1u64 / 0u64.\nchecked(input<u64>)\n";
    check_cases(source, &[(vec![("input", Data::U64(0))], None)]);
}

#[test]
fn function_invariant_precedes_the_later_out_of_bounds_read() {
    let source = "+> sequence\nchecked-read(xs<[u8]>, at<u64>) => (out<u64>) :=\n  readable! := (at >= 1u64) && (at <= sequence/length(xs))\n  out<u64> := xs[at].\nchecked-read(input<[u8]>, at<u64>)\n";
    let values = || Data::Matrix(vec![Data::U8(9), Data::U8(8), Data::U8(7)].into());
    check_cases(
        source,
        &[
            (
                vec![("input", values()), ("at", Data::U64(1))],
                Some(Data::U64(9)),
            ),
            (vec![("input", values()), ("at", Data::U64(0))], None),
            (vec![("input", values()), ("at", Data::U64(4))], None),
            (
                vec![("input", values()), ("at", Data::U64(2))],
                Some(Data::U64(8)),
            ),
        ],
    );
}

#[test]
fn constant_false_invariant_is_dormant_in_an_unselected_function_arm() {
    let source = "bad(n<u64>) => (out<u64>) :=\n  impossible! := false\n  out := 1u64 / n.\nchoose(flag<bool>, n<u64>) => (out<u64>) :=\n  out := (flag? | true => bad(n) | false => 17u64).\nchoose(flag<bool>, input<u64>)\n";
    check_cases(
        source,
        &[
            (
                vec![("flag", Data::Bool(false)), ("input", Data::U64(0))],
                Some(Data::U64(17)),
            ),
            (
                vec![("flag", Data::Bool(true)), ("input", Data::U64(0))],
                None,
            ),
            (
                vec![("flag", Data::Bool(false)), ("input", Data::U64(0))],
                Some(Data::U64(17)),
            ),
        ],
    );
}

#[test]
fn invariants_in_broadcasts_keep_empty_visits_and_discard_partial_results() {
    let source = "positive(n<u64>) => (out<u64>) :=\n  valid! := n > 0u64\n  out := n + 1u64.\npositive(input<[u64]>)\n";
    let matrix = |values: &[u64]| Data::Matrix(values.iter().copied().map(Data::U64).collect());
    check_cases(
        source,
        &[
            (vec![("input", matrix(&[]))], Some(matrix(&[]))),
            (vec![("input", matrix(&[2, 4]))], Some(matrix(&[3, 5]))),
            (vec![("input", matrix(&[2, 0, 4]))], None),
            (vec![("input", matrix(&[7]))], Some(matrix(&[8]))),
        ],
    );
}

#[test]
fn filtered_comprehensions_skip_a_function_invariant_for_filtered_elements() {
    let source = "positive(n<u64>) => (out<u64>) :=\n  valid! := n > 0u64\n  out := n + 1u64.\n[positive(n) | n <- input<[u64]>, n != 0u64]\n";
    let matrix = |values: &[u64]| Data::Matrix(values.iter().copied().map(Data::U64).collect());
    check_cases(
        source,
        &[
            (
                vec![("input", matrix(&[0, 2, 0, 4]))],
                Some(matrix(&[3, 5])),
            ),
            (vec![("input", matrix(&[0]))], Some(matrix(&[]))),
            (vec![("input", matrix(&[]))], Some(matrix(&[]))),
        ],
    );
}

#[test]
fn zero_repeated_application_does_not_execute_a_constant_false_invariant() {
    for count in [0, 1] {
        let source = format!(
            "step(n<u64>) => (out<u64>) :=\n  impossible! := false\n  out := n.\ninput<u64> |{count}> step\n"
        );
        check_cases(
            &source,
            &[(
                vec![("input", Data::U64(19))],
                if count == 0 {
                    Some(Data::U64(19))
                } else {
                    None
                },
            )],
        );
    }
}

#[test]
fn document_invariants_keep_their_existing_root_constraint_semantics() {
    for artifact in artifacts("positive! := input<u64> > 0u64\ninput<u64>\n").0 {
        assert_eq!(artifact.constraints().len(), 1);
        let memory = ManagedMemoryBudget::new(1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::U64(0))], &memory);
        let error = reference_result(&artifact, &args).unwrap_err();
        assert!(
            error.contains("Integrity") && !error.contains("LexicalIntegrity"),
            "{error}"
        );
        assert!(JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err());
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn lexical_integrity_has_its_own_failure_in_scalar_match_and_counted_profiles() {
    use mech_engine::native_artifact::{
        NativeScalarValue as Word, lower_scalar_iteration_artifact, lower_scalar_match_artifact,
    };
    use mech_gpu::native_artifact::{
        JitIterationArtifact, JitMatchArtifact, NativeExecutionLimits, NativeMatchExecutionError,
    };
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := 12u64 / n.\nchecked(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        let mut output = vec![Word::U64(99); plan.outputs().len()];
        for n in [2, 0, 3] {
            let previous = output.clone();
            let result = jit.evaluate_into(
                &[Word::U64(n)],
                &mut output,
                NativeExecutionLimits::default(),
                None,
            );
            if n == 0 {
                assert!(
                    matches!(result, Err(NativeMatchExecutionError::Integrity { .. })),
                    "{result:?}"
                );
                assert_eq!(output, previous);
            } else {
                result.unwrap();
                assert_eq!(output.last(), Some(&Word::U64(12 / n)));
            }
        }
    }
    let source = "step(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n - 1u64.\ninput<u64> |3> step\n";
    for artifact in artifacts(source).0 {
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![Word::U64(99); plan.outputs().len()];
        for n in [3, 2, 4] {
            let previous = output.clone();
            let result = jit.evaluate_into(
                &[Word::U64(n)],
                &mut output,
                NativeExecutionLimits::default(),
                None,
            );
            if n == 2 {
                assert!(
                    matches!(result, Err(NativeIterationExecutionError::Integrity { .. })),
                    "{result:?}"
                );
                assert_eq!(output, previous);
            } else {
                result.unwrap();
                assert_eq!(output.last(), Some(&Word::U64(n - 3)));
            }
        }
    }
}

#[test]
fn deflate_extension_checks_rocs_window_before_loading_or_advancing_the_length() {
    let source = format!(
        "{}\nextend-length(input<[u8]>, str-at<u64>, match-at<u64>, start<u64>, maximum<u64>)\n",
        include_str!("../fixtures/native/deflate-matchfinder.mec")
    );
    let named = |size, position, candidate, start, maximum| {
        vec![
            ("input", Data::Matrix(vec![Data::U8(1); size].into())),
            ("str-at", Data::U64(position)),
            ("match-at", Data::U64(candidate)),
            ("start", Data::U64(start)),
            ("maximum", Data::U64(maximum)),
        ]
    };
    check_cases(
        &source,
        &[
            (named(4, 2, 0, 0, 2), Some(Data::U64(2))),
            (named(4, 3, 0, 0, 2), None),
            (named(4, 2, 2, 0, 2), None),
            (named(0, 1, 0, u64::MAX, 0), None),
            (named(4, 2, 0, 0, 2), Some(Data::U64(2))),
        ],
    );
}

#[test]
fn binary_tree_table_and_window_checks_precede_the_later_reads() {
    let kernel = concat!(
        include_str!("../fixtures/native/deflate-matchfinder.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-binary-tree.mec")
    );
    for skip in [false, true] {
        let call = if skip {
            "bt-skip-byte(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, nice<u64>, depth<u64>)"
        } else {
            "bt-get-matches(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, maximum<u64>, nice<u64>, depth<u64>, cache<[u32]>, pointer<u64>)"
        };
        let source = format!("{kernel}\n{call}\n");
        let named = |table_len, input_len, position| {
            vec![
                ("tables", Data::Matrix(vec![Data::U16(0); table_len].into())),
                ("hash3", Data::U64(0)),
                ("hash4", Data::U64(0)),
                ("input", Data::Matrix(vec![Data::U8(0); input_len].into())),
                ("base", Data::U64(0)),
                ("position", Data::U64(position)),
                ("maximum", Data::U64(5)),
                ("nice", Data::U64(5)),
                ("depth", Data::U64(64)),
                ("cache", Data::Matrix(vec![Data::U32(0); 512].into())),
                ("pointer", Data::U64(0)),
            ]
        };
        check_cases(&source, &[(named(1, 0, u64::MAX), None)]);
        // Framing the complete checked invocation currently copies the full
        // argument tuple. Keep and expose the resident default's admission
        // denial; the explicitly qualified native path still must reach the
        // window check before any input load or table update.
        // Use the existing full-tree transition qualification policy. This
        // pays for the physical copying without changing any host default.
        let mut policy = limits();
        policy.library.max_output_elements = Some(262144);
        policy.execution.max_work = 10_000_000_000;
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        check_cases_with_policy(
            &source,
            &[(named(262144, 5, 4), None)],
            Some("TemporaryBytes"),
            policy,
        );
    }
}
