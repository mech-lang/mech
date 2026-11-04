//! Native segment temporaries have no physical owners until an actual export.
use super::*;
use std::collections::BTreeSet;

#[test]
fn private_segment_temporaries_keep_prices_publication_errors_and_retry_without_retirement() {
    let mut physical = 0;
    let mut omitted = 0;
    for artifact in artifacts(multiple_children::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let candidates = jit
            .descriptor()
            .metadata()
            .compositions
            .parents
            .iter()
            .flat_map(|parent| &parent.segments)
            .filter(|segment| segment.numeric.is_some())
            .flat_map(|segment| {
                let frames = segment.frame_plan.as_ref().unwrap();
                frames
                    .retirements
                    .iter()
                    .flat_map(|(site, values)| {
                        values.iter().filter_map(|&value| {
                            let private_frame =
                                frames.frames.iter().any(|frame| frame.value == value)
                                    && !frames.inputs.contains(&value);
                            let private_alias = frames
                                .aliases
                                .iter()
                                .any(|alias| alias.value.get() == value);
                            (private_frame || private_alias).then_some((*site, value))
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        assert!(
            !candidates.is_empty(),
            "the source must contain real private retirement candidates"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            let mut successes = 0;
            for allowed in [true, false, true] {
                let args = inputs(
                    &artifact,
                    &[
                        ("words", Data::Matrix(vec![Data::U64(7)].into())),
                        ("steps", Data::U64(3)),
                        ("flag", Data::Bool(false)),
                        ("first-allowed", Data::Bool(true)),
                        ("second-allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(1)),
                    ],
                    &memory,
                );
                let before = contents(&args);
                let aliases = args.clone();
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                assert!(counts.selected > 0);
                if allowed {
                    let report = result.unwrap();
                    assert_eq!(
                        report.work,
                        [197_450_724, 197_467_056][successes],
                        "retain the original charge for this publication state"
                    );
                    successes += 1;
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(
                            vec![Data::Matrix(vec![Data::U64(34)].into()), Data::Bool(true)].into()
                        )
                    );
                    physical += counts
                        .segment_retirements
                        .iter()
                        .filter(|(key, _)| candidates.contains(key))
                        .map(|(_, &count)| count)
                        .sum::<u64>();
                    omitted += counts.virtual_retirements.values().sum::<u64>();
                    println!(
                        "virtual-retirement-work={} physical={physical} omitted={omitted}",
                        report.work
                    );
                } else {
                    assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                    assert!(published.iter().zip(&prior).all(|(now, old)| {
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                drop((args, aliases, prior));
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
    assert_eq!(
        physical, 0,
        "private word/buffer bindings must not incur physical retirement"
    );
    assert!(
        omitted > 0,
        "the witness must actually reach the prepared virtual cleanup"
    );
}

#[test]
fn private_segment_cleanup_keeps_finite_denial_prefixes_and_retry() {
    resources::finite_boundaries(multiple_children::SOURCE, |artifact, memory| {
        inputs(
            artifact,
            &[
                ("words", Data::Matrix(vec![Data::U64(7)].into())),
                ("steps", Data::U64(3)),
                ("flag", Data::Bool(false)),
                ("first-allowed", Data::Bool(true)),
                ("second-allowed", Data::Bool(true)),
                ("divisor", Data::U64(1)),
            ],
            memory,
        )
    });
}

#[test]
fn private_segment_cleanup_is_rederived_and_cannot_be_supplied_as_image_authority() {
    for artifact in artifacts(multiple_children::SOURCE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        assert!(
            image["descriptor"].get("compositions").is_none(),
            "physical cleanup is a derived cache, not wire authority"
        );
        let mut recipe = serde_json::to_value(&jit.descriptor().metadata().compositions).unwrap();
        let mut changed = false;
        for parent in recipe["parents"].as_array_mut().unwrap() {
            for segment in parent["segments"].as_array_mut().unwrap() {
                if let Some(groups) = segment
                    .pointer_mut("/numeric/retirement_effects")
                    .and_then(|groups| groups.as_array_mut())
                {
                    for group in groups {
                        for step in group.as_array_mut().unwrap() {
                            if step.get("Pay").is_some() {
                                *step = serde_json::json!({"Pay":0});
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        assert!(changed, "the witness must have actual virtual cleanup");
        let mut bad = image;
        bad["descriptor"]["compositions"] = recipe;
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&bad).unwrap()
        ));
    }
}
