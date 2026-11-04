//! Generic witness for the fixed-size mutable children used by header loops.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-mutable-children.mec");

#[test]
fn fixed_size_mutable_children_keep_current_owners_through_nested_calls() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let reasons = metadata
            .plan
            .iterations()
            .iter()
            .filter_map(|loop_| {
                let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = loop_ else {
                    return None;
                };
                tail.parent().is_none().then(|| {
                    crate::native_artifact::program::prepare::readonly_numeric::derive(
                        &metadata.plan,
                        tail,
                        &metadata.buffer_frames,
                        &metadata.scalar_aggregates,
                        &metadata.calls,
                        &metadata.visits,
                    )
                    .map(|_| ())
                    .err()
                })
            })
            .collect::<Vec<_>>();
        assert!(
            metadata.closed_tails.regions.iter().any(|region| {
                region.tail.parent().is_none()
                    && region.numeric.as_ref().is_some_and(|proof| {
                        proof.phase.as_ref().is_some_and(|phase| phase.loops.len() > 1)
                            && proof.actions.iter().any(|(site, action)| {
                                matches!(action, crate::native_artifact::program::prepare::numeric_plan::Operation::Write { .. })
                                    && metadata.plan.blocks()[*site].iteration() != Some(region.iteration)
                            })
                    })
            }),
            "a parent must include the child's actual stores and array result: {:?}",
            reasons,
        );
        let memory = mode.memory_budget();
        for (prime, restored) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut output = vec![None; artifact.outputs().len()];
            let mut prior = None;
            for reject in [u64::MAX, 2, u64::MAX] {
                let args = inputs(
                    &artifact,
                    &[
                        ("values", Data::Matrix([4, 8, 12].map(Data::U32).into())),
                        ("rounds", Data::U64(2)),
                        ("amount", Data::U32(1)),
                        ("prime", Data::Bool(prime)),
                        ("reject", Data::U64(reject)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let original = contents(&args);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if reject == u64::MAX {
                    result.unwrap();
                    let answer = output.last().unwrap().as_ref().unwrap();
                    assert_eq!(
                        answer.canonical_data_draft().unwrap(),
                        Data::Matrix([if prime { 8 } else { 6 }, 10, 14].map(Data::U32).into()),
                    );
                    assert_eq!(counts.phases, 1);
                    prior = Some(answer.clone());
                } else {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Integrity"), "{error}");
                    assert!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(prior.as_ref().unwrap())
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                for (actual, alias) in args.iter().zip(&aliases) {
                    assert!(actual.shares_frozen_storage(alias));
                    assert_eq!(
                        actual.canonical_data_draft().unwrap(),
                        alias.canonical_data_draft().unwrap()
                    );
                }
                assert_eq!(contents(&args), original);
            }
            drop((output, prior));
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn fixed_size_child_compound_guard_keeps_conservative_fallback() {
    let source = SOURCE.replace(
        "(more? | true => (prime? | true => mutable-store(values, 0u64, amount) | false => values) | false => values)",
        "((more && prime)? | true => mutable-store(values, 0u64, amount) | false => values)",
    );
    assert_ne!(source, SOURCE);
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            !jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| region.tail.parent().is_none()
                    && region
                        .numeric
                        .as_ref()
                        .is_some_and(|proof| proof.phase.is_some()))
        );
        let memory = mode.memory_budget();
        for prime in [false, true] {
            let args = inputs(
                &artifact,
                &[
                    ("values", Data::Matrix([4, 8, 12].map(Data::U32).into())),
                    ("rounds", Data::U64(2)),
                    ("amount", Data::U32(1)),
                    ("prime", Data::Bool(prime)),
                    ("reject", Data::U64(u64::MAX)),
                ],
                &memory,
            );
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([if prime { 8 } else { 6 }, 10, 14].map(Data::U32).into())
            );
            assert_eq!(contents(&args), before);
            drop((args, output));
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
