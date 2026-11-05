//! The checked subtraction stays lexical; only a current exact guard can omit it.
use super::boolean_edges::guarded_source;
use super::read_packs::{arguments, unchanged_publication};
use super::*;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarOp as Op,
};
use std::collections::BTreeSet;

#[test]
fn guarded_u64_subtractions_keep_short_windows_funding_and_recovery() {
    for (condition, selected_truth, safe) in [
        ("length >= 8u64", true, true),
        ("length > 8u64", true, true),
        ("8u64 <= length", true, true),
        ("length < 8u64", false, true),
        ("length == 8u64", true, true),
        ("length != 8u64", false, true),
        ("length <= 8u64", true, false),
        ("length >= 4u64", true, false),
    ] {
        for order in [false, true] {
            let source = if selected_truth {
                guarded_source(order).replace("length >= 8u64", condition)
            } else {
                guarded_source(order).replace(
                    "out := ((length >= 8u64)? | false => 0u64 | true => ((",
                    &format!("out := (({condition})? | true => 0u64 | false => (("),
                )
            };
            for artifact in artifacts(&source) {
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
                let proven = regions
                    .iter()
                    .flat_map(|region| region["proven_u64_subtractions"].as_array().unwrap())
                    .map(|site| site.as_u64().unwrap() as usize)
                    .collect::<BTreeSet<_>>();
                let plan = &native.descriptor().metadata().plan;
                let actual = plan
                    .blocks()
                    .iter()
                    .enumerate()
                    .filter_map(|(site, block)| {
                        let Action::Scalar(Scalar::Primitive {
                            op: Op::U64Sub,
                            operands,
                            ..
                        }) = block.action()
                        else {
                            return None;
                        };
                        plan.constants()
                            .iter()
                            .any(|constant| {
                                constant.value() == operands[1]
                                    && matches!(
                                        constant.constant().data(),
                                        mech_core::ValueData::U64(8)
                                    )
                            })
                            .then_some(site)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(actual.len(), 1, "exact window subtraction must exist");
                assert!(
                    selected.contains(&actual[0]),
                    "positive and negative witnesses must execute in numeric code"
                );
                assert_eq!(
                    proven.contains(&actual[0]),
                    safe,
                    "{condition}, {selected_truth}"
                );
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut published = vec![None; artifact.outputs().len()];
                    let mut success_work = 0;
                    for (length, at, steps, denial) in [
                        (32, 0, 1, false),
                        (0, 0, 0, false),
                        (0, 0, 1, false),
                        (7, 0, 1, false),
                        (8, 0, 1, false),
                        (9, 0, 1, false),
                        (32, 1, 2, false),
                        (32, 1, 2, true),
                        (32, 1, 2, false),
                    ] {
                        let args = arguments(&artifact, &memory, [1, length], at, steps, 29);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let old = published.clone();
                        let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let selected = match condition {
                            "length >= 8u64" | "8u64 <= length" => length >= 8,
                            "length > 8u64" => length > 8,
                            "length < 8u64" => length < 8,
                            "length == 8u64" => length == 8,
                            "length != 8u64" => length != 8,
                            "length <= 8u64" => length <= 8,
                            "length >= 4u64" => length >= 4,
                            _ => unreachable!(),
                        } == selected_truth;
                        let underflow = steps > 0 && selected && length < 8;
                        let mut limits = mode.execution();
                        if denial {
                            limits.execution.max_work = success_work - 1;
                        }
                        let (result, _) = if restored {
                            observe::run_restored(&native, &args, &mut published, limits, &memory)
                        } else {
                            observe::run(&native, &args, &mut published, limits, &memory)
                        };
                        if underflow || denial {
                            let error = format!("{:?}", result.unwrap_err());
                            assert!(
                                error.contains(if underflow { "Arithmetic" } else { "WorkLimit" }),
                                "{error}"
                            );
                            unchanged_publication(&published, &old);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                            println!(
                                "guarded-sub-case condition={condition} truth={selected_truth} order={order} restored={restored} length={length} at={at} steps={steps} denial={denial} error={error}"
                            );
                        } else {
                            let report = result.unwrap();
                            if length == 32 && at == 1 && steps == 2 {
                                success_work = report.work;
                            }
                            let mut wanted = 29;
                            for visit in 0..steps {
                                if selected && at + visit <= length.saturating_sub(8) {
                                    let bytes = core::array::from_fn(|i| {
                                        ((at + visit + i as u64) as u8)
                                            .wrapping_mul(37)
                                            .wrapping_add(29)
                                    });
                                    wanted ^= u64::from_le_bytes(bytes);
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
                            println!(
                                "guarded-sub-case condition={condition} truth={selected_truth} order={order} restored={restored} length={length} at={at} steps={steps} denial={denial} work={}",
                                report.work
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
}
