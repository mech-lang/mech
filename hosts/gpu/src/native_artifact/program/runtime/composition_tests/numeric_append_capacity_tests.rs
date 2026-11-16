use super::*;

const SOURCE: &str = r#"+> sequence
emit-row(values<[u64]>, which<u64>, width<u64>, buffer<[u64]>) => (out<[u64]>) :=
  kind := width + 1u64
  short := kind == 2u64
  value := values[which + 1u64]
  out := (short?
    | true => sequence/append(buffer,value)
    | false => sequence/append(sequence/append(buffer,value),99u64)).
build-rows(values<[u64]>, at<u64>, end<u64>, buffer<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => emit-row(values,at,values[at + 1u64],buffer) | false => buffer)
  out := (more? | true => build-rows(values,at + 1u64,end,next) | false => buffer).
result := build-rows(input<[u64]>,0u64,rounds<u64>,sequence/with-capacity(0u64,capacity<u64>))
(result,input)
"#;
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    values: &[u64],
    rounds: u64,
    capacity: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "input",
                Data::Matrix(values.iter().copied().map(Data::U64).collect()),
            ),
            ("rounds", Data::U64(rounds)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

#[test]
fn refined_append_capacity_keeps_exact_short_rows_native_and_preserves_errors_aliases_and_retry() {
    use crate::native_artifact::program::prepare::readonly_numeric::Operation;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let old = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::RefinedAppendCapacity,
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let sites = native
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .flat_map(|p| p.actions.iter())
            .filter_map(|(site, op)| matches!(op, Operation::Append { .. }).then_some(*site as u64))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!sites.is_empty());
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let count = [
            "compiled_numeric_write_loops",
            "compiled_readonly_numeric_loops",
        ]
        .into_iter()
        .flat_map(|k| object.metadata()[k].as_array().unwrap().clone())
        .map(|r| r["refined_capacity_guards"].as_array().unwrap().len())
        .sum::<usize>();
        assert!(count > 0, "exercise real branch-dependent native guards");
        for restored in [false, true] {
            for capacity in [0, 1, 2, 4, 16] {
                let memory = mode.memory_budget();
                let mut outputs = [
                    vec![None; artifact.outputs().len()],
                    vec![None; artifact.outputs().len()],
                    vec![None; artifact.outputs().len()],
                ];
                for (values, rounds, accepts) in [
                    (&[1, 2, 1][..], 3, true),
                    (&[][..], 0, true),
                    (&[][..], 1, false),
                    (&[1][..], 2, false),
                    (&[u64::MAX][..], 1, false),
                    (&[1, 2, 1][..], 3, true),
                ] {
                    let input = args(&artifact, &memory, values, rounds, capacity);
                    let aliases = input.clone();
                    let before = contents(&input);
                    let mut errors = Vec::new();
                    let mut ordinary_appends = Vec::new();
                    for (jit, output) in [&native, &old, &ordinary].into_iter().zip(&mut outputs) {
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, calls) = if restored {
                            observe::run_restored(jit, &input, output, mode.execution(), &memory)
                        } else {
                            observe::run(jit, &input, output, mode.execution(), &memory)
                        };
                        assert_eq!(result.is_ok(), accepts, "{result:?}");
                        if accepts {
                            let expected = values
                                .iter()
                                .take(rounds as usize)
                                .flat_map(|v| {
                                    if *v == 1 {
                                        vec![Data::U64(*v)]
                                    } else {
                                        vec![Data::U64(*v), Data::U64(99)]
                                    }
                                })
                                .collect();
                            assert_eq!(
                                last(output),
                                Data::Tuple(
                                    vec![
                                        Data::Matrix(expected),
                                        Data::Matrix(
                                            values.iter().copied().map(Data::U64).collect()
                                        )
                                    ]
                                    .into()
                                )
                            );
                            ordinary_appends.push(
                                calls
                                    .admission_callbacks
                                    .iter()
                                    .filter(|(site, _)| sites.contains(site))
                                    .map(|(_, n)| *n)
                                    .sum::<u64>(),
                            );
                        } else {
                            errors.push(format!("{:?}", result.unwrap_err()));
                            assert!(output.iter().zip(prior).all(|(now, old)| match (now, old) {
                                (Some(a), Some(b)) => a.shares_frozen_storage(&b),
                                (None, None) => true,
                                _ => false,
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                    }
                    assert!(errors.windows(2).all(|p| p[0] == p[1]), "{errors:?}");
                    if accepts && rounds == 3 && capacity == 4 {
                        assert_eq!(
                            ordinary_appends[0], 0,
                            "exact short suffix and terminal visit stay native"
                        );
                        assert!(
                            ordinary_appends[1] > 0,
                            "ablation must exercise the old fallback"
                        );
                    }
                    assert_eq!(contents(&input), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn refined_append_capacity_retains_finite_denials_and_atomic_publication() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        args(artifact, memory, &[1, 2, 1], 3, 4)
    });
}

#[test]
fn refined_append_capacity_preserves_finite_tree_fuel_and_retry() {
    super::check_numeric_tree_finite_fuel(Some(OptimizationPass::RefinedAppendCapacity));
}
