use super::*;

#[test]
fn numeric_writes_refresh_owners_and_scalars_under_an_ordinary_parent() {
    let inner = WRITE.rsplit_once("\nwalk(").unwrap().0;
    let source = format!(
        "{inner}\nouter(first<[u32]>, second<[u32]>, left<u64>, flip<bool>, steps<u64>) => (out<[u32]>) :=\n  chosen := (flip? | true => second | false => first)\n  updated := walk(chosen, 0u64, steps)\n  out := ((left > 0u64)? | true => outer(updated, first, left - 1u64, flip == false, steps) | false => updated).\nouter(first<[u32]>, second<[u32]>, 2u64, false, steps<u64>)\n"
    );
    selected(&source, true);
    let artifact = artifacts(&source).0.into_iter().next().unwrap();
    let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    assert!(
        metadata["compiled_numeric_write_loops"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["parent"].is_number())
    );
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for second in [vec![10, 20, 30], vec![40, 50], vec![60, 70, 80]] {
            for steps in [0, 1, 2] {
                let args = inputs(
                    &artifact,
                    &[
                        ("first", matrix([Data::U32(1), Data::U32(2), Data::U32(3)])),
                        ("second", matrix(second.iter().copied().map(Data::U32))),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let expected = reference_result(&artifact, &args).unwrap();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                equal(&artifact, &out, &expected);
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
            }
        }
        drop(out);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_distinct_fields_with_actual_aliases_remain_independent() {
    let source = "walk(a<[u32]>, b<[u32]>, left<u64>, total<u64>) => (out<u64>) :=\n  old<u64> := b[1u64]\n  value := a[1u64] + 1u32\n  ~next := a\n  next[1u64] = value\n  out := ((left > 0u64)? | true => walk(next, b, left - 1u64, total + old) | false => total).\nwalk(first<[u32]>, second<[u32]>, 4u64, 0u64)\n";
    selected(source, true);
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut args = inputs(
            &artifact,
            &[
                ("first", matrix([Data::U32(5)])),
                ("second", matrix([Data::U32(5)])),
            ],
            &memory,
        );
        let first = artifact
            .inputs()
            .iter()
            .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "first")
            .unwrap();
        let second = 1 - first;
        assert_eq!(args[first].schema(), args[second].schema());
        args[second] = args[first].clone();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(result(&out), Data::U64(20));
        assert!(args[0].shares_frozen_storage(&args[1]));
        assert_eq!(
            args[0].canonical_data_draft().unwrap(),
            matrix([Data::U32(5)])
        );
        drop((args, out));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_late_assertion_arithmetic_load_failures_leave_publication_and_retry() {
    for order in [0, 1, 2] {
        let load = "  value := xs[at]\n";
        let check = "  enabled! := allowed\n";
        let arithmetic = "  divided := 12u64 / divisor\n";
        let checks = match order {
            0 => format!("{check}{load}{arithmetic}"),
            1 => format!("{load}{check}{arithmetic}"),
            _ => format!("{arithmetic}{check}{load}"),
        };
        let source = format!(
            "walk(xs<[u32]>, left<u64>, at<u64>, next-at<u64>, allowed<bool>, next-allowed<bool>, divisor<u64>, next-divisor<u64>) => (out<[u32]>) :=\n{checks}  ~updated := xs\n  updated[at] = value + 1u32\n  out := ((left > 0u64)? | true => walk(updated, left - 1u64, next-at, next-at, next-allowed, next-allowed, next-divisor, next-divisor) | false => updated).\nwalk(data<[u32]>, 1u64, 1u64, next-at<u64>, true, allowed<bool>, 3u64, divisor<u64>)\n"
        );
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for (allowed, divisor, next_at, word) in [
                (true, 3, 1, 7),
                (false, 0, 1, 7),
                (true, 2, 1, 9),
                (false, 3, 2, 7),
                (true, 0, 1, 7),
                (true, 3, 2, 7),
                (true, 3, 1, u32::MAX - 1),
                (true, 3, 1, 11),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix([Data::U32(word)])),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                        ("next-at", Data::U64(next_at)),
                    ],
                    &memory,
                );
                let before_args = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && next_at == 1 && word < u32::MAX - 1 {
                    native.unwrap();
                    equal(&artifact, &out, &resident.unwrap());
                    assert_eq!(result(&out), matrix([Data::U32(word + 2)]));
                } else {
                    let native = format!("{:?}", native.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion =
                        !allowed && !(order == 1 && next_at == 2) && !(order == 2 && divisor == 0);
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(
                        resident.contains("LexicalIntegrity"),
                        assertion,
                        "{resident}"
                    );
                    if allowed && next_at == 1 {
                        assert!(native.contains("Arithmetic"), "{native}");
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before_args
                );
            }
            drop(out);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn numeric_writes_finite_exact_one_short_failure_then_success() {
    for artifact in artifacts(WRITE).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix((0..64).map(Data::U32))),
                    ("steps", Data::U64(steps)),
                ],
                &memory,
            );
            // Include retirement of an already published result in the exact
            // receipt; an initially empty output has a different entry cost.
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
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(out.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop(out);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_first_copy_uses_original_clone_admission_and_recovers() {
    for artifact in artifacts(WRITE).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("data", matrix((0..8).map(Data::U32))),
                ("steps", Data::U64(8)),
            ],
            &memory,
        );
        let before_args = args
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        let prior = out.clone();
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut denied = mode.execution();
        denied.library.max_cloned_bytes = Some(0);
        let error = format!(
            "{:?}",
            jit.evaluate_into(&args, &mut out, denied, &memory)
                .unwrap_err()
        );
        assert!(
            error.contains("Library") && error.contains("ClonedBytes"),
            "{error}"
        );
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            before
        );
        assert!(out.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        assert_eq!(
            args.iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>(),
            before_args
        );
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        drop((args, out, prior));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
