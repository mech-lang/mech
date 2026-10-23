//! Original and decoded emitted span helpers; resident evaluation is an oracle.
//! Object emission below proves identity only, not linked Object execution.
use super::{artifacts, artifacts_mode, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, Value, ValueDataDraft as Data};
use mech_engine::native_artifact::{NativeCollectionInvocationError, NativeIterationAction};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramExecutionError, emit_program_object,
};

fn selected(jit: &JitProgramArtifact) -> usize {
    jit.descriptor()
        .plan()
        .collections()
        .iter()
        .filter(|collection| collection.string_span_projection().is_some())
        .count()
}

fn exercise(source: &str, calls: &[(&str, u64, u64, &str)]) {
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            selected(&jit),
            1,
            "must execute actual selected span strategy"
        );
        for predicate in [
            |action: &NativeIterationAction| {
                matches!(
                    action,
                    NativeIterationAction::PrepareStringSpanSelector { .. }
                )
            },
            |action: &NativeIterationAction| {
                matches!(action, NativeIterationAction::ProjectStringSpan { .. })
            },
        ] {
            assert_eq!(
                jit.descriptor()
                    .plan()
                    .blocks()
                    .iter()
                    .filter(|block| predicate(block.action()))
                    .count(),
                1
            );
        }
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let mut output = vec![None; artifact.outputs().len()];
        for &(text, start, end, separator) in calls {
            let args = inputs(
                &artifact,
                &[
                    ("alphabet", Data::String(text.into())),
                    ("begin", Data::U64(start)),
                    ("until", Data::U64(end)),
                    ("separator", Data::String(separator.into())),
                ],
                &memory,
            );
            match reference_result(&artifact, &args) {
                Ok(expected) => {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                }
                Err(error) => {
                    assert!(error.contains("InvalidInput"), "{error}");
                    let prior = output
                        .iter()
                        .map(|value| value.as_ref().unwrap().clone())
                        .collect::<Vec<Value>>();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(
                        matches!(
                            error,
                            NativeProgramExecutionError::Library {
                                error: NativeCollectionInvocationError::Kernel(
                                    ResidentKernelError::InvalidInput
                                ),
                                ..
                            }
                        ),
                        "{error:?}"
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                }
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_string_span_join_keeps_unicode_live_ranges_empty_and_portable_errors() {
    for range in ["..", "..="] {
        let source = format!(
            "+> text\ntext/join([(alphabet<string>)[begin<u64>{range}until<u64>] | attempt <- 1u64..=2u64], separator<string>)\n"
        );
        exercise(
            &source,
            &[
                ("aé🧑‍🚀é\r\n🇺🇳\0", 1, 3, "|é"),
                ("aé🧑‍🚀é\r\n🇺🇳\0", 3, 6, "\0"),
                ("x", 1, 1, "|"),
                ("", 9, 2, "|"),
                ("abc", 0, 1, "|"),
                ("abc", 2, 5, "|"),
                ("abc", u32::MAX as u64 + 1, u32::MAX as u64 + 1, "|"),
                ("🧑‍🚀é", 1, 2, "→"),
            ],
        );
    }
}

#[test]
fn selected_string_span_composes_full_histogram_and_guarded_query() {
    for source in [
        "+> map\nmap/histogram([(alphabet<string>)[begin<u64>..=until<u64>] | attempt <- 1u64..=3u64])\n",
        "+> map\nlookup(alphabet<string>, begin<u64>, until<u64>, query<string>) => (out<u64>) :=\n  words := [alphabet[begin..=until] | attempt <- 1u64..=3u64]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nlookup(alphabet<string>, begin<u64>, until<u64>, separator<string>)\n",
    ] {
        exercise(
            source,
            &[
                ("éé🧑‍🚀\0", 1, 2, "éé"),
                ("éé🧑‍🚀\0", 3, 4, "missing"),
                ("abc", 9, 2, ""),
                ("abc", 0, 1, ""),
                ("abc", 2, 4, ""),
                ("🇺🇳\r\n", 1, 2, "🇺🇳\r\n"),
            ],
        );
    }
}

#[test]
fn selected_string_span_preserves_late_failure_and_lazy_filter() {
    for filter in ["", ", attempt != 3u64"] {
        let source = format!(
            "+> text\ntext/join([(alphabet<string>)[1u64..=attempt] | attempt <- 1u64..=3u64{filter}], \"|\")\n"
        );
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 1);
            let mut output = vec![None; artifact.outputs().len()];
            for text in ["abcd", "é🧑‍🚀", "🇺🇳\r\nx"] {
                let args = inputs(
                    &artifact,
                    &[("alphabet", Data::String(text.into()))],
                    &memory,
                );
                match reference_result(&artifact, &args) {
                    Ok(expected) => {
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, &expected);
                    }
                    Err(_) => {
                        let prior = output
                            .iter()
                            .map(|value| value.as_ref().unwrap().clone())
                            .collect::<Vec<_>>();
                        let before = memory.used_bytes();
                        assert!(
                            jit.evaluate_into(&args, &mut output, limits(), &memory)
                                .is_err()
                        );
                        equal(&artifact, &output, &prior);
                        assert_eq!(memory.used_bytes(), before);
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn observable_selector_or_string_does_not_acquire_a_span_strategy() {
    for (case, source) in [
        "+> text\nwords := [(alphabet<string>)[1u64..=2u64] | attempt <- 1u64..=2u64]\nwords\ntext/join(words, \"\")\n",
        "+> text\ntext/join([text/join([piece | copy <- 1u64..=1u64], piece) | attempt <- 1u64..=2u64, piece := (alphabet<string>)[1u64..=2u64]], \"\")\n",
    ].into_iter().enumerate() {
        for artifact in artifacts_mode(source, true).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 0, "{source}");
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[("alphabet", Data::String("é🧑‍🚀x".into()))],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            if case == 0 {
                assert!(jit.descriptor().plan().collections().iter().any(|collection| matches!(collection.result(), mech_engine::native_artifact::NativeCollectionResult::Materialized)), "the observed Matrix must remain a real materialized collection");
                let observed = Data::Matrix(vec![Data::String("é🧑‍🚀".into()), Data::String("é🧑‍🚀".into())].into_boxed_slice());
                assert!(output.iter().flatten().any(|value| value.canonical_data_draft().unwrap() == observed), "interactive compilation must genuinely retain the Matrix[String] program observation, not just the final joined String");
            }
        }
    }
}
