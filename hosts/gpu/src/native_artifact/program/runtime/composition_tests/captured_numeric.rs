//! Immutable numeric captures in scalar FSMs use current, checked array owners.
//! None of these loops carries an array or updates a captured input.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"+> sequence
checked(allowed<bool>, value<u64>) => (out<u64>) :=
  allowed! := allowed
  out := value.
#Scan(xs<[u8]>, end<u64>, seed<u64>, initial<bool>, allowed<bool>, divisor<u64>) => <u64>
  | :Next(at<u64>, total<u64>, seen<bool>)
  | :Done(total<u64>, seen<bool>).
#Scan(xs, end, seed, initial, allowed, divisor) -> :Next(0u64, seed, initial)
  :Next(at, total, seen)
    | at < end -> {
        admitted := checked(allowed, total)
        size := sequence/length(xs)
        value<u64> := xs[at + 1u64]
        next := admitted + value / divisor + size
      } -> :Next(at + 1u64, next, seen || (value > 0u64))
    | * -> :Done(total, seen)
  :Done(total, seen) => (seen? | true => total | false => total + 1u64).
#Scan(values<[u8]>, steps<u64>, seed<u64>, initial<bool>, allowed<bool>, divisor<u64>)
"#;

fn source(width: u8) -> String {
    SOURCE
        .replace("checked(allowed, total)", "total")
        .replace("u8", &format!("u{width}"))
}

fn matrix(width: u8, cells: &[u64]) -> Data {
    Data::Matrix(
        cells
            .iter()
            .map(|&cell| match width {
                8 => Data::U8(cell.try_into().unwrap()),
                16 => Data::U16(cell.try_into().unwrap()),
                32 => Data::U32(cell.try_into().unwrap()),
                64 => Data::U64(cell),
                _ => unreachable!(),
            })
            .collect(),
    )
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    width: u8,
    cells: &[u64],
    steps: u64,
    seed: u64,
    initial: bool,
    allowed: bool,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", matrix(width, cells)),
            ("steps", Data::U64(steps)),
            ("seed", Data::U64(seed)),
            ("initial", Data::Bool(initial)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn drafts(outputs: &[Option<Value>]) -> Vec<Data> {
    outputs
        .iter()
        .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}

fn expected(cells: &[u64], steps: u64, seed: u64, initial: bool, divisor: u64) -> Data {
    let selected = &cells[..steps as usize];
    Data::U64(
        selected.iter().map(|v| v / divisor).sum::<u64>()
            + seed
            + cells.len() as u64 * steps
            + u64::from(!(initial || selected.iter().any(|v| *v != 0))),
    )
}

fn assert_prior(outputs: &[Option<Value>], prior: &[Option<Value>]) {
    assert_eq!(drafts(outputs), drafts(prior));
    assert!(outputs.iter().zip(prior).all(|(actual, old)| {
        actual
            .as_ref()
            .unwrap()
            .shares_frozen_storage(old.as_ref().unwrap())
    }));
}

fn capture_sites(artifact: &ProgramArtifact, jit: &JitProgramArtifact, width: u8) -> Vec<usize> {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    let emitted = metadata["compiled_captured_numeric_loops"]
        .as_array()
        .expect("captured native loop emission receipt");
    assert!(
        !emitted.is_empty(),
        "the capture proof must produce native code; candidates={}, closed_tails={}, scalar_loops={}",
        metadata["captured_numeric_candidates"],
        serde_json::to_string(&jit.descriptor().metadata().closed_tails).unwrap(),
        metadata["compiled_scalar_loops"],
    );
    for report in emitted {
        assert_eq!(report["successful_backedge_helpers"], 0);
        assert!(!report["typed_loads"].as_array().unwrap().is_empty());
        assert!(!report["length_sites"].as_array().unwrap().is_empty());
    }
    let mut sites = Vec::new();
    for region in &jit.descriptor().metadata().closed_tails.regions {
        if let Some(proof) = &region.captures {
            assert!(!proof.captures.is_empty());
            assert!(
                proof
                    .captures
                    .iter()
                    .all(|capture| capture.width == width / 8)
            );
            sites.extend(proof.actions.iter().map(|(site, _)| *site));
        }
    }
    assert!(
        !sites.is_empty(),
        "the scalar FSM must select its immutable typed array capture"
    );
    sites
}

#[test]
fn captured_unsigned_widths_execute_typed_reads_without_body_helpers() {
    for width in [8, 16, 32, 64] {
        for artifact in artifacts(&source(width)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let sites = capture_sites(&artifact, &jit, width);
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (cells, steps, seed, initial) in [
                    (vec![1u64 << (width - 1), 2, 0], 3, 7, false),
                    (vec![0, 0], 2, 19, true),
                    (vec![5], 1, 11, false),
                    (vec![], 0, 23, false),
                ] {
                    let args = arguments(
                        &artifact, &memory, width, &cells, steps, seed, initial, true, 2,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (result, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut outputs, mode.execution(), &memory)
                    };
                    result.unwrap();
                    assert_eq!(
                        drafts(&outputs).last(),
                        Some(&expected(&cells, steps, seed, initial, 2))
                    );
                    assert!(
                        counts
                            .helpers
                            .keys()
                            .all(|(site, _)| !sites.contains(&(*site as usize))),
                        "successful typed reads/lengths must not dispatch body helpers: {counts:?}"
                    );
                    assert_eq!(contents(&args), before);
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
fn captured_bounds_assertion_and_arithmetic_errors_match_ordinary_and_retry() {
    let mut errors = [None, None, None, None];
    for artifact in artifacts(SOURCE) {
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
        capture_sites(&artifact, &native, 8);
        for jit in [&ordinary, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (cells, steps, seed, initial, allowed, divisor, rejection) in [
                    (vec![3, 0], 2, 7, false, true, 1, None),
                    // A completed loop must not inspect captures or assert its body.
                    (vec![], 0, 29, true, false, 0, None),
                    // Assertion precedes both the absent read and division by zero.
                    (vec![], 1, 0, false, false, 0, Some((0, "Integrity"))),
                    (vec![], 1, 0, false, true, 0, Some((1, "Index"))),
                    (vec![1], 1, 0, false, true, 0, Some((2, "Arithmetic"))),
                    (
                        vec![1],
                        1,
                        u64::MAX,
                        false,
                        true,
                        1,
                        Some((3, "Arithmetic")),
                    ),
                    (vec![9, 1], 2, 17, false, true, 2, None),
                ] {
                    let args = arguments(
                        &artifact, &memory, 8, &cells, steps, seed, initial, allowed, divisor,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, &mut outputs, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, &mut outputs, mode.execution(), &memory).0
                    };
                    if let Some((index, name)) = rejection {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(name), "{error}");
                        assert_eq!(
                            error,
                            *errors[index].get_or_insert(error.clone()),
                            "native retry must retain the ordinary error and source origin"
                        );
                        assert_prior(&outputs, &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        // Avoid evaluating division in the deliberately empty body.
                        let wanted = if steps == 0 {
                            Data::U64(seed + u64::from(!initial))
                        } else {
                            expected(&cells, steps, seed, initial, divisor)
                        };
                        assert_eq!(drafts(&outputs).last(), Some(&wanted));
                    }
                    assert_eq!(contents(&args), before);
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
fn captured_zero_and_maximum_indices_keep_the_original_rejection() {
    for index in ["0u64", "18446744073709551615u64"] {
        let source = SOURCE.replace("xs[at + 1u64]", &format!("xs[{index}]"));
        let mut ordinary_error = None;
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
            capture_sites(&artifact, &native, 8);
            for jit in [&ordinary, &native] {
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut outputs = vec![None; artifact.outputs().len()];
                    for steps in [0, 1, 0] {
                        let args =
                            arguments(&artifact, &memory, 8, &[3, 7], steps, 29, true, true, 1);
                        let before = contents(&args);
                        let prior = outputs.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let result = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut outputs,
                                mode.execution(),
                                &memory,
                            )
                            .0
                        } else {
                            observe::run(jit, &args, &mut outputs, mode.execution(), &memory).0
                        };
                        if steps == 1 {
                            let error = format!("{:?}", result.unwrap_err());
                            assert!(
                                error.contains("Index") || error.contains("InvalidInput"),
                                "{error}"
                            );
                            assert_eq!(error, *ordinary_error.get_or_insert(error.clone()));
                            assert_prior(&outputs, &prior);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            result.unwrap();
                            assert_eq!(drafts(&outputs).last(), Some(&Data::U64(29)));
                        }
                        assert_eq!(contents(&args), before);
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
}

#[test]
fn captured_scalar_result_exact_and_one_short_limits_are_atomic() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        capture_sites(&artifact, &jit, 8);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 8, &[3, 0, 5], 3, 17, false, true, 1);
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                .unwrap();
            let wanted = drafts(&outputs);
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for dimension in [
                "control",
                "library",
                "comparison",
                "temporary-bytes",
                "retained-nodes",
                "output-bytes",
                "output-elements",
                "cloned-bytes",
            ] {
                let mut trial = |allowance| {
                    let mut policy = mode.execution();
                    match dimension {
                        "control" => policy.execution.max_work = allowance,
                        "library" => policy.library.max_compute_work = Some(allowance),
                        "comparison" => policy.library.max_comparison_work = Some(allowance),
                        "temporary-bytes" => policy.library.max_temporary_bytes = Some(allowance),
                        "retained-nodes" => policy.library.max_retained_nodes = Some(allowance),
                        "output-bytes" => policy.library.max_output_bytes = Some(allowance),
                        "output-elements" => policy.library.max_output_elements = Some(allowance),
                        "cloned-bytes" => policy.library.max_cloned_bytes = Some(allowance),
                        _ => unreachable!(),
                    }
                    let prior = outputs.clone();
                    let result = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, policy, &memory).0
                    } else {
                        observe::run(&jit, &args, &mut outputs, policy, &memory).0
                    };
                    if result.is_err() {
                        assert_prior(&outputs, &prior);
                    }
                    assert_eq!(drafts(&outputs), wanted);
                    drop(prior);
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    result.is_ok()
                };
                let (mut low, mut high) = (0, 1_000_000_000_000u64);
                assert!(trial(high));
                while low < high {
                    let middle = low + (high - low) / 2;
                    if trial(middle) {
                        high = middle;
                    } else {
                        low = middle + 1;
                    }
                }
                assert!(trial(low));
                if dimension == "cloned-bytes" {
                    assert_eq!(
                        low, 0,
                        "an immutable capture must not require cloning its owner"
                    );
                } else {
                    assert!(
                        low > 0 && !trial(low - 1),
                        "{dimension} must enforce its exact lower bound"
                    );
                }
                assert!(trial(1_000_000_000_000));
            }
            drop((args, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn captured_child_of_scalar_parent_refreshes_its_ancestor_array() {
    let definitions = SOURCE.split("#Scan(values<[u8]>").next().unwrap();
    let source = format!(
        "{definitions}{}",
        r#"#Repeat(xs<[u8]>, rounds<u64>, seed<u64>) => <u64>
  | :Next(left<u64>, total<u64>)
  | :Done(total<u64>).
#Repeat(xs, rounds, seed) -> :Next(rounds, seed)
  :Next(left, total)
    | left > 0u64 -> {
        result := #Scan(xs, sequence/length(xs), total, false, true, 1u64)
      } -> :Next(left - 1u64, result)
    | * -> :Done(total)
  :Done(total) => total.
#Repeat(values<[u8]>, rounds<u64>, seed<u64>)
"#
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let sites = capture_sites(&artifact, &jit, 8);
        for region in &jit.descriptor().metadata().closed_tails.regions {
            if region.numeric_tree.is_some() {
                let proof = region.numeric.as_ref().unwrap();
                assert!(
                    proof
                        .arrays
                        .iter()
                        .enumerate()
                        .all(|(array, _)| proof.view(array) > 0),
                    "tree entry bank transport must not overwrite its first array view"
                );
            }
        }
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region.captures.is_some()
                        && jit.descriptor().plan().iterations()[region.iteration as usize]
                            .as_ref()
                            .parent()
                            .is_some()
                }),
            "the selected capture must belong to a nested scalar child"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (cells, rounds, seed) in [
                (vec![2, 7, 0], 3, 11),
                (vec![1], 2, 19),
                (vec![], 0, 31),
                (vec![4, 0], 2, 5),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("values", matrix(8, &cells)),
                        ("rounds", Data::U64(rounds)),
                        ("seed", Data::U64(seed)),
                    ],
                    &memory,
                );
                let before = contents(&args);
                let aliases = args.clone();
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                let per_round = cells.iter().sum::<u64>() + (cells.len() as u64).pow(2);
                assert_eq!(
                    drafts(&output).last(),
                    Some(&Data::U64(seed + rounds * per_round))
                );
                assert!(
                    counts
                        .helpers
                        .keys()
                        .all(|(site, _)| !sites.contains(&(*site as usize))),
                    "{counts:?}"
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn changing_array_carrier_is_never_classified_as_an_immutable_capture() {
    let source = r#"+> sequence
put(xs<[u8]>, at<u64>) => (out<[u8]>) :=
  ~next := xs
  next[at] = 0u8
  out := next.
moving(xs<[u8]>, at<u64>, end<u64>, total<u64>) => (out<u64>) :=
  more := at < end
  value<u64> := (more? | true => xs[at + 1u64] | false => 0u8)
  updated := (more? | true => put(xs, at + 1u64) | false => xs)
  out := (more? | true => moving(updated, at + 1u64, end, total + value) | false => total).
moving(values<[u8]>, 0u64, steps<u64>, 0u64)
"#;
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .all(|region| region.captures.is_none())
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (cells, steps) in [(vec![2, 7, 0], 3), (vec![], 0), (vec![9, 1], 2)] {
                let args = inputs(
                    &artifact,
                    &[("values", matrix(8, &cells)), ("steps", Data::U64(steps))],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
                };
                result.unwrap();
                assert_eq!(
                    drafts(&outputs).last(),
                    Some(&Data::U64(cells[..steps as usize].iter().sum()))
                );
                assert_eq!(contents(&args), before);
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

#[test]
fn over_budget_captured_scalar_graph_keeps_the_ordinary_entry_and_calls() {
    let mut source = String::from(
        "+> sequence\n#Heavy(xs<[u8]>, end<u64>, seed<u64>) => <u64>\n  | :Next(at<u64>, total<u64>)\n  | :Done(total<u64>).\n#Heavy(xs, end, seed) -> :Next(0u64, seed)\n  :Next(at, total)\n    | at < end -> {\n        v0<u64> := xs[at + 1u64]\n",
    );
    const ADDS: usize = 260;
    for at in 1..=ADDS {
        source.push_str(&format!("        v{at} := v{} + at\n", at - 1));
    }
    source.push_str(&format!(
        "      }} -> :Next(at + 1u64, total + v{ADDS})\n    | * -> :Done(total)\n  :Done(total) => total.\n#Heavy(values<[u8]>, steps<u64>, seed<u64>)\n"
    ));
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let mut read_sites = Vec::new();
        for region in &jit.descriptor().metadata().closed_tails.regions {
            if let Some(proof) = &region.captures {
                read_sites.extend(proof.actions.iter().map(|(site, _)| *site));
            }
        }
        assert!(
            !read_sites.is_empty(),
            "the capture proof must reach the separate SSA budget gate"
        );
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        assert!(
            object.metadata()["compiled_captured_numeric_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    ("values", matrix(8, &[2, 3])),
                    ("steps", Data::U64(2)),
                    ("seed", Data::U64(7)),
                ],
                &memory,
            );
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut outputs, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(
                drafts(&outputs).last(),
                Some(&Data::U64(7 + 2 + 3 + ADDS as u64))
            );
            assert!(
                counts
                    .helpers
                    .keys()
                    .any(|(site, _)| read_sites.contains(&(*site as usize))),
                "the declined graph must execute its maintained library reads"
            );
            assert!(
                counts
                    .helpers
                    .keys()
                    .all(|(_, op)| *op != GlobalHelper::ClosedTailBegin.raw()),
                "an unselected scalar graph must never begin a capture loan"
            );
            assert_eq!(contents(&args), before);
            drop((args, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn escaping_public_tuple_result_keeps_ordinary_transport_and_reads() {
    let source = source(8)
        .replace("divisor<u64>) => <u64>", "divisor<u64>) => <(u64,bool)>")
        .replace(
            ":Done(total, seen) => (seen? | true => total | false => total + 1u64).",
            ":Done(total, seen) => (total, seen).",
        );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .all(|region| region.captures.is_none())
        );
        let read_sites = jit
            .descriptor()
            .metadata()
            .calls
            .iter()
            .enumerate()
            .filter_map(|(site, call)| {
                call.as_ref()
                    .is_some_and(|call| call.supports_unsigned_matrix_element())
                    .then_some(site)
            })
            .collect::<Vec<_>>();
        assert!(!read_sites.is_empty());
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 8, &[3, 0, 5], 3, 17, false, true, 1);
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut outputs, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(
                drafts(&outputs).last(),
                Some(&Data::Tuple([Data::U64(34), Data::Bool(true)].into()))
            );
            assert!(
                counts
                    .helpers
                    .keys()
                    .any(|(site, _)| read_sites.contains(&(*site as usize)))
            );
            assert_eq!(contents(&args), before);
            drop((args, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn signed_array_witness_stays_rejected_before_native_capture_selection() {
    let source = source(8).replace("u8", "i8");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .err()
        .expect("the signed read witness is outside the maintained native kernel interface");
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .err()
                .expect("capture selection must not broaden the signed read witness");
        assert!(format!("{ordinary:?}").contains("WitnessTypeMismatch"));
        assert_eq!(format!("{ordinary:?}"), format!("{native:?}"));
    }
}

#[test]
fn captured_native_identity_rejects_the_previous_uncaptured_ir_revision() {
    use crate::native_artifact::{NATIVE_PROGRAM_TYPED_IR_VERSION, NativeProgramObjectIdentity};
    for artifact in artifacts(&source(8)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| region.captures.is_some())
        );
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        assert!(NATIVE_PROGRAM_TYPED_IR_VERSION > 146);
        assert_eq!(
            u64::from_le_bytes(identity.bytes()[16..24].try_into().unwrap()),
            NATIVE_PROGRAM_TYPED_IR_VERSION
        );
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&146u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
    }
}

#[test]
fn captured_descriptor_rejects_changed_owner_width_and_factory_witness() {
    use crate::native_artifact::NativeProgramDescriptor;
    fn change_capture(value: &mut serde_json::Value, field: &str, replacement: u64) -> bool {
        match value {
            serde_json::Value::Object(fields) => {
                if let Some(captures) = fields
                    .get_mut("captures")
                    .and_then(serde_json::Value::as_array_mut)
                {
                    if let Some(capture) = captures
                        .first_mut()
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        if capture.contains_key("witness") && capture.contains_key(field) {
                            capture.insert(field.into(), serde_json::json!(replacement));
                            return true;
                        }
                    }
                }
                fields
                    .values_mut()
                    .any(|value| change_capture(value, field, replacement))
            }
            serde_json::Value::Array(values) => values
                .iter_mut()
                .any(|value| change_capture(value, field, replacement)),
            _ => false,
        }
    }
    for artifact in artifacts(&source(8)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        for (field, replacement) in [
            ("value", u32::MAX as u64),
            ("width", 3),
            ("witness", u32::MAX as u64),
        ] {
            let mut image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(change_capture(&mut image, field, replacement));
            let changed = serde_json::to_vec(&image).unwrap();
            // SAFETY: malformed build-owned descriptor diagnostic only. A
            // restored descriptor is never paired with code or executed here.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err(),
                "changed capture {field} must not bind a different array owner or factory"
            );
        }
    }
}
