//! Actual parent/child admissions, current roots, finite funding and recovery.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_core::ValueDataDraft as Data;
use mech_engine::ProgramArtifact;
mod admission_overlap;
mod bit_counts;
mod boolean_edges;
mod borrowed_record_map;
mod bounds_retry;
mod bulk_bytes;
mod captured_numeric;
mod captured_window_handoffs;
mod captured_windows;
mod child_cleanup;
mod child_fuel;
mod child_funding;
mod child_prepayment;
mod cold_child;
mod cold_intervals;
mod collection_lifetime;
mod current_case;
mod current_windows;
mod deep_nested_scalar;
mod exit_transport;
mod failure_transport;
mod fresh_traversals;
mod guarded_pack_failure;
mod guarded_read_packs;
mod hybrid_intervals;
mod invariant_numeric;
mod managed_carriers;
mod managed_projections;
mod mixed_string_map;
mod multiple_children;
mod mutable_children;
mod named_managed_projections;
mod nested_numeric;
mod nested_phase;
mod nested_phase_growing;
mod nested_phase_handoff;
mod nested_scalar_array;
mod numeric_bindings;
mod numeric_induction;
mod numeric_tree;
pub(super) mod observe;
mod ordered_reduction;
mod ordered_reduction_geometry;
mod ordered_reduction_terminal;
mod pack;
mod phase_descendants;
mod physical_exits;
mod read_packs;
mod record_lookup;
mod resources;
mod resumed_child;
mod retained_compound_map;
mod scalar_child;
mod scalar_selects;
mod schema_key;
mod tagged_map_carrier;
mod tuple_capture;
mod utf8_reads;
mod virtual_retirement;
mod wide_scalar_array;
mod word_collection_vectors;
mod word_collections;

/// Retain direct coverage of the independently proved around-child path. A
/// complete tree normally consumes these source sites before the pieces run;
/// this emission-only ablation leaves their source proofs and guards intact.
fn compile_parent_pieces(
    artifact: &ProgramArtifact,
    catalog: &mech_core::FunctionCatalog,
) -> Result<JitProgramArtifact, crate::native_artifact::NativeCompileError> {
    JitProgramArtifact::compile_for_test(
        artifact,
        catalog,
        crate::native_artifact::codegen::options::OptimizationPass::NumericTrees,
    )
}

#[test]
fn huffman_parent_cleanup_selects_funded_retirement_batches() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = compile_parent_pieces(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = huffman_inputs(&artifact, 3, vec![0, 1, 1, 2], &memory);
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut expected_work = None;
        for _ in 0..2 {
            output.fill(None);
            super::retirement::take_observation();
            super::retirement::take_child_observation();
            let (result, observed) =
                observe::run(&jit, &args, &mut output, mode.execution(), &memory);
            let report = result.unwrap();
            let batches = super::retirement::take_observation();
            let omitted = super::retirement::take_child_observation();
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([3, 1, 3, 2].map(Data::U8).into()),
            );
            assert_eq!(report.work, *expected_work.get_or_insert(report.work));
            assert_eq!(
                crate::native_artifact::NATIVE_PROGRAM_WORK_POLICY_VERSION,
                115
            );
            // Policies107/108/110/111 add fills, recursive frames, worker
            // transport and selected String-matrix scalar projections.
            // This fixture has none, so the policy105 decomposition below
            // remains exact and independently checked.
            // The policy104 checkpoint for this exact fixture was 88,531,834.
            // Emission ablation preserves the new tree's independently checked
            // proof and fallback inventory. Policy105 pays their larger shared
            // setup and source-site fees even when the smaller pieces execute.
            // Account for every extra unit from prepared bounds and actual
            // visits, rather than replacing the historical golden bill.
            let metadata = jit.descriptor().metadata();
            let fallback = metadata
                .closed_tails
                .regions
                .iter()
                .filter(|region| !region.numeric_subtree())
                .collect::<Vec<_>>();
            assert_eq!(fallback.len(), 1);
            let added_price = |site: usize| {
                let old_price = if fallback.iter().any(|region| {
                    site == region.tail.initialize() as usize
                        || site == region.tail.exit() as usize
                        || region
                            .numeric
                            .as_ref()
                            .is_some_and(|proof| proof.sites.contains(&site))
                }) {
                    crate::native_artifact::program::prepare::tail::OVERHEAD
                } else {
                    0
                };
                metadata.closed_tails.extra_work(site) - old_price
            };
            let ordinary = observed
                .admits
                .iter()
                .map(|(&site, &count)| added_price(site as usize) * count)
                .sum::<u64>();
            let pieces = observed
                .selected_sites
                .iter()
                .map(|(&site, &count)| {
                    let id = metadata.compositions.entry(site as usize).unwrap();
                    let (_, segment) = metadata.compositions.region(id).unwrap();
                    segment
                        .sites
                        .iter()
                        .map(|&site| added_price(site as usize))
                        .sum::<u64>()
                        * count
                })
                .sum::<u64>();
            // Policy104 had 26 logical closed-tail words and 27 argument
            // words. The session prices both word banks, then argument setup.
            let extra_arguments = metadata.max_library_inputs as u64 - 27;
            let setup = (metadata.closed_tails.logical_words as u64 - 26 + extra_arguments) * 4
                + extra_arguments * (core::mem::size_of::<Option<Value>>() as u64 * 8 + 64);
            let child_entries = observed
                .helpers
                .iter()
                .filter(|((_, op), _)| *op == GlobalHelper::ClosedTailBegin.raw())
                .map(|(_, count)| count)
                .sum::<u64>();
            let entries = (metadata.closed_tails.regions.len() as u64 - 1) * 128 * child_entries;
            assert_eq!(child_entries, 3);
            assert_eq!(
                (setup, entries, ordinary, pieces),
                (12_372, 384, 12_288, 94_208)
            );
            assert_eq!(
                report.work,
                88_531_834 + setup + entries + ordinary + pieces
            );
            println!("huffman-cleanup work={} batches={batches}", report.work);
            assert!(
                batches > 0,
                "original Huffman parent must use funded cleanup"
            );
            assert!(
                omitted > 0,
                "completed Huffman children must avoid repeated physical cleanup"
            );
        }
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn complete_numeric_tree_and_retained_parent_fallback_preserve_the_same_source_outputs() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = compile_parent_pieces(&artifact, &catalog).unwrap();
        let complete =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let region = complete
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| region.numeric_subtree())
            .unwrap();
        let boundaries = if let Some(tree) = &region.numeric_tree {
            tree.nodes
                .iter()
                .skip(1)
                .flat_map(|node| [node.tail.initialize(), node.tail.header(), node.tail.exit()])
                .collect::<std::collections::BTreeSet<_>>()
        } else {
            region
                .numeric
                .as_ref()
                .unwrap()
                .phase
                .as_ref()
                .unwrap()
                .loops
                .iter()
                .skip(1)
                .flat_map(|node| [node.initialize as u32, node.header as u32, node.exit as u32])
                .collect()
        };
        for restored in [false, true] {
            for (jit, tree_enabled) in [(&ordinary, false), (&complete, true)] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (width, counts, expected) in [
                    (0, vec![], vec![9, 9, 9, 9]),
                    (3, vec![0, 0, 0, 0], vec![9, 9, 9, 9]),
                    (3, vec![0, 1, 1, 2], vec![3, 1, 3, 2]),
                ] {
                    let args = huffman_inputs(&artifact, width, counts, &memory);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (result, observed) = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory)
                    };
                    result.unwrap();
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(expected.into_iter().map(Data::U8).collect())
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    if tree_enabled {
                        assert!(
                            observed
                                .closed_tail_exits
                                .iter()
                                .any(|row| row[0] == u64::from(region.tail.exit()) && row[3] == 0),
                            "the complete tree must actually lend and settle: {observed:?}"
                        );
                        for boundary in &boundaries {
                            assert_eq!(
                                observed
                                    .admission_callbacks
                                    .get(&u64::from(*boundary))
                                    .copied()
                                    .unwrap_or(0),
                                0
                            );
                            assert!(
                                observed
                                    .helpers
                                    .keys()
                                    .all(|(site, _)| *site != u64::from(*boundary))
                            );
                        }
                    } else {
                        assert!(
                            observed.selected > 0,
                            "the retained smaller proof must actually select: {observed:?}"
                        );
                        assert!(
                            !observed
                                .closed_tail_exits
                                .iter()
                                .any(|row| row[0] == u64::from(region.tail.exit()) && row[3] == 0),
                            "the ablation must not execute the complete tree: {observed:?}"
                        );
                    }
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

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let original = super::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, Data)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = named.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            let shape_values: Box<[u64]> = if let Data::Matrix(cells) = &data {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, cells.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .into()
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
            let mut reserve = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reserve).unwrap()
        })
        .collect()
}
fn huffman_inputs(
    artifact: &ProgramArtifact,
    width: u64,
    counts: Vec<u64>,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
            ("lens", Data::Matrix([9, 9, 9, 9].map(Data::U8).into())),
            (
                "counts",
                Data::Matrix(counts.into_iter().map(Data::U64).collect()),
            ),
            ("width", Data::U64(width)),
            ("start", Data::U64(0)),
        ],
        memory,
    )
}
fn contents(values: &[Value]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.canonical_data_draft().unwrap())
        .collect()
}

#[test]
fn composition_counts_real_child_initializations_and_completed_empty_calls() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = compile_parent_pieces(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let proof = &jit.descriptor().metadata().compositions.parents[0];
        let mut output = vec![None; artifact.outputs().len()];
        let memory = mode.memory_budget();
        for (width, counts, expected_calls, expected) in [
            (0, vec![], 0, vec![9, 9, 9, 9]),
            (3, vec![0, 0, 0, 0], 3, vec![9, 9, 9, 9]),
            (3, vec![0, 1, 1, 2], 3, vec![3, 1, 3, 2]),
        ] {
            let args = huffman_inputs(&artifact, width, counts, &memory);
            let before = contents(&args);
            let (result, actual) =
                observe::run(&jit, &args, &mut output, mode.execution(), &memory);
            result.unwrap();
            assert!(actual.selected > 0);
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(proof.children[0].child_initialize))
                    .copied()
                    .unwrap_or(0),
                expected_calls
            );
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(proof.children[0].child_exit))
                    .copied()
                    .unwrap_or(0),
                expected_calls
            );
            if width == 0 || expected == vec![9, 9, 9, 9] {
                let writes = jit
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .find(|r| r.iteration == proof.children[0].child)
                    .unwrap()
                    .numeric
                    .as_ref()
                    .unwrap();
                for &(site, _) in writes.actions.iter().filter(|(_, op)| matches!(op, crate::native_artifact::program::prepare::readonly_numeric::Operation::Write { .. })) {
                    assert!(
                        actual.helpers.keys().all(|&(s, _)| s != site as u64),
                        "empty child performed a first write: {actual:?}"
                    );
                }
            }
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.into_iter().map(Data::U8).collect())
            );
            println!(
                "{}",
                serde_json::json!({"witness":"actual-child-boundaries", "width":width, "child_calls":expected_calls, "selected_parent_fragments":actual.selected})
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn composition_native_parent_resumes_after_an_ordinary_empty_child() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = inputs(
            &artifact,
            &[
                ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
                ("lens", Data::Matrix(vec![Data::U8(9); 8].into())),
                ("counts", Data::Matrix([0, 0, 0, 0].map(Data::U64).into())),
                ("width", Data::U64(3)),
                ("start", Data::U64(0)),
            ],
            &memory,
        );
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = mode.execution();
        // The child preview must decline its possible eight-element write.
        // Its actual empty path constructs only the existing two-field result.
        // Parent frame packs have at most five fields and remain eligible.
        policy.library.max_output_elements = Some(5);
        let (result, counts) = observe::run(&jit, &args, &mut output, policy, &memory);
        result.unwrap();
        assert!(counts.selected > 0);
        let proof = &jit.descriptor().metadata().compositions.parents[0];
        let mech_engine::native_artifact::NativeControlLoop::Tail(child) =
            &jit.descriptor().plan().iterations()[proof.children[0].child as usize]
        else {
            panic!()
        };
        for site in [
            proof.children[0].child_initialize,
            child.body(),
            proof.children[0].child_exit,
        ] {
            assert_eq!(
                counts.admits.get(&u64::from(site)).copied().unwrap_or(0),
                3,
                "ordinary child was replayed or skipped: {counts:?}"
            );
        }
        assert_eq!(contents(&args), before);
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Matrix(vec![Data::U8(9); 8].into())
        );
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

mod uniform_fill;

mod numeric_maps;

#[test]
fn huffman_parent_cleanup_selects_funded_retirement_batches_latest_phase() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = huffman_inputs(&artifact, 3, vec![0, 1, 1, 2], &memory);
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut expected_work = None;
        for _ in 0..2 {
            output.fill(None);
            super::retirement::take_observation();
            super::retirement::take_child_observation();
            let report = jit
                .evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
            let batches = super::retirement::take_observation();
            let omitted = super::retirement::take_child_observation();
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([3, 1, 3, 2].map(Data::U8).into()),
            );
            assert_eq!(report.work, *expected_work.get_or_insert(report.work));
            assert_eq!(
                crate::native_artifact::NATIVE_PROGRAM_WORK_POLICY_VERSION,
                115
            );
            assert_eq!(
                report.work, 41_515_826,
                "policy115 retains child fallback banks (+296 setup work); the complete phase keeps its source prices"
            );
            println!("huffman-cleanup work={} batches={batches}", report.work);
            assert!(
                batches > 0,
                "original Huffman parent must use funded cleanup"
            );
            assert!(
                omitted > 0,
                "completed Huffman children must avoid repeated physical cleanup"
            );
        }
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn selected_native_sites(jit: &JitProgramArtifact) -> std::collections::BTreeSet<u64> {
    let metadata = jit.descriptor().metadata();
    metadata
        .compositions
        .parents
        .iter()
        .flat_map(|parent| &parent.segments)
        .filter(|piece| metadata.compositions.entry(piece.entry as usize).is_some())
        .flat_map(|piece| piece.sites.iter().map(|&site| u64::from(site)))
        .chain(
            metadata
                .closed_tails
                .regions
                .iter()
                .filter_map(|region| region.numeric.as_ref())
                .flat_map(|proof| proof.sites.iter().map(|&site| site as u64)),
        )
        .collect()
}

#[test]
fn composition_counts_real_child_initializations_and_completed_empty_calls_latest_phase() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let region = metadata
            .closed_tails
            .regions
            .iter()
            .find(|region| {
                region.tail.parent().is_none()
                    && region
                        .numeric
                        .as_ref()
                        .is_some_and(|proof| proof.phase.is_some())
            })
            .unwrap();
        let phase = region.numeric.as_ref().unwrap();
        assert_eq!(phase.phase.as_ref().unwrap().loops.len(), 2);
        let child = &phase.phase.as_ref().unwrap().loops[1];
        let mut output = vec![None; artifact.outputs().len()];
        let memory = mode.memory_budget();
        for (width, counts, expected_calls, expected) in [
            (0, vec![], 0, vec![9, 9, 9, 9]),
            (3, vec![0, 0, 0, 0], 3, vec![9, 9, 9, 9]),
            (3, vec![0, 1, 1, 2], 3, vec![3, 1, 3, 2]),
        ] {
            let args = huffman_inputs(&artifact, width, counts, &memory);
            let before = contents(&args);
            let (result, actual) =
                observe::run(&jit, &args, &mut output, mode.execution(), &memory);
            result.unwrap();
            assert_eq!(actual.phases, 1);
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(child.initialize as u64))
                    .copied()
                    .unwrap_or(0),
                0
            );
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(child.exit as u64))
                    .copied()
                    .unwrap_or(0),
                0
            );
            if width == 0 || expected == vec![9, 9, 9, 9] {
                let writes = phase;
                for &(site, _) in writes.actions.iter().filter(|(_, op)| matches!(op, crate::native_artifact::program::prepare::readonly_numeric::Operation::Write { .. })) {
                    assert!(
                        actual.helpers.keys().all(|&(s, _)| s != site as u64),
                        "empty child performed a first write: {actual:?}"
                    );
                }
            }
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.into_iter().map(Data::U8).collect())
            );
            println!(
                "{}",
                serde_json::json!({"witness":"actual-child-boundaries", "width":width, "source_child_calls":expected_calls, "selected_parent_fragments":actual.selected})
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn composition_native_parent_resumes_after_an_ordinary_empty_child_latest_phase() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = inputs(
            &artifact,
            &[
                ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
                ("lens", Data::Matrix(vec![Data::U8(9); 8].into())),
                ("counts", Data::Matrix([0, 0, 0, 0].map(Data::U64).into())),
                ("width", Data::U64(3)),
                ("start", Data::U64(0)),
            ],
            &memory,
        );
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = mode.execution();
        // The child preview must decline its possible eight-element write.
        // Its actual empty path constructs only the existing two-field result.
        // Parent frame packs have at most five fields and remain eligible.
        policy.library.max_output_elements = Some(5);
        let (result, counts) = observe::run(&jit, &args, &mut output, policy, &memory);
        result.unwrap();
        assert_eq!(
            counts.phases, 0,
            "tight limits must keep ordinary execution"
        );
        let metadata = jit.descriptor().metadata();
        let root = metadata
            .plan
            .iterations()
            .iter()
            .position(|loop_| loop_.as_ref().parent().is_none())
            .unwrap() as u32;
        let child_id = metadata
            .plan
            .iterations()
            .iter()
            .position(|loop_| loop_.as_ref().parent() == Some(root))
            .unwrap();
        let mech_engine::native_artifact::NativeControlLoop::Tail(child) =
            &jit.descriptor().plan().iterations()[child_id]
        else {
            panic!()
        };
        for site in [child.initialize(), child.body(), child.exit()] {
            assert_eq!(
                counts.admits.get(&u64::from(site)).copied().unwrap_or(0),
                3,
                "ordinary child was replayed or skipped: {counts:?}"
            );
        }
        assert_eq!(contents(&args), before);
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Matrix(vec![Data::U8(9); 8].into())
        );
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
