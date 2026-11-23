//! Exit-only transport keeps retirement/scalar exports and ignores interior words.
use super::*;

#[test]
fn native_exit_store_inventory_omits_views_and_private_fields() {
    for artifact in artifacts(bounds_retry::SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let report = object.metadata();
        let metadata = object.descriptor().metadata();
        let mut removed = 0;
        for row in report["compiled_numeric_segments"].as_array().unwrap() {
            let entry = row["entry"].as_u64().unwrap() as usize;
            let (_, segment) = metadata
                .compositions
                .region(metadata.compositions.entry(entry).unwrap())
                .unwrap();
            let proof = segment.numeric.as_ref().unwrap();
            let frames = segment.frame_plan.as_ref().unwrap();
            let source_homes = row["source_home_words"].as_u64().unwrap() as usize;
            assert_eq!(
                row["private_scratch_words"].as_u64().unwrap() as usize,
                proof.words
            );
            assert_eq!(
                row["register_words"].as_u64().unwrap() as usize,
                source_homes + proof.words
            );
            let words = row["exit_argument_words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|word| word.as_u64().unwrap() as usize)
                .collect::<Vec<_>>();
            assert!(words.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(words.iter().all(|&word| word < proof.words));
            for word in frames.scalar_words..frames.scalar_words + proof.views.len() * 4 {
                assert!(
                    !words.contains(&word),
                    "numeric view is not consumed at exit"
                );
            }
            for word in proof.words - frames.retirements.len()..proof.words {
                assert!(
                    words.contains(&word),
                    "reached retirement must survive every failure"
                );
            }
            let boundaries = row["exit_boundaries"].as_u64().unwrap() as usize;
            let stores = row["exit_argument_stores"].as_u64().unwrap() as usize;
            assert!(boundaries > 0);
            assert_eq!(stores, words.len() * boundaries);
            removed += proof.words * boundaries - stores;
        }
        assert!(
            removed > 0,
            "the witness must remove actual emitted IR stores"
        );
    }
}

#[test]
fn unused_exit_words_cannot_affect_current_results_errors_or_recovery() {
    let mut modes = std::collections::BTreeSet::new();
    let mut expected_errors = [None, None];
    for artifact in artifacts(bounds_retry::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (length, steps, failure) in [
                (3, 2, None),
                (0, 3, Some((0, "Index"))),
                (5, 0, None),
                (1, u64::MAX, Some((1, "Arithmetic"))),
                (7, 4, None),
            ] {
                let args = bounds_retry::arguments(&artifact, &memory, length, steps);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = output.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, observed) = observe::run_with_unused_exit_poison(
                    &jit,
                    &args,
                    &mut output,
                    mode.execution(),
                    &memory,
                    restored,
                );
                assert!(
                    observed.poisoned_exit_words > 0,
                    "the witness must exercise omitted words"
                );
                assert!(observed.poisoned_private_homes > 0);
                assert_eq!(
                    observed.checked_private_homes,
                    observed.poisoned_private_homes
                );
                modes.extend(observed.exit_modes.keys().copied());
                if let Some((index, name)) = failure {
                    let text = format!("{:?}", result.unwrap_err());
                    assert!(text.contains(name), "{text}");
                    assert_eq!(text, *expected_errors[index].get_or_insert(text.clone()));
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    result.unwrap();
                    let mut expected = vec![Data::U8(97); length];
                    if length > 0 {
                        expected[0] = Data::U8(97 + (length as u8 + 1) * steps as u8);
                    }
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
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
    assert_eq!(
        modes,
        [0, 2, 3].into(),
        "success, checked-read retry and arithmetic exit must execute"
    );
}
