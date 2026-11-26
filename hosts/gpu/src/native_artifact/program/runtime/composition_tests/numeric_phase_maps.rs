//! Checked whole-matrix maps inside existing native numeric FSM phases.
use super::*;

#[test]
fn numeric_phase_map_source_proof_keeps_exact_shapes_and_cold_range_boundaries() {
    use crate::native_artifact::program::prepare::numeric_phase::matrix_maps;
    for (range, transpose, expected) in [
        ("1u64..=4u64", false, Some([1, 4])),
        ("1u64..=4u64", true, Some([4, 1])),
        ("first<u64>..=4u64", true, None),
        ("1u64..=8193u64", false, None),
    ] {
        let suffix = if transpose { "'" } else { "" };
        let source = format!(
            "data := input<[f64]:4,1>\nanswer := [data[k] | k <- {range}]{suffix}\nanswer\n"
        );
        let mut previous = None;
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &catalog,
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let m = jit.descriptor().metadata();
            let maps = matrix_maps::inventory(&m.plan).unwrap();
            assert_eq!(
                maps.len(),
                usize::from(expected.is_some()),
                "{range} {suffix}"
            );
            if let Some(extents) = expected {
                let map = &maps[0];
                assert_eq!(map.count, 4);
                assert_eq!(map.extents, extents);
                let collection = &m.plan.collections()[map.region.collection as usize];
                let sites = map
                    .source_sites()
                    .collect::<std::collections::BTreeSet<_>>();
                assert!(sites.contains(&map.entry));
                assert!(sites.contains(&map.region.header));
                assert!(sites.contains(&collection.finish()));
                assert!(!sites.contains(&map.after));
                assert!(map.prefix.contains(&map.entry));
                if let Some(t) = &map.region.transpose {
                    assert!(sites.contains(&t.transpose));
                    assert_eq!(map.output, t.output);
                } else {
                    assert_eq!(map.output, map.region.output);
                }
                let scalars = map.scalar_values().collect::<Vec<_>>();
                assert!(scalars.contains(&map.first) && scalars.contains(&map.last));
            }
            if let Some(previous) = &previous {
                assert_eq!(previous, &maps);
            }
            previous = Some(maps);
        }
    }
}

#[test]
fn numeric_phase_map_source_proof_binds_training_map_to_its_exact_state_field() {
    use crate::native_artifact::program::prepare::numeric_phase::matrix_maps;
    let source = include_str!("../../../../../tests/fixtures/polybench/durbin.mec");
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let m = jit.descriptor().metadata();
        let maps = matrix_maps::inventory(&m.plan).unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].count, 40);
        assert_eq!(maps[0].extents, [40, 1]);
        let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = &m.plan.iterations()[0]
        else {
            panic!("training state machine");
        };
        let (frames, _) = m
            .buffer_frames
            .numeric_prefix_projection(&m.plan, tail.carrier())
            .unwrap();
        let roots =
            matrix_maps::destinations(&m.plan.facts(), tail, &frames, &m.calls, &maps).unwrap();
        assert_eq!(roots.get(&maps[0].output), Some(&0));
    }
}

#[test]
fn numeric_phase_map_source_proof_rejects_state_swaps_and_conflicting_destinations() {
    use crate::native_artifact::program::prepare::numeric_phase::matrix_maps;
    const SOURCE: &str = r#"#Swap(left<[f64]:4,1>, right<[f64]:4,1>, rounds<u64>) => <[f64]:4,1>
  | :Step(a<[f64]:4,1>, b<[f64]:4,1>, pass<u64>)
  | :Done(a<[f64]:4,1>, b<[f64]:4,1>).
#Swap(left, right, rounds) -> :Step(left, right, 0u64)
  :Step(a, b, pass)
    | pass < rounds -> {
        next := [a[5u64 - i] | i <- 1u64..=4u64]'
      } -> NEXT_STATE
    | * -> :Done(a,b)
  :Done(a,b) => a.
answer := #Swap(left<[f64]:4,1>, right<[f64]:4,1>, rounds<u64>)
answer
"#;
    for (next, valid) in [
        (":Step(next,b,pass + 1u64)", true),
        (":Step(b,next,pass + 1u64)", false),
        (":Step(next,next,pass + 1u64)", false),
    ] {
        for artifact in artifacts(&SOURCE.replace("NEXT_STATE", next)) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &catalog,
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let m = jit.descriptor().metadata();
            let maps = matrix_maps::inventory(&m.plan).unwrap();
            assert_eq!(maps.len(), 1);
            let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                &m.plan.iterations()[0]
            else {
                panic!("state machine");
            };
            let (frames, _) = m
                .buffer_frames
                .numeric_prefix_projection(&m.plan, tail.carrier())
                .unwrap();
            let roots = matrix_maps::destinations(&m.plan.facts(), tail, &frames, &m.calls, &maps);
            assert_eq!(roots.is_ok(), valid, "{next}: {roots:?}");
            if let Ok(roots) = roots {
                assert_eq!(roots.get(&maps[0].output), Some(&0));
            }
        }
    }
}

#[test]
fn numeric_phase_map_source_proof_funds_original_factories_and_rejects_missing_selection() {
    use crate::native_artifact::program::prepare::numeric_phase::matrix_maps;
    let source = include_str!("../../../../../tests/fixtures/polybench/durbin.mec");
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let m = jit.descriptor().metadata();
        let maps = matrix_maps::inventory(&m.plan).unwrap();
        let map = &maps[0];
        let bound = map.budget(&m.plan, &m.calls, &m.visits).unwrap();
        assert_eq!(bound.schema_bytes, m.observation_schema_bytes);
        assert_eq!(bound.elements, map.count);
        assert!(
            bound.compute
                > map
                    .source_sites()
                    .map(|s| m.visits[s as usize].work)
                    .sum::<u64>()
        );
        assert!(bound.temporary >= map.count * u64::from(map.region.output_width));
        for site in [
            map.region.reads[0].site,
            map.region.transpose.as_ref().unwrap().transpose,
        ] {
            assert!(
                map.budget(&m.plan, &m.calls[..site as usize], &m.visits)
                    .is_none()
            );
        }
    }
}

const REPLACEMENT_SOURCE: &str = r#"#Replace(initial<[f64]:4,1>, seed<f64>, divisor<f64>, bad<u64>, index<u64>, rounds<u64>) => <[f64]:4,1>
  | :Start(data<[f64]:4,1>)
  | :Step(data<[f64]:4,1>, pass<u64>)
  | :Done(data<[f64]:4,1>).
#Replace(initial, seed, divisor, bad, index, rounds) -> :Start(initial)
  :Start(data) -> {
      ~next := data
      next[1u64] = seed
    } -> :Step(next, 0u64)
  :Step(data, pass)
    | pass < rounds -> {
        next := [((i == bad)? | true => data[index] | false => data[5u64 - i] / divisor) | i <- 1u64..=4u64]'
      } -> :Step(next, pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[f64]:4,1>
answer := #Replace(data, seed<f64>, divisor<f64>, bad<u64>, index<u64>, rounds<u64>)
(answer,data)
"#;
fn replacements(
    jit: &JitProgramArtifact,
) -> Vec<&crate::native_artifact::program::prepare::numeric_phase::matrix_maps::Replacement> {
    jit.descriptor()
        .metadata()
        .closed_tails
        .regions
        .iter()
        .flat_map(|r| {
            r.numeric.iter().flat_map(|p| {
                p.actions.iter().filter_map(|(_, a)| match a {
                    crate::native_artifact::program::prepare::numeric_plan::Operation::Map {
                        replacement,
                    } => Some(replacement.as_ref()),
                    _ => None,
                })
            })
        })
        .collect()
}
#[test]
fn numeric_phase_maps_preserve_simultaneous_reads_aliases_lazy_errors_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    for artifact in artifacts(REPLACEMENT_SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        if replacements(&jit).is_empty() {
            let m = jit.descriptor().metadata();
            for control in m.plan.iterations() {
                if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
                    eprintln!(
                        "map phase reason {:?}",
                        crate::native_artifact::program::prepare::numeric_prefix::derive(
                            &m.plan.facts(),
                            tail,
                            &m.buffer_frames,
                            &m.scalar_aggregates,
                            &m.calls,
                            &m.visits,
                        )
                        .err()
                    );
                }
            }
        }
        assert_eq!(
            replacements(&jit).len(),
            1,
            "complete map belongs to one native phase"
        );
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(
            object.metadata()["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| !r["fused_matrix_maps"].as_array().unwrap().is_empty())
        );
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
            for (seed, divisor, bad, index, rounds, expect_error) in [
                (9.0, 1.0, 0, 0, 1, false),
                (9.0, 1.0, 0, 0, 3, false),
                (1.0, 1.0, 0, 0, 2, false), // Equal first write: borrowed bank takes source.
                (9.0, 1.0, 3, 0, 2, true),  // Scratch has a prefix; root must remain old.
                (9.0, 1.0, 3, 5, 1, true),
                (9.0, 0.0, 3, 0, 0, false), // Inactive invalid access and arithmetic.
                (9.0, 2.0, 3, 2, 2, false),
                (9.0, 2.0, 0, 0, 4, false), // Retry after failures on the same artifact.
            ] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                [1.0, 2.0, -0.0, 4.0]
                                    .map(|x| Data::F64(F64Bits::from_f64(x)))
                                    .to_vec()
                                    .into(),
                            ),
                            &[4, 1],
                        ),
                        ("seed", Data::F64(F64Bits::from_f64(seed)), &[]),
                        ("divisor", Data::F64(F64Bits::from_f64(divisor)), &[]),
                        ("bad", Data::U64(bad), &[]),
                        ("index", Data::U64(index), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let old = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                let (a, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                };
                let (b, _) = observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory);
                assert_eq!(
                    a.is_err(),
                    expect_error,
                    "{seed} {bad} {index} {rounds}: {a:?} {counts:?}"
                );
                assert_eq!(a.is_err(), b.is_err());
                if expect_error {
                    assert_eq!(
                        format!("{:?}", a.as_ref().err()),
                        format!("{:?}", b.as_ref().err()),
                        "original error"
                    );
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        old
                    );
                }
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                    "exact output bits"
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                if !expect_error && seed != 1.0 && rounds > 0 {
                    assert!(counts.phases > 0);
                    for map in replacements(&jit) {
                        assert_eq!(
                            counts
                                .admits
                                .get(&(map.source.entry as u64))
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
                                0,
                                "no per-map library operation: {counts:?}"
                            );
                        }
                    }
                }
            }
        }
        fast.fill(None);
        cold.fill(None);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_maps_preserve_all_typed_words_and_both_vector_orientations() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    const SOURCE: &str = r#"#Flip(initial<[K]:SHAPE>, marker<K>, rounds<u64>) => <[K]:SHAPE>
  | :Start(data<[K]:SHAPE>)
  | :Step(data<[K]:SHAPE>, pass<u64>)
  | :Done(data<[K]:SHAPE>).
#Flip(initial, marker, rounds) -> :Start(initial)
  :Start(data) -> {
      ~next := data
      next[1u64] = marker
    } -> :Step(next,0u64)
  :Step(data,pass)
    | pass < rounds -> { next := [data[7u64 - i] | i <- 1u64..=6u64]TRANSPOSE } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:SHAPE>
answer := #Flip(data,marker<K>,rounds<u64>)
(answer,data)
"#;
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for (shape, transpose, extents) in [("1,6", "", [1, 6]), ("6,1", "'", [6, 1])] {
            let source = SOURCE
                .replace("SHAPE", shape)
                .replace("TRANSPOSE", transpose)
                .replace('K', kind);
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let mode = NativeProgramLimitMode::NoLimits;
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &catalog,
                    mode.compilation(),
                )
                .unwrap();
                assert_eq!(replacements(&jit).len(), 1, "{kind} {shape}");
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
                    for rounds in [0, 1, 2, 3] {
                        let args = super::numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("input", Data::Matrix(cells.clone().into()), &extents),
                                ("marker", marker.clone(), &[]),
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
                            contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                            "{kind} {shape} rounds {rounds}"
                        );
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        if rounds > 0 {
                            for map in replacements(&jit) {
                                assert_eq!(
                                    counts
                                        .admits
                                        .get(&(map.source.entry as u64))
                                        .copied()
                                        .unwrap_or(0),
                                    0,
                                    "{kind} {shape}"
                                );
                            }
                        }
                    }
                }
                fast.fill(None);
                cold.fill(None);
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn numeric_phase_maps_images_rederive_destinations_views_source_and_all_funding() {
    for artifact in artifacts(REPLACEMENT_SOURCE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert!(!replacements(&jit).is_empty());
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let (region, action) = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find_map(|(r, region)| {
                region["numeric"]["actions"]
                    .as_array()?
                    .iter()
                    .enumerate()
                    .find_map(|(a, action)| action[1]["Map"].is_object().then_some((r, a)))
            })
            .unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(
            restored.typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let base = format!(
            "/descriptor/closed_tails/regions/{region}/numeric/actions/{action}/1/Map/replacement"
        );
        for field in [
            "target",
            "arrays/0",
            "source/entry",
            "source/after",
            "source/first",
            "source/last",
            "source/count",
            "source/output",
            "source/extents/0",
            "source/prefix/0",
            "source/completion/0",
            "source/region/matrices/0/value",
            "source/region/matrices/0/witness",
            "budget/schema_bytes",
            "budget/compute",
            "budget/replay",
            "budget/temporary",
            "budget/nodes",
            "budget/output",
            "budget/elements",
            "budget/cloned",
            "budget/comparison",
            "budget/setup",
            "budget/native_step",
        ] {
            let mut forged = image.clone();
            let path = format!("{base}/{field}");
            let word = forged
                .pointer_mut(&path)
                .unwrap_or_else(|| panic!("missing map proof {path}"));
            *word = (word.as_u64().unwrap() + 1).into();
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&forged).unwrap()
                ),
                "forged {field}"
            );
        }
        let mut forged = image.clone();
        let word = forged
            .pointer_mut(&format!("{base}/source/inclusive"))
            .unwrap();
        *word = (!word.as_bool().unwrap()).into();
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&forged).unwrap()
        ));
    }
}

#[test]
fn numeric_phase_maps_keep_scalar_snapshots_and_decline_observable_older_arrays() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    for scalar in [false, true] {
        let definition = if scalar {
            "saved := data[1u64]"
        } else {
            "saved := data"
        };
        let incoming = if scalar { "saved" } else { "saved[1u64]" };
        let source = REPLACEMENT_SOURCE.replace("        next := [", &format!("        {definition}\n        next := ["))
            .replace("]'\n      } -> :Step(next, pass", &format!("]'\n        ~chosen := next\n        chosen[1u64] = {incoming}\n      }} -> :Step(chosen, pass"));
        assert!(source.contains("chosen[1u64]"));
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            assert_eq!(
                !replacements(&jit).is_empty(),
                scalar,
                "older array must retain its functional version"
            );
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
                for rounds in [1, 3] {
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            (
                                "input",
                                Data::Matrix(
                                    [1.0, 2.0, 3.0, 4.0]
                                        .map(|v| Data::F64(F64Bits::from_f64(v)))
                                        .into(),
                                ),
                                &[4, 1],
                            ),
                            ("seed", Data::F64(F64Bits::from_f64(9.0)), &[]),
                            ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                            ("bad", Data::U64(0), &[]),
                            ("index", Data::U64(0), &[]),
                            ("rounds", Data::U64(rounds), &[]),
                        ],
                    );
                    if restored {
                        observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                            .0
                            .unwrap();
                    } else {
                        observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                            .0
                            .unwrap();
                    }
                    observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory)
                        .0
                        .unwrap();
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                }
            }
            fast.fill(None);
            cold.fill(None);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn numeric_phase_maps_preserve_partial_arithmetic_failure_finite_denial_cleanup_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    let source = REPLACEMENT_SOURCE.replace("data[5u64 - i]", "data[1u64 + i / (3u64 - i)]");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(!replacements(&jit).is_empty());
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix(
                        [1.0, 2.0, 3.0, 4.0]
                            .map(|v| Data::F64(F64Bits::from_f64(v)))
                            .into(),
                    ),
                    &[4, 1],
                ),
                ("seed", Data::F64(F64Bits::from_f64(9.0)), &[]),
                ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                ("bad", Data::U64(0), &[]),
                ("index", Data::U64(0), &[]),
                ("rounds", Data::U64(1), &[]),
            ],
        );
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
                    let mut cold = vec![None; artifact.outputs().len()];
                    let mut fast = cold.clone();
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
                        assert_eq!(a, b, "dimension {dimension} cap {cap}: {counts:?}");
                    }
                    assert!(cold.iter().all(Option::is_none) && fast.iter().all(Option::is_none));
                    drop((cold, fast));
                    assert_eq!(memory.used_bytes(), retained);
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
            assert!(result.is_err() && out.iter().all(Option::is_none));
            drop(out);
            assert_eq!(memory.used_bytes(), retained);
        }
        assert!(arithmetic > 0);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_maps_decline_a_fast_branch_without_any_private_bank_producer() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    let source = REPLACEMENT_SOURCE.replace("      next[1u64] = seed\n", "");
    assert!(!source.contains("next[1u64] = seed"));
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(
            replacements(&jit).is_empty(),
            "unavailable bank must retain ordinary collection execution"
        );
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let mut fast = vec![None; artifact.outputs().len()];
        let mut cold = fast.clone();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix(
                        [1.0, 2.0, 3.0, 4.0]
                            .map(|v| Data::F64(F64Bits::from_f64(v)))
                            .into(),
                    ),
                    &[4, 1],
                ),
                ("seed", Data::F64(F64Bits::from_f64(9.0)), &[]),
                ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                ("bad", Data::U64(0), &[]),
                ("index", Data::U64(0), &[]),
                ("rounds", Data::U64(3), &[]),
            ],
        );
        for restored in [false, true] {
            if restored {
                observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                    .0
                    .unwrap();
            } else {
                observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                    .0
                    .unwrap();
            }
            observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory)
                .0
                .unwrap();
            assert_eq!(
                contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
            );
        }
        fast.fill(None);
        cold.fill(None);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_maps_reify_current_results_for_errors_after_a_completed_map() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    use mech_core::snapshot::F64Bits;
    let source=REPLACEMENT_SOURCE.replace("]'\n      } -> :Step(next, pass",
        "]'\n        picked := next[index]\n        ~chosen := next\n        chosen[1u64] = picked\n      } -> :Step(chosen, pass");
    assert!(source.contains("picked := next[index]"));
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert_eq!(replacements(&jit).len(), 1);
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
            for (index, error) in [(1, false), (0, true), (5, true), (2, false)] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                [1.0, 2.0, 3.0, 4.0]
                                    .map(|v| Data::F64(F64Bits::from_f64(v)))
                                    .into(),
                            ),
                            &[4, 1],
                        ),
                        ("seed", Data::F64(F64Bits::from_f64(9.0)), &[]),
                        ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                        ("bad", Data::U64(0), &[]),
                        ("index", Data::U64(index), &[]),
                        ("rounds", Data::U64(2), &[]),
                    ],
                );
                let before = contents(&args);
                let previous = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                let (a, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                };
                let (b, _) = observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory);
                for map in replacements(&jit) {
                    assert_eq!(
                        counts
                            .admits
                            .get(&(map.source.entry as u64))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "map must actually complete natively before the later source error: {counts:?}"
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
                assert_eq!(a.is_err(), error, "{index} {a:?} {counts:?}");
                assert_eq!(a.is_err(), b.is_err());
                if error {
                    assert_eq!(
                        format!("{:?}", a.as_ref().err()),
                        format!("{:?}", b.as_ref().err()),
                        "current mapped input owns the source failure"
                    );
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        previous
                    );
                }
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                );
                assert_eq!(contents(&args), before);
            }
        }
        fast.fill(None);
        cold.fill(None);
        assert_eq!(memory.used_bytes(), 0);
    }
}
