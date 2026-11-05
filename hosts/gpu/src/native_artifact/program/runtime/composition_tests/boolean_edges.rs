//! Real packed-read guards retain current visits, source failures and funding.
use super::read_packs::{arguments, source, unchanged_publication};
use super::*;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarOp as Op,
};

fn guarded_source(true_first: bool) -> String {
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
    for true_first in [false, true] {
        for artifact in artifacts(&guarded_source(true_first)) {
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
            let guarded_folds = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|name| metadata[name].as_array().unwrap())
            .flat_map(|region| region["proven_boolean_successors"].as_array().unwrap())
            .filter(|row| {
                let site = row[0].as_u64().unwrap() as usize;
                let mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                    mech_engine::native_artifact::NativeMatchTerminator::LiteralBranch {
                        left,
                        right,
                        ..
                    },
                ) = facts.plan().blocks()[site].terminator()
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
                            "boolean-edge-case order={true_first} restored={restored} dimensions={dimensions:?} at={at} steps={steps} work={}",
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
                            "boolean-edge-case order={true_first} restored={restored} denial={denial} error={error}"
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
