//! Sole-consumer transposes retain exact kinds, original frontiers and owners.
use super::numeric_maps::{Flags, run, typed_arguments};
use super::*;
use mech_core::snapshot::F64Bits;
fn source(kind: &str) -> String {
    format!("[value | value <- words<[{kind}]>]'\n")
}
fn data(out: &[Option<Value>]) -> Vec<Data> {
    contents(&out.iter().flatten().cloned().collect::<Vec<_>>())
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    extents: &[u64],
) -> Vec<Value> {
    typed_arguments(
        artifact,
        memory,
        &[("words", Data::Matrix(cells.to_vec().into()), extents)],
    )
}
#[test]
fn numeric_map_transposes_keep_all_scalar_kinds_bits_empty_shapes_and_exact_publication() {
    for (kind, cells, _) in super::typed_numeric_phases::cases() {
        for artifact in artifacts(&source(kind)) {
            let catalog = mech_stdlib::source_catalog();
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let proof = jit
                .descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .find_map(|r| r.transpose.as_ref())
                .expect("sole scalar row transpose");
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            for restored in [false, true] {
                for (cells, extents) in [
                    (vec![], vec![1, 0]),
                    (cells[..1].to_vec(), vec![1, 1]),
                    (cells.clone(), vec![2, 3]),
                ] {
                    let memory = mode.memory_budget();
                    let args = arguments(&artifact, &memory, &cells, &extents);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    for numeric_only in [false, true] {
                        let mut fast = vec![None; artifact.outputs().len()];
                        let mut cold = fast.clone();
                        let (a, counts) = run(
                            &jit,
                            &args,
                            &mut fast,
                            &memory,
                            mode.execution(),
                            restored,
                            Flags {
                                numeric_only,
                                ..Default::default()
                            },
                        );
                        let (b, _) = run(
                            &jit,
                            &args,
                            &mut cold,
                            &memory,
                            mode.execution(),
                            restored,
                            Flags {
                                ordinary: true,
                                ..Default::default()
                            },
                        );
                        a.unwrap();
                        b.unwrap();
                        assert_eq!(data(&fast), data(&cold), "{kind} {extents:?}");
                        assert_eq!(
                            counts.transpose_complete, 1,
                            "actual direct column publication {counts:?}"
                        );
                        assert_eq!(
                            counts
                                .helpers
                                .get(&(u64::from(proof.transpose), GlobalHelper::Library.raw()))
                                .copied()
                                .unwrap_or(0),
                            0
                        );
                        let out = fast.last().unwrap().as_ref().unwrap();
                        let schema = out.validate_against(artifact.schemas()).unwrap();
                        let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                            panic!("matrix")
                        };
                        assert_eq!(
                            out.shape().resolve_dimension(&dimensions[0]).unwrap(),
                            cells.len() as u64
                        );
                        assert_eq!(out.shape().resolve_dimension(&dimensions[1]).unwrap(), 1);
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        drop((fast, cold));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                    drop((args, aliases));
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
fn numeric_map_transposes_keep_nine_finite_dimensions_distinct_atomic_outputs_cleanup_and_retry() {
    let cells = [1., 2., 3.].map(|x| Data::F64(F64Bits::from_f64(x)));
    let old_cells = [9., 8.].map(|x| Data::F64(F64Bits::from_f64(x)));
    for artifact in artifacts(&source("f64")) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = arguments(&artifact, &memory, &cells, &[1, 3]);
        let before = contents(&args);
        let old_args = arguments(&artifact, &memory, &old_cells, &[1, 2]);
        let mut prior = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &old_args,
            &mut prior,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        drop(old_args);
        let previous = data(&prior);
        let mut oracle = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &args,
            &mut oracle,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        let expected = data(&oracle);
        assert_ne!(previous, expected);
        for restored in [false, true] {
            for dimension in 0..9 {
                for cap in [0, 1, 3, 16, 255, 4095, 65535, 1_000_000, 100_000_000] {
                    let mut policy = mode.execution();
                    match dimension {
                        0 => policy.library.max_output_elements = Some(cap),
                        1 => policy.library.max_output_bytes = Some(cap),
                        2 => policy.library.max_temporary_bytes = Some(cap),
                        3 => policy.library.max_cloned_bytes = Some(cap),
                        4 => policy.library.max_retained_nodes = Some(cap),
                        5 => policy.library.max_compute_work = Some(cap),
                        6 => policy.library.max_comparison_work = Some(cap),
                        7 => policy.execution.max_work = cap,
                        _ => policy.execution.max_transport_bytes = cap,
                    }
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = prior.clone();
                    let mut cold = prior.clone();
                    let (a, _) = run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        policy,
                        restored,
                        Flags {
                            numeric_only: true,
                            ..Default::default()
                        },
                    );
                    let (b, _) = run(
                        &jit,
                        &args,
                        &mut cold,
                        &memory,
                        policy,
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    if dimension < 2 {
                        assert_eq!(
                            a.is_ok(),
                            b.is_ok(),
                            "logical constructor {dimension} {cap}: {a:?}/{b:?}"
                        );
                    }
                    assert_eq!(
                        data(&fast),
                        if a.is_ok() {
                            expected.clone()
                        } else {
                            previous.clone()
                        },
                        "atomic finite output {dimension} {cap}: {a:?}"
                    );
                    assert_eq!(
                        data(&cold),
                        if b.is_ok() {
                            expected.clone()
                        } else {
                            previous.clone()
                        }
                    );
                    drop((fast, cold));
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            let mut retry = vec![None; artifact.outputs().len()];
            let (a, c) = run(
                &jit,
                &args,
                &mut retry,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    numeric_only: true,
                    ..Default::default()
                },
            );
            a.unwrap();
            assert_eq!(c.transpose_complete, 1);
            assert_eq!(data(&retry), expected);
        }
        drop((args, prior, oracle));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_map_transposes_keep_partial_errors_inactive_reads_and_nested_iteration_versions() {
    let source = "data := words<[f64]:2,3>\nlast := end<u64>\n[([data[j,i] | i <- 1u64..=last]') + 0.5 | j <- 1u64..=2u64]\n";
    let conditional = "data := words<[f64]:2,3>\nlast := end<u64>\nchosen := row<u64>\nenabled := gate<bool>\n[((enabled? | true => data[chosen,i] | false => data[1u64,i])) | i <- 1u64..=last]'\n";
    let cells = [1., 2., 3., 4., 5., 6.].map(|x| Data::F64(F64Bits::from_f64(x)));
    for nested in [false, true] {
        for artifact in artifacts(if nested { source } else { conditional }) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .any(|r| r.transpose.is_some())
            );
            for restored in [false, true] {
                for (end, gate, row, error) in [
                    (3, false, 0, false),
                    (3, true, 2, false),
                    (4, true, 2, true),
                    (3, true, 3, true),
                    (3, false, 0, false),
                ] {
                    if nested && row == 3 {
                        continue;
                    }
                    let mut named = vec![
                        ("words", Data::Matrix(cells.to_vec().into()), &[2, 3][..]),
                        ("end", Data::U64(end), &[][..]),
                    ];
                    if !nested {
                        named.extend([
                            ("row", Data::U64(row), &[][..]),
                            ("gate", Data::Bool(gate), &[][..]),
                        ]);
                    }
                    let args = typed_arguments(&artifact, &memory, &named);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let before = contents(&args);
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, c) = run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            numeric_only: true,
                            ..Default::default()
                        },
                    );
                    let (b, _) = run(
                        &jit,
                        &args,
                        &mut cold,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    assert_eq!(a.is_err(), error, "{a:?}");
                    assert_eq!(a.is_err(), b.is_err());
                    if error {
                        assert_eq!(
                            format!("{:?}", a.err()),
                            format!("{:?}", b.err()),
                            "original partial bounds error"
                        );
                        assert_eq!(c.transpose_complete, 0);
                    } else {
                        assert_eq!(
                            c.transpose_complete,
                            if nested { 2 } else { 1 },
                            "real repeated projection {c:?}"
                        );
                    }
                    assert_eq!(data(&fast), data(&cold));
                    assert_eq!(contents(&args), before);
                    drop((fast, cold));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
        }
    }
}
#[test]
fn numeric_map_transposes_rederive_exact_table_proofs_and_refuse_forged_publication() {
    use serde::Deserialize;
    for artifact in artifacts(&source("f64")) {
        let catalog = mech_stdlib::source_catalog();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let original = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        for change in 0..7 {
            let mut forged = original.clone();
            let region = forged["descriptor"]["numeric_maps"]["regions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| !r["transpose"].is_null())
                .unwrap();
            let proof = &mut region["transpose"];
            match change {
                0 => proof["finish"] = 999.into(),
                1 => proof["continuation"][0] = 999.into(),
                2 => proof["transpose"] = 999.into(),
                3 => proof["output"] = 999.into(),
                4 => proof["elided_values"][0] = 999.into(),
                5 => region["transpose"] = serde_json::Value::Null,
                _ => region["output_width"] = 1.into(),
            }
            let bytes = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&forged).unwrap(),
            )
            .unwrap();
            // SAFETY: diagnostic decode only; forged metadata is never executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "forged column publication {change}"
            );
        }
    }
}
#[test]
fn numeric_map_transposes_reject_wrong_helper_frontiers_and_keep_escaped_or_shaped_sources_ordinary()
 {
    for artifact in artifacts(&source("f64")) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let cells = [1., 2., 3.].map(|x| Data::F64(F64Bits::from_f64(x)));
        let args = arguments(&artifact, &memory, &cells, &[1, 3]);
        let mut oracle = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &args,
            &mut oracle,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        for restored in [false, true] {
            for corrupt in [
                1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 17, 18, 19, 20, 22, 23, 24,
            ] {
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut out = vec![None; artifact.outputs().len()];
                let (a, _) = run(
                    &jit,
                    &args,
                    &mut out,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert!(a.is_err(), "mutation {corrupt}: {a:?}");
                assert!(out.iter().all(Option::is_none));
                drop(out);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            let mut out = vec![None; artifact.outputs().len()];
            let (a, c) = run(
                &jit,
                &args,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    interleave: true,
                    ..Default::default()
                },
            );
            a.unwrap();
            assert_eq!(c.transpose_complete, 1);
            assert_eq!(c.interleaved_refused, 4);
            assert_eq!(data(&out), data(&oracle));
        }
    }
    for text in [
        "mapped := [x | x <- words<[f64]>]\n(mapped,mapped')\n",
        "[[x + y | x <- 1u64..=2u64] | y <- 1u64..=3u64]'\n",
    ] {
        for artifact in artifacts(text) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .all(|r| r.transpose.is_none()),
                "{text}"
            );
        }
    }
}

#[test]
fn numeric_map_transposes_keep_real_activation_checkpoint_and_restored_completion() {
    for artifact in artifacts("[0.0 | i <- 1u64..=3u64]'\n") {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        assert!(!metadata.plan.activation_nodes().is_empty());
        let boundary = metadata
            .plan
            .activation_boundary()
            .expect("real activation checkpoint");
        let projection = metadata
            .numeric_maps
            .regions
            .iter()
            .find_map(|r| r.transpose.as_ref())
            .expect("activation scalar map projection");
        assert_ne!(boundary, projection.finish);
        assert!(!projection.continuation.contains(&boundary));
        for restored in [false, true] {
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            let (a, counts) = run(
                &jit,
                &[],
                &mut fast,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    numeric_only: true,
                    ..Default::default()
                },
            );
            let (b, _) = run(
                &jit,
                &[],
                &mut cold,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            );
            a.unwrap();
            b.unwrap();
            assert_eq!(counts.transpose_complete, 1);
            assert_eq!(data(&fast), data(&cold));
            drop((fast, cold));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
