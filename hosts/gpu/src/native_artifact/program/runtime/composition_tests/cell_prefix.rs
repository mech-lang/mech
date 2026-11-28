//! Source/decoded/restored comparisons for native point expression prefixes.
use super::ordered_map_dots::compare as compare_pair;
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::codegen::numeric_cell_prefix;
use mech_core::snapshot::F64Bits;
fn pair(artifact: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    let fast = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let old = JitProgramArtifact::compile_for_test(
        artifact,
        &catalog,
        OptimizationPass::CellExpressionPrefixes,
    )
    .unwrap();
    assert!(
        fast.descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .any(
                |p| !numeric_cell_prefix::derive(&fast.descriptor().metadata().plan.facts(), p)
                    .is_empty()
            ),
        "source-derived point prefix actually admitted"
    );
    crate::native_artifact::emit_program_object(artifact, &catalog).unwrap();
    (fast, old)
}
fn compare(
    fast: &JitProgramArtifact,
    old: &JitProgramArtifact,
    artifact: &ProgramArtifact,
    args: &[Value],
    memory: &ManagedMemoryBudget,
    limits: NativeProgramExecutionLimits,
    restored: bool,
) {
    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
    let aliases = args.to_vec();
    let mut outputs = vec![None; artifact.outputs().len()];
    let mut previous = outputs.clone();
    compare_pair(
        fast,
        old,
        args,
        memory,
        limits,
        restored,
        &mut outputs,
        &mut previous,
    );
    assert_eq!(contents(&aliases), contents(args));
    outputs.fill(None);
    previous.fill(None);
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        retained,
        "all prefix and fallback temporaries released"
    );
}
fn source(expression: &str, shape: [u64; 2]) -> String {
    let [rows, columns] = shape;
    format!(
        r#"+> math
#Walk(initial<[f64]:{rows},{columns}>, row<u64>, first<u64>, end<u64>, offset<u64>, gain<f64>) => <[f64]:{rows},{columns}>
  | :Sweep(a<[f64]:{rows},{columns}>, r<u64>, c<u64>, end<u64>, offset<u64>, gain<f64>)
  | :Done(a<[f64]:{rows},{columns}>).
#Walk(initial,row,first,end,offset,gain) -> :Sweep(initial,row,first,end,offset,gain)
  :Sweep(a,r,c,end,offset,gain)
    | c >= end -> :Done(a)
    | * -> {{
        value := {expression}
        ~next := a
        next[r,c] = value
      }} -> :Sweep(next,r,c + 1u64,end,offset,gain)
  :Done(a) => a.
a := input<[f64]:{rows},{columns}>
answer := #Walk(a,row<u64>,first<u64>,last<u64>,offset<u64>,gain<f64>)
(answer,a)
"#
    )
}

#[test]
fn cell_prefix_public_sequential_dependencies_bits_aliases_errors_and_restored_replay() {
    for expression in [
        "(a[r,c] + a[r,c + offset]) / gain",
        "a[c + offset] * gain + gain",
        "a[r,c] * gain + gain",
        "a[r,c - offset] - gain * a[r,c]",
        "-math/sqrt(1.0 / (a[r,c] * a[r,c + offset]))",
        "(a[r,c] + a[r,c + offset]) * (a[r,c] - a[r,c + offset]) / gain",
    ] {
        for shape in [[2, 5], [5, 2], [1, 10]] {
            for artifact in artifacts(&source(expression, shape)) {
                let (fast, old) = pair(&artifact);
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let samples = [
                    0u64,
                    (-0.0f64).to_bits(),
                    1,
                    1.25f64.to_bits(),
                    f64::MAX.to_bits(),
                    f64::INFINITY.to_bits(),
                    f64::NEG_INFINITY.to_bits(),
                    0x7ff8_1234_5678_9abc,
                    0x7ff0_1234_5678_9abc,
                    0xfff8_4321_1234_abcd,
                ];

                for seed in 0..samples.len() {
                    let cells = (0..10)
                        .map(|i| Data::F64(F64Bits::from_bits(samples[(seed + i) % samples.len()])))
                        .collect::<Vec<_>>();
                    for (row, first, last, offset) in [
                        (1, 1, shape[1], 0),
                        (1, 2, shape[1], 1),
                        (shape[0], 1, shape[1] + 1, 1),
                        (0, 1, 3, 0),
                        (1, 0, 3, 1),
                        (1, 1, 3, u64::MAX),
                        (1, 1, 1, 0),
                    ] {
                        let args = numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("input", Data::Matrix(cells.clone().into()), &shape),
                                ("row", Data::U64(row), &[]),
                                ("first", Data::U64(first), &[]),
                                ("last", Data::U64(last), &[]),
                                ("offset", Data::U64(offset), &[]),
                                ("gain", Data::F64(F64Bits::from_f64(9.0)), &[]),
                            ],
                        );
                        for restored in [false, true] {
                            compare(
                                &fast,
                                &old,
                                &artifact,
                                &args,
                                &memory,
                                mode.execution(),
                                restored,
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn cell_prefix_public_finite_caps_exact_reports_and_cleanup_after_prior_changes() {
    for expression in [
        "(a[r,c] + a[r,c + offset]) / gain",
        "a[c + offset] * gain + gain",
    ] {
        for artifact in artifacts(&source(expression, [3, 4])) {
            let (fast, old) = pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for restored in [false, true] {
                for zero in [false, true] {
                    let cells = (0..12)
                        .map(|v| {
                            Data::F64(F64Bits::from_f64(if zero { 0.0 } else { v as f64 + 1.0 }))
                        })
                        .collect::<Vec<_>>();
                    let mut outputs = vec![None; artifact.outputs().len()];
                    let mut previous = outputs.clone();
                    for last in [4, 5] {
                        let args = numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("input", Data::Matrix(cells.clone().into()), &[3, 4]),
                                ("row", Data::U64(2), &[]),
                                ("first", Data::U64(1), &[]),
                                ("last", Data::U64(last), &[]),
                                ("offset", Data::U64(1), &[]),
                                ("gain", Data::F64(F64Bits::from_f64(2.0)), &[]),
                            ],
                        );
                        compare_pair(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            mode.execution(),
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                        let mut cap = mode.execution();
                        cap.library.max_compute_work = Some(1);
                        compare_pair(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            cap,
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                        cap = mode.execution();
                        cap.library.max_cloned_bytes = Some(0);
                        compare_pair(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            cap,
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                        compare_pair(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            mode.execution(),
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                    }
                    outputs.fill(None);
                    previous.fill(None);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained,
                        "cleanup after prior published values, late errors, finite denial and retry"
                    );
                }
            }
        }
    }
}
#[test]
fn cell_prefix_proof_tracks_full_source_read_tree_and_single_export_without_loop_crossing() {
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/polybench/seidel-2d.mec"
    )) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let metadata = jit.descriptor().metadata();
        let prefixes = metadata
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .flat_map(|p| numeric_cell_prefix::derive(&metadata.plan.facts(), p))
            .collect::<Vec<_>>();
        assert!(
            prefixes
                .iter()
                .any(|p| p.read_sites.len() == 9 && p.exports.len() == 1),
            "complete nine-read ordered cell expression"
        );
        for prefix in prefixes {
            assert!(prefix.sites.len() < 128);
            assert!(!prefix.sites.contains(&(prefix.next as usize)));
        }
    }
}

#[test]
fn cell_prefix_source_proof_declines_integer_reads_writes_and_branch_crossings() {
    let ordinary = source("a[r,c] + a[r,c + offset]", [2, 5]);
    let split = ordinary.replace("value := a[r,c] + a[r,c + offset]", "x := a[r,c]\n        ~changed := a\n        changed[r,c] = gain\n        value := x + changed[r,c + offset]");
    let branch = ordinary.replace("value := a[r,c] + a[r,c + offset]", "x := a[r,c]\n        choice := gain > 0.0\n        value := x + (choice? | true => a[r,c + offset] | false => a[r,c])");
    let cases = [ordinary.replace("f64", "u64"), split, branch];
    for (index, source) in cases.into_iter().enumerate() {
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let metadata = jit.descriptor().metadata();
            let prefixes = metadata
                .closed_tails
                .regions
                .iter()
                .filter_map(|r| r.numeric.as_ref())
                .flat_map(|p| numeric_cell_prefix::derive(&metadata.plan.facts(), p))
                .collect::<Vec<_>>();
            if index == 0 {
                assert!(
                    prefixes.is_empty(),
                    "integer reads cannot enter a floating expression"
                );
            }
            for prefix in prefixes {
                // A profitable single-read suffix is valid after a write or
                // branch; neither source boundary may belong to the prefix.
                assert_eq!(prefix.read_sites.len(), 1);
                for site in prefix.sites {
                    use mech_engine::native_artifact::{
                        NativeCollectionPrimitive as P, NativeIterationAction as A,
                        NativeIterationTerminator as T, NativeMatchTerminator as B,
                    };
                    let block = &metadata.plan.blocks()[site];
                    assert!(matches!(block.terminator(), T::Scalar(B::Jump(_))));
                    assert!(
                        match block.action() {
                            A::Scalar(_) => true,
                            A::LibraryCall { template, .. } => matches!(
                                template.primitive(),
                                P::MatrixCellAccess | P::MatrixScalarAccess
                            ),
                            _ => false,
                        },
                        "no prefix contains the intervening source write or branch"
                    );
                }
            }
        }
    }
}

#[test]
fn cell_prefix_load_only_escaping_values_keep_original_checks_and_public_errors() {
    let ordinary = source("a[r,c] + a[r,c + offset]", [2, 5]);
    let text = ordinary.replace("value := a[r,c] + a[r,c + offset]", "left := a[r,c]\n        right := a[r,c + offset]\n        value := ((left < right)? | true => right | false => left)");
    assert_ne!(ordinary, text);
    let partial = text.replace(
        "right := a[r,c + offset]",
        "right := a[r,c + offset] + gain",
    );
    let bare_linear = source("a[c + offset]", [2, 5]);
    for text in [text, partial, bare_linear] {
        for artifact in artifacts(&text) {
            let catalog = mech_stdlib::source_catalog();
            let fast = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::CellExpressionPrefixes,
            )
            .unwrap();
            let metadata = fast.descriptor().metadata();
            assert!(
                metadata
                    .closed_tails
                    .regions
                    .iter()
                    .filter_map(|r| r.numeric.as_ref())
                    .all(
                        |proof| numeric_cell_prefix::derive(&metadata.plan.facts(), proof)
                            .is_empty()
                    ),
                "two escaping loads collapse no floating classifications"
            );
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert!(
                object.metadata()["compiled_numeric_write_loops"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["cell_expression_prefixes"]
                        .as_array()
                        .unwrap()
                        .is_empty()),
                "ordinary native checks actually emitted"
            );
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            for special in [
                None,
                Some(0x7ff0_1234_5678_9abcu64),
                Some(f64::INFINITY.to_bits()),
            ] {
                let mut cells = (0..10)
                    .map(|n| Data::F64(F64Bits::from_f64(n as f64 / 13.0)))
                    .collect::<Vec<_>>();
                if let Some(bits) = special {
                    cells[3] = Data::F64(F64Bits::from_bits(bits));
                }
                for last in [5, 6] {
                    let args = numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 5]),
                            ("row", Data::U64(1), &[]),
                            ("first", Data::U64(1), &[]),
                            ("last", Data::U64(last), &[]),
                            ("offset", Data::U64(1), &[]),
                            ("gain", Data::F64(F64Bits::from_f64(9.0)), &[]),
                        ],
                    );
                    for restored in [false, true] {
                        compare(
                            &fast,
                            &old,
                            &artifact,
                            &args,
                            &memory,
                            mode.execution(),
                            restored,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cell_prefix_source_proof_admits_single_linear_recurrences_without_crossing_writes() {
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/polybench/deriche.mec"
    )) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let metadata = jit.descriptor().metadata();
        let prefixes = metadata
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .flat_map(|p| numeric_cell_prefix::derive(&metadata.plan.facts(), p))
            .collect::<Vec<_>>();
        assert_eq!(prefixes.len(), 2, "both source causal filter orientations");
        for prefix in prefixes {
            assert_eq!(prefix.read_sites.len(), 1);
            assert_eq!(
                prefix.exports.len(),
                1,
                "only the ordered recurrence result escapes"
            );
            assert!(prefix.sites.len() >= 8);
            assert!(!prefix.sites.contains(&(prefix.next as usize)));
        }
    }
}
