//! Nested numeric loops retain value semantics across reads, writes and failures.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

pub(super) const SOURCE: &str = r#"+> sequence
<tree-rank> := <((u8,u64),u64)>
<tree-state> := <([u8],tree-rank)>
tree-put(data<[u8]>, at<u64>, value<u8>) => (out<[u8]>) :=
  ~next := data
  next[at] = value
  out := next.
#TreeProbe(data<[u8]>, base<u64>, width<u64>, minimum<u8>) => <bool>
  | :Next(at<u64>, found<bool>)
  | :Done(found<bool>).
#TreeProbe(data, base, width, minimum) -> :Next(0u64, false)
  :Next(at, found)
    | at < width -> :Next(at + 1u64, found || (data[base + at + 1u64] > minimum))
    | * -> :Done(found)
  :Done(found) => found.
tree-lane(data<[u8]>, base<u64>, at<u64>, process<bool>, rank<tree-rank>) => (out<tree-state>) :=
  position := base + at
  count := data[position + 1u64]
  chosen := ((process && (count > rank.1.1))? | true => (count, position) | false => rank.1)
  next-rank := (chosen, rank.2 + 1u64)
  next := tree-put(data, position + 1u64, 0u8)
  out := (next, next-rank).
tree-child(data<[u8]>, base<u64>, at<u64>, width<u64>, process<bool>, rank<tree-rank>) => (out<tree-state>) :=
  more := at < width
  step := (more? | true => tree-lane(data, base, at, process, rank) | false => (data, rank))
  (next, next-rank) := step
  out := (more? | true => tree-child(next, base, at + 1u64, width, process, next-rank) | false => (next, next-rank)).
tree-parent(data<[u8]>, round<u64>, rounds<u64>, width<u64>, rank<tree-rank>) => (out<tree-state>) :=
  more := round < rounds
  base := round * width
  process := (more? | true => #TreeProbe(data, base, width, rank.1.1) | false => false)
  step := (more? | true => tree-child(data, base, 0u64, width, process, rank) | false => (data, rank))
  (next, next-rank) := step
  out := (more? | true => tree-parent(next, round + 1u64, rounds, width, next-rank) | false => (next, next-rank)).
result := tree-parent(bytes<[u8]>, 0u64, rounds<u64>, width<u64>, ((minimum<u8>, winner<u64>), marker<u64>))
(result.1, result.2.1.1, result.2.1.2, result.2.2, bytes)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    bytes: &[u8],
    rounds: u64,
    width: u64,
    marker: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "bytes",
                Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
            ),
            ("rounds", Data::U64(rounds)),
            ("width", Data::U64(width)),
            ("minimum", Data::U8(7)),
            ("winner", Data::U64(93)),
            ("marker", Data::U64(marker)),
        ],
        memory,
    )
}

fn last(output: &[Option<Value>]) -> Data {
    output
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}

fn expected(bytes: &[u8], rounds: usize, width: usize, marker: u64) -> Data {
    let mut current = bytes.to_vec();
    let (mut best, mut winner, mut visits) = (7, 93, marker);
    for round in 0..rounds {
        let base = round * width;
        let process = current[base..base + width]
            .iter()
            .any(|count| *count > best);
        for at in base..base + width {
            if process && current[at] > best {
                best = current[at];
                winner = at as u64;
            }
            visits += 1;
            current[at] = 0;
        }
    }
    Data::Tuple(
        [
            Data::Matrix(current.into_iter().map(Data::U8).collect()),
            Data::U8(best),
            Data::U64(winner),
            Data::U64(visits),
            Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
        ]
        .into(),
    )
}

#[test]
fn numeric_tree_source_liveness_ablation_keeps_outputs_errors_and_retry() {
    check_emission_ablation(OptimizationPass::SourceLiveness);
}

#[test]
fn numeric_tree_boolean_branch_ablation_keeps_reports_errors_and_retry() {
    check_emission_ablation(OptimizationPass::BooleanBranches);
}

#[test]
fn boolean_branch_proof_keeps_unknown_inputs_library_words_and_integer_matches() {
    use crate::native_artifact::codegen::common::boolean_branches;
    use crate::native_artifact::codegen::control::plan::CodePlan;
    use crate::native_artifact::codegen::options::OptimizationOptions;
    use crate::native_artifact::program::prepare::PreparedNativeProgram;
    use mech_engine::native_artifact::{
        NativeIterationTerminator as T, NativeMatchTerminator as B,
    };
    for text in [
        "gate := flag<bool>\n(gate? | true => 1u64 | false => 2u64)\n",
        "+> map\nindex<{string:u64}> := map/new()\ngate := word<string> ∈ index\n(gate? | true => 1u64 | false => 2u64)\n",
        "value := number<u64>\n(value? | 0u64 => 1u64 | 1u64 => 2u64 | * => 3u64)\n",
    ] {
        for artifact in artifacts(text) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let prepared = PreparedNativeProgram::prepare(
                &artifact,
                &catalog,
                native.descriptor().plan().clone(),
            )
            .unwrap();
            let code = CodePlan::Program(&prepared);
            let mut branches = 0;
            for (site, block) in code.blocks().iter().enumerate() {
                if matches!(block.terminator(), T::Scalar(B::LiteralBranch { .. })) {
                    branches += 1;
                    assert!(
                        boolean_branches::target(code, site, OptimizationOptions::default())
                            .is_none(),
                        "an unproved result or integer match must retain its original test: {site}"
                    );
                }
            }
            assert!(branches >= 2);
        }
    }
}

fn check_emission_ablation(pass: OptimizationPass) {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let conservative = JitProgramArtifact::compile_for_test(&artifact, &catalog, pass).unwrap();
        if pass == OptimizationPass::BooleanBranches {
            assert!(
                native.descriptor().code_bytes() < conservative.descriptor().code_bytes(),
                "native-produced Boolean branches must remove emitted comparisons"
            );
        }
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for (rounds, width, marker, fails) in [
                (0, 0, 17, false),
                (2, 2, 11, false),
                (2, 3, 11, true),
                (1, 2, u64::MAX, true),
                (1, 4, 23, false),
            ] {
                let args = arguments(&artifact, &memory, &[7, 12, 31, 2], rounds, width, marker);
                let aliases = args.clone();
                let before = contents(&args);
                let mut results = Vec::new();
                for (jit, output) in [&native, &conservative].into_iter().zip(&mut outputs) {
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory).0
                    };
                    if fails {
                        results.push(format!("{:?}", result.unwrap_err()));
                        assert!(output.iter().zip(prior).all(|(now, old)| {
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        results.push(format!("{:?}", result.unwrap()));
                        assert_eq!(
                            last(output),
                            expected(&[7, 12, 31, 2], rounds as usize, width as usize, marker)
                        );
                    }
                }
                assert_eq!(results[0], results[1]);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_tree_nested_read_then_write_uses_current_cells_and_nested_scalars() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (bytes, rounds, width, marker) in [
                (vec![], 0, 0, 81),
                (vec![7, 12, 12, 2, 31, 31, 3, 9], 2, 4, 17),
                (vec![7, 7, 7, 7], 2, 2, 99),
                (vec![1, 4, 9, 15, 15, 11], 3, 2, 3),
                (vec![31, 12], 3, 0, 41),
                (vec![8, 13, 21, 34], 0, 4, 57),
            ] {
                let args = arguments(&artifact, &memory, &bytes, rounds, width, marker);
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap();
                assert_eq!(
                    last(&output),
                    expected(&bytes, rounds as usize, width as usize, marker)
                );
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

#[test]
fn numeric_tree_generated_child_edges_do_not_reenter_runtime_boundaries() {
    for artifact in artifacts(SOURCE) {
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
                region
                    .numeric_tree
                    .as_ref()
                    .is_some_and(|tree| tree.nodes.len() >= 3)
            })
            .expect("the parent, captured read-only probe and writer must share one selected tree");
        let tree = region.numeric_tree.as_ref().unwrap();
        assert_eq!(tree.nodes[0].parent, None);
        let root_initialize = tree.nodes[0].tail.initialize();
        let mut child_boundaries = Vec::new();
        for node in &tree.nodes[1..] {
            assert!(node.parent.is_some());
            child_boundaries.extend([node.tail.initialize(), node.tail.header(), node.tail.exit()]);
        }
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let bytes = [7, 12, 12, 2, 31, 31, 3, 9, 11, 19, 27, 5];
            let args = arguments(&artifact, &memory, &bytes, 3, 4, 17);
            let mut output = vec![None; artifact.outputs().len()];
            let (result, calls) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(last(&output), expected(&bytes, 3, 4, 17));
            assert!(
                calls
                    .helpers
                    .get(&(
                        u64::from(root_initialize),
                        GlobalHelper::ClosedTailBegin.raw()
                    ))
                    .copied()
                    .unwrap_or(0)
                    > 0,
                "the real generated entry must select its root loan: {calls:?}"
            );
            for site in &child_boundaries {
                assert_eq!(
                    calls
                        .admission_callbacks
                        .get(&u64::from(*site))
                        .copied()
                        .unwrap_or(0),
                    0,
                    "child boundary admission remains a callback at {site}: {calls:?}"
                );
                assert!(
                    calls
                        .helpers
                        .keys()
                        .all(|(called, _)| *called != u64::from(*site)),
                    "child boundary remains a runtime callback at {site}: {calls:?}"
                );
            }
            let first_writes: u64 = calls
                .helpers
                .iter()
                .filter(|((_, operation), _)| *operation == GlobalHelper::NumericWritePrepare.raw())
                .map(|(_, visits)| *visits)
                .sum();
            assert_eq!(
                first_writes, 1,
                "the same array must keep its prepared owner across all three writer activations: {calls:?}"
            );
            drop((args, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_tree_cold_index_and_scalar_errors_keep_old_outputs_and_allow_retry() {
    let mut reference_errors = [None, None];
    for artifact in artifacts(SOURCE) {
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
        for jit in [&ordinary, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (rounds, width, marker, rejected) in [
                    (1, 2, 11, None),
                    (2, 3, 11, Some((0, "Index"))),
                    (1, 2, u64::MAX, Some((1, "Arithmetic"))),
                    (2, 2, 23, None),
                ] {
                    let args =
                        arguments(&artifact, &memory, &[7, 12, 31, 2], rounds, width, marker);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory).0
                    };
                    if let Some((index, kind)) = rejected {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(kind), "{error}");
                        assert_eq!(error, *reference_errors[index].get_or_insert(error.clone()));
                        for (now, old) in output.iter().zip(&prior) {
                            assert!(
                                now.as_ref()
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
                        assert_eq!(
                            last(&output),
                            expected(&[7, 12, 31, 2], rounds as usize, width as usize, marker)
                        );
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

#[test]
fn numeric_tree_finite_boundaries_preserve_publication_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[7, 12, 31, 2], 2, 2, 11)
    });
}

#[test]
fn numeric_tree_finite_fuel_reconstructs_each_live_loop_before_continuing() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| {
                region
                    .numeric_tree
                    .as_ref()
                    .is_some_and(|tree| tree.nodes.len() >= 3)
            })
            .expect("exercise the complete numeric tree");
        let tree = region.numeric_tree.as_ref().unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let bytes = [7, 12, 12, 2, 31, 31, 3, 9, 11, 19, 27, 5];
            let args = arguments(&artifact, &memory, &bytes, 3, 4, 17);
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            let (result, _) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            result.unwrap();
            // Calibrate with the same prior publication as every finite trial.
            // A first invocation has a different admitted output geometry.
            let mut warm = output.clone();
            let (result, full) = if restored {
                observe::run_restored(&jit, &args, &mut warm, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut warm, mode.execution(), &memory)
            };
            result.unwrap();
            drop(warm);
            let terminal = full
                .closed_tail_exits
                .iter()
                .find(|row| row[0] == u64::from(region.tail.exit()) && row[3] == 0)
                .expect("the actual shared loan must reach its terminal boundary");
            let used = *full.closed_tail_work.get(&terminal[0]).unwrap();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let prior = output.clone();
            let mut reached = std::collections::BTreeSet::new();
            for node in &tree.nodes {
                for withheld in 1..=20 {
                    let Some(limit) = node
                        .compute
                        .checked_mul(withheld)
                        .and_then(|debit| used.checked_sub(debit))
                    else {
                        continue;
                    };
                    let mut policy = mode.execution();
                    policy.execution.max_work = limit;
                    let mut trial = prior.clone();
                    let (result, actual) = if restored {
                        observe::run_restored(&jit, &args, &mut trial, policy, &memory)
                    } else {
                        observe::run(&jit, &args, &mut trial, policy, &memory)
                    };
                    for row in actual.closed_tail_exits.iter().filter(|row| row[3] == 1) {
                        assert!(
                            tree.nodes
                                .iter()
                                .any(|node| u64::from(node.tail.header()) == row[0])
                        );
                        reached.insert(row[0]);
                    }
                    if result.is_ok() {
                        assert_eq!(last(&trial), expected(&bytes, 3, 4, 17));
                    } else {
                        for (now, old) in trial.iter().zip(&prior) {
                            assert!(
                                now.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            );
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    drop(trial);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            assert!(
                reached.len() >= 2,
                "finite fuel must exercise actual different loop headers, not just setup refusal: {reached:?}; used={used}; full={:?}",
                full.closed_tail_exits
            );
            jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
            assert_eq!(last(&output), expected(&bytes, 3, 4, 17));
            drop((args, aliases, output, prior));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

const OLD_ALIAS: &str = r#"+> sequence
<alias-pair> := <([u8],[u8])>
alias-put(data<[u8]>, at<u64>, value<u8>) => (out<[u8]>) :=
  ~next := data
  next[at] = value
  out := next.
alias-child(data<[u8]>, at<u64>, end<u64>) => (out<[u8]>) :=
  more := at < end
  next := (more? | true => alias-put(data, at + 1u64, data[at + 1u64] + 1u8) | false => data)
  out := (more? | true => alias-child(next, at + 1u64, end) | false => next).
alias-parent(data<[u8]>, old<[u8]>, rounds<u64>, width<u64>) => (out<alias-pair>) :=
  more := rounds > 0u64
  next := (more? | true => alias-child(data, 0u64, width) | false => data)
  out := (more? | true => alias-parent(next, old, rounds - 1u64, width) | false => (data, old)).
result := alias-parent(bytes<[u8]>, bytes, rounds<u64>, width<u64>)
(result.1, result.2, bytes)
"#;

const APPEND_TREE: &str = r#"+> sequence
append-inner(data<[u64]>, a<u64>, b<u64>, c<u64>, end<u64>) => (out<[u64]>) :=
  more := c < end
  word := (a * end + b) * end + c
  next := (more? | true => sequence/append(data, word) | false => data)
  out := (more? | true => append-inner(next, a, b, c + 1u64, end) | false => data).
append-middle(data<[u64]>, a<u64>, b<u64>, end<u64>) => (out<[u64]>) :=
  more := b + 1u64 < end
  next := (more? | true => append-inner(data, a, b, b + 1u64, end) | false => data)
  out := (more? | true => append-middle(next, a, b + 1u64, end) | false => data).
append-outer(data<[u64]>, a<u64>, end<u64>) => (out<[u64]>) :=
  more := a + 2u64 < end
  next := (more? | true => append-middle(data, a, a + 1u64, end) | false => data)
  out := (more? | true => append-outer(next, a + 1u64, end) | false => data).
buffer := sequence/with-capacity(0u64, capacity<u64>)
append-outer(buffer, 0u64, end<u64>)
"#;

const APPEND_CAPTURE_TREE: &str = r#"+> sequence
capture-inner(data<[u64]>, ids<[u64]>, a<u64>, b<u64>, c<u64>, end<u64>) => (out<[u64]>) :=
  more := c < end
  word := (more? | true => (ids[a + 1u64] * end + ids[b + 1u64]) * end + ids[c + 1u64] | false => 0u64)
  next := (more? | true => sequence/append(data, word) | false => data)
  out := (more? | true => capture-inner(next, ids, a, b, c + 1u64, end) | false => data).
capture-middle(data<[u64]>, ids<[u64]>, a<u64>, b<u64>, end<u64>) => (out<[u64]>) :=
  more := b + 1u64 < end
  next := (more? | true => capture-inner(data, ids, a, b, b + 1u64, end) | false => data)
  out := (more? | true => capture-middle(next, ids, a, b + 1u64, end) | false => data).
capture-outer(data<[u64]>, ids<[u64]>, a<u64>, end<u64>) => (out<[u64]>) :=
  more := a + 2u64 < end
  next := (more? | true => capture-middle(data, ids, a, a + 1u64, end) | false => data)
  out := (more? | true => capture-outer(next, ids, a + 1u64, end) | false => data).
buffer := sequence/with-capacity(0u64, capacity<u64>)
capture-outer(buffer, ids<[u64]>, 0u64, end<u64>)
"#;

#[test]
fn numeric_tree_nested_appends_preserve_exact_capacity_and_growth_fallback() {
    for artifact in artifacts(APPEND_TREE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region
                        .numeric_tree
                        .as_ref()
                        .is_some_and(|tree| tree.nodes.len() >= 2)
                }),
            "exercise nested append lowering, not just the ordinary path"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (end, capacity) in [
                (0, 0),
                (2, 0),
                (3, 1),
                (5, 10),
                (8, 56),
                (8, 55),
                (8, 0),
                (8, 57),
            ] {
                let args = inputs(
                    &artifact,
                    &[("end", Data::U64(end)), ("capacity", Data::U64(capacity))],
                    &memory,
                );
                let expected: Vec<_> = (0..end)
                    .flat_map(|a| {
                        (a + 1..end).flat_map(move |b| {
                            (b + 1..end).map(move |c| Data::U64((a * end + b) * end + c))
                        })
                    })
                    .collect();
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap_or_else(|error| panic!("end={end}, capacity={capacity}: {error:?}"));
                assert_eq!(last(&output), Data::Matrix(expected.into()));
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_tree_append_cold_fallback_keeps_readonly_aliases_needed_after_child_return() {
    for artifact in artifacts(APPEND_CAPTURE_TREE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region
                        .numeric_tree
                        .as_ref()
                        .is_some_and(|tree| tree.nodes.len() >= 2)
                })
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (end, capacity) in [(0, 0), (3, 1), (5, 10), (8, 56), (8, 55), (8, 0), (8, 57)] {
                let args = inputs(
                    &artifact,
                    &[
                        ("end", Data::U64(end)),
                        ("capacity", Data::U64(capacity)),
                        ("ids", Data::Matrix((10..18).map(Data::U64).collect())),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let expected: Vec<_> = (0..end)
                    .flat_map(|a| {
                        (a + 1..end).flat_map(move |b| {
                            (b + 1..end)
                                .map(move |c| Data::U64(((a + 10) * end + b + 10) * end + c + 10))
                        })
                    })
                    .collect();
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap_or_else(|error| panic!("end={end}, capacity={capacity}: {error:?}"));
                assert_eq!(last(&output), Data::Matrix(expected.into()));
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

#[test]
fn numeric_tree_cold_alias_publication_keeps_strict_finite_boundaries_and_retry() {
    resources::finite_boundaries(APPEND_CAPTURE_TREE, |artifact, memory| {
        inputs(
            artifact,
            &[
                ("end", Data::U64(3)),
                ("capacity", Data::U64(1)),
                ("ids", Data::Matrix((10..18).map(Data::U64).collect())),
            ],
            memory,
        )
    });
}

#[test]
fn numeric_tree_keeps_an_escaped_old_version_distinct_from_the_current_writer() {
    for artifact in artifacts(OLD_ALIAS) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (rounds, width) in [(0, 4), (3, 0), (1, 4), (3, 2)] {
                let original = vec![7u8, 12, 31, 2];
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "bytes",
                            Data::Matrix(original.iter().copied().map(Data::U8).collect()),
                        ),
                        ("rounds", Data::U64(rounds)),
                        ("width", Data::U64(width)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap();
                let mut changed = original.clone();
                for value in &mut changed[..width as usize] {
                    *value += rounds as u8;
                }
                let old = Data::Matrix(original.into_iter().map(Data::U8).collect());
                assert_eq!(
                    last(&output),
                    Data::Tuple(
                        [
                            Data::Matrix(changed.into_iter().map(Data::U8).collect()),
                            old.clone(),
                            old,
                        ]
                        .into()
                    )
                );
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

#[test]
fn numeric_tree_cold_fallback_keeps_ancestor_mixed_frame_captures_or_declines() {
    let source = APPEND_CAPTURE_TREE.replace(
        "  next := (more? | true => capture-middle(data, ids, a, a + 1u64, end) | false => data)",
        "  saved := (ids, a)\n  next := (more? | true => capture-middle(data, ids, a, a + 1u64, end) | false => data)",
    ).replace(
        "  out := (more? | true => capture-outer(next, ids, a + 1u64, end) | false => data).",
        "  saved-ids := saved.1\n  saved-a := saved.2\n  out := (more? | true => capture-outer(next, saved-ids, saved-a + 1u64, end) | false => data).",
    );
    assert!(source.contains("saved-ids := saved.1"));
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (end, capacity) in [(0, 0), (3, 1), (5, 10), (8, 56), (8, 55), (8, 0), (8, 57)] {
                let args = inputs(
                    &artifact,
                    &[
                        ("end", Data::U64(end)),
                        ("capacity", Data::U64(capacity)),
                        ("ids", Data::Matrix((10..18).map(Data::U64).collect())),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let expected: Vec<_> = (0..end)
                    .flat_map(|a| {
                        (a + 1..end).flat_map(move |b| {
                            (b + 1..end)
                                .map(move |c| Data::U64(((a + 10) * end + b + 10) * end + c + 10))
                        })
                    })
                    .collect();
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap_or_else(|error| {
                    panic!("ancestor mixed frame end={end},capacity={capacity}: {error:?}")
                });
                assert_eq!(last(&output), Data::Matrix(expected.into()));
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

fn append_scalar_frame_source() -> String {
    APPEND_CAPTURE_TREE.replace(
        "+> sequence\n", "+> sequence\n<capture-rank> := <((u64,u64),u64)>\n",
    ).replace(
        "capture-outer(data<[u64]>, ids<[u64]>, a<u64>, end<u64>)",
        "capture-outer(data<[u64]>, ids<[u64]>, a<u64>, end<u64>, rank<capture-rank>)",
    ).replace(
        "  next := (more? | true => capture-middle(data, ids, a, a + 1u64, end) | false => data)",
        "  saved := rank.1\n  next := (more? | true => capture-middle(data, ids, a, a + 1u64, end) | false => data)",
    ).replace(
        "  out := (more? | true => capture-outer(next, ids, a + 1u64, end) | false => data).",
        "  saved-a := saved.1\n  saved-b := saved.2\n  next-a := saved-a + 1u64\n  out := (more? | true => capture-outer(next, ids, next-a, end, ((next-a, saved-b), rank.2)) | false => data).",
    ).replace(
        "capture-outer(buffer, ids<[u64]>, 0u64, end<u64>)",
        "capture-outer(buffer, ids<[u64]>, 0u64, end<u64>, ((0u64, 29u64), 37u64))",
    )
}

#[test]
fn numeric_tree_append_cold_fallback_restores_projected_ancestor_scalar_frame() {
    for artifact in artifacts(&append_scalar_frame_source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (end, capacity) in [(0, 0), (3, 1), (5, 10), (8, 56), (8, 55), (8, 0), (8, 57)] {
                let args = inputs(
                    &artifact,
                    &[
                        ("end", Data::U64(end)),
                        ("capacity", Data::U64(capacity)),
                        ("ids", Data::Matrix((10..18).map(Data::U64).collect())),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let expected: Vec<_> = (0..end)
                    .flat_map(|a| {
                        (a + 1..end).flat_map(move |b| {
                            (b + 1..end)
                                .map(move |c| Data::U64(((a + 10) * end + b + 10) * end + c + 10))
                        })
                    })
                    .collect();
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
                };
                result.unwrap_or_else(|error| {
                    panic!("ancestor scalar frame end={end},capacity={capacity}: {error:?}")
                });
                assert_eq!(last(&output), Data::Matrix(expected.into()));
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

#[test]
fn numeric_tree_scalar_cold_frame_preserves_strict_finite_boundaries_and_retry() {
    resources::finite_boundaries(&append_scalar_frame_source(), |artifact, memory| {
        inputs(
            artifact,
            &[
                ("end", Data::U64(3)),
                ("capacity", Data::U64(1)),
                ("ids", Data::Matrix((10..18).map(Data::U64).collect())),
            ],
            memory,
        )
    });
}
