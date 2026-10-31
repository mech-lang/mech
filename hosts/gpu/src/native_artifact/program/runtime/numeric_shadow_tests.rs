//! Scratch reuse keeps readiness, fuel, arguments and views invocation-current.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramDescriptor, NativeProgramLimitMode};
use std::collections::BTreeSet;

const SOURCE: &str = "walk(words<[u32]>, left<u64>, total<u64>, expected<u64>, at<u64>, allowed<bool>) => (out<bool>) :=\n  allowed! := allowed\n  more := left > 0u64\n  value<u64> := (more? | true => words[at] | false => 0u32)\n  out := (more? | true => walk(words, left - 1u64, total + value, expected, at, allowed) | false => total == expected).\nfirst := walk(a<[u32]>, steps<u64>, 0u64, expected<u64>, 1u64, true)\nsecond := walk(b<[u32]>, other<u64>, seed<u64>, expected-other<u64>, at<u64>, allowed<bool>)\n(first, second)\n";

fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(SOURCE);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn numeric_regions_share_only_scratch_and_keep_independent_flags() {
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let layout = &jit.descriptor().metadata().closed_tails;
        let encoded = serde_json::to_value(layout).unwrap();
        let numeric = layout
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.numeric.is_some())
            .collect::<Vec<_>>();
        assert!(
            numeric.len() >= 2,
            "requires multiple selected native regions"
        );
        let shadows = numeric
            .iter()
            .map(|(i, r)| {
                encoded["regions"][i]["shadow"]
                    .as_u64()
                    .map_or(r.words + 3, |n| n as usize)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            shadows.len(),
            1,
            "nonoverlapping numeric lifetimes must share scratch"
        );
        let flags = numeric
            .iter()
            .flat_map(|(_, r)| r.words..r.words + 3)
            .collect::<BTreeSet<_>>();
        assert_eq!(flags.len(), numeric.len() * 3);
        let shadow = *shadows.first().unwrap();
        let maximum = numeric
            .iter()
            .map(|(_, r)| r.numeric.as_ref().unwrap().words)
            .max()
            .unwrap();
        assert!((shadow..shadow + maximum).all(|word| !flags.contains(&word)));
    }
}

#[test]
fn shared_numeric_image_rederives_offsets_and_rejects_aliased_flags() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let ids = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| !r["numeric"].is_null())
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert!(ids.len() >= 2);
        assert!(
            regions[ids[0]]["shadow"].is_number(),
            "shared scratch needs an explicit offset"
        );
        // SAFETY: unchanged same-build descriptor, confined to this JIT owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(
            serde_json::to_value(&restored.metadata().closed_tails).unwrap(),
            image["descriptor"]["closed_tails"]
        );
        for field in ["shadow", "words"] {
            let mut changed = image.clone();
            let old = changed["descriptor"]["closed_tails"]["regions"][ids[1]][field]
                .as_u64()
                .unwrap();
            changed["descriptor"]["closed_tails"]["regions"][ids[1]][field] = (old + 1).into();
            let changed = serde_json::to_vec(&changed).unwrap();
            // SAFETY: wire-only diagnostic; no malformed result executes.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err()
            );
        }
    }
}

type Case = (u32, u32, u64, u64, u64, u64, u64, u64, bool);
fn inputs(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    row: Case,
) -> Vec<Value> {
    let (a, b, steps, other, seed, expected, target, at, allowed) = row;
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "a" | "b" => ValueDataDraft::Matrix(
                    vec![ValueDataDraft::U32(if name == "a" { a } else { b })].into(),
                ),
                "steps" => ValueDataDraft::U64(steps),
                "other" => ValueDataDraft::U64(other),
                "seed" => ValueDataDraft::U64(seed),
                "expected" => ValueDataDraft::U64(expected),
                "expected-other" => ValueDataDraft::U64(target),
                "at" => ValueDataDraft::U64(at),
                "allowed" => ValueDataDraft::Bool(allowed),
                _ => panic!("unexpected {name}"),
            };
            let shape_values = if matches!(data, ValueDataDraft::Matrix(_)) {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, 1],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut claim = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut claim).unwrap()
        })
        .collect()
}

#[test]
fn shared_scratch_refreshes_views_arguments_and_recovers_after_failure() {
    for artifact in artifacts() {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let mut published = vec![None; artifact.outputs().len()];
        for (row, result, error) in [
            ((3, 7, 2, 3, 0, 6, 21, 1, true), Some((true, true)), None),
            ((4, 5, 1, 2, 1, 4, 99, 1, true), Some((true, false)), None),
            ((4, 5, 1, 2, 1, 4, 99, 0, false), None, Some("Integrity")),
            ((4, 5, 1, 2, 1, 4, 99, 0, true), None, None),
            (
                (4, 7, 1, 1, u64::MAX, 4, 99, 1, true),
                None,
                Some("Arithmetic"),
            ),
            ((4, 5, 1, 0, 9, 4, 9, 0, true), Some((true, true)), None),
            ((9, 2, 3, 4, 0, 27, 8, 1, true), Some((true, true)), None),
        ] {
            let args = inputs(&artifact, &memory, row);
            let aliases = args.clone();
            let contents = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = published.clone();
            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let actual = jit.evaluate_into(&args, &mut published, mode.execution(), &memory);
            if let Some((a, b)) = result {
                actual.unwrap();
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    ValueDataDraft::Tuple(
                        vec![ValueDataDraft::Bool(a), ValueDataDraft::Bool(b)].into()
                    )
                );
            } else {
                let actual = format!("{:?}", actual.unwrap_err());
                if let Some(error) = error {
                    assert!(actual.contains(error), "{actual}");
                } else {
                    assert!(
                        !actual.contains("Integrity") && !actual.contains("Arithmetic"),
                        "load must retain its rejection: {actual}"
                    );
                }
                for (value, old) in published.iter().zip(&prior) {
                    assert!(
                        value
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    );
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    held
                );
            }
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                contents
            );
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
