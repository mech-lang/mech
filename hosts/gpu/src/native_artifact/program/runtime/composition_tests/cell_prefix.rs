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
    for artifact in artifacts(&source("(a[r,c] + a[r,c + offset]) / gain", [3, 4])) {
        let (fast, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            for zero in [false, true] {
                let cells = (0..12)
                    .map(|v| Data::F64(F64Bits::from_f64(if zero { 0.0 } else { v as f64 + 1.0 })))
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
    for source in cases {
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let metadata = jit.descriptor().metadata();
            assert!(
                metadata
                    .closed_tails
                    .regions
                    .iter()
                    .filter_map(|r| r.numeric.as_ref())
                    .all(|p| numeric_cell_prefix::derive(&metadata.plan.facts(), p).is_empty()),
                "no prefix crosses an unsupported read kind, write or branch"
            );
        }
    }
}

const WINDOW: &str = "(a[r - 1u64,c - 1u64] + a[r - 1u64,c] + a[r - 1u64,c + 1u64] + a[r,c - 1u64] + a[r,c] + a[r,c + 1u64] + a[r + 1u64,c - 1u64] + a[r + 1u64,c] + a[r + 1u64,c + 1u64]) / gain";
fn rolling_recipes(jit: &JitProgramArtifact) -> Vec<numeric_cell_prefix::rolling::Recipe> {
    let metadata = jit.descriptor().metadata();
    metadata
        .closed_tails
        .regions
        .iter()
        .flat_map(|region| {
            let Some(proof) = region.numeric.as_ref() else {
                return Vec::new();
            };
            numeric_cell_prefix::derive(&metadata.plan.facts(), proof)
                .iter()
                .filter_map(|prefix| {
                    numeric_cell_prefix::rolling::Recipe::derive(
                        &metadata.plan.facts(),
                        &metadata.buffer_frames,
                        proof,
                        &region.tail,
                        prefix,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
fn rolling_pair(
    artifact: &ProgramArtifact,
) -> (JitProgramArtifact, JitProgramArtifact, JitProgramArtifact) {
    let (fast, original) = pair(artifact);
    let uncached = JitProgramArtifact::compile_for_test(
        artifact,
        &mech_stdlib::source_catalog(),
        OptimizationPass::RollingCellPrefixes,
    )
    .unwrap();
    let recipes = rolling_recipes(&fast);
    assert!(
        recipes
            .iter()
            .any(|r| r.keys.len() == 9 && r.new_reads == 3 && r.old.is_some()),
        "actual source-frame continuation and nine-to-three reuse recipe"
    );
    let object =
        crate::native_artifact::emit_program_object(artifact, &mech_stdlib::source_catalog())
            .unwrap();
    let rows = object.metadata()["compiled_numeric_write_loops"]
        .as_array()
        .unwrap()
        .clone();
    assert!(
        rows.iter().any(|r| r["rolling_cell_prefixes"]
            .as_array()
            .is_some_and(|v| v.iter().any(|v| v[3] == 9 && v[4] == 3))),
        "genuine Object emitted the rolling recipe"
    );
    assert!(
        rows.iter().any(|r| r["reused_cell_write_sources"]
            .as_array()
            .is_some_and(|v| !v.is_empty())),
        "exact center read supplies old-first write comparison"
    );
    (fast, uncached, original)
}
#[test]
fn cell_prefix_rolling_public_ordered_bits_late_failure_aliases_and_restored() {
    for shape in [[3, 3], [5, 8], [8, 5]] {
        for artifact in artifacts(&source(WINDOW, shape)) {
            let (fast, uncached, original) = rolling_pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            for special in [
                None,
                Some(0u64),
                Some((-0.0f64).to_bits()),
                Some(f64::MAX.to_bits()),
                Some(f64::INFINITY.to_bits()),
                Some(0x7ff0_1234_5678_9abc),
                Some(0xfff8_4321_1234_abcd),
            ] {
                let count = (shape[0] * shape[1]) as usize;
                let mut cells = (0..count)
                    .map(|n| Data::F64(F64Bits::from_f64((n as f64 - 7.0) / 13.0)))
                    .collect::<Vec<_>>();
                if let Some(bits) = special {
                    cells[count / 2] = Data::F64(F64Bits::from_bits(bits));
                }
                for (row, first, last) in [
                    (2, 2, shape[1]),
                    (2, 2, shape[1] + 1),
                    (0, 2, shape[1]),
                    (2, 0, shape[1]),
                    (u64::MAX, 2, shape[1]),
                    (2, 2, 2),
                ] {
                    let args = numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &shape),
                            ("row", Data::U64(row), &[]),
                            ("first", Data::U64(first), &[]),
                            ("last", Data::U64(last), &[]),
                            ("offset", Data::U64(1), &[]),
                            ("gain", Data::F64(F64Bits::from_f64(9.0)), &[]),
                        ],
                    );
                    for restored in [false, true] {
                        for old in [&uncached, &original] {
                            compare(
                                &fast,
                                old,
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
fn reset_source() -> String {
    format!(
        r#"#Roll(initial<[f64]:5,8>, turns<u64>, end<u64>, gain<f64>) => <[f64]:5,8>
  | :Sweep(a<[f64]:5,8>, remaining<u64>, r<u64>, c<u64>, end<u64>, gain<f64>)
  | :Done(a<[f64]:5,8>).
#Roll(initial,turns,end,gain) -> :Sweep(initial,turns,2u64,2u64,end,gain)
  :Sweep(a,remaining,r,c,end,gain)
    | remaining == 0u64 -> :Done(a)
    | r >= 5u64 -> :Sweep(a,remaining - 1u64,2u64,2u64,end,gain)
    | c >= end -> :Sweep(a,remaining,r + 1u64,2u64,end,gain)
    | * -> {{
        value := {WINDOW}
        ~next := a
        next[r,c] = value
      }} -> :Sweep(next,remaining,r,c + 1u64,end,gain)
  :Done(a) => a.
a := input<[f64]:5,8>
answer := #Roll(a,turns<u64>,last<u64>,gain<f64>)
(answer,a)
"#
    )
}
#[test]
fn cell_prefix_rolling_public_state_resets_noop_caps_previous_outputs_and_cleanup() {
    for artifact in artifacts(&reset_source()) {
        let (fast, uncached, original) = rolling_pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for old in [&uncached, &original] {
            for restored in [false, true] {
                for zero in [false, true] {
                    let cells = (0..40)
                        .map(|n| {
                            Data::F64(F64Bits::from_f64(if zero {
                                0.0
                            } else {
                                (n as f64 + 0.5) / 13.0
                            }))
                        })
                        .collect::<Vec<_>>();
                    let mut outputs = vec![None; artifact.outputs().len()];
                    let mut previous = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    for last in [8, 9] {
                        for gain in [9.0, 0.0] {
                            let args = numeric_maps::typed_arguments(
                                &artifact,
                                &memory,
                                &[
                                    ("input", Data::Matrix(cells.clone().into()), &[5, 8]),
                                    ("turns", Data::U64(3), &[]),
                                    ("last", Data::U64(last), &[]),
                                    ("gain", Data::F64(F64Bits::from_f64(gain)), &[]),
                                ],
                            );
                            let aliases = args.clone();
                            compare_pair(
                                &fast,
                                old,
                                &args,
                                &memory,
                                mode.execution(),
                                restored,
                                &mut outputs,
                                &mut previous,
                            );
                            for restriction in 0..3 {
                                let mut cap = mode.execution();
                                match restriction {
                                    0 => cap.library.max_compute_work = Some(1),
                                    1 => cap.library.max_cloned_bytes = Some(0),
                                    _ => cap.execution.max_work = 3,
                                };
                                compare_pair(
                                    &fast,
                                    old,
                                    &args,
                                    &memory,
                                    cap,
                                    restored,
                                    &mut outputs,
                                    &mut previous,
                                );
                            }
                            compare_pair(
                                &fast,
                                old,
                                &args,
                                &memory,
                                mode.execution(),
                                restored,
                                &mut outputs,
                                &mut previous,
                            );
                            assert_eq!(contents(&aliases), contents(&args));
                        }
                    }
                    outputs.fill(None);
                    previous.fill(None);
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
fn cell_prefix_rolling_source_refuses_unproved_column_row_matrix_and_mutation_edges() {
    let ordinary = source(WINDOW, [5, 8]);
    let cases = [
        ordinary.replace(":Sweep(next,r,c + 1u64", ":Sweep(next,r,c + 2u64"),
        ordinary.replace(":Sweep(next,r,c + 1u64", ":Sweep(next,r + 1u64,c + 1u64"),
        ordinary.replace(":Sweep(next,r,c + 1u64", ":Sweep(a,r,c + 1u64"),
        ordinary.replace("c - 1u64", "c - offset"),
        ordinary.replace(
            "next[r,c] = value",
            "next[r,c] = value\n        next[r,1u64] = gain",
        ),
    ];
    for text in cases {
        for artifact in artifacts(&text) {
            let native =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                rolling_recipes(&native).is_empty(),
                "no cache crosses an unproved state transition or additional mutation"
            );
        }
    }
}

#[test]
fn cell_prefix_rolling_first_change_after_noops_denial_previous_outputs_and_retry() {
    for artifact in artifacts(&source(WINDOW, [5, 8])) {
        let (fast, uncached, original) = rolling_pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let mut cells = vec![Data::F64(F64Bits::from_f64(1.0)); 40];
        // Row2 has four no-op points before this right-hand cell first
        // changes an output, after the cache has already been reused.
        cells[2 * 8 + 6] = Data::F64(F64Bits::from_f64(2.0));
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("input", Data::Matrix(cells.into()), &[5, 8]),
                ("row", Data::U64(2), &[]),
                ("first", Data::U64(2), &[]),
                ("last", Data::U64(8), &[]),
                ("offset", Data::U64(1), &[]),
                ("gain", Data::F64(F64Bits::from_f64(9.0)), &[]),
            ],
        );
        let aliases = args.clone();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for old in [&uncached, &original] {
            for restored in [false, true] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let mut previous = outputs.clone();
                compare_pair(
                    &fast,
                    old,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                let published = super::ordered_map_dots::output(&outputs);
                let (a, counts) = observe::run_with_first_change_copy_limit(
                    &fast,
                    &args,
                    &mut outputs,
                    mode.execution(),
                    &memory,
                    restored,
                    0,
                );
                let (c, other) = observe::run_with_first_change_copy_limit(
                    old,
                    &args,
                    &mut previous,
                    mode.execution(),
                    &memory,
                    restored,
                    0,
                );
                assert!(a.is_err(), "copy denial occurs after original no-op points");
                assert_eq!(
                    a.as_ref().map_err(|e| format!("{e:?}")),
                    c.as_ref().map_err(|e| format!("{e:?}"))
                );
                assert_eq!(counts.helpers, other.helpers);
                assert_eq!(counts.admits, other.admits);
                assert_eq!(super::ordered_map_dots::output(&outputs), published);
                assert_eq!(super::ordered_map_dots::output(&previous), published);
                assert_eq!(contents(&aliases), contents(&args));
                compare_pair(
                    &fast,
                    old,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                outputs.fill(None);
                previous.fill(None);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
        }
    }
}

#[test]
fn cell_prefix_cfg_readiness_source_entries_have_no_control_words_and_match_token() {
    for artifact in artifacts(&reset_source()) {
        let (fast, _, _) = rolling_pair(&artifact);
        let token = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::CellContextPrefixes,
        )
        .unwrap();
        let object =
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        assert!(
            object.metadata()["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["cfg_cell_prefixes"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|v| v[2].as_u64().unwrap() > 0 && v[3] == 0))),
            "genuine CFG fresh/reusable/write entries; zero carried scalar control words"
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for special in [None, Some(0x7ff0_1234_5678_9abcu64)] {
            let mut cells = (0..40)
                .map(|n| Data::F64(F64Bits::from_f64((n as f64 + 1.0) / 13.0)))
                .collect::<Vec<_>>();
            if let Some(bits) = special {
                cells[22] = Data::F64(F64Bits::from_bits(bits));
            }
            for (turns, last) in [(0, 8), (2, 8), (2, 9)] {
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", Data::Matrix(cells.clone().into()), &[5, 8]),
                        ("turns", Data::U64(turns), &[]),
                        ("last", Data::U64(last), &[]),
                        ("gain", Data::F64(F64Bits::from_f64(9.0)), &[]),
                    ],
                );
                for restored in [false, true] {
                    compare(
                        &fast,
                        &token,
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
