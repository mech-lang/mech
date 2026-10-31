use super::*;

#[test]
fn completed_child_cleanup_keeps_unentered_paths_returns_and_failure_retry() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let mut published = vec![None; artifact.outputs().len()];
        for (width, counts, start, expected) in [
            (3, vec![0, 1, 1, 2], 0, Some(vec![3, 1, 3, 2])),
            (0, vec![0, 1, 1, 2], u64::MAX, Some(vec![9, 9, 9, 9])),
            (3, vec![0, 0, 0, 0], 0, Some(vec![9, 9, 9, 9])),
            (1, vec![0, 1], u64::MAX, None),
            (2, vec![0, 1, 1], 0, Some(vec![1, 9, 2, 9])),
            (3, vec![0, 1, 1, 2], 0, Some(vec![3, 1, 3, 2])),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
                    ("lens", Data::Matrix([9, 9, 9, 9].map(Data::U8).into())),
                    (
                        "counts",
                        Data::Matrix(counts.into_iter().map(Data::U64).collect()),
                    ),
                    ("width", Data::U64(width)),
                    ("start", Data::U64(start)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let before = contents(&args);
            let prior = published.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            super::super::retirement::take_child_observation();
            let result = jit.evaluate_into(&args, &mut published, mode.execution(), &memory);
            let omitted = super::super::retirement::take_child_observation();
            if let Some(expected) = expected {
                result.unwrap();
                assert!(omitted > 0, "successful parent must use its child summary");
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into_iter().map(Data::U8).collect())
                );
            } else {
                assert!(format!("{:?}", result.unwrap_err()).contains("Arithmetic"));
                for (actual, old) in published.iter().zip(&prior) {
                    let (actual, old) = (actual.as_ref().unwrap(), old.as_ref().unwrap());
                    assert!(actual.shares_frozen_storage(old));
                    assert_eq!(
                        actual.canonical_data_draft().unwrap(),
                        old.canonical_data_draft().unwrap()
                    );
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
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
