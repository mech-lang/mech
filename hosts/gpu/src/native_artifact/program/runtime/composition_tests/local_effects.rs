//! Current-visit facts survive checked replacements, never opaque continuations.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-parent-facts.mec");

fn source(write: bool, child: bool) -> String {
    SOURCE
        .replace(
            "fixed-store(nodes, at, bias)",
            if write {
                "fixed-store(nodes, at, bias)"
            } else {
                "nodes"
            },
        )
        .replace(
            "#PhaseCandidates(values, allowed, divisor, 1u64, 0u64)",
            if child {
                "#PhaseCandidates(values, allowed, divisor, 1u64, 0u64)"
            } else {
                "0u64"
            },
        )
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    values: &[u32],
    steps: u64,
    allowed: bool,
    divisor: u64,
    bias: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "values",
                Data::Matrix(values.iter().copied().map(Data::U32).collect()),
            ),
            ("nodes", Data::Matrix(vec![Data::U64(9); 4].into())),
            ("steps", Data::U64(steps)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
            ("bias", Data::U64(bias)),
        ],
        memory,
    )
}

fn owned_read_guards(artifact: &ProgramArtifact) -> (usize, usize) {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    let plan = object.descriptor().plan();
    [
        "compiled_numeric_write_loops",
        "compiled_readonly_numeric_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .fold((0, 0), |(writes, reads), region| {
        let header = region["header"].as_u64().unwrap() as usize;
        let owner = plan.blocks()[header].iteration();
        let local = region["proven_scalar_read_guards"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| plan.blocks()[row[0].as_u64().unwrap() as usize].iteration() == owner)
            .count();
        (
            writes + region["first_change_boundaries"].as_array().unwrap().len(),
            reads + local,
        )
    })
}

#[test]
fn numeric_local_facts_keep_replacement_bounds_and_stop_at_child_continuations() {
    let mode = NativeProgramLimitMode::NoLimits;
    let mut selected = 0;
    for (write, child) in [(false, false), (true, false), (false, true), (true, true)] {
        for artifact in artifacts(&source(write, child)) {
            let (writes, reads) = owned_read_guards(&artifact);
            if writes != 0 {
                selected += 1;
                assert_eq!(
                    reads, 1,
                    "the checked input read before the child needs a current proof"
                );
            } else {
                // The combined shape currently uses the maintained fallback;
                // do not turn a fact optimization into a new funding policy.
                assert!(write && child);
            }
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (values, steps, divisor, bias) in [
                    (vec![3, 5, 7], 3, 1, 1),
                    (vec![2, 4, 6], 2, 2, 5),
                    (vec![3, 5, 7], 4, 1, 0),
                    (vec![], 0, 0, u64::MAX),
                    (vec![3, 5, 7], 3, 1, 1),
                ] {
                    let args = arguments(&artifact, &memory, &values, steps, true, divisor, bias);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let expected = ordered_reduction_exhaustion::resident(&artifact, &args);
                    let result = if restored {
                        observe::run_restored(
                            &jit,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(&jit, &args, &mut published, mode.execution(), &memory).0
                    };
                    result.unwrap();
                    assert_eq!(
                        contents(
                            &published
                                .iter()
                                .map(|v| v.clone().unwrap())
                                .collect::<Vec<_>>()
                        ),
                        expected
                    );
                    let number = if child && steps != 0 {
                        (1..=values.len())
                            .map(|end| {
                                values[..end].iter().map(|v| u64::from(*v)).sum::<u64>() / divisor
                            })
                            .sum::<u64>()
                    } else {
                        0
                    };
                    let expected = (0..4)
                        .map(|k| {
                            Data::U64(if k < steps as usize {
                                number
                                    + k as u64
                                    + values.get(k).map_or(0, |v| u64::from(*v))
                                    + bias
                            } else {
                                9
                            })
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(expected.into())
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
    assert!(
        selected >= 6,
        "both artifact forms must exercise each qualified native shape"
    );
    // A read after an opaque child is outside this proof, even if an unrelated
    // predecessor can reach its join. No child result or old permission leaks.
    let after = source(false, true).replace(
        "  next := (more?",
        "  after<u64> := (more? | true => (readable? | true => values[at + 1u64] | false => 0u32) | false => 0u32)\n  next := (more?",
    ).replace("number + at + word + bias", "number + at + word + after + bias");
    for artifact in artifacts(&after) {
        assert_eq!(
            owned_read_guards(&artifact).1,
            1,
            "a child continuation cannot inherit a prefix fact"
        );
    }
}

#[test]
fn numeric_local_fact_errors_keep_order_publication_aliases_and_recovery() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source(false, true)) {
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let mut expected_errors = std::collections::BTreeMap::new();
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (steps, allowed, divisor, bias, error_kind) in [
                    (3, true, 1, 0, None),
                    (3, false, 0, 0, Some("Integrity")),
                    (3, true, 0, 0, Some("Arithmetic")),
                    (3, true, 1, u64::MAX, Some("Arithmetic")),
                    (0, false, 0, u64::MAX, None),
                    (3, true, 1, 0, None),
                ] {
                    let args = arguments(
                        &artifact,
                        &memory,
                        &[3, 5, 7],
                        steps,
                        allowed,
                        divisor,
                        bias,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, &mut published, mode.execution(), &memory)
                            .0
                    } else {
                        observe::run(jit, &args, &mut published, mode.execution(), &memory).0
                    };
                    if let Some(kind) = error_kind {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(kind), "{error}");
                        let key = (steps, allowed, divisor, bias);
                        if reference {
                            expected_errors.entry(key).or_insert(error.clone());
                        }
                        assert_eq!(error, expected_errors[&key]);
                        read_packs::unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    } else {
                        result.unwrap();
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_local_fact_finite_boundaries_preserve_publication_and_retry() {
    for (write, child) in [(true, false), (false, true)] {
        resources::finite_boundaries(&source(write, child), |artifact, memory| {
            arguments(artifact, memory, &[3, 5, 7], 3, true, 1, 1)
        });
    }
}
