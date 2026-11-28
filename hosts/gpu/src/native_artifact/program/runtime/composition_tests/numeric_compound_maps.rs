//! Independent source-domain, value, branch, admission and atomicity checks.
use super::numeric_masked_maps::{check_success, output_data, replacements};
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::numeric_phase::masks::{
    Axis, Comparison, Mask, domains::Domain,
};

fn records(value: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .flat_map(|(name, value)| {
                if name == key {
                    value.as_array().unwrap().clone()
                } else {
                    records(value, key)
                }
            })
            .collect(),
        serde_json::Value::Array(values) => values
            .iter()
            .flat_map(|value| records(value, key))
            .collect(),
        _ => Vec::new(),
    }
}
fn controls(artifact: &ProgramArtifact) -> Vec<JitProgramArtifact> {
    let catalog = mech_stdlib::source_catalog();
    [
        OptimizationPass::CompoundMatrixMasks,
        OptimizationPass::SelectiveScratchInitialization,
        OptimizationPass::MaskedMatrixMaps,
    ]
    .map(|pass| JitProgramArtifact::compile_for_test(artifact, &catalog, pass).unwrap())
    .into()
}

#[test]
fn numeric_compound_maps_prove_unchanged_prefix_and_intersection_and_actual_objects() {
    let catalog = mech_stdlib::source_catalog();
    for (source, expected) in [
        (
            include_str!("../../../../../tests/fixtures/polybench/durbin.mec"),
            vec![(0, 3)],
        ),
        (
            include_str!("../../../../../tests/fixtures/polybench/gramschmidt.mec"),
            vec![(0, 2), (1, 6)],
        ),
    ] {
        for artifact in artifacts(source) {
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let plan = &jit.descriptor().metadata().plan;
            let mut selected = 0;
            for (proof, replacement) in replacements(&jit) {
                let old = Mask::derive(plan, proof, replacement);
                let Some(domain) = Domain::derive(plan, proof, replacement)
                    .filter(|d| d.strictly_refines(old.as_ref()))
                else {
                    continue;
                };
                selected += 1;
                assert_eq!(
                    domain
                        .description()
                        .into_iter()
                        .map(|(axis, _, bits)| (axis, bits))
                        .collect::<Vec<_>>(),
                    expected
                );
                assert!(domain.constraint(Axis::Row).is_some());
                let mut changed = replacement.clone();
                changed.target = usize::MAX;
                assert!(Domain::derive(plan, proof, &changed).is_none());
                let mut changed = replacement.clone();
                changed.source.elements += 1;
                assert!(Domain::derive(plan, proof, &changed).is_none());
                let mut changed = replacement.clone();
                changed.source.region.range = false;
                assert!(Domain::derive(plan, proof, &changed).is_none());
                let bound = domain.constraint(Axis::Row).unwrap().bound();
                let mut changed = replacement.clone();
                changed.source.region.captures.retain(|&id| id != bound);
                assert!(Domain::derive(plan, proof, &changed).is_none());
            }
            assert_eq!(selected, 1);
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(records(&object.metadata(), "compound_matrix_maps").len(), 1);
            let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
                &artifact,
                &catalog,
                OptimizationPass::CompoundMatrixMasks,
            )
            .unwrap();
            assert!(records(&old.metadata(), "compound_matrix_maps").is_empty());
        }
    }
    assert_eq!((2u64..=40).sum::<u64>(), 819);
    assert_eq!((1u64..=30).sum::<u64>(), 465);
    for (bits, cmp) in [
        (1, Comparison::Lt),
        (2, Comparison::Eq),
        (3, Comparison::Le),
        (4, Comparison::Gt),
        (5, Comparison::Ne),
        (6, Comparison::Ge),
    ] {
        assert_eq!(Comparison::from_relation_bits(bits), Some(cmp));
        assert_eq!(cmp.relation_bits(), bits);
    }
    assert_eq!(Comparison::from_relation_bits(0), None);
    assert_eq!(Comparison::from_relation_bits(7), None);
}

const PREFIX: &str = r#"#Prefix(initial<[u64]:5,1>, seed<[u64]:5,1>, bound<u64>, bad<u64>, rounds<u64>) => <[u64]:5,1>
  | :Step(data<[u64]:5,1>,pass<u64>)
  | :Done(data<[u64]:5,1>).
#Prefix(initial,seed,bound,bad,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [((i < bound)? | true => data[i] + data[((i == bad)? | true => 0u64 | false => 6u64 - i)] | false => ((i == bound)? | true => data[i] + 1u64 | false => $IDENTITY)) | i <- 1u64..=5u64]'
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:5,1>
seed := other<[u64]:5,1>
answer := #Prefix(data,seed,bound<u64>,bad<u64>,rounds<u64>)
(answer,data,seed)
"#;
fn prefix_args(
    artifact: &ProgramArtifact,
    memory: &mech_core::ManagedMemoryBudget,
    bound: u64,
    bad: u64,
    rounds: u64,
    overflow: bool,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            (
                "input",
                Data::Matrix(
                    (0..5)
                        .map(|i| Data::U64(if overflow && i == 0 { u64::MAX } else { i + 1 }))
                        .collect(),
                ),
                &[5, 1],
            ),
            (
                "other",
                Data::Matrix((0..5).map(|i| Data::U64(i + 101)).collect()),
                &[5, 1],
            ),
            ("bound", Data::U64(bound), &[]),
            ("bad", Data::U64(bad), &[]),
            ("rounds", Data::U64(rounds), &[]),
        ],
    )
}
#[test]
fn numeric_compound_maps_prefix_union_keeps_old_reads_and_same_different_banks() {
    let catalog = mech_stdlib::source_catalog();
    for identity in ["data[i]", "seed[i]"] {
        for artifact in artifacts(&PREFIX.replace("$IDENTITY", identity)) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let variants = controls(&artifact);
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(records(&object.metadata(), "compound_matrix_maps").len(), 1);
            for bound in [0, 1, 2, 5, 6, u64::MAX] {
                for rounds in [1, 2, 3] {
                    let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                    let args = prefix_args(&artifact, &memory, bound, 0, rounds, false);
                    for control in &variants {
                        check_success(
                            &artifact,
                            &native,
                            control,
                            &args,
                            &memory,
                            "nested prefix/old reads/banks",
                        );
                    }
                    let mut expected = [1u64, 2, 3, 4, 5];
                    for _ in 0..rounds {
                        let old = expected;
                        for (i, value) in expected.iter_mut().enumerate() {
                            *value = if ((i + 1) as u64) < bound {
                                old[i] + old[4 - i]
                            } else if (i + 1) as u64 == bound {
                                old[i] + 1
                            } else if identity == "seed[i]" {
                                i as u64 + 101
                            } else {
                                old[i]
                            };
                        }
                    }
                    let mut actual = vec![None; artifact.outputs().len()];
                    observe::run(
                        &native,
                        &args,
                        &mut actual,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &memory,
                    )
                    .0
                    .unwrap();
                    let data = output_data(&actual);
                    let Data::Tuple(fields) = &data[0] else {
                        panic!("tuple")
                    };
                    let Data::Matrix(cells) = &fields[0] else {
                        panic!("matrix")
                    };
                    assert_eq!(
                        cells.as_ref(),
                        expected.map(Data::U64).as_slice(),
                        "{identity} bound{bound} rounds{rounds}"
                    );
                    drop((actual, args));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
            }
        }
    }
}

const RECT: &str = r#"#Rect(initial<[u64]:3,4>, rb<u64>, cb<u64>, rounds<u64>) => <[u64]:3,4>
  | :Step(data<[u64]:3,4>,pass<u64>)
  | :Done(data<[u64]:3,4>).
#Rect(initial,rb,cb,rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [[$EXPR | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:3,4>
answer := #Rect(data,rb<u64>,cb<u64>,rounds<u64>)
(answer,data)
"#;
fn relation(
    axis: &str,
    bound: &str,
    bits: u8,
    active: &str,
    identity: &str,
    reversed: bool,
) -> String {
    let predicate = |op: &str| {
        if reversed {
            format!(
                "{bound} {} {axis}",
                match op {
                    "<" => ">",
                    "==" => "==",
                    ">" => "<",
                    _ => unreachable!(),
                }
            )
        } else {
            format!("{axis} {op} {bound}")
        }
    };
    let mut expression = identity.to_owned();
    for (bit, op) in [(4, ">"), (2, "=="), (1, "<")] {
        if bits & bit != 0 {
            expression = format!(
                "(({})? | true => {active} | false => {expression})",
                predicate(op)
            );
        }
    }
    expression
}
#[test]
fn numeric_compound_maps_cartesian_independent_bounds_all_relations_and_empty_domains() {
    let catalog = mech_stdlib::source_catalog();
    for (row_bits, col_bits) in [(1, 3), (2, 6), (3, 3), (4, 5), (5, 6), (6, 2), (5, 5)] {
        for reversed in [false, true] {
            let col = relation(
                "j",
                "cb",
                col_bits,
                "data[i,j] + 1u64",
                "data[i,j]",
                reversed,
            );
            let expression = relation("i", "rb", row_bits, &col, "data[i,j]", reversed);
            for artifact in artifacts(&RECT.replace("$EXPR", &expression)) {
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let variants = controls(&artifact);
                let object =
                    crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                assert_eq!(
                    records(&object.metadata(), "compound_matrix_maps").len(),
                    1,
                    "{row_bits}/{col_bits}/{reversed}"
                );
                for (rb, cb) in [(0, 0), (1, 1), (2, 3), (3, 4), (4, 5), (u64::MAX, u64::MAX)] {
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
                            ("rb", Data::U64(rb), &[]),
                            ("cb", Data::U64(cb), &[]),
                            ("rounds", Data::U64(3), &[]),
                        ],
                    );
                    for control in &variants {
                        check_success(
                            &artifact,
                            &native,
                            control,
                            &args,
                            &memory,
                            "Cartesian domains",
                        );
                    }
                    let mut actual = vec![None; artifact.outputs().len()];
                    observe::run(
                        &native,
                        &args,
                        &mut actual,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &memory,
                    )
                    .0
                    .unwrap();
                    let data = output_data(&actual);
                    let Data::Tuple(fields) = &data[0] else {
                        panic!("tuple")
                    };
                    let Data::Matrix(cells) = &fields[0] else {
                        panic!("matrix")
                    };
                    let matches = |value: u64, bound: u64, bits: u8| {
                        let bit = if value < bound {
                            1
                        } else if value == bound {
                            2
                        } else {
                            4
                        };
                        bits & bit != 0
                    };
                    for row in 0..3 {
                        for col in 0..4 {
                            let i = row * 4 + col;
                            let selected = matches(row as u64 + 1, rb, row_bits)
                                && matches(col as u64 + 1, cb, col_bits);
                            assert_eq!(
                                cells[i],
                                Data::U64(i as u64 + 10 + if selected { 3 } else { 0 })
                            );
                        }
                    }
                    drop((actual, args));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
            }
        }
    }
}

#[test]
fn numeric_compound_maps_nonfactorable_conflicting_and_fallible_source_declines() {
    let catalog = mech_stdlib::source_catalog();
    let different_bases=RECT
        .replace("initial<[u64]:3,4>, rb", "initial<[u64]:3,4>, seed<[u64]:3,4>, rb")
        .replace("#Rect(initial,rb,cb,rounds)", "#Rect(initial,seed,rb,cb,rounds)")
        .replace("answer := #Rect(data,rb", "seed := other<[u64]:3,4>\nanswer := #Rect(data,seed,rb")
        .replace("$EXPR", "((i < rb)? | true => data[i,j] + 1u64 | false => ((j < cb)? | true => data[i,j] | false => seed[i,j]))");
    let sources=[
        different_bases,
        RECT.replace("$EXPR","((i < rb)? | true => data[i,j] + 1u64 | false => ((j > cb)? | true => data[i,j] + 1u64 | false => data[i,j]))"),
        RECT.replace("$EXPR","((i < rb)? | true => data[i,j] + 1u64 | false => ((i == cb)? | true => data[i,j] + 1u64 | false => data[i,j]))"),
        PREFIX.replace("$IDENTITY","data[6u64 - i]"),
        PREFIX.replace("$IDENTITY","data[i]").replace("i < bound","i < (bound + 1u64)"),
    ];
    for source in sources {
        for artifact in artifacts(&source) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let plan = &native.descriptor().metadata().plan;
            assert!(
                replacements(&native)
                    .iter()
                    .all(|(proof, replacement)| Domain::derive(plan, proof, replacement).is_none())
            );
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert!(records(&object.metadata(), "compound_matrix_maps").is_empty());
            if source.contains("(bound + 1u64)") {
                let variants = controls(&artifact);
                let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = prefix_args(&artifact, &memory, u64::MAX, 0, 1, false);
                let baseline = (memory.used_bytes(), memory.retained_nodes());
                for control in &variants {
                    let mut a = vec![None; artifact.outputs().len()];
                    let mut b = a.clone();
                    let limits = NativeProgramLimitMode::NoLimits.execution();
                    let left = observe::run(&native, &args, &mut a, limits, &memory).0;
                    let right = observe::run(control, &args, &mut b, limits, &memory).0;
                    assert!(left.is_err());
                    assert_eq!(format!("{left:?}"), format!("{right:?}"));
                    assert!(a.iter().all(Option::is_none));
                    assert!(b.iter().all(Option::is_none));
                    drop((a, b));
                }
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                drop(args);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn numeric_compound_maps_f64_bits_packed_tails_and_checked_errors_are_atomic() {
    use mech_core::snapshot::F64Bits;
    let catalog = mech_stdlib::source_catalog();
    let source = PREFIX
        .replace("$IDENTITY", "data[i]")
        .replace("[u64]", "[f64]")
        .replace("data[i] + 1u64", "data[i] + 0.25");
    for artifact in artifacts(&source) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let variants = controls(&artifact);
        for bound in [0, 1, 3, 5, u64::MAX] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let bits = [0, 1 << 63, 0x7ff8_0000_0000_0017, 1, 0x3ff0_0000_0000_0001];
            let data = Data::Matrix(bits.map(|v| Data::F64(F64Bits::from_bits(v))).into());
            let args = super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("input", data.clone(), &[5, 1]),
                    ("other", data, &[5, 1]),
                    ("bound", Data::U64(bound), &[]),
                    ("bad", Data::U64(0), &[]),
                    ("rounds", Data::U64(3), &[]),
                ],
            );
            for control in &variants {
                check_success(
                    &artifact,
                    &native,
                    control,
                    &args,
                    &memory,
                    "compound F64 bits",
                );
            }
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
    for artifact in artifacts(&PREFIX.replace("$IDENTITY", "data[i]")) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let variants = controls(&artifact);
        for (bad, overflow) in [(0, false), (2, false), (0, true), (0, false)] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = prefix_args(&artifact, &memory, 4, bad, 3, overflow);
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            if bad == 0 && !overflow {
                for control in &variants {
                    check_success(
                        &artifact,
                        &native,
                        control,
                        &args,
                        &memory,
                        "compound retry",
                    );
                }
            } else {
                for control in &variants {
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
                        let right = observe::run(control, &args, &mut b, limits, &memory).0;
                        assert!(left.is_err());
                        assert_eq!(format!("{left:?}"), format!("{right:?}"));
                        assert!(a.iter().all(Option::is_none));
                        assert!(b.iter().all(Option::is_none));
                        drop((a, b));
                    }
                }
            }
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn numeric_compound_maps_logical_physical_caps_cleanup_and_retry() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&PREFIX.replace("$IDENTITY", "data[i]")) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let variants = controls(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = prefix_args(&artifact, &memory, 3, 0, 3, false);
        let baseline = (memory.used_bytes(), memory.retained_nodes());
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
            for control in &variants {
                for restored in [false, true] {
                    let mut a = vec![None; artifact.outputs().len()];
                    let mut b = a.clone();
                    let left = if restored {
                        observe::run_restored(&native, &args, &mut a, limits, &memory)
                    } else {
                        observe::run(&native, &args, &mut a, limits, &memory)
                    }
                    .0;
                    let right = observe::run(control, &args, &mut b, limits, &memory).0;
                    assert_eq!(left.is_ok(), right.is_ok(), "cap{case}: {left:?}/{right:?}");
                    if left.is_ok() {
                        assert_eq!(output_data(&a), output_data(&b));
                    } else {
                        assert!(a.iter().all(Option::is_none));
                        assert!(b.iter().all(Option::is_none));
                    }
                    drop((a, b));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                }
            }
        }
        for dimension in ["bytes", "nodes"] {
            let mut denied = 0;
            let mut succeeded = 0;
            for allowance in [0, 16, 4096, 1_048_576, 1_000_000_000_000] {
                let bounded = if dimension == "bytes" {
                    memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                } else {
                    memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                };
                let mut outputs = vec![None; artifact.outputs().len()];
                let result =
                    observe::run_restored(&native, &args, &mut outputs, mode.execution(), &bounded)
                        .0;
                if result.is_ok() {
                    succeeded += 1;
                } else {
                    denied += 1;
                    assert!(outputs.iter().all(Option::is_none));
                }
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
            assert!(
                denied > 0 && succeeded > 0,
                "{dimension} denied{denied}/ok{succeeded}"
            );
        }
        for control in &variants {
            check_success(
                &artifact,
                &native,
                control,
                &args,
                &memory,
                "after all caps",
            );
        }
        drop(args);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
