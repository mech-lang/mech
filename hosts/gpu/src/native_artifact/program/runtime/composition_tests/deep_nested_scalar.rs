//! Deep scalar paths retain finite funding and atomic publication beside arrays.
use super::*;

const DEPTH: usize = 10;

fn source() -> String {
    let mut text = String::from("+> sequence\n<rank-0> := <(u64,u64)>\n");
    for depth in 1..=DEPTH {
        text.push_str(&format!("<rank-{depth}> := <(u64,rank-{})>\n", depth - 1));
    }
    text.push_str(&format!("<state> := <([u8],rank-{DEPTH})>\n"));
    text.push_str("bump-0(value<rank-0>, delta<u64>) => (out<rank-0>) :=\n  out := (value.1 + delta, value.2 + delta).\n");
    for depth in 1..=DEPTH {
        text.push_str(&format!(
            "bump-{depth}(value<rank-{depth}>, delta<u64>) => (out<rank-{depth}>) :=\n  out := (value.1 + delta, bump-{}(value.2, delta)).\n",
            depth - 1
        ));
    }
    text.push_str("put(xs<[u8]>, at<u64>) => (out<[u8]>) :=\n  ~next := xs\n  next[at] = 0u8\n  out := next.\n");
    text.push_str(&format!(
        "lane(xs<[u8]>, at<u64>, rank<rank-{DEPTH}>) => (out<state>) :=\n  delta<u64> := xs[at + 1u64]\n  updated := bump-{DEPTH}(rank, delta)\n  zeroed := put(xs, at + 1u64)\n  out := (zeroed, updated).\n"
    ));
    text.push_str(&format!(
        "walk(xs<[u8]>, at<u64>, end<u64>, rank<rank-{DEPTH}>) => (out<state>) :=\n  more := at < end\n  step := (more? | true => lane(xs, at, rank) | false => (xs, rank))\n  (next, updated) := step\n  out := (more? | true => walk(next, at + 1u64, end, updated) | false => (next, updated)).\n"
    ));
    let mut initial = format!("(seed + {DEPTH}u64, seed + {}u64)", DEPTH + 1);
    for index in (0..DEPTH).rev() {
        initial = format!("(seed + {index}u64, {initial})");
    }
    text.push_str("seed := initial<u64>\n");
    text.push_str(&format!(
        "result := walk(bytes<[u8]>, 0u64, steps<u64>, {initial})\n"
    ));
    let leaves = (0..DEPTH)
        .map(|index| format!("result.2{}.1", ".2".repeat(index)))
        .chain([
            format!("result.2{}.1", ".2".repeat(DEPTH)),
            format!("result.2{}.2", ".2".repeat(DEPTH)),
        ])
        .collect::<Vec<_>>();
    text.push_str(&format!("(result.1, {}, bytes)\n", leaves.join(", ")));
    text
}

fn arguments(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("bytes", Data::Matrix([1, 2, 3].map(Data::U8).into())),
            ("steps", Data::U64(2)),
            ("initial", Data::U64(7)),
        ],
        memory,
    )
}

fn drafts(outputs: &[Option<Value>]) -> Vec<Data> {
    outputs
        .iter()
        .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}

fn expected() -> Data {
    let mut fields = vec![Data::Matrix([0, 0, 3].map(Data::U8).into())];
    fields.extend((0..DEPTH + 2).map(|index| Data::U64(10 + index as u64)));
    fields.push(Data::Matrix([1, 2, 3].map(Data::U8).into()));
    Data::Tuple(fields.into())
}

#[test]
fn deep_scalar_paths_keep_finite_exact_funding_and_retryable_publication() {
    for artifact in artifacts(&source()) {
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
                .filter_map(|region| region.numeric.as_ref())
                .flat_map(|proof| &proof.frames)
                .any(|frame| {
                    frame
                        .scalar_paths
                        .iter()
                        .map(|(path, _)| path.len())
                        .sum::<usize>()
                        > 2 * frame.scalar_nodes()
                }),
            "exercise a selected numeric tree whose path-walk work exceeds twice its unique-node count"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory);
            let aliases = args.clone();
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                .unwrap();
            assert_eq!(drafts(&outputs).last(), Some(&expected()));
            let published = drafts(&outputs);
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for dimension in [
                "control",
                "library",
                "memory",
                "nodes",
                "transport",
                "output-bytes",
            ] {
                let mut trial = |allowance| {
                    let mut policy = mode.execution();
                    let mut budget = memory.clone();
                    match dimension {
                        "control" => policy.execution.max_work = allowance,
                        "library" => policy.library.max_compute_work = Some(allowance),
                        "memory" => budget = memory.with_limits(allowance, u64::MAX),
                        "nodes" => budget = memory.with_limits(u64::MAX, allowance),
                        "transport" => policy.execution.max_transport_bytes = allowance,
                        "output-bytes" => policy.library.max_output_bytes = Some(allowance),
                        _ => unreachable!(),
                    }
                    let prior = outputs.clone();
                    let result = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, policy, &budget).0
                    } else {
                        observe::run(&jit, &args, &mut outputs, policy, &budget).0
                    };
                    if result.is_err() {
                        assert!(outputs.iter().zip(&prior).all(|(actual, previous)| {
                            actual
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(previous.as_ref().unwrap())
                        }));
                    }
                    assert_eq!(drafts(&outputs), published);
                    drop(prior);
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    result.is_ok()
                };
                let (mut low, mut high) = (0, 100_000_000_000_000u64);
                assert!(trial(high), "{dimension}");
                while low < high {
                    let middle = low + (high - low) / 2;
                    if trial(middle) {
                        high = middle;
                    } else {
                        low = middle + 1;
                    }
                }
                assert!(trial(low), "{dimension}");
                assert!(low > 0 && !trial(low - 1), "{dimension}");
                assert!(trial(100_000_000_000_000), "{dimension}");
            }
            drop((args, aliases, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
