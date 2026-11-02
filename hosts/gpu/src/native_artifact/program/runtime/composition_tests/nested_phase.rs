//! Full scan witness: stable U32 input views, one U64 append owner, and
//! synchronous scalar children with their current arguments and failure order.
use super::*;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-nested-phase.mec");

fn sources() -> [String; 2] {
    let state_source = SOURCE
        .replacen(
            "+> sequence\n",
            concat!(
                "+> sequence\n",
                include_str!(
                    "../../../../../tests/fixtures/native/numeric-nested-phase-states.mec"
                ),
                "\n",
            ),
            1,
        )
        .replacen(
            "phase-candidates(values, 1u64, 0u64, permitted, divisor)",
            "#PhaseCandidates(values, permitted, divisor, 1u64, 0u64)",
            1,
        );
    [SOURCE.to_owned(), state_source]
}

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    flag: bool,
    allowed: bool,
    deny_at: u64,
    divisor: u64,
    bias: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", Data::Matrix([3, 5, 7].map(Data::U32).into())),
            ("nodes", Data::Matrix([Data::U64(9)].into())),
            ("steps", Data::U64(steps)),
            ("flag", Data::Bool(flag)),
            ("allowed", Data::Bool(allowed)),
            ("deny-at", Data::U64(deny_at)),
            ("divisor", Data::U64(divisor)),
            ("bias", Data::U64(bias)),
        ],
        memory,
    )
}

fn expected(steps: u64, mut flag: bool, divisor: u64, bias: u64) -> Data {
    let values = [3u64, 5, 7];
    let sum: u64 = (1..=values.len())
        .map(|end| values[..end].iter().sum::<u64>() / divisor)
        .sum();
    let mut nodes = vec![Data::U64(9)];
    for at in 0..steps {
        nodes.push(Data::U64(sum + at + if flag { 0 } else { 100 } + bias));
        flag = !flag;
    }
    Data::Matrix(nodes.into())
}

#[test]
fn nested_numeric_phase_keeps_aliases_current_arguments_and_failure_recovery() {
    for artifact in sources().iter().flat_map(|source| artifacts(source)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        qualify_nested_inventory(&jit);
        for restored in [false, true] {
            let mut published = vec![None; artifact.outputs().len()];
            for (steps, flag, allowed, deny_at, divisor, bias, rejection) in [
                (3, true, true, u64::MAX, 1, 0, None),
                (3, false, true, u64::MAX, 2, 0, None),
                (3, true, false, u64::MAX, 0, 0, Some("Integrity")),
                (3, true, true, 1, 1, 0, Some("Integrity")),
                (3, true, true, u64::MAX, 0, 0, Some("Arithmetic")),
                (
                    3,
                    true,
                    true,
                    u64::MAX,
                    1,
                    u64::MAX - 26,
                    Some("Arithmetic"),
                ),
                (0, false, false, 0, 0, u64::MAX, None),
                (1, false, true, u64::MAX, 1, 0, None),
                (3, true, true, u64::MAX, 1, 0, None),
            ] {
                let args = arguments(
                    &artifact, &memory, steps, flag, allowed, deny_at, divisor, bias,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (actual, old) in published.iter().zip(&prior) {
                        let (actual, old) = (actual.as_ref().unwrap(), old.as_ref().unwrap());
                        assert!(actual.shares_frozen_storage(old));
                        assert_eq!(
                            actual.canonical_data_draft().unwrap(),
                            old.canonical_data_draft().unwrap()
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    assert_eq!(counts.phases, 1);
                    assert!(counts.admits.keys().all(|&site| {
                        !jit.descriptor()
                            .metadata()
                            .closed_tails
                            .regions
                            .iter()
                            .filter_map(|r| r.numeric.as_ref().filter(|p| p.phase.is_some()))
                            .flat_map(|p| &p.sites)
                            .any(|&s| s as u64 == site)
                    }));
                    assert_eq!(
                        counts.helpers.values().sum::<u64>(),
                        if steps == 0 { 7 } else { 8 }
                    );
                    let expected = if steps == 0 {
                        Data::Matrix([Data::U64(9)].into())
                    } else {
                        expected(steps, flag, divisor, bias)
                    };
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected,
                    );
                    println!(
                        "{}",
                        serde_json::json!({"witness":"nested-numeric-phase", "restored":restored,
                        "positions":steps,"reserved_nodes":305257,"selected_pieces":counts.selected,
                        "admissions":counts.admits.values().sum::<u64>(),
                        "helpers":counts.helpers.values().sum::<u64>()})
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

fn qualify_nested_inventory(jit: &JitProgramArtifact) {
    use crate::native_artifact::program::prepare::numeric_phase::Forest;
    use mech_engine::native_artifact::NativeControlLoop;
    let plan = jit.descriptor().plan();
    let NativeControlLoop::Tail(root) = &plan.iterations()[0] else {
        panic!("nested phase must have a Tail root")
    };
    let forest = Forest::derive(plan, root).unwrap();
    let regions = &jit.descriptor().metadata().closed_tails.regions;
    assert_eq!(regions.len(), 1);
    let selected = regions[0].numeric.as_ref().expect("complete native phase");
    assert_eq!(selected.phase.as_ref().unwrap().loops.len(), 3);
    assert_eq!(
        selected
            .sites
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        forest.sites
    );
    assert!(jit.descriptor().metadata().compositions.parents.is_empty());
    let metadata = jit.descriptor().metadata();
    assert!(
        forest
            .validate_child_actions(
                plan,
                &metadata.buffer_frames,
                &metadata.scalar_aggregates,
                &metadata.calls,
            )
            .unwrap()
            > 0
    );
    let bounds = forest
        .bounded_children(
            plan,
            &metadata.buffer_frames,
            &metadata.scalar_aggregates,
            &metadata.calls,
        )
        .expect("bounded native children");
    assert_eq!(bounds.len(), 2);
    let schedule = crate::native_artifact::program::prepare::numeric_phase::Schedule::derive(
        &forest,
        bounds.clone(),
        &metadata.visits,
        &[],
        0,
        0,
        crate::native_artifact::program::prepare::tail::FIXED_OVERHEAD,
    )
    .unwrap();
    assert_eq!(schedule.loops.len(), 3);
    assert_ne!(
        schedule.compute_bound,
        crate::native_artifact::program::prepare::numeric_phase::Bound::Constant(0)
    );
    assert_eq!(
        schedule.comparison_bound,
        crate::native_artifact::program::prepare::numeric_phase::Bound::Constant(0)
    );
    for bound in &bounds {
        assert!(bound.guard > 0);
        assert!(bound.iteration > 0);
        assert!(matches!(
            bound.bound,
            crate::native_artifact::program::prepare::numeric_phase::Bound::Add(..)
        ));
    }
    assert_eq!(forest.loops.len(), 3);
    assert_eq!(forest.order.len(), forest.sites.len());
    assert_eq!(forest.order.first().copied(), Some(root.header() as usize));
    assert!(!forest.sites.contains(&(root.initialize() as usize)));
    assert!(!forest.sites.contains(&(root.exit() as usize)));
    assert_eq!(
        forest
            .loops
            .iter()
            .map(|loop_| loop_.sites.len())
            .sum::<usize>(),
        forest.sites.len(),
    );
    for loop_ in &forest.loops {
        let NativeControlLoop::Tail(tail) = &plan.iterations()[loop_.iteration as usize] else {
            panic!("non-Tail inventory")
        };
        assert_eq!(
            (
                loop_.header,
                loop_.latch,
                loop_.initialize,
                loop_.finish,
                loop_.exit,
                loop_.parent
            ),
            (
                tail.header() as usize,
                tail.latch() as usize,
                tail.initialize() as usize,
                tail.finish() as usize,
                tail.exit() as usize,
                tail.parent()
            ),
        );
        assert!(loop_.sites.iter().all(|&site| forest.sites.contains(&site)));
    }
}

#[test]
fn nested_numeric_phase_finite_limits_keep_publication_and_retry() {
    for source in sources() {
        resources::finite_boundaries(&source, |artifact, memory| {
            arguments(artifact, memory, 2, true, true, u64::MAX, 1, 0)
        });
    }
}

#[path = "nested_phase_controls.rs"]
mod controls;
#[path = "nested_phase_fallback.rs"]
mod fallback;

#[path = "nested_phase_stride.rs"]
mod stride;
