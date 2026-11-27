//! Total private map updates, identity tails and exact cold-boundary replay.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::numeric_phase::total_maps;

const DIRECT: &str = r#"#Replace(initial<[K]:2,3>, rounds<u64>, selected<u64>) => <[K]:2,3>
  | :Step(data<[K]:2,3>, pass<u64>)
  | :Done(data<[K]:2,3>).
#Replace(initial,rounds,selected) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        column := [data[i,1u64] | i <- 1u64..=2u64]'
        next := [[((j == selected)? | true => column[i] | false => data[i,j]) | i <- 1u64..=2u64]' | j <- 1u64..=3u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Replace(data,rounds<u64>,selected<u64>)
(answer,data)
"#;
fn totals(jit: &JitProgramArtifact) -> Vec<total_maps::Total> {
    let m = jit.descriptor().metadata();
    replacements(jit)
        .into_iter()
        .filter_map(|replacement| total_maps::derive(&m.plan, replacement))
        .collect()
}
#[test]
fn numeric_total_maps_prove_identity_paths_and_decline_cross_cell_or_unbounded_reads() {
    for (read, expected) in [
        ("data[i,j]", 2usize),
        ("data[3u64 - i,j]", 1),
        ("data[i,index<u64>]", 1),
    ] {
        let source = DIRECT.replace('K', "f64").replace("data[i,j]", read);
        for artifact in artifacts(&source) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let total = totals(&jit);
            assert_eq!(total.len(), expected, "{read}");
            assert_eq!(
                total.iter().map(|p| p.identity_sites.len()).sum::<usize>(),
                usize::from(expected == 2),
                "{read}"
            );
            let m = jit.descriptor().metadata();
            for replacement in replacements(&jit) {
                let mut forged = replacement.clone();
                forged.source.count += 1;
                assert!(total_maps::derive(&m.plan, &forged).is_none());
                let mut forged = replacement.clone();
                forged.source.elements += 1;
                assert!(total_maps::derive(&m.plan, &forged).is_none());
                let mut forged = replacement.clone();
                forged.source.extents.swap(0, 1);
                if forged.source.extents != replacement.source.extents {
                    assert!(total_maps::derive(&m.plan, &forged).is_none());
                }
            }
        }
    }
    let source = DIRECT
        .replace('K', "u64")
        .replace("data[i,j]", "data[i,j] + step<u64>");
    for artifact in artifacts(&source) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert_eq!(
            totals(&jit).len(),
            1,
            "unknown checked integer overflow retains scratch"
        );
    }
}
#[test]
fn numeric_total_maps_keep_all_bits_aliases_identity_cells_and_recurrent_updates() {
    for (kind, cells, _) in super::super::typed_numeric_phases::cases() {
        for artifact in artifacts(&DIRECT.replace('K', kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert_eq!(
                totals(&jit).len(),
                if kind == "bool" { 0 } else { 2 },
                "{kind}"
            );
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let memory = mode.memory_budget();
            for restored in [false, true] {
                for rounds in [0u64, 1, 2, 3, 7, 1] {
                    for selected in [0u64, 1, 2, 3, 4, u64::MAX] {
                        let args = super::super::numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                                ("rounds", Data::U64(rounds), &[]),
                                ("selected", Data::U64(selected), &[]),
                            ],
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let mut fast = vec![None; artifact.outputs().len()];
                        let mut cold = fast.clone();
                        let (result, counts) = if restored {
                            observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                        } else {
                            observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                        };
                        result.unwrap();
                        observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory)
                            .0
                            .unwrap();
                        assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                            "{kind} rounds={rounds} selected={selected}"
                        );
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        if rounds > 0 {
                            for map in replacements(&jit) {
                                assert_eq!(
                                    counts
                                        .helpers
                                        .get(&(
                                            map.source.region.header as u64,
                                            GlobalHelper::NumericMapPrepare.raw()
                                        ))
                                        .copied()
                                        .unwrap_or(0),
                                    0
                                );
                            }
                        }
                    }
                }
            }
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
#[test]
fn numeric_total_maps_preserve_completed_maps_at_cold_errors_finite_denial_and_retry() {
    let source = DIRECT.replace('K', "f64").replace("      } -> :Step(next,pass + 1u64)", "        picked := next[index<u64>,1u64]\n        ~changed := next\n        changed[1u64,1u64] = picked\n      } -> :Step(changed,pass + 1u64)");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert_eq!(totals(&jit).len(), 2);
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        for restored in [false, true] {
            for index in [1u64, 0, 2, 3, u64::MAX, 1] {
                let args = super::super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                (1..=6)
                                    .map(|n| {
                                        Data::F64(mech_core::snapshot::F64Bits::from_f64(n as f64))
                                    })
                                    .collect(),
                            ),
                            &[2, 3],
                        ),
                        ("rounds", Data::U64(3), &[]),
                        ("selected", Data::U64(2), &[]),
                        ("index", Data::U64(index), &[]),
                    ],
                );
                let before = contents(&args);
                let aliases = args.clone();
                let retained = memory.used_bytes();
                for cap in [None, Some(0u64), Some(1), Some(4095), Some(1_000_000_000)] {
                    let mut policy = mode.execution();
                    policy.library.max_compute_work = cap;
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, _) = observe::run(&ordinary, &args, &mut cold, policy, &memory);
                    let (b, _) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, policy, &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, policy, &memory)
                    };
                    match (a, b) {
                        (Ok(_), Ok(_)) => assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                        ),
                        (Err(a), Err(b)) => {
                            if cap.is_none() {
                                assert_eq!(format!("{a:?}"), format!("{b:?}"));
                            }
                            assert!(
                                fast.iter().all(Option::is_none)
                                    && cold.iter().all(Option::is_none)
                            );
                        }
                        (a, b) => panic!("index={index} cap={cap:?}: {a:?} / {b:?}"),
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    drop((fast, cold));
                    assert_eq!(memory.used_bytes(), retained);
                }
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[test]
fn numeric_total_maps_prove_the_training_rectangles_without_changing_the_ports() {
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/polybench/gramschmidt.mec"
    )) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let total = totals(&jit);
        assert_eq!(
            total.len(),
            3,
            "only the three complete total pointwise state replacements"
        );
        assert!(total.iter().all(|p| !p.identity_sites.is_empty()));
    }
}
