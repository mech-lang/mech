//! Terminal errors after a private append retain their exact ordinary origin.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");

#[test]
fn numeric_bounds_error_snapshot_omits_unconsumed_scalar_temporaries() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let report = object.metadata();
        let rows = report["compiled_numeric_write_loops"].as_array().unwrap();
        let mut checks = 0;
        for row in rows {
            let all = row["register_words"].as_u64().unwrap();
            for pair in row["bounds_error_spill_words"].as_array().unwrap() {
                let saved = pair[1].as_u64().unwrap();
                assert!(saved > 0 && saved < all, "{row}");
                checks += 1;
            }
        }
        assert!(checks > 0, "the witness must contain a native read");
    }
}

fn failing_source(load: bool, load_first: bool) -> String {
    let mut source = SOURCE
        .replace(
            "  permit := allowed && (at != deny-at)\n",
            "  permit := allowed || (at != deny-at)\n  selected-divisor := ((at == deny-at)? | true => 0u64 | false => divisor)\n",
        )
        .replace(
            "0u64, permit, divisor)",
            "0u64, permit, selected-divisor)",
        );
    assert_ne!(source, SOURCE);
    if load {
        source = source.replace(
            "  word := (more? | true => nodes[at + 1u64] | false => 0u64)\n",
            "  read-at := ((divisor == 0u64)? | true => 0u64 | false => at + 1u64)\n  word := (more? | true => nodes[read-at] | false => 0u64)\n",
        );
    }
    if load_first {
        source = source.replace("  allowed! := allowed\n", "");
        source = source.replace(
            "  word := (more? | true => nodes[read-at] | false => 0u64)\n",
            "  word := (more? | true => nodes[read-at] | false => 0u64)\n  allowed! := allowed\n",
        );
    }
    source
}

#[test]
fn numeric_terminal_errors_after_append_match_ordinary_origins_and_recover() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for (load, load_first) in [(false, false), (true, false), (true, true)] {
        let source = failing_source(load, load_first);
        let mut errors = [None, None, None];
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            for (reference, jit) in [(true, &ordinary), (false, &native)] {
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut published = vec![None; artifact.outputs().len()];
                    // The maintained selected reader reports InvalidInput for
                    // index zero. Compare its complete ordinary error/origin.
                    for (steps, allowed, deny_at, divisor, failure) in [
                        (3, true, u64::MAX, 1, None),
                        (
                            3,
                            true,
                            1,
                            1,
                            Some((0, if load { "InvalidInput" } else { "Arithmetic" })),
                        ),
                        (
                            3,
                            false,
                            1,
                            1,
                            Some((
                                1,
                                if load_first {
                                    "InvalidInput"
                                } else {
                                    "Integrity"
                                },
                            )),
                        ),
                        (
                            3,
                            true,
                            u64::MAX,
                            0,
                            Some((2, if load { "InvalidInput" } else { "Arithmetic" })),
                        ),
                        (0, false, 0, 0, None),
                        (3, true, u64::MAX, 1, None),
                    ] {
                        let args = inputs(
                            &artifact,
                            &[
                                ("nodes", Data::Matrix([Data::U64(9)].into())),
                                ("steps", Data::U64(steps)),
                                ("allowed", Data::Bool(allowed)),
                                ("deny-at", Data::U64(deny_at)),
                                ("divisor", Data::U64(divisor)),
                            ],
                            &memory,
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let prior = published.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, counts) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut published,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut published, mode.execution(), &memory)
                        };
                        if let Some((index, name)) = failure {
                            let error = format!("{:?}", result.unwrap_err());
                            assert!(
                                error.contains(name),
                                "load={load} load_first={load_first} reference={reference} restored={restored} deny_at={deny_at}: {error}"
                            );
                            assert_eq!(error, *errors[index].get_or_insert(error.clone()));
                            assert!(published.iter().zip(&prior).all(|(actual, old)| {
                                actual
                                    .as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                            if !reference && divisor != 0 {
                                assert!(
                                    !counts.first_change_backings.is_empty(),
                                    "failure must follow the first append: {counts:?}"
                                );
                            }
                        } else {
                            result.unwrap();
                            let expected = if steps == 0 {
                                vec![9]
                            } else {
                                vec![9, 9, 28, 75]
                            };
                            assert_eq!(
                                published
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                Data::Matrix(expected.into_iter().map(Data::U64).collect())
                            );
                            if !reference {
                                assert_eq!(counts.phases, 1, "{counts:?}");
                            }
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
}
