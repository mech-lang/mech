//! Source-derived masks retain exact identity alternatives and current bounds.
use super::*;
use crate::native_artifact::program::prepare::{
    numeric_phase::{
        masks::{Axis, Comparison, Mask},
        matrix_maps::Replacement,
    },
    numeric_plan::Operation,
};
pub(super) fn replacements(
    jit: &JitProgramArtifact,
) -> Vec<(
    &crate::native_artifact::program::prepare::readonly_numeric::Proof,
    &Replacement,
)> {
    jit.descriptor()
        .metadata()
        .closed_tails
        .regions
        .iter()
        .flat_map(|r| {
            r.numeric.iter().flat_map(|p| {
                p.actions.iter().filter_map(move |(_, a)| match a {
                    Operation::Map { replacement } => Some((p, replacement.as_ref())),
                    _ => None,
                })
            })
        })
        .collect()
}
#[test]
fn numeric_masked_maps_prove_three_optimization_masks_from_exact_source_cfg() {
    let source = include_str!("../../../../../tests/fixtures/polybench/gramschmidt.mec");
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = &jit.descriptor().metadata().plan;
        let masks = replacements(&jit)
            .into_iter()
            .filter_map(|(proof, r)| Mask::derive(plan, proof, r))
            .collect::<Vec<_>>();
        if masks.len() != 3 {
            for (_, map) in replacements(&jit) {
                eprintln!("map {} {:?}", map.source.region.header, map.source.extents);
                for site in map.source.region.body.iter().take(16) {
                    eprintln!(
                        "{site}: {:?} / {:?}",
                        plan.blocks()[*site as usize].action(),
                        plan.blocks()[*site as usize].terminator()
                    );
                }
            }
        }
        assert_eq!(masks.len(), 3);
        for mask in &masks {
            assert!(mask.rectangular());
            assert!(mask.rows() > 0 && mask.columns() > 0);
            assert!(
                plan.values()[mask.bound() as usize].scalar_kind()
                    == Some(mech_engine::native_artifact::NativeScalarKind::U64)
            );
            assert!(matches!(
                plan.blocks()[mask.selector() as usize].terminator(),
                mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                    mech_engine::native_artifact::NativeMatchTerminator::Branch { .. }
                        | mech_engine::native_artifact::NativeMatchTerminator::LiteralBranch { .. }
                )
            ));
        }
        assert!(
            masks
                .iter()
                .any(|m| m.axis() == Axis::Row && m.comparison() == Comparison::Eq)
        );
        assert!(
            masks
                .iter()
                .any(|m| m.axis() == Axis::Column && m.comparison() == Comparison::Eq)
        );
        assert!(
            masks
                .iter()
                .any(|m| m.axis() == Axis::Column && m.comparison() == Comparison::Gt)
        );
        for (proof, replacement) in replacements(&jit) {
            if Mask::derive(plan, proof, replacement).is_none() {
                continue;
            }
            let mut changed = replacement.clone();
            changed.target = usize::MAX;
            assert!(Mask::derive(plan, proof, &changed).is_none());
            let mut changed = replacement.clone();
            changed.source.elements += 1;
            assert!(Mask::derive(plan, proof, &changed).is_none());
            let mut changed = replacement.clone();
            changed.source.region.range = false;
            assert!(Mask::derive(plan, proof, &changed).is_none());
            let mut changed = replacement.clone();
            changed.source.region.reads.clear();
            assert!(Mask::derive(plan, proof, &changed).is_none());
        }
    }
}

pub(super) const MATRIX_SOURCE: &str = r#"#Masked(initial<[u64]:3,4>, bound<u64>, rounds<u64>) => <[u64]:3,4>
  | :Step(data<[u64]:3,4>,pass<u64>)
  | :Done(data<[u64]:3,4>).
#Masked(initial,bound,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [[(($SELECT)? | true => data[i,j] + 1u64 | false => data[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:3,4>
answer := #Masked(data,bound<u64>,rounds<u64>)
(answer,data)
"#;
pub(super) fn emitted_masks(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| {
                if key == "masked_matrix_maps" {
                    value.as_array().unwrap().len()
                } else {
                    emitted_masks(value)
                }
            })
            .sum(),
        serde_json::Value::Array(values) => values.iter().map(emitted_masks).sum(),
        _ => 0,
    }
}
pub(super) fn output_data(outputs: &[Option<Value>]) -> Vec<Data> {
    outputs
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
#[test]
fn numeric_masked_maps_all_unsigned_comparisons_axes_and_extreme_bounds_match_dense() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for axis in ["i", "j"] {
        for op in ["==", "!=", "<", "<=", ">", ">="] {
            for reversed in [false, true] {
                let selector = if reversed {
                    format!("bound {op} {axis}")
                } else {
                    format!("{axis} {op} bound")
                };
                let source = MATRIX_SOURCE.replace("$SELECT", &selector);
                for artifact in artifacts(&source) {
                    let catalog = mech_stdlib::source_catalog();
                    let mode = NativeProgramLimitMode::NoLimits;
                    let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                    let dense = JitProgramArtifact::compile_for_test(
                        &artifact,
                        &catalog,
                        OptimizationPass::MaskedMatrixMaps,
                    )
                    .unwrap();
                    let object =
                        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                    let dense_object =
                        crate::native_artifact::program::compiler::emit_program_object_for_test(
                            &artifact,
                            &catalog,
                            OptimizationPass::MaskedMatrixMaps,
                        )
                        .unwrap();
                    assert!(emitted_masks(&object.metadata()) > 0, "{selector}");
                    assert_eq!(emitted_masks(&dense_object.metadata()), 0);
                    for bound in [0, 1, 2, 3, 4, 5, u64::MAX] {
                        let memory = mode.memory_budget();
                        let args = super::numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                (
                                    "input",
                                    Data::Matrix((0..12).map(|x| Data::U64(x + 10)).collect()),
                                    &[3, 4],
                                ),
                                ("bound", Data::U64(bound), &[]),
                                ("rounds", Data::U64(3), &[]),
                            ],
                        );
                        let before = contents(&args);
                        let aliases = args.clone();
                        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let mut actual = vec![None; artifact.outputs().len()];
                        let mut expected = actual.clone();
                        let (a, _) =
                            observe::run(&native, &args, &mut actual, mode.execution(), &memory);
                        let (b, _) =
                            observe::run(&dense, &args, &mut expected, mode.execution(), &memory);
                        assert!(a.is_ok() && b.is_ok(), "{selector} {bound}: {a:?} / {b:?}");
                        assert_eq!(
                            output_data(&actual),
                            output_data(&expected),
                            "{selector} {bound}"
                        );
                        assert_eq!(
                            a.unwrap(),
                            b.unwrap(),
                            "original source counters: {selector} {bound}"
                        );
                        let retained = actual.clone();
                        let (retry, _) = observe::run_restored(
                            &native,
                            &args,
                            &mut actual,
                            mode.execution(),
                            &memory,
                        );
                        assert!(retry.is_ok(), "restored {selector} {bound}: {retry:?}");
                        assert_eq!(output_data(&actual), output_data(&retained));
                        assert_eq!(contents(&args), before);
                        assert!(
                            args.iter()
                                .zip(&aliases)
                                .all(|(a, b)| a.shares_frozen_storage(b))
                        );
                        drop((actual, expected, retained));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            baseline
                        );
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
}

#[test]
fn numeric_masked_maps_preserve_all_scalar_bits_and_packed_width_tails() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    let fp = [
        0,
        1 << 63,
        0x7ff8_0000_0000_0017,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0000,
        1,
        0x7fef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0001,
    ];
    for kind in ["bool", "u8", "u16", "u32", "u64", "i64", "f64"] {
        let active = match kind {
            "bool" => "true".to_owned(),
            "f64" => "data[i,j] + 0.25".to_owned(),
            _ => format!("data[i,j] + 1{kind}"),
        };
        for selector in ["i == bound", "j > bound", "j != bound"] {
            let source = MATRIX_SOURCE
                .replace("[u64]", &format!("[{kind}]"))
                .replace("data[i,j] + 1u64", &active)
                .replace("$SELECT", selector);
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let mode = NativeProgramLimitMode::NoLimits;
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let dense = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::MaskedMatrixMaps,
                )
                .unwrap();
                let object =
                    crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                assert!(emitted_masks(&object.metadata()) > 0, "{kind} {selector}");
                for bound in [0, 1, 3, 4, u64::MAX] {
                    let memory = mode.memory_budget();
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
                            ("rounds", Data::U64(3), &[]),
                        ],
                    );
                    let before = contents(&args);
                    let aliases = args.clone();
                    let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    let (a, _) =
                        observe::run(&native, &args, &mut actual, mode.execution(), &memory);
                    let (b, _) =
                        observe::run(&dense, &args, &mut expected, mode.execution(), &memory);
                    assert!(
                        a.is_ok() && b.is_ok(),
                        "{kind} {selector} {bound}: {a:?}/{b:?}"
                    );
                    assert_eq!(
                        output_data(&actual),
                        output_data(&expected),
                        "{kind} {selector} {bound}"
                    );
                    assert_eq!(
                        a.unwrap(),
                        b.unwrap(),
                        "{kind} {selector} {bound} source counters"
                    );
                    let retained = actual.clone();
                    let (retry, _) = observe::run_restored(
                        &native,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                    );
                    assert!(retry.is_ok(), "{kind} restored {retry:?}");
                    assert_eq!(output_data(&actual), output_data(&retained));
                    assert_eq!(contents(&args), before);
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    drop((actual, expected, retained));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
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

pub(super) const VECTOR_SOURCE: &str = r#"#Vector(initial<[u64]:$SHAPE>, bound<u64>, rounds<u64>) => <[u64]:$SHAPE>
  | :Step(data<[u64]:$SHAPE>,pass<u64>)
  | :Done(data<[u64]:$SHAPE>).
#Vector(initial,bound,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [(($SELECT)? | true => $YES | false => $NO) | i <- 1u64..=4u64]$TRANSPOSE
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:$SHAPE>
answer := #Vector(data,bound<u64>,rounds<u64>)
(answer,data)
"#;
pub(super) fn check_success(
    artifact: &ProgramArtifact,
    native: &JitProgramArtifact,
    dense: &JitProgramArtifact,
    args: &[Value],
    memory: &mech_core::ManagedMemoryBudget,
    label: &str,
) {
    let mode = NativeProgramLimitMode::NoLimits;
    let before = contents(args);
    let aliases = args.to_vec();
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut actual = vec![None; artifact.outputs().len()];
    let mut expected = actual.clone();
    let (a, _) = observe::run(native, args, &mut actual, mode.execution(), memory);
    let (b, _) = observe::run(dense, args, &mut expected, mode.execution(), memory);
    assert!(a.is_ok() && b.is_ok(), "{label}: {a:?}/{b:?}");
    assert_eq!(output_data(&actual), output_data(&expected), "{label}");
    assert_eq!(a.unwrap(), b.unwrap(), "{label} source reports");
    let retained = actual.clone();
    let (retry, _) = observe::run_restored(native, args, &mut actual, mode.execution(), memory);
    assert!(retry.is_ok(), "{label}: {retry:?}");
    assert_eq!(output_data(&actual), output_data(&retained));
    assert_eq!(contents(args), before);
    assert!(
        args.iter()
            .zip(&aliases)
            .all(|(a, b)| a.shares_frozen_storage(b))
    );
    drop((actual, expected, retained, aliases));
    assert_eq!(
        (memory.used_bytes(), memory.retained_nodes()),
        baseline,
        "{label}"
    );
}
#[test]
fn numeric_masked_maps_keep_vector_orientations_false_active_edges_and_neighbour_reads() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for (shape, dimensions, transpose) in [("4,1", [4, 1], "'"), ("1,4", [1, 4], "")] {
        for identity_yes in [false, true] {
            let active = "data[5u64 - i] + 1u64";
            let identity = "data[i]";
            let source = VECTOR_SOURCE
                .replace("$SHAPE", shape)
                .replace("$TRANSPOSE", transpose)
                .replace("$SELECT", "i != bound")
                .replace("$YES", if identity_yes { identity } else { active })
                .replace("$NO", if identity_yes { active } else { identity });
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let dense = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::MaskedMatrixMaps,
                )
                .unwrap();
                let object =
                    crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                assert!(
                    emitted_masks(&object.metadata()) > 0,
                    "{shape} polarity{identity_yes}"
                );
                for bound in [0, 1, 2, 4, 5, u64::MAX] {
                    let mode = NativeProgramLimitMode::NoLimits;
                    let memory = mode.memory_budget();
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            (
                                "input",
                                Data::Matrix([3, 7, 11, 19].map(Data::U64).into()),
                                &dimensions,
                            ),
                            ("bound", Data::U64(bound), &[]),
                            ("rounds", Data::U64(3), &[]),
                        ],
                    );
                    check_success(
                        &artifact,
                        &native,
                        &dense,
                        &args,
                        &memory,
                        &format!("{shape} polarity{identity_yes} bound{bound}"),
                    );
                    drop(args);
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
            }
        }
    }
}

pub(super) const SEQUENCE_SOURCE: &str = r#"#Sequence(initial<[u64]:3,4>, bound<u64>, bad<u64>, badj<u64>, rounds<u64>) => <[u64]:3,4>
  | :Step(data<[u64]:3,4>,pass<u64>)
  | :Done(data<[u64]:3,4>).
#Sequence(initial,bound,bad,badj,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        first := [[((j == bound)? | true => data[i,j] + 1u64 | false => data[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
        next := [[((j >= bound)? | true => first[((i == bad)? | true => ((j == badj)? | true => 0u64 | false => i) | false => i),j] + 1u64 | false => first[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:3,4>
answer := #Sequence(data,bound<u64>,bad<u64>,badj<u64>,rounds<u64>)
(answer,data)
"#;
#[test]
fn numeric_masked_maps_keep_prior_results_and_checked_failures_atomic_then_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(SEQUENCE_SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let dense = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::MaskedMatrixMaps,
        )
        .unwrap();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(emitted_masks(&object.metadata()) >= 2);
        for (bad, badj, overflow) in [(0, 0, false), (3, 3, false), (0, 0, true), (0, 0, false)] {
            let memory = mode.memory_budget();
            let data = Data::Matrix(
                (0..12)
                    .map(|i| Data::U64(if overflow && i == 6 { u64::MAX } else { i + 10 }))
                    .collect(),
            );
            let args = super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("input", data, &[3, 4]),
                    ("bound", Data::U64(2), &[]),
                    ("bad", Data::U64(bad), &[]),
                    ("badj", Data::U64(badj), &[]),
                    ("rounds", Data::U64(2), &[]),
                ],
            );
            let before = contents(&args);
            let aliases = args.clone();
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            if bad == 0 && !overflow {
                check_success(
                    &artifact,
                    &native,
                    &dense,
                    &args,
                    &memory,
                    "completed prior map / success",
                );
            } else {
                let mut expected = vec![None; artifact.outputs().len()];
                let (reference, _) =
                    observe::run(&dense, &args, &mut expected, mode.execution(), &memory);
                assert!(reference.is_err());
                assert!(expected.iter().all(Option::is_none));
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let (result, _) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut actual,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&native, &args, &mut actual, mode.execution(), &memory)
                    };
                    assert_eq!(
                        format!("{result:?}"),
                        format!("{reference:?}"),
                        "bad{bad} badj{badj} overflow{overflow}"
                    );
                    assert!(actual.iter().all(Option::is_none));
                }
            }
            assert_eq!(contents(&args), before);
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            drop((args, aliases));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn numeric_masked_maps_decline_neighbour_identity_and_fallible_prefix_without_hiding_errors() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for source in [
        MATRIX_SOURCE
            .replace("$SELECT", "j == bound")
            .replace("false => data[i,j]", "false => data[4u64 - i,j]"),
        MATRIX_SOURCE.replace("$SELECT", "j == (bound + 1u64)"),
    ] {
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let dense = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::MaskedMatrixMaps,
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(emitted_masks(&object.metadata()), 0);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "input",
                        Data::Matrix((0..12).map(Data::U64).collect()),
                        &[3, 4],
                    ),
                    ("bound", Data::U64(u64::MAX), &[]),
                    ("rounds", Data::U64(2), &[]),
                ],
            );
            let before = contents(&args);
            let mut a = vec![None; artifact.outputs().len()];
            let mut b = a.clone();
            let (left, _) = observe::run(&native, &args, &mut a, mode.execution(), &memory);
            let (right, _) = observe::run(&dense, &args, &mut b, mode.execution(), &memory);
            assert_eq!(format!("{left:?}"), format!("{right:?}"));
            assert_eq!(
                a.iter()
                    .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                    .collect::<Vec<_>>(),
                b.iter()
                    .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                    .collect::<Vec<_>>()
            );
            assert_eq!(contents(&args), before);
            drop((a, b, args));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn numeric_masked_maps_finite_admission_is_atomic_and_physical_denials_cleanly_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(&MATRIX_SOURCE.replace("$SELECT", "i != bound")) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let dense = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::MaskedMatrixMaps,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix((0..12).map(|x| Data::U64(x + 10)).collect()),
                    &[3, 4],
                ),
                ("bound", Data::U64(2), &[]),
                ("rounds", Data::U64(3), &[]),
            ],
        );
        let before = contents(&args);
        let aliases = args.clone();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut reference = vec![None; artifact.outputs().len()];
        observe::run(&dense, &args, &mut reference, mode.execution(), &memory)
            .0
            .unwrap();
        let expected = output_data(&reference);
        drop(reference);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        for case in 0..19 {
            let mut limits = mode.execution();
            match case {
                0 => limits.library.max_output_elements = None,
                1 => limits.library.max_output_elements = Some(0),
                2 => limits.library.max_output_bytes = None,
                3 => limits.library.max_output_bytes = Some(0),
                4 => limits.library.max_temporary_bytes = None,
                5 => limits.library.max_temporary_bytes = Some(0),
                6 => limits.library.max_cloned_bytes = None,
                7 => limits.library.max_cloned_bytes = Some(0),
                8 => limits.library.max_retained_nodes = None,
                9 => limits.library.max_retained_nodes = Some(0),
                10 => limits.library.max_compute_work = None,
                11 => limits.library.max_compute_work = Some(0),
                12 => limits.library.max_comparison_work = None,
                13 => limits.library.max_comparison_work = Some(0),
                14 => limits.library.max_scalar_instructions = None,
                15 => limits.library.max_transfer_bytes = None,
                16 => limits.library.max_storage_buffer_bytes = None,
                17 => limits.library.max_storage_bindings = None,
                _ => limits.execution.max_work = 1,
            }
            for restored in [false, true] {
                let mut actual = vec![None; artifact.outputs().len()];
                let mut ordinary = actual.clone();
                let (a, _) = if restored {
                    observe::run_restored(&native, &args, &mut actual, limits, &memory)
                } else {
                    observe::run(&native, &args, &mut actual, limits, &memory)
                };
                let (b, _) = observe::run(&dense, &args, &mut ordinary, limits, &memory);
                assert_eq!(
                    a.is_ok(),
                    b.is_ok(),
                    "cap{case} restored{restored}: {a:?}/{b:?}"
                );
                if a.is_err() {
                    assert!(actual.iter().all(Option::is_none));
                    assert!(ordinary.iter().all(Option::is_none));
                } else {
                    assert_eq!(output_data(&actual), expected);
                    assert_eq!(output_data(&ordinary), expected);
                }
                drop((actual, ordinary));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_nodes()),
                    baseline,
                    "cap{case} restored{restored}"
                );
            }
        }
        for restored in [false, true] {
            for dimension in ["bytes", "nodes"] {
                let allowances: &[u64] = if dimension == "bytes" {
                    &[
                        0,
                        4096,
                        65536,
                        1048576,
                        16777216,
                        64000000,
                        1_000_000_000_000,
                    ]
                } else {
                    &[0, 16, 256, 4096, 65536, 1_000_000_000]
                };
                let mut denied = 0;
                let mut succeeded = 0;
                for allowance in allowances {
                    let bounded = if dimension == "bytes" {
                        memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                    } else {
                        memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                    };
                    let mut outputs = vec![None; artifact.outputs().len()];
                    let (result, _) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut outputs,
                            mode.execution(),
                            &bounded,
                        )
                    } else {
                        observe::run(&native, &args, &mut outputs, mode.execution(), &bounded)
                    };
                    if result.is_ok() {
                        succeeded += 1;
                        assert_eq!(output_data(&outputs), expected);
                    } else {
                        denied += 1;
                        assert!(outputs.iter().all(Option::is_none));
                    }
                    drop(outputs);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_nodes()),
                        baseline,
                        "{dimension} {allowance}"
                    );
                    assert_eq!(contents(&args), before);
                }
                assert!(
                    denied > 0 && succeeded > 0,
                    "{dimension} denied{denied} succeeded{succeeded}"
                );
            }
        }
        check_success(
            &artifact,
            &native,
            &dense,
            &args,
            &memory,
            "retry after every finite/physical denial",
        );
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        drop((args, aliases));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

fn forge_map(value: &mut serde_json::Value, which: usize) -> bool {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(replacement) = fields.get_mut("Map").and_then(|v| v.get_mut("replacement"))
            {
                match which {
                    0 => replacement["target"] = (u32::MAX as u64).into(),
                    1 => replacement["arrays"][0] = (u32::MAX as u64).into(),
                    2 => {
                        replacement["source"]["region"]["reads"][0]["indices"][0] =
                            (u32::MAX as u64).into()
                    }
                    3 => replacement["source"]["region"]["body"][0] = (u32::MAX as u64).into(),
                    4 => {
                        replacement["source"]["elements"] =
                            (replacement["source"]["elements"].as_u64().unwrap() + 1).into()
                    }
                    _ => replacement["mask"] = serde_json::json!({"skip_all":true,"owner":true}),
                };
                return true;
            }
            fields.values_mut().any(|v| forge_map(v, which))
        }
        serde_json::Value::Array(values) => values.iter_mut().any(|v| forge_map(v, which)),
        _ => false,
    }
}
#[test]
fn numeric_masked_maps_rederive_source_bindings_and_reject_wire_masks_and_forged_views() {
    for artifact in artifacts(&MATRIX_SOURCE.replace("$SELECT", "j == bound")) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        for wire in [
            encoded.clone(),
            crate::native_artifact::program::image::packing::pack(&encoded).unwrap(),
        ] {
            // SAFETY: metadata from this genuine checked JIT; no entry is fabricated.
            let decoded =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                    .unwrap();
            assert_eq!(decoded.encode_build_owned_image().unwrap(), encoded);
        }
        for which in 0..6 {
            let mut changed = image.clone();
            assert!(forge_map(&mut changed, which));
            let bytes = serde_json::to_vec(&changed).unwrap();
            for wire in [
                bytes.clone(),
                crate::native_artifact::program::image::packing::pack(&bytes).unwrap(),
            ] {
                // SAFETY: negative wire-only decoding. A changed descriptor never executes.
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                        .is_err(),
                    "forgery{which}"
                );
            }
        }
    }
}
