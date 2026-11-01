//! A finished ordinary generator is finite; shape finish and live aliases stay.
use super::*;
use mech_engine::native_artifact::{NativeCollectionPrimitive, NativeControlLoop};

#[test]
fn ordinary_collection_source_lifetime_keeps_finish_later_reads_aliases_and_retry() {
    const SOURCE: &str =
        include_str!("../../../../../tests/fixtures/native/collection-source-lifetime.mec");
    for later_read in [false, true] {
        let source = format!("{SOURCE}{}", if later_read { "words\n" } else { "" });
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let plan = jit.descriptor().plan();
            let generator = plan
                .iterations()
                .iter()
                .find_map(|g| match g {
                    NativeControlLoop::Collection(g) => Some(g),
                    _ => None,
                })
                .unwrap();
            let old_source = generator.source().materialized_value().unwrap();
            let calls = plan.unsigned_buffer_calls();
            let update = plan
                .blocks()
                .iter()
                .enumerate()
                .find_map(|(site, b)| {
                    matches!(b.action(), NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == NativeCollectionPrimitive::MatrixIndexedReplace)
                    .then_some(site)
                })
                .unwrap();
            assert_eq!(
                calls[update]
                    .as_ref()
                    .unwrap()
                    .owned_source_slots()
                    .contains(&old_source),
                !later_read,
                "only the finished source with no later reader may retire"
            );
            let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
            assert_eq!(
                restored.metadata().unsigned_buffer_calls[update]
                    .as_ref()
                    .unwrap()
                    .owned_source_slots(),
                calls[update].as_ref().unwrap().owned_source_slots()
            );
            for descriptor in [jit.descriptor(), &restored] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (data, slide, allowed, expected_error) in [
                    (vec![0u16, 32768, 65535], true, true, None),
                    (vec![4, 5], false, true, None),
                    (vec![], true, false, Some("Integrity")),
                    (vec![], true, true, Some("Library")),
                    (vec![32769, 65535], true, true, None),
                    (vec![7], false, true, None),
                ] {
                    let original = Data::Matrix(data.iter().copied().map(Data::U16).collect());
                    let args = inputs(
                        &artifact,
                        &[
                            ("words", original.clone()),
                            ("slide", Data::Bool(slide)),
                            ("allowed", Data::Bool(allowed)),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = observe::run_descriptor(
                        &jit,
                        descriptor,
                        &args,
                        &mut published,
                        mode.execution(),
                        &memory,
                    );
                    if let Some(error) = expected_error {
                        assert!(format!("{:?}", result.unwrap_err()).contains(error));
                        assert!(published.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        let mut expected = data
                            .iter()
                            .map(|&n| if slide { n.saturating_sub(32768) } else { n })
                            .collect::<Vec<_>>();
                        expected[0] = 9;
                        assert_eq!(
                            published[0]
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(expected.into_iter().map(Data::U16).collect())
                        );
                        if later_read {
                            assert_eq!(
                                published
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                original
                            );
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
