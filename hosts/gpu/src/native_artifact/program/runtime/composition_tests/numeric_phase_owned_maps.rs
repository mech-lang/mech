//! Owned temporary/rectangular map results across native phase boundaries.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const RECTANGLE: &str = r#"#Rect(initial<[K]:2,3>, rounds<u64>) => <[K]:2,3>
  | :Step(data<[K]:2,3>, pass<u64>)
  | :Done(data<[K]:2,3>).
#Rect(initial, rounds) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        column := [data[3u64 - i,1u64] | i <- 1u64..=2u64]'
        next := [[((j == 1u64)? | true => column[i] | false => data[3u64 - i,j]) | i <- 1u64..=2u64]' | j <- 1u64..=3u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Rect(data,rounds<u64>)
(answer,data)
"#;

const PIPELINE: &str = r#"+> stats
#Pair(initial<[f64]:3,4>, peer<[f64]:3,4>, rounds<u64>, failpass<u64>, bad<u64>, badj<u64>, index<u64>) => <([f64]:3,4,[f64]:3,4)>
  | :Step(a<[f64]:3,4>, b<[f64]:3,4>, pass<u64>)
  | :Done(a<[f64]:3,4>, b<[f64]:3,4>).
#Pair(initial, peer, rounds, failpass, bad, badj, index) -> :Step(initial,peer,0u64)
  :Step(a,b,pass)
    | pass < rounds -> {
        guard := 10u64 / ((pass == failpass)? | true => 0u64 | false => 1u64)
        column := [a[i,guard - 9u64] | i <- 1u64..=3u64]'
        projection := [stats/sum/column([column[((j == badj)? | true => index | false => i)] * a[i,j] | i <- 1u64..=3u64])[1u64] | j <- 1u64..=4u64]
        next-a := [[a[1u64 / ((i == bad)? | true => 0u64 | false => 1u64),j] + column[i] * projection[j] | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
        picked := next-a[index,1u64]
        ~changed := b
        changed[1u64,1u64] = picked
        next-b := [[changed[i,j] - projection[j] | i <- 1u64..=3u64]' | j <- 1u64..=4u64]
      } -> :Step(next-a,next-b,pass + 1u64)
    | * -> :Done(a,b)
  :Done(a,b) => (a,b).
data := input<[f64]:3,4>
peer-data := peer<[f64]:3,4>
answer := #Pair(data,peer-data,rounds<u64>,failpass<u64>,bad<u64>,badj<u64>,index<u64>)
(answer,data,peer-data)
"#;

fn diagnose(jit: &JitProgramArtifact) {
    let m = jit.descriptor().metadata();
    for (id, control) in m.plan.iterations().iter().enumerate() {
        if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
            eprintln!(
                "owned phase {id}: {:?}",
                crate::native_artifact::program::prepare::numeric_prefix::derive(
                    &m.plan.facts(),
                    tail,
                    &m.buffer_frames,
                    &m.scalar_aggregates,
                    &m.calls,
                    &m.visits
                )
                .err()
            );
        }
    }
}
fn owned_bank_count(jit: &JitProgramArtifact) -> usize {
    jit.descriptor()
        .metadata()
        .closed_tails
        .regions
        .iter()
        .filter_map(|r| r.numeric.as_ref())
        .map(|p| p.map_banks.len())
        .sum()
}
fn no_map_callbacks(jit: &JitProgramArtifact, counts: &observe::Counts) {
    for map in replacements(jit) {
        assert_eq!(
            counts
                .admits
                .get(&(map.source.entry as u64))
                .copied()
                .unwrap_or(0),
            0,
            "the complete map must run in the parent phase: {counts:?}"
        );
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
        for read in &map.source.region.reads {
            assert_eq!(
                counts
                    .helpers
                    .get(&(read.site as u64, GlobalHelper::Library.raw()))
                    .copied()
                    .unwrap_or(0),
                0
            );
        }
    }
}

#[test]
fn numeric_phase_owned_rectangles_keep_all_scalar_bits_local_lifetimes_and_input_aliases() {
    for (kind, cells, _) in super::super::typed_numeric_phases::cases() {
        for artifact in artifacts(&RECTANGLE.replace('K', kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            if replacements(&jit).len() != 2 {
                diagnose(&jit);
            }
            assert_eq!(replacements(&jit).len(), 2, "{kind}");
            assert_eq!(owned_bank_count(&jit), 2, "{kind}");
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            for restored in [false, true] {
                for rounds in [0, 1, 2, 3, 1] {
                    let args = super::super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("rounds", Data::U64(rounds), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (a, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                    };
                    a.unwrap();
                    observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory)
                        .0
                        .unwrap();
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                        "{kind} {rounds}"
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    if rounds > 0 {
                        no_map_callbacks(&jit, &counts);
                    }
                }
            }
            fast.fill(None);
            cold.fill(None);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    memory: &mech_core::ManagedMemoryBudget,
    rounds: u64,
    failpass: u64,
    bad: u64,
    badj: u64,
    index: u64,
) -> Vec<Value> {
    use mech_core::snapshot::F64Bits;
    super::super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            (
                "input",
                Data::Matrix(
                    (1..=12)
                        .map(|i| Data::F64(F64Bits::from_f64(i as f64)))
                        .collect(),
                ),
                &[3, 4],
            ),
            (
                "peer",
                Data::Matrix(
                    (1..=12)
                        .rev()
                        .map(|i| Data::F64(F64Bits::from_f64(i as f64)))
                        .collect(),
                ),
                &[3, 4],
            ),
            ("rounds", Data::U64(rounds), &[]),
            ("failpass", Data::U64(failpass), &[]),
            ("bad", Data::U64(bad), &[]),
            ("badj", Data::U64(badj), &[]),
            ("index", Data::U64(index), &[]),
        ],
    )
}

#[test]
fn numeric_phase_owned_local_maps_nested_folds_and_rectangles_preserve_failure_order_cleanup_and_retry()
 {
    for artifact in artifacts(PIPELINE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        if replacements(&jit).len() != 4 {
            diagnose(&jit);
        }
        assert_eq!(replacements(&jit).len(), 4);
        assert_eq!(
            owned_bank_count(&jit),
            3,
            "the second carried bank retains its scalar first-change factory"
        );
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let memory = mode.memory_budget();
        let mut fast = vec![None; artifact.outputs().len()];
        let mut cold = fast.clone();
        for restored in [false, true] {
            for (rounds, failpass, bad, badj, index, error) in [
                (0, 0, 0, 0, 0, false),
                (1, 999, 0, 0, 1, false),
                (3, 999, 0, 0, 1, false),
                (2, 1, 0, 0, 1, true),
                (2, 999, 2, 0, 1, true),
                (2, 999, 0, 2, 0, true),
                (2, 999, 0, 0, 0, true),
                (2, 999, 0, 0, 1, false),
            ] {
                let args = arguments(&artifact, &memory, rounds, failpass, bad, badj, index);
                let aliases = args.clone();
                let before = contents(&args);
                let previous = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                let (a, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                };
                let (b, _) = observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory);
                assert_eq!(
                    a.is_err(),
                    error,
                    "{rounds} {failpass} {bad} {badj} {index}: {a:?} {counts:?}"
                );
                assert_eq!(a.is_err(), b.is_err());
                if error {
                    assert_eq!(
                        format!("{:?}", a.as_ref().err()),
                        format!("{:?}", b.as_ref().err()),
                        "exact original source failure"
                    );
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        previous
                    );
                } else if rounds > 0 {
                    no_map_callbacks(&jit, &counts);
                }
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
        }
        fast.fill(None);
        cold.fill(None);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_owned_map_images_rederive_banks_locals_rectangles_and_nested_sources() {
    for artifact in artifacts(PIPELINE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert_eq!(owned_bank_count(&jit), 3);
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let (id, numeric) = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find_map(|(i, r)| {
                r["numeric"]["map_banks"]
                    .as_array()
                    .filter(|b| !b.is_empty())
                    .map(|_| (i, &r["numeric"]))
            })
            .unwrap();
        let base = format!("/descriptor/closed_tails/regions/{id}/numeric");
        let mut fields = Vec::new();
        for (i, _) in numeric["map_banks"].as_array().unwrap().iter().enumerate() {
            for f in ["array", "elements", "extents/0", "extents/1"] {
                fields.push(format!("map_banks/{i}/{f}"));
            }
        }
        for (i, array) in numeric["arrays"].as_array().unwrap().iter().enumerate() {
            if array["storage"]["PhaseLocal"].is_object() {
                for f in ["value", "extents/0", "extents/1"] {
                    fields.push(format!("arrays/{i}/storage/PhaseLocal/{f}"));
                }
            }
        }
        for (i, action) in numeric["actions"].as_array().unwrap().iter().enumerate() {
            let r = &action[1]["Map"]["replacement"];
            if !r.is_object() {
                continue;
            }
            fields.push(format!("actions/{i}/1/Map/replacement/source/elements"));
            if r["source"]["region"]["rectangle"].is_object() {
                fields.push(format!(
                    "actions/{i}/1/Map/replacement/source/region/rectangle/bounds/Fixed/rows"
                ));
            }
            if !r["source"]["region"]["nested"]
                .as_array()
                .unwrap()
                .is_empty()
            {
                fields.push(format!(
                    "actions/{i}/1/Map/replacement/source/region/nested/0/matrices/0"
                ));
            }
        }
        for field in fields {
            let mut forged = image.clone();
            let word = forged.pointer_mut(&format!("{base}/{field}")).unwrap();
            *word = (word.as_u64().unwrap() + 1).into();
            assert!(
                super::super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&forged).unwrap()
                ),
                "{field}"
            );
        }
        let mut forged = image.clone();
        *forged.pointer_mut(&format!("{base}/map_banks")).unwrap() = serde_json::json!([]);
        assert!(super::super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&forged).unwrap()
        ));
    }
}

#[test]
fn numeric_phase_owned_maps_preserve_finite_resource_denial_and_release_all_bank_preparations() {
    for artifact in artifacts(PIPELINE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert_eq!(replacements(&jit).len(), 4);
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 2, 999, 2, 0, 1);
        let before = contents(&args);
        let retained = memory.used_bytes();
        let mut arithmetic = 0;
        for restored in [false, true] {
            for dimension in 0..9 {
                for cap in [0, 1, 16, 255, 4095, 65535, 1_000_000, 1_000_000_000_000] {
                    let mut policy = mode.execution();
                    match dimension {
                        0 => policy.library.max_output_elements = Some(cap),
                        1 => policy.library.max_output_bytes = Some(cap),
                        2 => policy.library.max_temporary_bytes = Some(cap),
                        3 => policy.library.max_cloned_bytes = Some(cap),
                        4 => policy.library.max_retained_nodes = Some(cap),
                        5 => policy.library.max_compute_work = Some(cap),
                        6 => policy.library.max_comparison_work = Some(cap),
                        7 => policy.execution.max_work = cap,
                        _ => policy.library.max_scalar_instructions = Some(cap),
                    }
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, _) = observe::run(&ordinary, &args, &mut cold, policy, &memory);
                    let (b, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, policy, &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, policy, &memory)
                    };
                    let a = format!("{:?}", a.unwrap_err());
                    let b = format!("{:?}", b.unwrap_err());
                    if a.contains("Arithmetic") {
                        arithmetic += 1;
                        assert_eq!(a, b, "{dimension} {cap}: {counts:?}");
                    }
                    assert!(fast.iter().all(Option::is_none) && cold.iter().all(Option::is_none));
                    drop((fast, cold));
                    assert_eq!(memory.used_bytes(), retained);
                    assert_eq!(contents(&args), before);
                }
            }
            let mut policy = mode.execution();
            policy.execution.max_native_stack_bytes = 0;
            let mut out = vec![None; artifact.outputs().len()];
            let (result, _) = if restored {
                observe::run_restored(&jit, &args, &mut out, policy, &memory)
            } else {
                observe::run(&jit, &args, &mut out, policy, &memory)
            };
            assert!(result.is_err());
            assert!(out.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), retained);
        }
        assert!(arithmetic > 0);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_owned_maps_keep_scalar_snapshots_and_decline_observable_older_array_versions() {
    const SOURCE: &str = r#"#Snapshot(initial<[f64]:4,1>, rounds<u64>) => <([f64]:4,1,f64)>
  | :Step(data<[f64]:4,1>, total<f64>, pass<u64>)
  | :Done(data<[f64]:4,1>, total<f64>).
#Snapshot(initial,rounds) -> :Step(initial,0.0,0u64)
  :Step(data,total,pass)
    | pass < rounds -> {
        old := data
        before := old[1u64]
        next := [data[5u64 - i] | i <- 1u64..=4u64]'
        contribution := SNAPSHOT
      } -> :Step(next,total + contribution,pass + 1u64)
    | * -> :Done(data,total)
  :Done(data,total) => (data,total).
data := input<[f64]:4,1>
answer := #Snapshot(data,rounds<u64>)
(answer,data)
"#;
    for (capture, native) in [("before", true), ("old[1u64]", false)] {
        for artifact in artifacts(&SOURCE.replace("SNAPSHOT", capture)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert_eq!(owned_bank_count(&jit), usize::from(native), "{capture}");
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            for restored in [false, true] {
                for rounds in [0, 1, 3, 2] {
                    let args = super::super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            (
                                "input",
                                Data::Matrix(
                                    [1.0, 2.0, 3.0, 4.0]
                                        .map(|v| {
                                            Data::F64(mech_core::snapshot::F64Bits::from_f64(v))
                                        })
                                        .into(),
                                ),
                                &[4, 1],
                            ),
                            ("rounds", Data::U64(rounds), &[]),
                        ],
                    );
                    let before = contents(&args);
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
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                    assert_eq!(contents(&args), before);
                    if native && rounds > 0 {
                        no_map_callbacks(&jit, &counts);
                    }
                }
            }
            fast.fill(None);
            cold.fill(None);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn numeric_phase_owned_local_result_survives_until_exact_terminal_publication() {
    const SOURCE: &str = r#"#Terminal(initial<[f64]:6,1>, rounds<u64>) => <([f64]:6,1,[f64]:6,1)>
  | :Step(data<[f64]:6,1>, pass<u64>)
  | :Done(data<[f64]:6,1>).
#Terminal(initial,rounds) -> SEED
  :Step(data,pass)
    | pass < rounds -> :Step(data,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => (data,[data[7u64 - i] | i <- 1u64..=6u64]').
data := input<[f64]:6,1>
answer := #Terminal(data,rounds<u64>)
(answer,data)
"#;
    for seed in [":Step(initial,0u64)", ":Done(initial)"] {
        for artifact in artifacts(&SOURCE.replace("SEED", seed)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            if owned_bank_count(&jit) != 1 {
                diagnose(&jit);
            }
            assert_eq!(owned_bank_count(&jit), 1, "{seed}");
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            for restored in [false, true] {
                for rounds in [0, 1, 3] {
                    let args = super::super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            (
                                "input",
                                Data::Matrix(
                                    [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
                                        .map(|v| {
                                            Data::F64(mech_core::snapshot::F64Bits::from_f64(v))
                                        })
                                        .into(),
                                ),
                                &[6, 1],
                            ),
                            ("rounds", Data::U64(rounds), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
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
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    no_map_callbacks(&jit, &counts);
                }
            }
            fast.fill(None);
            cold.fill(None);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn numeric_phase_owned_maps_decline_physical_bank_admission_cleanly_and_retry() {
    for artifact in artifacts(&RECTANGLE.replace('K', "f64")) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert_eq!(owned_bank_count(&jit), 2);
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = super::super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix(
                        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
                            .map(|v| Data::F64(mech_core::snapshot::F64Bits::from_f64(v)))
                            .into(),
                    ),
                    &[2, 3],
                ),
                ("rounds", Data::U64(3), &[]),
            ],
        );
        let before = contents(&args);
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut expected = vec![None; artifact.outputs().len()];
        observe::run(&ordinary, &args, &mut expected, mode.execution(), &memory)
            .0
            .unwrap();
        let expected = contents(&expected.into_iter().flatten().collect::<Vec<_>>());
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        for restored in [false, true] {
            for dimension in ["bytes", "nodes"] {
                let allowances: &[u64] = if dimension == "bytes" {
                    &[
                        0,
                        4096,
                        16384,
                        65536,
                        262144,
                        1048576,
                        4194304,
                        16777216,
                        64000000,
                        1_000_000_000_000,
                    ]
                } else {
                    &[
                        0,
                        16,
                        64,
                        256,
                        1024,
                        4096,
                        16384,
                        65536,
                        99000,
                        1_000_000_000,
                    ]
                };
                let mut denied = 0;
                let mut fallback = 0;
                let mut native = 0;
                for &allowance in allowances {
                    let bounded = if dimension == "bytes" {
                        memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                    } else {
                        memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                    };
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut output, mode.execution(), &bounded)
                    } else {
                        observe::run(&jit, &args, &mut output, mode.execution(), &bounded)
                    };
                    if result.is_ok() {
                        assert_eq!(
                            contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                            expected
                        );
                        if replacements(&jit).iter().any(|map| {
                            counts
                                .admits
                                .get(&u64::from(map.source.entry))
                                .copied()
                                .unwrap_or(0)
                                > 0
                        }) {
                            fallback += 1;
                        } else {
                            native += 1;
                            no_map_callbacks(&jit, &counts);
                        }
                    } else {
                        denied += 1;
                        assert!(
                            output.iter().all(Option::is_none),
                            "{dimension} {allowance}: {result:?}"
                        );
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_nodes()),
                        baseline,
                        "{dimension} {allowance}"
                    );
                    assert_eq!(contents(&args), before);
                }
                assert!(
                    denied > 0 && native > 0 && fallback > 0,
                    "{dimension}: denied={denied}, native={native}, fallback={fallback}"
                );
            }
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            result.unwrap();
            no_map_callbacks(&jit, &counts);
            assert_eq!(
                contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        drop(args);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
