//! Scalar result words compose with a numeric leaf's managed carrier.
use super::super::{equal, reference_result};
use super::*;

pub(super) const PAIR: &str = r#"walk(xs<[u32]>, i<u64>, end<u64>, sum<u64>, count<u64>) => (out<(u64,u64)>) :=
  more := i < end
  word<u64> := (more? | true => xs[i + 1u64] | false => 0u32)
  out := (more? | true => walk(xs, i + 1u64, end, sum + word, count + 1u64) | false => (sum + 17u64, count + 5u64)).
(a,b) := walk(data<[u32]>, 0u64, steps<u64>, seed<u64>, 0u64)
(a + 0u64, b + 0u64)
"#;

fn record_source() -> String {
    PAIR.replace("out<(u64,u64)>", "out<{total<u64> count<u64>}>")
        .replace(
            "(sum + 17u64, count + 5u64)",
            "{total: sum + 17u64, count: count + 5u64}",
        )
        .replace("(a,b) := walk", "result := walk")
        .replace(
            "(a + 0u64, b + 0u64)",
            "(result.total + 0u64, result.count + 0u64)",
        )
}

#[test]
fn scalar_results_keep_current_bool_signed_and_exact_float_bits() {
    use mech_core::snapshot::F64Bits;
    let source = r#"walk(xs<[u32]>, left<u64>, flag<bool>, byte<u8>, half<u16>, packed<u32>, total<u64>, signed<i64>, fraction<f64>) => (out<(bool,u8,u16,u32,u64,i64,f64)>) :=
  word<u64> := xs[1u64]
  out := ((left > 0u64)? | true => walk(xs,left - 1u64,flag == false,byte,half,packed,total + word,signed + 1i64,fraction) | false => (flag,byte,half,packed,total + 17u64,signed,fraction)).
(a,b,c,d,e,f,g) := walk(data<[u32]>, steps<u64>, flag<bool>, byte<u8>, half<u16>, packed<u32>, 5u64, signed<i64>, fraction<f64>)
(a == false,b,c,d,e + 0u64,f,g)
"#;
    selected(&source);
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (steps, flag, signed, bits) in [
            (3, true, i64::MIN, 0x8000_0000_0000_0000),
            (2, false, -2, 0x7ff8_0000_0000_0042),
            (1, false, 0, 0x3ff0_0000_0000_0000),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix(vec![Data::U32(1)].into())),
                    ("steps", Data::U64(steps)),
                    ("flag", Data::Bool(flag)),
                    ("byte", Data::U8(239)),
                    ("half", Data::U16(56506)),
                    ("packed", Data::U32(0x9abc_def0)),
                    ("signed", Data::I64(signed)),
                    ("fraction", Data::F64(F64Bits::from_bits(bits))),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Bool(!(flag ^ (steps % 2 == 1))),
                        Data::U8(239),
                        Data::U16(56506),
                        Data::U32(0x9abc_def0),
                        Data::U64(22 + steps),
                        Data::I64(signed + steps as i64),
                        Data::F64(F64Bits::from_bits(bits))
                    ]
                    .into()
                )
            );
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn scalar_results_preserve_failure_order_private_cleanup_and_retry() {
    for arithmetic_first in [false, true] {
        let earlier = if arithmetic_first {
            "  earlier := 12u64 / divisor\n"
        } else {
            ""
        };
        let put = GENERIC.split("walk(xs").next().unwrap();
        let source = format!(
            "{put}\nwalk(xs<[u32]>, i<u64>, left<u64>, sum<u64>, allowed<bool>, divisor<u64>) => (out<(u64,u64)>) :=\n{earlier}  enabled! := ((i == 0u64)? | true => true | false => allowed)\n  word<u64> := xs[1u64]\n  next := put(xs, 1u64, xs[1u64] + 1u32)\n  out := ((left > 0u64)? | true => walk(next, i + 1u64, left - 1u64, sum + word, allowed, divisor) | false => (sum + word, 12u64 / divisor)).\n(a,b) := walk(data<[u32]>, 0u64, 2u64, 0u64, allowed<bool>, divisor<u64>)\n(a + 0u64,b + 0u64)\n"
        );
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let metadata = emit_program_object(&artifact, &catalog).unwrap().metadata();
            assert!(
                !metadata["compiled_numeric_write_loops"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let mut out = vec![None; artifact.outputs().len()];
            for (allowed, divisor, empty) in [
                (true, 3, false),
                (false, 0, false),
                (true, 2, false),
                (true, 0, false),
                (true, 3, true),
                (true, 3, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U32(7)] }.into()),
                        ),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && !empty {
                    native.unwrap();
                    equal(&artifact, &out, &resident.unwrap());
                    assert_eq!(
                        out.last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(vec![Data::U64(24), Data::U64(12 / divisor)].into())
                    );
                } else {
                    let native = format!("{:?}", native.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion = !allowed && !(arithmetic_first && divisor == 0);
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(
                        resident.contains("LexicalIntegrity"),
                        assertion,
                        "{resident}"
                    );
                    assert_eq!(
                        before,
                        (memory.used_bytes(), memory.retained_snapshot_nodes())
                    );
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                assert_eq!(
                    args.iter()
                        .map(|a| a.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    aliases
                        .iter()
                        .map(|a| a.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>()
                );
            }
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn scalar_results_keep_finite_work_and_rederive_image_lanes() {
    use mech_gpu::native_artifact::NativeProgramDescriptor;
    for artifact in artifacts(PAIR).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: same-process image of this exact maintained descriptor;
        // restored metadata is only inspected, never given a different entry.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for path in [
            "/descriptor/closed_tails/regions/0/numeric/result/Aggregate/value",
            "/descriptor/closed_tails/regions/0/numeric/actions",
            "/descriptor/scalar_aggregates/slots",
        ] {
            let mut changed = original.clone();
            let target = changed.pointer_mut(path).unwrap();
            *target = if let Some(value) = target.as_u64() {
                serde_json::json!(value + 1)
            } else {
                serde_json::json!([])
            };
            // SAFETY: deliberately corrupt metadata is confined to rejection.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&changed).unwrap(),
                        &catalog,
                    )
                }
                .is_err(),
                "accepted {path}"
            );
        }
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix((1..=64).map(Data::U32).collect())),
                    ("steps", Data::U64(steps)),
                    ("seed", Data::U64(11)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let report = jit
                .evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let mut exact = mode.execution();
            exact.execution.max_work = report.work;
            jit.evaluate_into(&args, &mut out, exact, &memory).unwrap();
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            exact.execution.max_work -= 1;
            assert!(jit.evaluate_into(&args, &mut out, exact, &memory).is_err());
            assert_eq!(
                before,
                (memory.used_bytes(), memory.retained_snapshot_nodes())
            );
            assert!(out.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn selected(source: &str) {
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let regions = metadata["compiled_readonly_numeric_loops"]
            .as_array()
            .unwrap();
        assert!(
            !regions.is_empty(),
            "private scalar results must select the existing numeric leaf: {}",
            metadata["readonly_numeric_candidates"]
        );
        assert!(
            regions
                .iter()
                .all(|r| r["successful_backedge_helpers"] == 0)
        );
    }
}

#[test]
fn scalar_results_select_tuple_and_named_record_numeric_leaves() {
    // The one-scalar control already selects; only the return representation
    // differs from the pair. Both keep the same managed read-only carrier.
    let scalar = PAIR
        .replace("out<(u64,u64)>", "out<u64>")
        .replace("(sum + 17u64, count + 5u64)", "sum + 17u64")
        .replace("(a,b) :=", "a :=")
        .replace("(a + 0u64, b + 0u64)", "a + 0u64");
    selected(&scalar);
    selected(PAIR);
    selected(&record_source());
}

#[test]
fn scalar_results_preserve_fresh_visits_without_transport_slope() {
    for (name, source) in [("tuple", PAIR.to_owned()), ("record", record_source())] {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            let mut observations = Vec::new();
            for steps in [0, 1, 8, 64, 256, 1] {
                let seed = 29 + steps;
                let args = inputs(
                    &artifact,
                    &[
                        ("data", Data::Matrix((1..=256).map(Data::U32).collect())),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(seed)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let report = run.unwrap();
                assert_eq!(
                    out.last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::U64(seed + steps * (steps + 1) / 2 + 17),
                            Data::U64(steps + 5)
                        ]
                        .into()
                    )
                );
                if steps <= 8 {
                    equal(
                        &artifact,
                        &out,
                        &reference_result(&artifact, &args).unwrap(),
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                observations.push((steps, requests, bytes, report.visits));
            }
            println!(
                "{}",
                serde_json::json!({"witness":"scalar-numeric-results","case":name,"form":form,"observations":observations,"allocator":"System/probe; excluded from primary timing"})
            );
            assert!(
                observations.iter().all(|r| (r.1, r.2, r.3)
                    == (observations[0].1, observations[0].2, observations[0].3)),
                "numeric scalar-result transport remains: {observations:?}"
            );
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
