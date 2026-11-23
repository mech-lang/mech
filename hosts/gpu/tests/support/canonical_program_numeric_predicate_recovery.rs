use super::*;

#[test]
fn predicate_versions_late_ordered_failures_preserve_aliases_publication_and_retry() {
    for order in [0, 1, 2] {
        let load = "  old := xs[i + 1u64]\n";
        let check = "  enabled! := allowed\n";
        let arithmetic = "  quotient := 12u64 / divisor\n";
        let checks = match order {
            0 => format!("{check}{load}{arithmetic}"),
            1 => format!("{load}{check}{arithmetic}"),
            _ => format!("{arithmetic}{check}{load}"),
        };
        let put = PREFIX.split("step(xs").next().unwrap();
        let source = format!(
            "{put}\nstep(xs<[u64]>, used<u64>, i<u64>, allowed<bool>, divisor<u64>) => (out<([u64],u64)>) :=\n{checks}  next := put(xs, i + 1u64, used)\n  out := (next, used + old + quotient * 0u64).\nscan(xs<[u64]>, n<u64>, used<u64>, i<u64>, allowed<bool>, next-allowed<bool>, divisor<u64>, next-divisor<u64>) => (out<([u64],u64)>) :=\n  more := i < n\n  next := (more? | true => step(xs, used, i, allowed, divisor) | false => (xs, used))\n  (ys, total) := next\n  out := (more? | true => scan(ys, n, total, i + 1u64, next-allowed, next-allowed, next-divisor, next-divisor) | false => (xs, used)).\n(result,total) := scan(data<[u64]>, steps<u64>, seed<u64>, 1u64, true, allowed<bool>, 3u64, divisor<u64>)\n(result,total + 0u64)\n"
        );
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for (allowed, divisor, length, seed, steps) in [
                (true, 3, 3, 7, 3),
                (false, 0, 2, 7, 3),
                (true, 3, 3, 9, 3),
                (false, 3, 3, 7, 3),
                (true, 0, 3, 7, 3),
                (true, 3, 2, 7, 3),
                (true, 3, 3, u64::MAX - 5, 3),
                (true, 3, 3, 11, 3),
                // The unselected step must not load, assert, or divide at EOF.
                (false, 0, 0, 13, 1),
                (true, 3, 3, 17, 3),
            ] {
                let words = [2, 3, 5][..length]
                    .iter()
                    .copied()
                    .map(Data::U64)
                    .collect::<Vec<_>>();
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix(words.clone())),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(seed)),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before_args = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let resident = super::super::reference_result(&artifact, &args);
                let succeeds =
                    steps == 1 || allowed && divisor != 0 && length == 3 && seed < u64::MAX - 5;
                if succeeds {
                    native.unwrap();
                    super::super::equal(&artifact, &out, &resident.unwrap());
                    let expected = if steps == 1 {
                        words
                    } else {
                        vec![Data::U64(2), Data::U64(seed), Data::U64(seed + 3)]
                    };
                    assert_eq!(
                        result(&out),
                        Data::Tuple(
                            vec![
                                matrix(expected),
                                Data::U64(seed + if steps == 1 { 0 } else { 8 })
                            ]
                            .into()
                        )
                    );
                } else {
                    let error = format!("{:?}", native.unwrap_err());
                    let oracle = resident.unwrap_err();
                    let assertion =
                        !allowed && !(order == 1 && length == 2) && !(order == 2 && divisor == 0);
                    let arithmetic = !assertion
                        && (order == 2 && divisor == 0
                            || allowed && length == 3 && (divisor == 0 || seed == u64::MAX - 5));
                    assert_eq!(error.contains("Integrity"), assertion, "{error}");
                    assert_eq!(oracle.contains("LexicalIntegrity"), assertion, "{oracle}");
                    assert_eq!(error.contains("Arithmetic"), arithmetic, "{error}");
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
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
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
fn predicate_versions_finite_work_copy_denial_and_retry_use_the_same_owner_path() {
    for (_, source) in sources() {
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix((0..19).map(Data::U64).collect())),
                    ("steps", Data::U64(19)),
                    ("seed", Data::U64(5)),
                ],
                &memory,
            );
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let report = jit
                .evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let mut exact = mode.execution();
            exact.execution.max_work = report.work;
            jit.evaluate_into(&args, &mut out, exact, &memory).unwrap();
            let mut short = exact;
            short.execution.max_work -= 1;
            let mut no_copy = mode.execution();
            no_copy.library.max_cloned_bytes = Some(0);
            for limits in [short, no_copy] {
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                assert!(jit.evaluate_into(&args, &mut out, limits, &memory).is_err());
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
                super::super::equal(
                    &artifact,
                    &out,
                    &super::super::reference_result(&artifact, &args).unwrap(),
                );
            }
            drop((out, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
