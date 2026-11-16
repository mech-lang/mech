//! Prove only pure invariant unsigned yields with the exact selected range.
use super::artifacts;
use mech_engine::native_artifact::lower_native_program_artifact;

#[test]
fn uniform_word_facts_keep_the_exact_region_and_reject_effects_or_varying_yields() {
    for width in [8, 16, 32, 64] {
        let source = format!("[word<u{width}> | i <- 0u64..=count<u64>, i < count]\n");
        for artifact in artifacts(&source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let regions = plan.facts().uniform_unsigned_collections();
            assert_eq!(regions.len(), 1, "{width}: {plan:?}");
            let region = &regions[0];
            assert_eq!(region.iteration(), 0);
            assert_eq!(region.collection(), 0);
            assert_eq!(
                plan.values()[region.word().get() as usize].scalar_kind(),
                Some(region.kind())
            );
        }
    }
    for source in [
        "[i | i <- 0u64..=count<u64>, i < count]\n",
        "[word<u32> | i <- 1u64..=count<u64>, i < count]\n",
        "[word<u32> | i <- 0u64..=count<u64>, i < count, i > 0u64]\n",
        "allow := allowed<bool>\nchecked(x<u32>) => (out<u32>) :=\n  enabled! := allow\n  out := x.\n[checked(word<u32>) | i <- 0u64..=count<u64>, i < count]\n",
    ] {
        for artifact in artifacts(source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                plan.facts().uniform_unsigned_collections().is_empty(),
                "{source}"
            );
        }
    }
}

#[test]
fn uniform_unsigned_values_keep_clone_quotas_prior_outputs_and_recovery() {
    use super::{inputs, limits, reference};
    use mech_core::{
        ManagedMemoryBudget, MemoryBudgetDimension as Dimension, ResidentKernelError,
        ValueDataDraft as Data,
    };
    use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramExecutionError as Error};
    for (width, word) in [
        (8, Data::U8(0xf3)),
        (16, Data::U16(0xf123)),
        (32, Data::U32(0xf0123456)),
        (64, Data::U64(0xf0123456789abcde)),
    ] {
        let source = format!("[word<u{width}> | i <- 0u64..=count<u64>, i < count]\n");
        for artifact in artifacts(&source).0 {
            assert_eq!(
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog())
                    .unwrap()
                    .facts()
                    .uniform_unsigned_collections()
                    .len(),
                1
            );
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for count in [0, 3, 31, 2] {
                let args = inputs(
                    &artifact,
                    &[("word", word.clone()), ("count", Data::U64(count))],
                    &memory,
                );
                let report = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                assert!(
                    report.visits < 32,
                    "uniform construction must not visit each element: {report:?}"
                );
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(vec![word.clone(); count as usize].into())
                );
                if count <= 3 {
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        reference(&artifact, &args)
                            .last()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap()
                    );
                }
                if count == 31 {
                    let prior = output.clone();
                    let retained = memory.used_bytes();
                    let mut denied = limits();
                    denied.library.max_cloned_bytes = Some(count * (width / 8) - 1);
                    let error = jit
                        .evaluate_into(&args, &mut output, denied, &memory)
                        .unwrap_err();
                    assert!(
                        matches!(error,Error::Collection {error:ResidentKernelError::ResourceLimit {
                        dimension:Dimension::ClonedBytes,required,..},..} if required==count*(width/8)),
                        "{error:?}"
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    // The first write exceeds this output cap, before a later
                    // write could exhaust the simultaneously tighter copy cap.
                    denied.library.max_output_bytes = Some(1);
                    let error = jit
                        .evaluate_into(&args, &mut output, denied, &memory)
                        .unwrap_err();
                    assert!(
                        matches!(
                            error,
                            Error::Collection {
                                error: ResidentKernelError::ResourceLimit {
                                    dimension: Dimension::OutputBytes,
                                    ..
                                },
                                ..
                            }
                        ),
                        "{error:?}"
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn uniform_constructor_keeps_range_overflow_before_output_rejection() {
    use super::{inputs, limits};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramExecutionError as Error};
    for artifact in artifacts("[7u32 | i <- 0u64..=count<u64>, i < count]\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for count in [3, u64::MAX, 2] {
            let args = inputs(&artifact, &[("count", Data::U64(count))], &memory);
            if count == u64::MAX {
                let prior = output.clone();
                let retained = memory.used_bytes();
                let mut denied = limits();
                denied.library.max_output_bytes = Some(1);
                let error = jit
                    .evaluate_into(&args, &mut output, denied, &memory)
                    .unwrap_err();
                assert!(matches!(error, Error::Library { .. }), "{error:?}");
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(vec![Data::U32(7); count as usize].into())
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn uniform_optimizer_leaves_selected_assertions_in_the_ordinary_body() {
    use super::{inputs, limits};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramExecutionError as Error};
    let source = "permit := allowed<bool>\nchecked(x<u32>) => (out<u32>) :=\n  permitted! := permit\n  out := x.\n[checked(9u32) | i <- 0u64..=count<u64>, i < count]\n";
    for artifact in artifacts(source).0 {
        assert!(
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .facts()
                .uniform_unsigned_collections()
                .is_empty()
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (allowed, count) in [(true, 3), (false, 3), (false, 0), (true, 2)] {
            let args = inputs(
                &artifact,
                &[
                    ("allowed", Data::Bool(allowed)),
                    ("count", Data::U64(count)),
                ],
                &memory,
            );
            let prior = output.clone();
            let retained = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            if !allowed && count != 0 {
                assert!(
                    matches!(
                        result,
                        Err(Error::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Integrity { .. }
                        ))
                    ),
                    "{result:?}"
                );
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(vec![Data::U32(9); count as usize].into())
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn repeated_uniform_calls_read_the_current_word_and_count() {
    use super::{inputs, limits, reference};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    use mech_gpu::native_artifact::JitProgramArtifact;
    let source = "repeat(word<u32>, count<u64>) => (out<[u32]>) :=\n  out := [word | i <- 0u64..=count, i < count].\nfirst := repeat(word<u32>, count<u64>)\nsecond := repeat(other<u32>, count + 1u64)\n{first: first, second: second}\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (word, count) in [(7, 31), (19, 2)] {
            let args = inputs(
                &artifact,
                &[
                    ("word", Data::U32(word)),
                    ("other", Data::U32(word + 1)),
                    ("count", Data::U64(count)),
                ],
                &memory,
            );
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert!(report.visits < 128, "{report:?}");
            let actual = output
                .iter()
                .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let Data::Record(fields) = actual.last().unwrap() else {
                panic!("pair result")
            };
            let matrices = fields.iter().map(|f| &f.value).collect::<Vec<_>>();
            assert_eq!(
                matrices,
                vec![
                    &Data::Matrix(vec![Data::U32(word); count as usize].into()),
                    &Data::Matrix(vec![Data::U32(word + 1); (count + 1) as usize].into())
                ]
            );
            if count <= 3 {
                assert_eq!(
                    actual,
                    reference(&artifact, &args)
                        .iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>()
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
