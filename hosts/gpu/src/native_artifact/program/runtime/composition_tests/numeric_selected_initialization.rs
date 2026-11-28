//! Conditional scratch initialization must preserve the exact selected domain,
//! simultaneous source reads and full initialization for other-bank publication.
use super::numeric_masked_maps::{
    MATRIX_SOURCE, SEQUENCE_SOURCE, check_success, emitted_masks, output_data,
};
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn spans(value: &serde_json::Value) -> Vec<u32> {
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .flat_map(|(key, value)| {
                if key == "guarded_map_initializations" {
                    value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|row| {
                            row.as_array().unwrap()[2]
                                .as_u64()
                                .unwrap()
                                .try_into()
                                .unwrap()
                        })
                        .collect()
                } else {
                    spans(value)
                }
            })
            .collect(),
        serde_json::Value::Array(values) => values.iter().flat_map(spans).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn numeric_selected_initialization_actual_objects_retain_masks_under_copy_ablation() {
    for source in [
        MATRIX_SOURCE.replace("$SELECT", "j == bound"),
        include_str!("../../../../../tests/fixtures/polybench/gramschmidt.mec").to_owned(),
    ] {
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let ordinary =
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let initialized =
                crate::native_artifact::program::compiler::emit_program_object_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::SelectiveScratchInitialization,
                )
                .unwrap();
            let dense = crate::native_artifact::program::compiler::emit_program_object_for_test(
                &artifact,
                &catalog,
                OptimizationPass::MaskedMatrixMaps,
            )
            .unwrap();
            let actual = spans(&ordinary.metadata());
            assert!(!actual.is_empty());
            assert!(actual.iter().all(|&bytes| bytes > 0 && bytes <= 65536));
            assert_eq!(actual.len(), emitted_masks(&ordinary.metadata()));
            assert_eq!(
                emitted_masks(&ordinary.metadata()),
                emitted_masks(&initialized.metadata())
            );
            assert!(spans(&initialized.metadata()).is_empty());
            assert!(spans(&dense.metadata()).is_empty());
            if source.contains("#Orthogonalize") {
                assert!(actual.contains(&4800));
                assert!(actual.contains(&7200));
            }
        }
    }
}

#[test]
fn numeric_selected_initialization_comparisons_axes_repeated_and_empty_domains_match_full_copy() {
    let catalog = mech_stdlib::source_catalog();
    for axis in ["i", "j"] {
        for op in ["==", "!=", "<", "<=", ">", ">="] {
            let source = MATRIX_SOURCE.replace("$SELECT", &format!("{axis} {op} bound"));
            for artifact in artifacts(&source) {
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let full = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::SelectiveScratchInitialization,
                )
                .unwrap();
                for bound in [0, 2, 4, u64::MAX] {
                    for rounds in [1, 2, 4] {
                        let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                        let args = super::numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                (
                                    "input",
                                    Data::Matrix((0..12).map(|i| Data::U64(i + 10)).collect()),
                                    &[3, 4],
                                ),
                                ("bound", Data::U64(bound), &[]),
                                ("rounds", Data::U64(rounds), &[]),
                            ],
                        );
                        check_success(
                            &artifact,
                            &native,
                            &full,
                            &args,
                            &memory,
                            &format!("{axis}{op}{bound} rounds{rounds}"),
                        );
                        drop(args);
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                    }
                }
            }
        }
    }
}

const DIFFERENT_SOURCE: &str = r#"#Different(initial<[u64]:3,4>, seed<[u64]:3,4>, bound<u64>, rounds<u64>) => <[u64]:3,4>
  | :Step(data<[u64]:3,4>,pass<u64>)
  | :Done(data<[u64]:3,4>).
#Different(initial,seed,bound,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [[((j == bound)? | true => data[4u64 - i,j] + 1u64 | false => seed[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:3,4>
seed := other<[u64]:3,4>
answer := #Different(data,seed,bound<u64>,rounds<u64>)
(answer,data,seed)
"#;

#[test]
fn numeric_selected_initialization_other_bank_keeps_identity_complement_and_old_reads() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(DIFFERENT_SOURCE) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let full = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::SelectiveScratchInitialization,
        )
        .unwrap();
        let dense = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::MaskedMatrixMaps,
        )
        .unwrap();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(!spans(&object.metadata()).is_empty());
        for bound in [0, 2, 4, u64::MAX] {
            for rounds in [1, 2, 3] {
                let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix((0..12).map(|i| Data::U64(i + 10)).collect()),
                            &[3, 4],
                        ),
                        (
                            "other",
                            Data::Matrix((0..12).map(|i| Data::U64(i + 1000)).collect()),
                            &[3, 4],
                        ),
                        ("bound", Data::U64(bound), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                    ],
                );
                check_success(
                    &artifact,
                    &native,
                    &full,
                    &args,
                    &memory,
                    "different identity bank/full copy",
                );
                check_success(
                    &artifact,
                    &native,
                    &dense,
                    &args,
                    &memory,
                    "different identity bank/dense",
                );
                let mut outputs = vec![None; artifact.outputs().len()];
                observe::run(
                    &native,
                    &args,
                    &mut outputs,
                    NativeProgramLimitMode::NoLimits.execution(),
                    &memory,
                )
                .0
                .unwrap();
                let observed = output_data(&outputs);
                let Data::Tuple(fields) = &observed[0] else {
                    panic!("tuple output")
                };
                let Data::Matrix(cells) = &fields[0] else {
                    panic!("matrix result in tuple")
                };
                for row in 0..3 {
                    for col in 0..4 {
                        let index = row * 4 + col;
                        let expected = if col + 1 == bound as usize {
                            let old_row = if rounds % 2 == 1 { 2 - row } else { row };
                            Data::U64((old_row * 4 + col) as u64 + 10 + rounds)
                        } else {
                            Data::U64(index as u64 + 1000)
                        };
                        assert_eq!(
                            cells[index], expected,
                            "bound{bound} rounds{rounds} cell{index}"
                        );
                    }
                }
                drop((outputs, args));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn numeric_selected_initialization_exact_bits_and_odd_packed_tails_match_full_copy() {
    use mech_core::snapshot::F64Bits;
    let fp = [
        0,
        1u64 << 63,
        0x7ff8_0000_0000_0017,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0000,
        1,
        0x7fef_ffff_ffff_ffff,
    ];
    let catalog = mech_stdlib::source_catalog();
    for kind in ["bool", "u8", "u16", "u32", "u64", "i64", "f64"] {
        let active = match kind {
            "bool" => "true".to_owned(),
            "f64" => "data[i,j] + 0.25".to_owned(),
            _ => format!("data[i,j] + 1{kind}"),
        };
        let source = MATRIX_SOURCE
            .replace("[u64]", &format!("[{kind}]"))
            .replace("data[i,j] + 1u64", &active)
            .replace("$SELECT", "j != bound");
        for artifact in artifacts(&source) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let full = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::SelectiveScratchInitialization,
            )
            .unwrap();
            for bound in [0, 2, u64::MAX] {
                let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                let data = Data::Matrix(
                    (0..12)
                        .map(|i| match kind {
                            "bool" => Data::Bool(i % 2 == 0),
                            "u8" => Data::U8(i as u8 + 3),
                            "u16" => Data::U16(i as u16 + 3),
                            "u32" => Data::U32(i as u32 + 3),
                            "u64" => Data::U64(i + 3),
                            "i64" => Data::I64(i as i64 - 6),
                            "f64" => Data::F64(F64Bits::from_bits(fp[i as usize % fp.len()])),
                            _ => unreachable!(),
                        })
                        .collect(),
                );
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", data, &[3, 4]),
                        ("bound", Data::U64(bound), &[]),
                        ("rounds", Data::U64(4), &[]),
                    ],
                );
                check_success(&artifact, &native, &full, &args, &memory, kind);
                drop(args);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn numeric_selected_initialization_checked_failure_keeps_prior_results_private_then_retries() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(SEQUENCE_SOURCE) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let full = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::SelectiveScratchInitialization,
        )
        .unwrap();
        for (bad, badj) in [(0, 0), (3, 3), (0, 0)] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "input",
                        Data::Matrix((0..12).map(|i| Data::U64(i + 10)).collect()),
                        &[3, 4],
                    ),
                    ("bound", Data::U64(2), &[]),
                    ("bad", Data::U64(bad), &[]),
                    ("badj", Data::U64(badj), &[]),
                    ("rounds", Data::U64(3), &[]),
                ],
            );
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            for restored in [false, true] {
                let mut a = vec![None; artifact.outputs().len()];
                let mut b = a.clone();
                let limits = NativeProgramLimitMode::NoLimits.execution();
                let left = if restored {
                    observe::run_restored(&native, &args, &mut a, limits, &memory)
                } else {
                    observe::run(&native, &args, &mut a, limits, &memory)
                }
                .0;
                let right = observe::run(&full, &args, &mut b, limits, &memory).0;
                assert_eq!(format!("{left:?}"), format!("{right:?}"));
                if bad != 0 {
                    assert!(left.is_err());
                    assert!(a.iter().all(Option::is_none));
                    assert!(b.iter().all(Option::is_none));
                } else {
                    assert!(left.is_ok());
                    assert_eq!(output_data(&a), output_data(&b));
                }
                drop((a, b));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
