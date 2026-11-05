//! Real packed-read guards retain current visits, source failures and funding.
use super::read_packs::{arguments, source, unchanged_publication};
use super::*;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeScalarKind,
    NativeScalarOp as Op,
};
use std::collections::BTreeSet;

fn folded_sites(metadata: &serde_json::Value) -> BTreeSet<usize> {
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|name| metadata[name].as_array().unwrap())
    .filter_map(|region| region["proven_boolean_successors"].as_array())
    .flatten()
    .map(|row| row[0].as_u64().unwrap() as usize)
    .collect()
}

pub(super) fn guarded_source(true_first: bool) -> String {
    let case = if true_first {
        "| true => pack(xs, at) | false => 0u64"
    } else {
        "| false => 0u64 | true => pack(xs, at)"
    };
    source(8, 1)
        .replace("+> bits", "+> bits\n+> sequence")
        .replace("walk(xs<[u8]>", &format!("guarded(xs<[u8]>, at<u64>) => (out<u64>) :=\n  length := sequence/length(xs)\n  out := ((length >= 8u64)? | false => 0u64 | true => ((at <= length - 8u64)? {case})).\nwalk(xs<[u8]>"))
        .replace("true => pack(xs, at) | false => 0u64)\n  out := (more?", "true => guarded(xs, at) | false => 0u64)\n  out := (more?")
}

fn expected(dimensions: [u64; 2], at: u64, steps: u64, nonce: u8) -> u64 {
    let [rows, columns] = dimensions;
    let mut total = u64::from(nonce);
    for visit in 0..steps {
        let position = at + visit;
        if rows * columns < 8 || position > rows * columns - 8 {
            continue;
        }
        let word = core::array::from_fn(|i| {
            let ordinal = position + i as u64;
            let physical = (ordinal % rows) * columns + ordinal / rows;
            (physical as u8).wrapping_mul(37).wrapping_add(nonce)
        });
        total ^= u64::from_le_bytes(word);
    }
    total
}

#[test]
fn complementary_read_guards_keep_current_windows_errors_and_retry() {
    for (true_first, checked_add) in [(false, false), (true, false), (false, true), (true, true)] {
        let source = if checked_add {
            guarded_source(true_first).replace("at <= length - 8u64", "at + 8u64 <= length")
        } else {
            guarded_source(true_first)
        };
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let facts = native.descriptor().metadata().plan.facts();
            let guarded_folds = folded_sites(&metadata)
                .into_iter()
                .filter(|&site| {
                    let Term::Scalar(Branch::LiteralBranch { left, right, .. }) =
                        facts.plan().blocks()[site].terminator()
                    else {
                        return false;
                    };
                    [left, right].into_iter().any(|v| {
                        facts.definition_sites(v.get()).ok().is_some_and(|defs| {
                            defs.iter().any(|d| {
                                d.site().is_some_and(|s| {
                                    matches!(
                                        facts.plan().blocks()[s as usize].action(),
                                        Action::Scalar(Scalar::Primitive { op: Op::U64Le, .. })
                                    )
                                })
                            })
                        })
                    })
                })
                .count();
            assert!(
                guarded_folds > 0,
                "the readable-window case must actually fold: {metadata}"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                let mut successful_work = 0;
                for (dimensions, at, steps, denial) in [
                    ([1, 32], 0, 3, "none"),
                    ([32, 1], 7, 2, "none"),
                    ([4, 8], 2, 3, "none"),
                    ([1, 32], 24, 3, "none"),
                    ([1, 7], 0, 1, "none"),
                    ([0, 1], u64::MAX, 0, "none"),
                    ([1, 32], u64::MAX, 1, "arithmetic"),
                    ([1, 32], 0, 3, "late-work"),
                    ([1, 32], 0, 3, "compute"),
                    ([1, 32], 0, 3, "none"),
                ] {
                    let args = arguments(&artifact, &memory, dimensions, at, steps, 29);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut limits = mode.execution();
                    if denial == "late-work" {
                        limits.execution.max_work = successful_work - 1;
                    }
                    if denial == "compute" {
                        limits.library.max_compute_work = Some(0);
                    }
                    let (result, _) = if restored {
                        observe::run_restored(&native, &args, &mut published, limits, &memory)
                    } else {
                        observe::run(&native, &args, &mut published, limits, &memory)
                    };
                    if denial == "none" {
                        let report = result.unwrap();
                        if dimensions == [1, 32] && at == 0 && steps == 3 {
                            successful_work = report.work;
                        }
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected(dimensions, at, steps, 29))
                        );
                        println!(
                            "boolean-edge-case order={true_first} checked_add={checked_add} restored={restored} dimensions={dimensions:?} at={at} steps={steps} work={}",
                            report.work
                        );
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        if denial == "arithmetic" {
                            assert!(error.contains("Arithmetic"), "{error}");
                        }
                        unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                        println!(
                            "boolean-edge-case order={true_first} checked_add={checked_add} restored={restored} denial={denial} error={error}"
                        );
                    }
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

fn alternate_source(true_first: bool, joined: bool) -> String {
    let ready = if joined {
        "(gate? | true => at <= length - 8u64 | false => at < length - 8u64)"
    } else {
        "gate"
    };
    let case = if true_first {
        "| true => pack(xs, at) | false => 0u64"
    } else {
        "| false => 0u64 | true => pack(xs, at)"
    };
    source(8, 1)
        .replace("+> bits", "+> bits\n+> sequence")
        .replace("walk(xs<[u8]>", &format!("guarded(xs<[u8]>, at<u64>, gate<bool>) => (out<u64>) :=\n  length := sequence/length(xs)\n  ready := {ready}\n  out := ((length >= 8u64)? | false => 0u64 | true => (ready? {case})).\nwalk(xs<[u8]>"))
        .replace("left<u64>, total<u64>)", "left<u64>, total<u64>, gate<bool>)")
        .replace("true => pack(xs, at) | false => 0u64)\n  out := (more?", "true => guarded(xs, at, gate) | false => 0u64)\n  out := (more?")
        .replace("bits/xor(total, word))", "bits/xor(total, word), gate)")
        .replace("steps<u64>, seed<u64>)", "steps<u64>, seed<u64>, gate<bool>)")
}

#[test]
fn input_and_joined_booleans_keep_literal_checks_and_current_arguments() {
    for (true_first, joined) in [(false, false), (true, false), (false, true), (true, true)] {
        for artifact in artifacts(&alternate_source(true_first, joined)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let folds = folded_sites(&metadata);
            assert!(!folds.is_empty(), "positive numeric cases must still fold");
            let regions = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|name| metadata[name].as_array().unwrap())
            .collect::<Vec<_>>();
            let selected = regions
                .iter()
                .flat_map(|region| region["source_sites"].as_array().unwrap())
                .map(|site| site.as_u64().unwrap() as usize)
                .collect::<BTreeSet<_>>();
            let facts = native.descriptor().metadata().plan.facts();
            let unsupported = facts.plan().blocks().iter().enumerate().filter_map(|(site, block)| {
                let Term::Scalar(Branch::LiteralBranch { left, right, .. }) = block.terminator() else { return None; };
                [left, right].into_iter().any(|value| {
                    facts.plan().values()[value.get() as usize].scalar_kind() == Some(NativeScalarKind::Bool)
                        && !facts.plan().constants().iter().any(|constant| constant.value() == value)
                        && !matches!(facts.definition_sites(value.get()).unwrap(), [definition]
                            if definition.site().is_some_and(|producer| matches!(facts.plan().blocks()[producer as usize].action(),
                                Action::Scalar(Scalar::Primitive { op: Op::U64Eq | Op::U64Ne | Op::U64Lt | Op::U64Le | Op::U64Gt | Op::U64Ge, .. }))))
                }).then_some(site)
            }).collect::<Vec<_>>();
            assert!(
                unsupported.len() >= 2,
                "both input/joined literal cases must exist"
            );
            assert!(
                unsupported.iter().all(|site| selected.contains(site)),
                "refusal controls must remain inside generated numeric regions"
            );
            if !folds.is_empty() {
                assert!(
                    unsupported.iter().all(|site| regions.iter().any(|region| {
                        region["source_sites"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|value| value.as_u64() == Some(*site as u64))
                            && region["proven_boolean_successors"]
                                .as_array()
                                .is_some_and(|values| !values.is_empty())
                    })),
                    "negative and positive cases must share selected numeric regions"
                );
            }
            assert!(unsupported.iter().all(|site| !folds.contains(site)));
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (at, steps, gate) in [
                    (0, 2, true),
                    (0, 2, false),
                    (24, 1, false),
                    (24, 1, true),
                    (0, 2, false),
                    (0, 2, true),
                ] {
                    let args = inputs(
                        &artifact,
                        &[
                            (
                                "bytes",
                                Data::Matrix(
                                    (0u8..32)
                                        .map(|i| Data::U8(i.wrapping_mul(37).wrapping_add(29)))
                                        .collect(),
                                ),
                            ),
                            ("at", Data::U64(at)),
                            ("steps", Data::U64(steps)),
                            ("seed", Data::U64(29)),
                            ("gate", Data::Bool(gate)),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (result, _) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&native, &args, &mut published, mode.execution(), &memory)
                    };
                    let report = result.unwrap();
                    let mut wanted = 29;
                    for visit in 0..steps {
                        let position = at + visit;
                        let selected = if joined {
                            if gate { position <= 24 } else { position < 24 }
                        } else {
                            gate
                        };
                        if selected {
                            wanted ^= expected([1, 32], position, 1, 29) ^ 29;
                        }
                    }
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(wanted)
                    );
                    assert_eq!(contents(&aliases), before);
                    println!(
                        "boolean-edge-alternate order={true_first} joined={joined} restored={restored} at={at} steps={steps} gate={gate} work={}",
                        report.work
                    );
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
