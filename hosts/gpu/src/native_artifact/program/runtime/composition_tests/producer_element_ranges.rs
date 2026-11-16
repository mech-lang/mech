//! Producer content/length facts are properties of exact source versions, not
//! of an integer width, a function argument or a caller-provided array shape.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"
+> sequence
<built-target> := <([u64],bool)>
put(values<[u32]>, at<u64>, value<u32>) => (out<[u32]>) :=
  ~next := values
  next[at] = value
  out := next.
target(at<u64>, count<u64>, allowed<bool>, values<[u64]>) => (out<built-target>) :=
  more := at < count
  next := (more? | true => sequence/append(values, at) | false => values)
  out := (more?
    | true => (allowed? | true => target(at + 1u64, count, allowed, next) | false => (values, false))
    | false => (values, true)).
fill(at<u64>, count<u64>, values<[u32]>) => (out<[u32]>) :=
  more := at < count
  word<u32> := at
  next := (more? | true => put(values, at + 1u64, word) | false => values)
  out := (more? | true => fill(at + 1u64, count, next) | false => values).
step(ids<[u32]>, masks<[u64]>, at<u64>, total<u64>) => (out<u64>) :=
  first := masks[1u64]
  post<u64> := ids[at + 1u64]
  out := total + first + masks[post + 1u64].
scan(ids<[u32]>, masks<[u64]>, at<u64>, end<u64>, total<u64>) => (out<u64>) :=
  more := at < end
  next := (more? | true => step(ids, masks, at, total) | false => total)
  out := (more? | true => scan(ids, masks, at + 1u64, end, next) | false => total).
count := n<u64>
prepared := target(0u64, count, allowed<bool>, sequence/with-capacity(0u64, count))
(masks, okay) := prepared
initial := ((count == 0u64)? | true => sequence/with-capacity(0u32, 0u64) | false => [0u32 | i <- 1u64..=count])
filled := fill(0u64, count, initial)
ids := filled
out := (okay? | true => scan(ids, masks, 0u64, requested<u64>, seed<u64>) | false => 0u64)
out
"#;

fn replace(source: &str, from: &str, to: &str) -> String {
    assert_eq!(
        source.matches(from).count(),
        1,
        "unique fixture edit: {from}"
    );
    source.replace(from, to)
}
fn outer_visits() -> String {
    let (functions, body) = SOURCE.split_once("count := n<u64>\n").unwrap();
    let body = body
        .strip_suffix("out\n")
        .unwrap()
        .trim_end()
        .replace("requested<u64>", "requested")
        .replace("seed<u64>", "seed")
        .replace("allowed<bool>", "allowed");
    let body = body
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{functions}\nonce(count<u64>, requested<u64>, seed<u64>, allowed<bool>) => (out<u64>) :=\n{body}.\nrepeat(at<u64>, end<u64>, count<u64>, total<u64>, allowed<bool>) => (out<u64>) :=\n  more := at < end\n  next := (more? | true => once(count + at, count, total, allowed) | false => total)\n  out := (more? | true => repeat(at + 1u64, end, count, next, allowed) | false => total).\nrepeat(0u64, 2u64, n<u64>, seed<u64>, allowed<bool>)\n"
    )
}
fn proof_sites(object: &crate::native_artifact::NativeProgramObject) -> Vec<usize> {
    let metadata = object.metadata();
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .flat_map(|region| region["producer_element_read_guards"].as_array().unwrap())
    .map(|row| row[0].as_u64().unwrap() as usize)
    .collect()
}
fn object(source: &str) -> Vec<crate::native_artifact::NativeProgramObject> {
    artifacts(source)
        .iter()
        .map(|artifact| {
            crate::native_artifact::emit_program_object_with_limits(
                artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn producer_element_ranges_prove_bounded_scatter_into_indirect_same_bound_target() {
    for object in object(SOURCE) {
        let sites = proof_sites(&object);
        assert!(
            !sites.is_empty(),
            "must remove the actual source indirect guard using producer facts"
        );
        let metadata = object.metadata();
        let reported = metadata["compiled_readonly_numeric_loops"]
            .as_array()
            .unwrap()
            .iter()
            .chain(metadata["compiled_numeric_write_loops"].as_array().unwrap())
            .flat_map(|r| r["proven_scalar_read_guards"].as_array().unwrap())
            .map(|r| r[0].as_u64().unwrap() as usize)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(sites.iter().all(|s| reported.contains(s)));
    }
}

#[test]
fn producer_element_ranges_decline_unproved_versions_and_partial_successes() {
    let cases = [
        // A successful early return does not establish count-sized geometry.
        replace(
            SOURCE,
            "| false => (values, false))",
            "| false => (values, true))",
        ),
        // A preexisting cell prevents the counted append length equation.
        replace(
            SOURCE,
            "sequence/with-capacity(0u64, count))",
            "sequence/append(sequence/with-capacity(0u64, count), 99u64))",
        ),
        // The zero-filled seed is a content fact; an arbitrary seed is not.
        replace(
            SOURCE,
            "[0u32 | i <- 1u64..=count]",
            "[unbounded<u32> | i <- 1u64..=count]",
        ),
        // Target and producer must share the same symbolic bound.
        replace(
            SOURCE,
            "target(0u64, count, allowed<bool>",
            "target(0u64, count - 1u64, allowed<bool>",
        ),
        // A replacement after the proved producer is a distinct content version.
        replace(
            SOURCE,
            "ids := filled",
            "ids := put(filled, 1u64, unbounded<u32>)",
        ),
        // An arbitrary caller's array cannot inherit the producer's facts.
        replace(SOURCE, "ids := filled", "ids := external<[u32]>"),
        // A merge with an unknown replacement requires every alias version to prove.
        replace(
            SOURCE,
            "ids := filled",
            "ids := (choose<bool>? | true => filled | false => put(filled, 1u64, unbounded<u32>))",
        ),
        // A failed global target selection cannot be bypassed as proof of length.
        replace(
            SOURCE,
            "out := (okay? | true => scan(ids, masks, 0u64, requested<u64>, seed<u64>) | false => 0u64)",
            "out := scan(ids, masks, 0u64, requested<u64>, seed<u64>)",
        ),
        // A result/bound from a previous outer visit is not global authority.
        outer_visits(),
    ];
    for (case, source) in cases.iter().enumerate() {
        for object in object(source) {
            assert!(proof_sites(&object).is_empty(), "negative case {case}");
        }
    }
}

#[test]
fn producer_element_ranges_keep_zero_bounds_errors_arithmetic_and_fresh_owners() {
    let mode = NativeProgramLimitMode::NoLimits;
    let zeros = replace(
        SOURCE,
        "initial := ((count == 0u64)? | true => sequence/with-capacity(0u32, 0u64) | false => [0u32 | i <- 1u64..=count])",
        "initial := [0u32 | i <- 1u64..=1u64]",
    );
    for artifact in [SOURCE.to_owned(), zeros]
        .iter()
        .flat_map(|source| artifacts(source))
    {
        let catalog = mech_stdlib::source_catalog();
        let pair = [
            JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ProducerElementRanges,
            )
            .unwrap(),
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap(),
        ];
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs: [_; 2] = std::array::from_fn(|_| vec![None; artifact.outputs().len()]);
            for (n, requested, seed, allowed) in [
                (8, 8, 0, true),
                (0, 0, 0, true),
                (0, 1, 0, true),
                (8, 9, 0, true),
                (8, 8, u64::MAX, true),
                (8, 8, 0, false),
                (8, 8, 0, true),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("n", Data::U64(n)),
                        ("requested", Data::U64(requested)),
                        ("seed", Data::U64(seed)),
                        ("allowed", Data::Bool(allowed)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut errors = vec![];
                let mut work = vec![];
                for (side, jit) in pair.iter().enumerate() {
                    let prior = outputs[side].clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(
                            jit,
                            &args,
                            &mut outputs[side],
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(jit, &args, &mut outputs[side], mode.execution(), &memory).0
                    };
                    match result {
                        Ok(report) => {
                            errors.push(None);
                            work.push(Some(report.work));
                        }
                        Err(error) => {
                            errors.push(Some(format!("{error:?}")));
                            work.push(None);
                            read_packs::unchanged_publication(&outputs[side], &prior);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                assert_eq!(errors[0], errors[1]);
                assert_eq!(work[0], work[1]);
                if errors[0].is_none() {
                    assert_eq!(
                        contents(
                            &outputs[0]
                                .iter()
                                .filter_map(Clone::clone)
                                .collect::<Vec<_>>()
                        ),
                        contents(
                            &outputs[1]
                                .iter()
                                .filter_map(Clone::clone)
                                .collect::<Vec<_>>()
                        )
                    );
                }
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
fn producer_element_ranges_preserve_original_finite_admission_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        inputs(
            artifact,
            &[
                ("n", Data::U64(4)),
                ("requested", Data::U64(4)),
                ("seed", Data::U64(0)),
                ("allowed", Data::Bool(true)),
            ],
            memory,
        )
    });
}
