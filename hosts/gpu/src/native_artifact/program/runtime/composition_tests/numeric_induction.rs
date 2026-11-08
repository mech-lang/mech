//! A parent threads two current roots through nested writers and a sibling.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"+> sequence
<induce-pair> := <([u64],[u64])>
induce-put(data<[u64]>, at<u64>, value<u64>) => (out<[u64]>) :=
  ~next := data
  next[at] = value
  out := next.
induce-hit(counts<[u64]>, ids<[u64]>, at<u64>) => (out<[u64]>) :=
  slot := ids[at + 1u64] + 1u64
  out := induce-put(counts, slot, counts[slot] + 1u64).
induce-inner(counts<[u64]>, ids<[u64]>, at<u64>, entries<u64>) => (out<[u64]>) :=
  more := at < entries
  next := (more? | true => induce-hit(counts, ids, at) | false => counts)
  out := (more? | true => induce-inner(next, ids, at + 1u64, entries) | false => counts).
induce-count(counts<[u64]>, ids<[u64]>, round<u64>, repeats<u64>, entries<u64>) => (out<[u64]>) :=
  more := round < repeats
  next := (more? | true => induce-inner(counts, ids, 0u64, entries) | false => counts)
  out := (more? | true => induce-count(next, ids, round + 1u64, repeats, entries) | false => counts).
induce-clear-step(counts<[u64]>, totals<[u64]>, at<u64>) => (out<induce-pair>) :=
  amount := counts[at + 1u64]
  zeroed := induce-put(counts, at + 1u64, 0u64)
  added := induce-put(totals, at + 1u64, totals[at + 1u64] + amount)
  out := (zeroed, added).
induce-clear(counts<[u64]>, totals<[u64]>, at<u64>, width<u64>) => (out<induce-pair>) :=
  more := at < width
  step := (more? | true => induce-clear-step(counts, totals, at) | false => (counts, totals))
  (next, accumulated) := step
  out := (more? | true => induce-clear(next, accumulated, at + 1u64, width) | false => (next, accumulated)).
induce-parent(counts<[u64]>, totals<[u64]>, ids<[u64]>, round<u64>, rounds<u64>, repeats<u64>, entries<u64>, width<u64>) => (out<induce-pair>) :=
  more := round < rounds
  counted := (more? | true => induce-count(counts, ids, 0u64, repeats, entries) | false => counts)
  step := (more? | true => induce-clear(counted, totals, 0u64, width) | false => (counted, totals))
  (next, accumulated) := step
  out := (more? | true => induce-parent(next, accumulated, ids, round + 1u64, rounds, repeats, entries, width) | false => (next, accumulated)).
result := induce-parent(counts<[u64]>, totals<[u64]>, ids<[u64]>, 0u64, rounds<u64>, repeats<u64>, entries<u64>, width<u64>)
(result.1, result.2, counts, totals, ids)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    counts: &[u64],
    totals: &[u64],
    ids: &[u64],
    rounds: u64,
    repeats: u64,
    entries: u64,
    width: u64,
) -> Vec<Value> {
    let matrix = |xs: &[u64]| Data::Matrix(xs.iter().copied().map(Data::U64).collect());
    inputs(
        artifact,
        &[
            ("counts", matrix(counts)),
            ("totals", matrix(totals)),
            ("ids", matrix(ids)),
            ("rounds", Data::U64(rounds)),
            ("repeats", Data::U64(repeats)),
            ("entries", Data::U64(entries)),
            ("width", Data::U64(width)),
        ],
        memory,
    )
}
fn expected(rounds: usize, repeats: usize, entries: usize, width: usize) -> Data {
    let mut counts = vec![0u64; 4];
    let mut totals = vec![1u64, 2, 3, 4];
    let ids = [1usize, 3, 1];
    for _ in 0..rounds {
        for _ in 0..repeats {
            for &id in &ids[..entries] {
                counts[id] += 1;
            }
        }
        for at in 0..width {
            totals[at] += counts[at];
            counts[at] = 0;
        }
    }
    let matrix = |xs: Vec<u64>| Data::Matrix(xs.into_iter().map(Data::U64).collect());
    Data::Tuple(
        vec![
            matrix(counts),
            matrix(totals),
            matrix(vec![0; 4]),
            matrix(vec![1, 2, 3, 4]),
            matrix(vec![1, 3, 1]),
        ]
        .into(),
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

#[test]
fn numeric_induction_threads_two_roots_through_nested_and_sibling_children() {
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
                    .is_some_and(|tree| tree.nodes.len() >= 4)
            })
            .expect("the parent must contain both nested counting and the sibling two-root writer");
        let tree = region.numeric_tree.as_ref().unwrap();
        let child_boundaries: Vec<_> = tree
            .nodes
            .iter()
            .skip(1)
            .flat_map(|node| [node.tail.initialize(), node.tail.header(), node.tail.exit()])
            .collect();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (rounds, repeats, entries, width) in [
                (0, 2, 3, 4),
                (3, 0, 3, 4),
                (1, 2, 3, 4),
                (3, 2, 3, 4),
                (2, 1, 3, 0),
                (2, 1, 0, 4),
            ] {
                let args = arguments(
                    &artifact,
                    &memory,
                    &[0; 4],
                    &[1, 2, 3, 4],
                    &[1, 3, 1],
                    rounds,
                    repeats,
                    entries,
                    width,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let (result, calls) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                assert_eq!(
                    last(&output),
                    expected(
                        rounds as usize,
                        repeats as usize,
                        entries as usize,
                        width as usize
                    )
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                for site in &child_boundaries {
                    assert!(
                        !calls.admission_callbacks.contains_key(&u64::from(*site)),
                        "{calls:?}"
                    );
                    assert!(
                        calls.helpers.keys().all(|(at, _)| *at != u64::from(*site)),
                        "{calls:?}"
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

#[test]
fn numeric_induction_keeps_exact_errors_atomic_outputs_aliases_and_retry() {
    let mut errors = [None, None];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let reference = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        for jit in [&reference, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (ids, totals, rejected) in [
                    (vec![1, 3, 1], vec![1, 2, 3, 4], None),
                    (vec![1, 9, 1], vec![1, 2, 3, 4], Some((0, "Index"))),
                    (
                        vec![1, 3, 1],
                        vec![1, u64::MAX, 3, 4],
                        Some((1, "Arithmetic")),
                    ),
                    (vec![1, 3, 1], vec![1, 2, 3, 4], None),
                ] {
                    let args = arguments(&artifact, &memory, &[0; 4], &totals, &ids, 2, 1, 3, 4);
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
                        assert_eq!(error, *errors[index].get_or_insert(error.clone()));
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
                        assert_eq!(last(&output), expected(2, 1, 3, 4));
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
fn numeric_induction_two_root_finite_boundaries_preserve_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(
            artifact,
            memory,
            &[0; 4],
            &[1, 2, 3, 4],
            &[1, 3, 1],
            2,
            1,
            3,
            4,
        )
    });
}

const APPEND_FRONTIER: &str = r#"+> sequence
frontier-read(ids<[u64]>, at<u64>, total<u64>) => (out<u64>) :=
  more := at < sequence/length(ids)
  next := (more? | true => total + ids[at + 1u64] | false => total)
  out := (more? | true => frontier-read(ids, at + 1u64, next) | false => next).
frontier-parent(buffer<[u64]>, ids<[u64]>, round<u64>, rounds<u64>) => (out<[u64]>) :=
  more := round < rounds
  sum := (more? | true => frontier-read(ids, 0u64, 0u64) | false => 0u64)
  next := (more?
    | true => ((round < 2u64)?
      | true => sequence/append(buffer, sum)
      | false => sequence/append(buffer, sum + 1u64))
    | false => buffer)
  out := (more? | true => frontier-parent(next, ids, round + 1u64, rounds) | false => next).
buffer := sequence/with-capacity(0u64, capacity<u64>)
result := frontier-parent(buffer, ids<[u64]>, 0u64, rounds<u64>)
(result, ids)
"#;

#[test]
fn numeric_induction_append_frontier_uses_maximum_branch_path_at_exact_capacity() {
    for artifact in artifacts(APPEND_FRONTIER) {
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
                    .is_some_and(|tree| tree.nodes.len() >= 2)
                    && region
                        .numeric
                        .as_ref()
                        .is_some_and(|proof| proof.arrays.iter().any(|a| a.appends))
            })
            .expect("append parent and readonly child must share one native tree");
        let tree = region.numeric_tree.as_ref().unwrap();
        let root = u64::from(region.tail.header());
        let root_exit = u64::from(region.tail.exit());
        let children = tree
            .nodes
            .iter()
            .skip(1)
            .flat_map(|node| [node.tail.initialize(), node.tail.header(), node.tail.exit()])
            .map(u64::from)
            .collect::<Vec<_>>();
        for restored in [false, true] {
            for rounds in [0, 1, 2, 4] {
                let memory = mode.memory_budget();
                let args = inputs(
                    &artifact,
                    &[
                        ("ids", Data::Matrix(vec![Data::U64(3), Data::U64(4)].into())),
                        ("rounds", Data::U64(rounds)),
                        ("capacity", Data::U64(rounds)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut output = vec![None; artifact.outputs().len()];
                let (result, calls) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                let expected = Data::Tuple(
                    vec![
                        Data::Matrix(
                            (0..rounds)
                                .map(|i| Data::U64(if i < 2 { 7 } else { 8 }))
                                .collect(),
                        ),
                        Data::Matrix(vec![Data::U64(3), Data::U64(4)].into()),
                    ]
                    .into(),
                );
                assert_eq!(last(&output), expected);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    calls.closed_tail_exits.iter().any(|exit| {
                        exit[0] == root || (exit[0] == root_exit && exit[3] == 0)
                    }),
                    "{calls:?}"
                );
                for site in &children {
                    assert!(
                        !calls.admission_callbacks.contains_key(site),
                        "rounds={rounds}: {calls:?}"
                    );
                    assert!(
                        calls.helpers.keys().all(|(at, _)| at != site),
                        "rounds={rounds}: {calls:?}"
                    );
                }
                drop(output);
                drop(aliases);
                drop(args);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
