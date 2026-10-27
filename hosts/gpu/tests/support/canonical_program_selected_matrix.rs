//! Computed selected extents retain contents, geometry, and atomic publication.
use super::{artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn concatenated_row_branch_preserves_selected_contents() {
    let source = "choose(values<[u64]>, padding<[u64]>, flag<bool>) => (out<[u64]>) :=\n  out := (flag? | true => [values padding] | false => values).\nchoose(values<[u64]>, padding<[u64]>, flag<bool>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for flag in [true, false, true] {
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    (
                        "values",
                        Data::Matrix(vec![Data::U64(7), Data::U64(9)].into()),
                    ),
                    ("padding", Data::Matrix(vec![Data::U64(0)].into())),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let expected = if flag {
                vec![Data::U64(7), Data::U64(9), Data::U64(0)]
            } else {
                vec![Data::U64(7), Data::U64(9)]
            };
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.into())
            );
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_computed_extent_is_lazy_and_recovers_without_republishing_on_failure() {
    for reversed in [false, true] {
        let arms = if reversed {
            "false => values | true => ((1u64 / divisor == 1u64)? | true => [values padding] | false => [values padding])"
        } else {
            "true => ((1u64 / divisor == 1u64)? | true => [values padding] | false => [values padding]) | false => values"
        };
        let source = format!(
            "choose(values<[u64]>, padding<[u64]>, flag<bool>, divisor<u64>) => (out<[u64]>) :=\n  out := (flag? | {arms}).\nchoose(values<[u64]>, padding<[u64]>, flag<bool>, divisor<u64>)\n"
        );
        let (forms, last) = artifacts(&source);
        for artifact in forms {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (flag, divisor, words) in [
                (true, 1, vec![7, 9]),
                (true, 0, vec![7, 9]),
                (false, 0, vec![]),
                (true, 1, vec![]),
                (false, 1, vec![11]),
            ] {
                let mut expected = words.iter().copied().map(Data::U64).collect::<Vec<_>>();
                if flag {
                    expected.push(Data::U64(1));
                }
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "values",
                            Data::Matrix(words.into_iter().map(Data::U64).collect()),
                        ),
                        ("padding", Data::Matrix(vec![Data::U64(1)].into())),
                        ("flag", Data::Bool(flag)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let previous = output.clone();
                let retained = memory.used_bytes();
                let reference = reference_result(&artifact, &args);
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                if flag && divisor == 0 {
                    assert!(result.is_err());
                    assert!(reference.is_err());
                    assert!(output.iter().zip(&previous).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(memory.used_bytes(), retained);
                } else {
                    result.unwrap();
                    let reference = reference.unwrap();
                    assert_eq!(
                        reference[last].canonical_data_draft().unwrap(),
                        Data::Matrix(expected.clone().into())
                    );
                    assert_eq!(
                        output[last]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(expected.into())
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                drop((args, aliases, previous));
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn selected_computed_extent_keeps_fixed_geometry_and_element_errors() {
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    for source in [
        "(flag<bool>? | true => [1u64 2u64] | false => [3u64; 4u64])\n",
        "(flag<bool>? | true => [3u64; 4u64] | false => [1u64 2u64])\n",
        "(flag<bool>? | true => [values<[u64]> padding<[u64]>] | false => floating<[f64]>)\n",
        "(flag<bool>? | true => floating<[f64]> | false => [values<[u64]> padding<[u64]>])\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x53454c), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let error = CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
            .err()
            .expect("incompatible branch kinds must remain rejected");
        assert_eq!(
            error.code,
            "source-semantics/incompatible-match-result-kind"
        );
    }
}

#[test]
fn already_matching_computed_results_do_not_add_conversion_calls() {
    use mech_engine::native_artifact::{NativeCollectionPrimitive, NativeIterationAction};
    let source =
        "(flag<bool>? | true => [values<[u64]> padding<[u64]>] | false => [values padding])\n";
    let (forms, last) = artifacts(source);
    for artifact in forms {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(!jit.descriptor().plan().blocks().iter().any(|block| matches!(block.action(), NativeIterationAction::LibraryCall { template, .. } if template.primitive() == NativeCollectionPrimitive::MatrixConversion)));
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        for flag in [false, true] {
            let args = inputs(
                &artifact,
                &[
                    ("values", Data::Matrix(vec![Data::U64(7)].into())),
                    ("padding", Data::Matrix(vec![Data::U64(9)].into())),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output[last]
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(vec![Data::U64(7), Data::U64(9)].into())
            );
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
