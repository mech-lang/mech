//! One native owner bank spans different FSM tags, with exact checked exits.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(kind: &str) -> String {
    r#"#Scan(initial<[K]:2,3>, start<u64>, end<u64>, row<u64>, marker<K>) => <[K]:2,3>
  | :Read(data<[K]:2,3>, at<u64>, carried<K>)
  | :Write(data<[K]:2,3>, at<u64>, carried<K>, old<K>)
  | :Done(data<[K]:2,3>).
#Scan(initial, start, end, row, marker) -> :Read(initial, start, marker)
  :Read(data, at, carried)
    | at < end -> :Write(data, at, carried, data[row,at])
    | * -> :Done(data)
  :Write(data, at, carried, old) -> {
      ~next := data
      next[row,at] = carried
    } -> :Read(next, at + 1u64, old)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Scan(data,start<u64>,end<u64>,row<u64>,marker<K>)
(answer,data)
"#
    .replace('K', kind)
}

#[test]
fn prefix_numeric_phases_span_alternating_tags_keep_bits_aliases_and_source_errors() {
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for artifact in artifacts(&source(kind)) {
            let catalog = mech_stdlib::source_catalog();
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let metadata = jit.descriptor().metadata();
            let region = metadata.closed_tails.regions.iter().position(|r| {
                r.numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some())
            });
            if region.is_none() {
                for control in metadata.plan.iterations() {
                    if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
                        let reason =
                            crate::native_artifact::program::prepare::numeric_prefix::derive(
                                &metadata.plan.facts(),
                                tail,
                                &metadata.buffer_frames,
                                &metadata.scalar_aggregates,
                                &metadata.calls,
                                &metadata.visits,
                            )
                            .err();
                        eprintln!(
                            "{kind} prefix candidate: {reason:?}; carrier {} {:?}; fields {:?}",
                            tail.carrier().get(),
                            metadata.plan.values()[tail.carrier().get() as usize]
                                .definition()
                                .body(),
                            metadata.buffer_frames.fields(tail.carrier())
                        );
                        if let Some([root]) = metadata.buffer_frames.fields(tail.carrier()) {
                            if let mech_engine::native_artifact::NativeBufferFrameLane::Tagged {
                                count,
                                ..
                            } = root.lane()
                            {
                                for ordinal in 0..count {
                                    let field = metadata
                                        .buffer_frames
                                        .tagged_variant(*root, ordinal as u32)
                                        .unwrap();
                                    eprintln!(
                                        "variant {ordinal}: {field:?} children {:?}",
                                        metadata.buffer_frames.children(field)
                                    );
                                }
                            }
                        }
                    }
                }
            }
            let region = region.expect("actual complete common-prefix native phase");
            let proof = metadata.closed_tails.regions[region]
                .numeric
                .as_ref()
                .unwrap();
            assert!(proof.exits.is_empty());
            assert_eq!(
                proof
                    .tagged
                    .as_ref()
                    .unwrap()
                    .prefix
                    .as_ref()
                    .unwrap()
                    .counts,
                [3, 4, 1]
            );
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            for restored in [false, true] {
                for (start, end, row, error) in [
                    (1, 4, 1, false),
                    (1, 4, 2, false),
                    (1, 1, 0, false),
                    (0, 2, 1, true),
                    (1, 5, 1, true),
                    (1, 3, 3, true),
                    (1, 3, 1, false),
                ] {
                    let arguments = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("marker", marker.clone(), &[]),
                            ("start", Data::U64(start), &[]),
                            ("end", Data::U64(end), &[]),
                            ("row", Data::U64(row), &[]),
                        ],
                    );
                    let aliases = arguments.clone();
                    let before = contents(&arguments);
                    let old = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                    let (a, observed) = if restored {
                        observe::run_restored(
                            &jit,
                            &arguments,
                            &mut fast,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &arguments, &mut fast, mode.execution(), &memory)
                    };
                    let (b, _) =
                        observe::run(&ordinary, &arguments, &mut cold, mode.execution(), &memory);
                    assert_eq!(a.is_err(), error, "{kind} {start}..{end} row {row}: {a:?}");
                    assert_eq!(a.is_err(), b.is_err());
                    if error {
                        assert_eq!(
                            format!("{:?}", a.as_ref().err()),
                            format!("{:?}", b.as_ref().err()),
                            "exact checked source error"
                        );
                    }
                    assert_eq!(contents(&arguments), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                        "{kind} {start}..{end} row {row}: {observed:?}"
                    );
                    if error {
                        assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            old
                        );
                    } else {
                        assert_eq!(
                            observed
                                .tail_entries
                                .iter()
                                .filter(|(_, id, _)| *id == region)
                                .count(),
                            1,
                            "one loan across every tag {kind}: {observed:?}"
                        );
                        for (site, op) in &proof.actions {
                            if matches!(op,crate::native_artifact::program::prepare::readonly_numeric::Operation::Frame
                                |crate::native_artifact::program::prepare::readonly_numeric::Operation::CellRead{..}
                                |crate::native_artifact::program::prepare::readonly_numeric::Operation::CellWrite{..}) {
                                assert_eq!(observed.helpers.get(&(*site as u64,GlobalHelper::Library.raw())).copied().unwrap_or(0),0,"native tag and cell operations {kind} site {site}");
                            }
                        }
                    }
                }
            }
        }
    }
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    marker: Data,
    start: u64,
    end: u64,
    row: u64,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("input", Data::Matrix(cells.to_vec().into()), &[2, 3]),
            ("marker", marker, &[]),
            ("start", Data::U64(start), &[]),
            ("end", Data::U64(end), &[]),
            ("row", Data::U64(row), &[]),
        ],
    )
}

#[test]
fn prefix_numeric_phases_keep_finite_limits_atomic_publication_cleanup_and_retry() {
    let (_, cells, marker) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&source("f64")) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r
                    .numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some()))
        );
        let args = arguments(&artifact, &memory, &cells, marker.clone(), 1, 4, 1);
        let before = contents(&args);
        let mut oracle = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&args, &mut oracle, mode.execution(), &memory)
            .unwrap();
        let expected = contents(&oracle.iter().flatten().cloned().collect::<Vec<_>>());
        let prior_args = arguments(&artifact, &memory, &cells, marker.clone(), 1, 2, 2);
        let mut prior = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&prior_args, &mut prior, mode.execution(), &memory)
            .unwrap();
        drop(prior_args);
        let previous = contents(&prior.iter().flatten().cloned().collect::<Vec<_>>());
        assert_ne!(
            previous, expected,
            "atomic publication oracle must be distinct"
        );
        for restored in [false, true] {
            for dimension in 0..9 {
                for cap in [0, 1, 3, 16, 255, 4095, 65535] {
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
                        _ => policy.execution.max_transport_bytes = cap,
                    }
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = prior.clone();
                    let mut cold = prior.clone();
                    let a = if restored {
                        observe::run_restored(&jit, &args, &mut fast, policy, &memory).0
                    } else {
                        observe::run(&jit, &args, &mut fast, policy, &memory).0
                    };
                    let b = if restored {
                        observe::run_restored(&ordinary, &args, &mut cold, policy, &memory).0
                    } else {
                        observe::run(&ordinary, &args, &mut cold, policy, &memory).0
                    };
                    if dimension < 2 {
                        assert_eq!(
                            a.is_ok(),
                            b.is_ok(),
                            "logical constructor limit {dimension} {cap}: {a:?} / {b:?}"
                        );
                    }
                    // Optimized work/temporary storage can be smaller. Every
                    // successful result still obeys the full semantic oracle.
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        if a.is_ok() {
                            expected.clone()
                        } else {
                            previous.clone()
                        },
                        "success or atomic prior output {dimension} {cap}: {a:?}"
                    );
                    assert_eq!(
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                        if b.is_ok() {
                            expected.clone()
                        } else {
                            previous.clone()
                        },
                        "ordinary success or atomic prior output {dimension} {cap}: {b:?}"
                    );
                    drop((fast, cold));
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained,
                        "release every failed phase {dimension} {cap}"
                    );
                }
            }
            let mut out = vec![None; artifact.outputs().len()];
            let result = if restored {
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory).0
            } else {
                observe::run(&jit, &args, &mut out, mode.execution(), &memory).0
            };
            result.unwrap();
            assert_eq!(
                contents(&out.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
        }
        drop((args, oracle, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn prefix_numeric_phases_reject_forged_tag_payload_owner_and_funding_metadata() {
    for artifact in artifacts(&source("u64")) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        for change in 0..6 {
            let mut forged = original.clone();
            let numeric = forged["descriptor"]["closed_tails"]["regions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find_map(|r| {
                    let numeric = &mut r["numeric"];
                    (!numeric["tagged"]["prefix"].is_null()).then_some(numeric)
                })
                .expect("prefix proof in admitted image");
            match change {
                0 => numeric["tagged"]["prefix"]["counts"][0] = 99.into(),
                1 => numeric["tagged"]["prefix"]["tags"][0][1] = 0.into(),
                2 => numeric["tagged"]["prefix"]["variants"][0]["ordinal"] = 99.into(),
                3 => numeric["tagged"]["prefix"]["variants"][0]["arrays"][0][0] = 999.into(),
                4 => numeric["arrays"][0]["storage"]["TaggedCarrier"]["field"] = 99.into(),
                _ => numeric["phase"]["loops"][0]["compute"] = 0.into(),
            }
            let bytes = serde_json::to_vec(&forged).unwrap();
            // SAFETY: diagnostic decode only; no forged descriptor is ever
            // linked to a native entry, even if this assertion should fail.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "forged prefix {change}"
            );
        }
    }
}

#[test]
fn prefix_numeric_phases_decline_older_versions_and_foreign_transition_roots() {
    let older = source("u64")
        .replace(
            "next[row,at] = carried",
            "next[row,at] = carried\n      previous := data[row,at]",
        )
        .replace(
            ":Read(next, at + 1u64, old)",
            ":Read(next, at + 1u64, previous)",
        );
    let foreign = source("u64")
        .replace("marker<u64>) =>", "marker<u64>, other<[u64]:2,3>) =>")
        .replace(
            "#Scan(initial, start, end, row, marker) ->",
            "#Scan(initial, start, end, row, marker, other) ->",
        )
        .replace(
            ":Read(next, at + 1u64, old)",
            ":Read(other, at + 1u64, old)",
        )
        .replace(
            "#Scan(data,start<u64>,end<u64>,row<u64>,marker<u64>)",
            "#Scan(data,start<u64>,end<u64>,row<u64>,marker<u64>,other<[u64]:2,3>)",
        );
    for (text, extra) in [(older, false), (foreign, true)] {
        for artifact in artifacts(&text) {
            let catalog = mech_stdlib::source_catalog();
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .all(|r| r
                        .numeric
                        .as_ref()
                        .and_then(|p| p.tagged.as_ref())
                        .is_none_or(|v| v.prefix.is_none())),
                "distinct functional versions cannot share one native owner"
            );
            let cells = [3, 7, 11, 13, 17, 19].map(Data::U64);
            let mut fields = vec![
                ("input", Data::Matrix(cells.to_vec().into()), &[2u64, 3][..]),
                ("marker", Data::U64(23), &[][..]),
                ("start", Data::U64(1), &[][..]),
                ("end", Data::U64(4), &[][..]),
                ("row", Data::U64(1), &[][..]),
            ];
            if extra {
                fields.push((
                    "other",
                    Data::Matrix([29, 31, 37, 41, 43, 47].map(Data::U64).into()),
                    &[2, 3],
                ));
            }
            let args = super::numeric_maps::typed_arguments(&artifact, &memory, &fields);
            let aliases = args.clone();
            let before = contents(&args);
            for restored in [false, true] {
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                let a = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory).0
                };
                a.unwrap();
                ordinary
                    .evaluate_into(&args, &mut cold, mode.execution(), &memory)
                    .unwrap();
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
        }
    }
}

#[test]
fn prefix_numeric_phases_refusal_stays_sticky_across_all_state_transitions() {
    for artifact in artifacts(&source("u64")) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r
                    .numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some()))
        );
        let cells = [3, 7, 11, 13, 17, 19].map(Data::U64);
        let args = arguments(&artifact, &memory, &cells, Data::U64(23), 1, 4, 1);
        let before = contents(&args);
        for restored in [false, true] {
            for _ in 0..2 {
                let mut out = vec![None; artifact.outputs().len()];
                // Reconstruct the independently checked descriptor before
                // applying a runtime-only admission refusal to this entry.
                let descriptor =
                    restored.then(|| super::super::tests::restore_admitted_helper_descriptor(&jit));
                let (result, observed) = observe::without_numeric_phase(|| {
                    if let Some(descriptor) = &descriptor {
                        observe::run_descriptor(
                            &jit,
                            descriptor,
                            &args,
                            &mut out,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &args, &mut out, mode.execution(), &memory)
                    }
                });
                result.unwrap();
                assert!(observed.tail_entries.is_empty());
                assert_eq!(
                    observed
                        .helpers
                        .iter()
                        .filter(|((_, op), _)| *op == GlobalHelper::ClosedTailBegin.raw())
                        .map(|(_, n)| n)
                        .sum::<u64>(),
                    1,
                    "one refused preview per fresh Tail: {observed:?}"
                );
                assert_eq!(
                    out.last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Matrix([23, 3, 7, 13, 17, 19].map(Data::U64).into()),
                            Data::Matrix(cells.clone().into())
                        ]
                        .into()
                    )
                );
                assert_eq!(contents(&args), before);
            }
        }
    }
}

#[test]
fn prefix_numeric_phases_restore_owners_after_checked_arithmetic_failure_and_retry() {
    let text = source("u64")
        .replace("marker<u64>) =>", "marker<u64>, jump<u64>) =>")
        .replace(
            "#Scan(initial, start, end, row, marker) ->",
            "#Scan(initial, start, end, row, marker, jump) ->",
        )
        .replace("at + 1u64", "at + jump")
        .replace(
            "#Scan(data,start<u64>,end<u64>,row<u64>,marker<u64>)",
            "#Scan(data,start<u64>,end<u64>,row<u64>,marker<u64>,jump<u64>)",
        );
    for artifact in artifacts(&text) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r
                    .numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some()))
        );
        let make = |jump| {
            super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "input",
                        Data::Matrix([3, 7, 11, 13, 17, 19].map(Data::U64).into()),
                        &[2, 3],
                    ),
                    ("marker", Data::U64(23), &[]),
                    ("start", Data::U64(1), &[]),
                    ("end", Data::U64(4), &[]),
                    ("row", Data::U64(1), &[]),
                    ("jump", Data::U64(jump), &[]),
                ],
            )
        };
        let good = make(1);
        let bad = make(u64::MAX);
        let before = contents(&bad);
        let mut prior = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&good, &mut prior, mode.execution(), &memory)
            .unwrap();
        let expected = contents(&prior.iter().flatten().cloned().collect::<Vec<_>>());
        for restored in [false, true] {
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut out = prior.clone();
            let mut cold = prior.clone();
            let (a, observed) = if restored {
                observe::run_restored(&jit, &bad, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &bad, &mut out, mode.execution(), &memory)
            };
            let b = ordinary.evaluate_into(&bad, &mut cold, mode.execution(), &memory);
            assert!(
                a.is_err() && b.is_err(),
                "checked overflow after native change: {a:?} / {b:?}"
            );
            assert_eq!(
                format!("{:?}", a.err()),
                format!("{:?}", b.err()),
                "exact source arithmetic failure"
            );
            assert_eq!(
                observed.first_change_sites.len(),
                1,
                "a private write preceded the overflow"
            );
            assert_eq!(
                contents(&out.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
            assert_eq!(
                contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
            assert_eq!(contents(&bad), before);
            drop((out, cold));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            let mut out = vec![None; artifact.outputs().len()];
            let result = if restored {
                observe::run_restored(&jit, &good, &mut out, mode.execution(), &memory).0
            } else {
                observe::run(&jit, &good, &mut out, mode.execution(), &memory).0
            };
            result.unwrap();
            assert_eq!(
                contents(&out.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
        }
        drop((good, bad, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn prefix_numeric_phases_close_fixed_array_results_without_losing_recurrent_numeric_work() {
    let text = r#"#Scan(initial<[u64]:2,3>, end<u64>, marker<u64>) => <[u64]:2,3>
  | :Walk(data<[u64]:2,3>, at<u64>, carried<u64>)
  | :Done(data<[u64]:2,3>).
#Scan(initial,end,marker) -> :Walk(initial,1u64,marker)
  :Walk(data,at,carried)
    | at < end -> {
        old := data[1u64,at]
        ~next := data
        next[1u64,at] = carried
      } -> :Walk(next,at + 1u64,old)
    | * -> :Done(data)
  :Done(data) => data.
#Scan(input<[u64]:2,3>,end<u64>,marker<u64>)
"#;
    for artifact in artifacts(text) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let metadata = jit.descriptor().metadata();
        let region = metadata
            .closed_tails
            .regions
            .iter()
            .find(|r| r.numeric.as_ref().is_some_and(|p| p.tagged.is_some()))
            .unwrap();
        assert!(
            region
                .numeric
                .as_ref()
                .unwrap()
                .tagged
                .as_ref()
                .unwrap()
                .prefix
                .is_some(),
            "close the exact array result at unchanged recurring numeric coverage"
        );
        crate::native_artifact::program::prepare::numeric_prefix::derive(
            &metadata.plan.facts(),
            &region.tail,
            &metadata.buffer_frames,
            &metadata.scalar_aggregates,
            &metadata.calls,
            &metadata.visits,
        )
        .expect("complete prefix is independently eligible");
        assert!(region.numeric.as_ref().unwrap().exits.is_empty());
        assert!(
            region
                .numeric
                .as_ref()
                .unwrap()
                .result
                .direct_array()
                .is_some()
        );
        let narrow = crate::native_artifact::program::prepare::readonly_numeric::tagged::candidate(
            &metadata.plan.facts(),
            &region.tail,
            &metadata.buffer_frames,
            &metadata.scalar_aggregates,
            &metadata.calls,
            &metadata.visits,
            0,
        )
        .expect("the constant-tag specialization remains available");
        assert!(!narrow.exits.is_empty());
        let numeric =
            |proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof| {
                proof.actions.iter().filter(|(_, op)| matches!(op,
                crate::native_artifact::program::prepare::readonly_numeric::Operation::CellRead { .. }
                | crate::native_artifact::program::prepare::readonly_numeric::Operation::CellWrite { .. }
            )).count()
            };
        assert!(numeric(&narrow) > 0);
        assert_eq!(numeric(&narrow), numeric(region.numeric.as_ref().unwrap()));
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix([3, 7, 11, 13, 17, 19].map(Data::U64).into()),
                    &[2, 3],
                ),
                ("end", Data::U64(4), &[]),
                ("marker", Data::U64(23), &[]),
            ],
        );
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut out, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(counts.tail_entries.len(), 1);
            for &site in &region.numeric.as_ref().unwrap().sites {
                for helper in [GlobalHelper::Library, GlobalHelper::StructuralObservation] {
                    assert_eq!(
                        counts
                            .helpers
                            .get(&(site as u64, helper.raw()))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "complete native constructor/extraction at source{site}"
                    );
                }
            }
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([23, 3, 7, 13, 17, 19].map(Data::U64).into())
            );
        }
    }
}

#[test]
fn prefix_numeric_phases_cover_recurring_scalar_transition_states() {
    let text = r#"#Scan(initial<[u64]:2,3>, end<u64>, marker<u64>) => <[u64]:2,3>
  | :Walk(data<[u64]:2,3>, at<u64>, carried<u64>)
  | :Advance(data<[u64]:2,3>, at<u64>, carried<u64>)
  | :Done(data<[u64]:2,3>).
#Scan(initial,end,marker) -> :Walk(initial,1u64,marker)
  :Walk(data,at,carried)
    | at >= end -> :Done(data)
    | at % 2u64 == 0u64 -> :Advance(data,at,carried)
    | * -> {
        old := data[1u64,at]
        ~next := data
        next[1u64,at] = carried
      } -> :Walk(next,at + 1u64,old)
  :Advance(data,at,carried) -> :Walk(data,at + 1u64,carried)
  :Done(data) => data.
#Scan(input<[u64]:2,3>,end<u64>,marker<u64>)
"#;
    for artifact in artifacts(text) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|r| {
                r.numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some())
            })
            .expect("one complete loan includes recurring scalar-only states");
        let metadata = jit.descriptor().metadata();
        let narrow = crate::native_artifact::program::prepare::readonly_numeric::tagged::candidate(
            &metadata.plan.facts(),
            &region.tail,
            &metadata.buffer_frames,
            &metadata.scalar_aggregates,
            &metadata.calls,
            &metadata.visits,
            0,
        )
        .expect("the narrow recurring numeric state is independently eligible");
        let numeric =
            |proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof| {
                proof.actions.iter().filter(|(_,operation)|matches!(operation,crate::native_artifact::program::prepare::readonly_numeric::Operation::CellRead{..}|crate::native_artifact::program::prepare::readonly_numeric::Operation::CellWrite{..})).count()
            };
        assert_eq!(
            numeric(&narrow),
            numeric(region.numeric.as_ref().unwrap()),
            "additional recurring scalar state, with identical numeric operation coverage"
        );
        assert!(numeric(&narrow) > 0);
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix([3, 7, 11, 13, 17, 19].map(Data::U64).into()),
                    &[2, 3],
                ),
                ("end", Data::U64(4), &[]),
                ("marker", Data::U64(23), &[]),
            ],
        );
        let before = contents(&args);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let mut reference = out.clone();
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut out, mode.execution(), &memory)
            };
            result.unwrap();
            ordinary
                .evaluate_into(&args, &mut reference, mode.execution(), &memory)
                .unwrap();
            assert_eq!(counts.tail_entries.len(), 1);
            assert_eq!(
                contents(&out.iter().flatten().cloned().collect::<Vec<_>>()),
                contents(&reference.iter().flatten().cloned().collect::<Vec<_>>())
            );
            for (site, operation) in &region.numeric.as_ref().unwrap().actions {
                if matches!(
                    operation,
                    crate::native_artifact::program::prepare::readonly_numeric::Operation::Frame
                ) {
                    assert_eq!(
                        counts
                            .helpers
                            .get(&(*site as u64, GlobalHelper::StructuralObservation.raw()))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "scalar transition payloads remain native"
                    );
                }
            }
            assert_eq!(contents(&args), before);
        }
    }
}

#[test]
fn prefix_numeric_phases_close_captured_array_children_inside_block_comprehensions() {
    let text = r#"#Scan(source<[f64]:3,1>, a<f64>, end<u64>) => <[f64]:3,1>
  | :Walk(values<[f64]:3,1>, at<u64>, prior<f64>)
  | :Done(values<[f64]:3,1>).
#Scan(source,a,end) -> :Walk([0.0 | j <- 1u64..=3u64]',1u64,0.0)
  :Walk(values,at,prior)
    | at < end -> {
        value := a * source[at] + prior
        ~next := values
        next[at] = value
      } -> :Walk(next,at + 1u64,value)
    | * -> :Done(values)
  :Done(values) => values.
image := input<[f64]:2,3>
answer := [(#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64>,end<u64>)) | row <- 1u64..=2u64]'
(answer,image)
"#;
    for artifact in artifacts(text) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let region = metadata
            .closed_tails
            .regions
            .iter()
            .find(|r| {
                r.numeric
                    .as_ref()
                    .and_then(|p| p.tagged.as_ref())
                    .is_some_and(|v| v.prefix.is_some())
            })
            .expect("complete child kernel under the original collection");
        let proof = region.numeric.as_ref().unwrap();
        assert!(region.tail.parent().is_some());
        assert!(proof.exits.is_empty() && proof.result.direct_array().is_some());
        assert!(proof.arrays.iter().any(|a| a.immutable_capture()));
        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        for (cells, a) in [
            (vec![1., 2., 3., 4., 5., 6.], 0.25),
            (vec![-0., 0., f64::MAX, -f64::MAX, f64::INFINITY, 1.], 2.),
            (
                vec![f64::from_bits(0x7ff8000000000042), 1., -1., 2., 3., 4.],
                1.,
            ),
        ] {
            let mut prior = vec![None; artifact.outputs().len()];
            for (end, error) in [(4, false), (1, false), (5, true), (3, false)] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                cells
                                    .iter()
                                    .map(|v| Data::F64(mech_core::snapshot::F64Bits::from_f64(*v)))
                                    .collect(),
                            ),
                            &[2, 3],
                        ),
                        (
                            "a",
                            Data::F64(mech_core::snapshot::F64Bits::from_f64(a)),
                            &[],
                        ),
                        ("end", Data::U64(end), &[]),
                    ],
                );
                let before = contents(&args);
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = prior.clone();
                    let mut expected = prior.clone();
                    let (result, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut actual, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut actual, mode.execution(), &memory)
                    };
                    let oracle =
                        ordinary.evaluate_into(&args, &mut expected, mode.execution(), &memory);
                    assert_eq!(result.is_err(), error, "end{end}: {result:?}");
                    assert_eq!(
                        format!("{:?}", result.as_ref().err()),
                        format!("{:?}", oracle.as_ref().err())
                    );
                    assert_eq!(
                        contents(&actual.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&expected.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                    if !error {
                        assert_eq!(
                            counts
                                .tail_entries
                                .iter()
                                .filter(|(_, id, _)| metadata.closed_tails.regions[*id].iteration
                                    == region.iteration)
                                .count(),
                            2
                        );
                        for &site in &proof.sites {
                            for helper in
                                [GlobalHelper::Library, GlobalHelper::StructuralObservation]
                            {
                                assert_eq!(
                                    counts
                                        .helpers
                                        .get(&(site as u64, helper.raw()))
                                        .copied()
                                        .unwrap_or(0),
                                    0,
                                    "native child terminal site{site}"
                                );
                            }
                        }
                    } else {
                        for (a, p) in actual.iter().zip(&prior) {
                            assert!(
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(p.as_ref().unwrap())
                            );
                        }
                    }
                    assert_eq!(contents(&args), before);
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
                if !error {
                    ordinary
                        .evaluate_into(&args, &mut prior, mode.execution(), &memory)
                        .unwrap();
                }
            }
            drop(prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
