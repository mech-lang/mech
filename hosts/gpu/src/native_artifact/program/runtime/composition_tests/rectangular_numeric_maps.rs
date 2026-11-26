//! Rectangular numeric source programs: native selection and exact fallback.
use super::*;

#[test]
fn rectangular_numeric_maps_execute_one_native_rectangle_and_match_ordinary_bits() {
    use super::numeric_maps::{Flags, run, typed_arguments};
    use mech_core::snapshot::F64Bits;
    let source =
        "data := values<[f64]:1,4>\n[[data[i] * data[j] | i <- 1u64..=3u64]' | j <- 1u64..=4u64]\n";
    let artifact = super::super::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    for artifact in [artifact, decoded] {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let rectangles = jit
            .descriptor()
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .filter(|r| r.rectangle.is_some())
            .collect::<Vec<_>>();
        assert_eq!(rectangles.len(), 1);
        let rectangle = rectangles[0];
        let mech_engine::native_artifact::NativeControlLoop::Collection(outer) =
            &jit.descriptor().plan().iterations()[rectangle.iteration as usize]
        else {
            panic!("rectangle outer source")
        };
        assert!(
            matches!(
                outer.source(),
                mech_engine::native_artifact::NativeCollectionSource::U64Range(_)
            ),
            "block source must use a cursor while the rectangle executes natively"
        );
        crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let cells = Data::Matrix(
                [1.25, -2.0, 3.5, 0.0]
                    .map(|x| Data::F64(F64Bits::from_f64(x)))
                    .into(),
            );
            let args = typed_arguments(&artifact, &memory, &[("values", cells, &[1, 4])]);
            let inputs = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut expected = vec![None; artifact.outputs().len()];
            let (reference, ordinary) = run(
                &jit,
                &args,
                &mut expected,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            );
            reference.unwrap();
            assert_eq!(ordinary.complete, 0);
            assert_eq!(ordinary.block_permutations, 1);
            assert_eq!(ordinary.direct_rectangles, 0);
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = run(
                &jit,
                &args,
                &mut output,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            );
            result.unwrap();
            assert_eq!(counts.complete, 1, "{counts:?}");
            assert_eq!(counts.selected, 1, "{counts:?}");
            assert_eq!(counts.block_permutations, 0, "{counts:?}");
            assert_eq!(counts.direct_rectangles, 1, "{counts:?}");
            for &site in &rectangle.body {
                assert_eq!(
                    counts
                        .helpers
                        .keys()
                        .filter(|(at, _)| *at == u64::from(site))
                        .count(),
                    0,
                    "per-cell helper at {site}"
                );
            }
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap()
            );
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                inputs
            );
            drop((output, expected));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            drop(args);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

fn snapshots(output: &[Option<Value>]) -> Vec<Option<(mech_core::SchemaId, Vec<u64>, Data)>> {
    output
        .iter()
        .map(|v| {
            v.as_ref().map(|v| {
                (
                    v.schema(),
                    v.shape().parameter_values().to_vec(),
                    v.canonical_data_draft().unwrap(),
                )
            })
        })
        .collect()
}
fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let artifact = super::super::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}

#[test]
fn rectangular_numeric_maps_all_exact_kinds_keep_branches_shapes_fresh_payloads_and_empty_sources()
{
    use super::numeric_maps::{Flags, run, typed_arguments};
    use mech_core::snapshot::F64Bits;
    let cases = [
        (
            "bool",
            vec![Data::Bool(true), Data::Bool(false), Data::Bool(true)],
        ),
        ("u8", vec![Data::U8(0), Data::U8(255), Data::U8(17)]),
        ("u16", vec![Data::U16(0), Data::U16(65535), Data::U16(17)]),
        (
            "u32",
            vec![Data::U32(0), Data::U32(u32::MAX), Data::U32(17)],
        ),
        (
            "u64",
            vec![Data::U64(0), Data::U64(u64::MAX), Data::U64(17)],
        ),
        (
            "i64",
            vec![Data::I64(i64::MIN), Data::I64(i64::MAX), Data::I64(-17)],
        ),
        (
            "f64",
            [0x8000_0000_0000_0000, 0x7ff8_0000_0000_1234, 1]
                .map(|x| Data::F64(F64Bits::from_bits(x)))
                .to_vec(),
        ),
    ];
    for (kind, cells) in cases {
        let source = format!(
            "data := values<[{kind}]:1,3>\n[[((j == 1u64)? | true => data[i] | false => data[4u64 - i]) | i <- 1u64..=3u64]' | j <- items<[u64]>]\n"
        );
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert_eq!(
                jit.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .filter(|r| r.rectangle.is_some())
                    .count(),
                1,
                "{kind}"
            );
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            for restored in [false, true] {
                for items in [vec![], vec![1u64], vec![2u64, 1, 2, 3]] {
                    for values in [cells.clone(), cells.iter().rev().cloned().collect()] {
                        let memory = mode.memory_budget();
                        let args = typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("values", Data::Matrix(values.into()), &[1, 3]),
                                (
                                    "items",
                                    Data::Matrix(items.iter().copied().map(Data::U64).collect()),
                                    &[1, items.len() as u64],
                                ),
                            ],
                        );
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let before = args
                            .iter()
                            .map(|v| v.canonical_data_draft().unwrap())
                            .collect::<Vec<_>>();
                        let mut reference = vec![None; artifact.outputs().len()];
                        run(
                            &jit,
                            &args,
                            &mut reference,
                            &memory,
                            mode.execution(),
                            restored,
                            Flags {
                                ordinary: true,
                                ..Default::default()
                            },
                        )
                        .0
                        .unwrap();
                        let mut output = vec![None; artifact.outputs().len()];
                        let (result, counts) = run(
                            &jit,
                            &args,
                            &mut output,
                            &memory,
                            mode.execution(),
                            restored,
                            Flags::default(),
                        );
                        result.unwrap();
                        assert_eq!(
                            counts.complete,
                            u64::from(!items.is_empty()),
                            "{kind} {counts:?}"
                        );
                        assert_eq!(counts.block_permutations, 0, "{kind} {counts:?}");
                        assert_eq!(
                            counts.direct_rectangles,
                            u64::from(items.len() > 1),
                            "{kind} {counts:?}"
                        );
                        assert_eq!(snapshots(&output), snapshots(&reference), "{kind}");
                        assert_eq!(
                            args.iter()
                                .map(|v| v.canonical_data_draft().unwrap())
                                .collect::<Vec<_>>(),
                            before
                        );
                        drop((output, reference));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                        drop(args);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            (0, 0)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn rectangular_numeric_maps_replay_partial_bounds_and_arithmetic_errors_with_finite_limits() {
    use super::numeric_maps::{Flags, run, typed_arguments};
    let source = "data := values<[u64]:1,3>\ndivisor := divisor<u64>\n[[((j == 1u64)? | true => data[i] / divisor | false => data[9u64]) | i <- 1u64..=3u64]' | j <- items<[u64]>]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(
            jit.descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .filter(|r| r.rectangle.is_some())
                .count(),
            1
        );
        for restored in [false, true] {
            let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
            let make = |divisor, items: &[u64]| {
                typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "values",
                            Data::Matrix(vec![Data::U64(2), Data::U64(4), Data::U64(6)].into()),
                            &[1, 3],
                        ),
                        ("divisor", Data::U64(divisor), &[]),
                        (
                            "items",
                            Data::Matrix(items.iter().copied().map(Data::U64).collect()),
                            &[1, items.len() as u64],
                        ),
                    ],
                )
            };
            let good = make(2, &[1, 1]);
            let mut prior = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &good,
                &mut prior,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            )
            .0
            .unwrap();
            for args in [make(0, &[1]), make(2, &[2]), make(2, &[1, 2])] {
                let mut reference = prior.clone();
                let mut output = prior.clone();
                let expected = run(
                    &jit,
                    &args,
                    &mut reference,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                )
                .0
                .unwrap_err();
                let (actual, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                assert!(
                    counts.selected > 0,
                    "native attempt must actually decline: {counts:?}"
                );
                assert_eq!(
                    format!("{:?}", actual.unwrap_err()),
                    format!("{expected:?}")
                );
                assert_eq!(snapshots(&output), snapshots(&prior));
                assert_eq!(snapshots(&reference), snapshots(&prior));
            }
            // Complete finite bounds admit a real rectangle. Tight variants
            // retain the original source success/error and old outputs.
            let mut generous = mode.execution();
            generous.execution.max_work = 1_000_000_000_000_000;
            generous.library.max_temporary_bytes = Some(512 * 1024 * 1024);
            generous.library.max_output_elements = Some(1_000_000);
            generous.library.max_output_bytes = Some(512 * 1024 * 1024);
            generous.library.max_retained_nodes = Some(1_000_000);
            generous.library.max_compute_work = Some(1_000_000_000_000_000);
            generous.library.max_comparison_work = Some(1_000_000_000_000_000);
            generous.library.max_cloned_bytes = Some(1_000_000_000_000_000);
            let mut output = prior.clone();
            let (result, counts) = run(
                &jit,
                &good,
                &mut output,
                &memory,
                generous,
                restored,
                Flags::default(),
            );
            result.unwrap();
            assert_eq!(counts.complete, 1);
            drop(output);
            for dimension in 0..9 {
                let mut policy = generous;
                match dimension {
                    0 => policy.execution.max_work = 100_000,
                    1 => policy.library.max_output_elements = Some(5),
                    2 => policy.library.max_output_bytes = Some(512),
                    3 => policy.library.max_temporary_bytes = Some(8192),
                    4 => policy.library.max_cloned_bytes = Some(32),
                    5 => policy.library.max_retained_nodes = Some(8),
                    6 => policy.library.max_compute_work = Some(1_000_000),
                    7 => policy.library.max_comparison_work = Some(0),
                    8 => policy.execution.max_transport_bytes = 64,
                    _ => unreachable!(),
                }
                let mut reference = prior.clone();
                let mut output = prior.clone();
                let (expected, _) = run(
                    &jit,
                    &good,
                    &mut reference,
                    &memory,
                    policy,
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                let (actual, _) = run(
                    &jit,
                    &good,
                    &mut output,
                    &memory,
                    policy,
                    restored,
                    Flags::default(),
                );
                // The existing inner native map retains an exact three-cell
                // unsigned bank; ordinary Vec growth retains four cells.
                // A later transpose therefore reports a smaller actual clone
                // requirement. Keep exact origin/phase/dimension/limit and
                // rejection parity without treating spare capacity as data.
                use mech_core::{MemoryBudgetDimension, ResidentKernelError};
                use mech_engine::native_artifact::NativeCollectionInvocationError as LeafError;
                match (actual.as_ref().err(), expected.as_ref().err()) {
                    (
                        Some(NativeProgramExecutionError::Library {
                            origin: a,
                            error:
                                LeafError::Kernel(ResidentKernelError::ResourceLimit {
                                    phase: ap,
                                    dimension: MemoryBudgetDimension::ClonedBytes,
                                    required: ar,
                                    limit: al,
                                }),
                        }),
                        Some(NativeProgramExecutionError::Library {
                            origin: b,
                            error:
                                LeafError::Kernel(ResidentKernelError::ResourceLimit {
                                    phase: bp,
                                    dimension: MemoryBudgetDimension::ClonedBytes,
                                    required: br,
                                    limit: bl,
                                }),
                        }),
                    ) => {
                        assert_eq!((a, ap, al), (b, bp, bl));
                        assert!(
                            ar <= br,
                            "exact capacity may only lower the measured clone bound"
                        );
                    }
                    // Eliminating repeated selected-call preparation can
                    // finish within a compute cap that ordinary execution
                    // exhausts. Verify the complete semantic oracle in that
                    // case; the generated attempt still prepays its own work.
                    (
                        None,
                        Some(NativeProgramExecutionError::Collection {
                            error:
                                ResidentKernelError::ResourceLimit {
                                    dimension: MemoryBudgetDimension::ComputeWork,
                                    ..
                                },
                            ..
                        }),
                    ) => {
                        assert!(actual.is_ok());
                        assert_eq!(snapshots(&output), snapshots(&prior));
                    }
                    _ => assert_eq!(
                        format!("{:?}", actual.as_ref().err()),
                        format!("{:?}", expected.as_ref().err()),
                        "dimension {dimension}"
                    ),
                }
                assert_eq!(
                    snapshots(&output),
                    if actual.is_ok() {
                        snapshots(&prior)
                    } else {
                        snapshots(&reference)
                    },
                    "dimension {dimension}"
                );
            }
            drop((good, prior));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn rectangular_numeric_maps_reject_changed_loans_and_interleaving_then_retry() {
    use super::numeric_maps::{Flags, run, typed_arguments};
    use mech_core::snapshot::F64Bits;
    let source = "data := values<[f64]:1,3>\n[[data[i] | i <- 1u64..=3u64]' | j <- items<[u64]>]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "values",
                        Data::Matrix(
                            [1.0, 2.0, 3.0]
                                .map(|x| Data::F64(F64Bits::from_f64(x)))
                                .into(),
                        ),
                        &[1, 3],
                    ),
                    (
                        "items",
                        Data::Matrix(vec![Data::U64(1), Data::U64(2)].into()),
                        &[1, 2],
                    ),
                ],
            );
            let mut prior = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &args,
                &mut prior,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            )
            .0
            .unwrap();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for corrupt in [1, 3, 5, 8, 9, 14, 16, 17] {
                let mut output = prior.clone();
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert!(result.is_err(), "corruption {corrupt}");
                assert!(counts.selected > 0);
                assert_eq!(snapshots(&output), snapshots(&prior));
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                let mut output = prior.clone();
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        interleave: true,
                        ..Default::default()
                    },
                );
                result.unwrap();
                assert_eq!(counts.complete, 1);
                assert_eq!(counts.interleaved_refused, 2);
                assert_eq!(snapshots(&output), snapshots(&prior));
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            drop((prior, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn rectangular_numeric_maps_keep_dynamic_and_ragged_inner_ranges_on_the_original_path() {
    for source in [
        "data := values<[f64]:1,3>\nlast := last<u64>\n[[data[i] | i <- 1u64..=last]' | j <- items<[u64]>]\n",
        "data := values<[f64]:1,3>\n[[data[i] | i <- 1u64..=j]' | j <- items<[u64]>]\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .all(|r| r.rectangle.is_none())
            );
        }
    }
}
