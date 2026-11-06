//! Pointwise source functions keep their complete scalar CFG inside the loop.
use super::*;
use mech_engine::native_artifact::{NativeCollectionSource, NativeControlLoop};

const SOURCE: &str = r#"
shift(value<u16>, threshold<u64>, enabled<bool>) => (out<u16>) :=
  wide<u64> := value
  shifted := ((wide < threshold)? | true => 0u64 | false => wide - threshold)
  out<u16> := (enabled? | true => shifted | false => wide).
transform(values<[u16]>, threshold<u64>, enabled<bool>) => (out<[u16]>) :=
  out := [shift(value, threshold, enabled) | value <- values].
answer := transform(input<[u16]>, threshold<u64>, enabled<bool>)
answer
"#;

#[test]
fn pointwise_unsigned_branches_run_without_per_element_runtime_callbacks() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let generator = metadata
            .plan
            .iterations()
            .iter()
            .find_map(|control| match control {
                NativeControlLoop::Collection(generator)
                    if matches!(generator.source(), NativeCollectionSource::Value(_)) =>
                {
                    Some(generator)
                }
                _ => None,
            })
            .unwrap();
        let push = metadata.plan.collections()[generator.collection() as usize].push();
        assert!(
            metadata
                .word_collections
                .iter()
                .any(|region| region.collection == generator.collection()),
            "total map proof must be reconstructed: {:?}",
            metadata.word_collections
        );
        let memory = mode.memory_budget();
        let cells = [0u16, 1, 32767, 32768, 65534, 65535];
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (threshold, enabled) in [
                (32768, true),
                (1, true),
                (u64::MAX, true),
                (5, false),
                (0, true),
                (1, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("input", Data::Matrix(cells.map(Data::U16).into())),
                        ("threshold", Data::U64(threshold)),
                        ("enabled", Data::Bool(enabled)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                let expected = cells
                    .into_iter()
                    .map(|value| {
                        Data::U16(if enabled {
                            u64::from(value).saturating_sub(threshold) as u16
                        } else {
                            value
                        })
                    })
                    .collect();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected)
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                for site in [generator.body(), push] {
                    assert_eq!(
                        counts
                            .helpers
                            .get(&(u64::from(site), GlobalHelper::Collection.raw()))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "the complete pointwise scalar computation must use the admitted source/sink views, not a callback per element: {counts:?}"
                    );
                }
            }
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn unsigned_word_collection_widths_extents_and_work_match_ordinary_execution() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for (kind, maximum, cell) in [
        (
            "u8",
            u8::MAX as u64,
            (|v| Data::U8(v as u8)) as fn(u64) -> Data,
        ),
        (
            "u16",
            u16::MAX as u64,
            (|v| Data::U16(v as u16)) as fn(u64) -> Data,
        ),
        (
            "u32",
            u32::MAX as u64,
            (|v| Data::U32(v as u32)) as fn(u64) -> Data,
        ),
        ("u64", u64::MAX, Data::U64 as fn(u64) -> Data),
    ] {
        for artifact in artifacts(&SOURCE.replace("u16", kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert_eq!(native.descriptor().metadata().word_collections.len(), 1);
            let memory = mode.memory_budget();
            for length in [0, 1, 3, 4, 5, 16, 17] {
                let cells: Vec<_> = (0..length)
                    .map(|i| [0, 1, maximum / 2, maximum][i % 4])
                    .collect();
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(cells.iter().copied().map(cell).collect()),
                        ),
                        ("threshold", Data::U64(1)),
                        ("enabled", Data::Bool(true)),
                    ],
                    &memory,
                );
                let before = contents(&args);
                let mut work = None;
                for jit in [&ordinary, &native] {
                    for restored in [false, true] {
                        let mut output = vec![None; artifact.outputs().len()];
                        let (result, _) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut output, mode.execution(), &memory)
                        };
                        let report = result.unwrap();
                        assert_eq!(
                            report.work,
                            *work.get_or_insert(report.work),
                            "{kind}, {length}, restored={restored}"
                        );
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(cells.iter().map(|v| cell(v.saturating_sub(1))).collect())
                        );
                        assert_eq!(contents(&args), before);
                    }
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn fallible_word_collection_keeps_ordinary_errors_publication_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let unguarded = SOURCE.replace(
        "((wide < threshold)? | true => 0u64 | false => wide - threshold)",
        "wide - threshold",
    );
    let narrowed = SOURCE
        .replace("out<u16>", "out<u8>")
        .replace("out<[u16]>", "out<[u8]>");
    for (source, bad_threshold, bad_cells) in [
        (unguarded, 1, vec![0u16, 2]),
        (narrowed, 0, vec![256u16, 65535]),
    ] {
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert!(native.descriptor().metadata().word_collections.is_empty());
            let mut expected_error = None;
            for jit in [&ordinary, &native] {
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut output = vec![None; artifact.outputs().len()];
                    for fail in [false, true, false] {
                        let cells = if fail { &bad_cells[..] } else { &[1, 2][..] };
                        let args = inputs(
                            &artifact,
                            &[
                                (
                                    "input",
                                    Data::Matrix(cells.iter().copied().map(Data::U16).collect()),
                                ),
                                ("threshold", Data::U64(if fail { bad_threshold } else { 0 })),
                                ("enabled", Data::Bool(true)),
                            ],
                            &memory,
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, _) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut output, mode.execution(), &memory)
                        };
                        if fail {
                            let error = format!("{:?}", result.unwrap_err());
                            assert_eq!(error, *expected_error.get_or_insert(error.clone()));
                            for (value, old) in output.iter().zip(&prior) {
                                assert!(
                                    value
                                        .as_ref()
                                        .unwrap()
                                        .shares_frozen_storage(old.as_ref().unwrap())
                                );
                            }
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            result.unwrap();
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
        }
    }
}

#[test]
fn word_collection_finite_boundaries_keep_publication_and_recovery() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        inputs(
            artifact,
            &[
                (
                    "input",
                    Data::Matrix([0u16, 1, 32768, 65535, 17].map(Data::U16).into()),
                ),
                ("threshold", Data::U64(32768)),
                ("enabled", Data::Bool(true)),
            ],
            memory,
        )
    });
}

#[test]
fn word_collection_image_reconstructs_totality_and_exact_geometry() {
    for artifact in artifacts(SOURCE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        assert_eq!(
            image["descriptor"]["word_collections"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        for (path, value) in [
            ("initialize", serde_json::json!(0)),
            ("header", serde_json::json!(0)),
            ("source", serde_json::json!(u32::MAX)),
            ("input_width", serde_json::json!(8)),
            ("output_width", serde_json::json!(8)),
            ("visit_work", serde_json::json!(0)),
            ("sites", serde_json::json!([])),
            ("captures", serde_json::json!([])),
            ("words", serde_json::json!([])),
        ] {
            let mut bad = image.clone();
            bad["descriptor"]["word_collections"][0][path] = value;
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap()
                ),
                "{path}"
            );
        }
        let mut missing = image;
        missing["descriptor"]
            .as_object_mut()
            .unwrap()
            .remove("word_collections");
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&missing).unwrap()
        ));
    }
}
